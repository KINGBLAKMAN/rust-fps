//! Co-op networking over plain UDP.
//!
//! The host runs the whole simulation. Clients send their position, aim and
//! the shots they fire; the host sends back a snapshot of every player, enemy
//! and projectile about 30 times a second. Clients move their own character
//! locally, so movement feels instant even with some lag.

use bevy::prelude::*;
use bincode::Options;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket};

use crate::level::{spawn_replicated, GameAssets};
use crate::player::LocalPlayer;
use crate::sim::TracerEvents;
use crate::{
    EnemyLook, MatchState, NetKind, Phase, PlayerInfo, Replicated, Role, Roster, Session,
    ShotQueue, Tracers, EYE_HEIGHT, MAX_PLAYERS,
};

pub const DEFAULT_PORT: u16 = 7777;
/// Bump when the message format changes so old builds can't join.
const PROTOCOL_VERSION: u32 = 1;
const SNAPSHOT_INTERVAL: f32 = 1.0 / 30.0;
const TIMEOUT_SECS: f64 = 5.0;
const MAX_PACKET: u64 = 60_000;
const MAX_NAME_LEN: usize = 16;

#[derive(Serialize, Deserialize)]
enum ClientMsg {
    Hello {
        version: u32,
        name: String,
    },
    Input {
        pos: [f32; 3],
        yaw: f32,
        pitch: f32,
        shots: Vec<([f32; 3], [f32; 3])>,
    },
    Bye,
}

#[derive(Serialize, Deserialize)]
enum ServerMsg {
    Welcome { id: u8 },
    Reject { reason: String },
    Snapshot(Snapshot),
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
    tick: u32,
    wave: u32,
    game_over: bool,
    started: bool,
    players: Vec<PlayerInfo>,
    entities: Vec<NetEntity>,
    tracers: Vec<(u8, [f32; 3], [f32; 3])>,
}

#[derive(Serialize, Deserialize)]
struct NetEntity {
    id: u32,
    kind: NetKind,
    pos: [f32; 3],
    yaw: f32,
    flash: bool,
}

/// Size-limited encoding so a bad packet can't make us allocate gigabytes.
fn codec() -> impl Options {
    bincode::DefaultOptions::new().with_limit(MAX_PACKET)
}

fn encode<T: Serialize>(msg: &T) -> Vec<u8> {
    codec().serialize(msg).expect("message encodes")
}

fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Option<T> {
    codec().deserialize(bytes).ok()
}

struct Conn {
    id: u8,
    last_heard: f64,
}

#[derive(Resource)]
pub struct Net {
    socket: UdpSocket,
    /// Client only: the host's address.
    server: Option<SocketAddr>,
    /// Host only: connected players by address.
    clients: HashMap<SocketAddr, Conn>,
    last_heard: f64,
    timer: f32,
    tick: u32,
    last_tick: u32,
}

impl Net {
    fn new(socket: UdpSocket, server: Option<SocketAddr>) -> std::io::Result<Self> {
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            server,
            clients: HashMap::new(),
            last_heard: 0.0,
            timer: 0.0,
            tick: 0,
            last_tick: 0,
        })
    }

    fn send_to<T: Serialize>(&self, msg: &T, addr: SocketAddr) {
        // UDP is fire-and-forget; a dropped packet is replaced by the next one.
        let _ = self.socket.send_to(&encode(msg), addr);
    }

    /// Reads every waiting packet. Errors other than "nothing waiting" are
    /// skipped (Windows reports a reset when a peer disappears).
    fn drain(&self) -> Vec<(Vec<u8>, SocketAddr)> {
        let mut out = Vec::new();
        let mut buf = vec![0u8; 65_536];
        for _ in 0..1000 {
            match self.socket.recv_from(&mut buf) {
                Ok((n, addr)) => out.push((buf[..n].to_vec(), addr)),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(_) => continue,
            }
        }
        out
    }
}

/// Parses the command line into a session and, for host/join, a socket.
pub fn from_args() -> Result<(Session, Option<Net>), String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut name = None;
    if let Some(i) = args.iter().position(|a| a == "--name") {
        if i + 1 >= args.len() {
            return Err("--name needs a value".into());
        }
        name = Some(args.remove(i + 1));
        args.remove(i);
    }
    let name: String = name
        .unwrap_or_else(|| "Player".into())
        .chars()
        .take(MAX_NAME_LEN)
        .collect();

    let session = |role, status: String| Session {
        role,
        my_id: 0,
        my_name: name.clone(),
        status,
        connected: role != Role::Client,
    };

    match args.first().map(String::as_str) {
        None | Some("solo") => Ok((session(Role::Solo, String::new()), None)),
        Some("host") => {
            let port = match args.get(1) {
                Some(p) => p.parse().map_err(|_| format!("bad port: {p}"))?,
                None => DEFAULT_PORT,
            };
            let socket = UdpSocket::bind(("0.0.0.0", port))
                .map_err(|e| format!("can't listen on port {port}: {e}"))?;
            let ip = lan_ip().map(|ip| ip.to_string()).unwrap_or_else(|| "<your IP>".into());
            let status = format!("Hosting - friends join with: join {ip}:{port}");
            println!("{status}");
            let net = Net::new(socket, None).map_err(|e| e.to_string())?;
            Ok((session(Role::Host, status), Some(net)))
        }
        Some("join") => {
            let target = args.get(1).ok_or("join needs an address, e.g. join 192.168.1.20")?;
            let with_port = if target.contains(':') {
                target.clone()
            } else {
                format!("{target}:{DEFAULT_PORT}")
            };
            let addr = with_port
                .to_socket_addrs()
                .map_err(|e| format!("can't resolve {with_port}: {e}"))?
                .find(SocketAddr::is_ipv4)
                .ok_or_else(|| format!("no IPv4 address for {with_port}"))?;
            let socket =
                UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("can't open socket: {e}"))?;
            let net = Net::new(socket, Some(addr)).map_err(|e| e.to_string())?;
            Ok((session(Role::Client, format!("Connecting to {addr}...")), Some(net)))
        }
        Some(other) => Err(format!("unknown command: {other}")),
    }
}

/// Best guess at this machine's LAN address (no packets are actually sent).
fn lan_ip() -> Option<IpAddr> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    Some(s.local_addr().ok()?.ip())
}

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Puppets>()
            .add_systems(Update, host_receive.in_set(Phase::NetIn).run_if(is_host))
            .add_systems(Update, host_send.in_set(Phase::NetOut).run_if(is_host))
            .add_systems(Update, client_receive.in_set(Phase::NetIn).run_if(is_client))
            .add_systems(Update, client_send.in_set(Phase::NetOut).run_if(is_client))
            .add_systems(
                Update,
                move_puppets.in_set(Phase::Present).run_if(is_client),
            )
            .add_systems(Last, say_goodbye.run_if(is_client));
    }
}

fn is_host(session: Res<Session>) -> bool {
    session.role == Role::Host
}

fn is_client(session: Res<Session>) -> bool {
    session.role == Role::Client
}

// ---------------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------------

fn host_receive(
    time: Res<Time>,
    mut net: ResMut<Net>,
    mut roster: ResMut<Roster>,
    mut shots: ResMut<ShotQueue>,
) {
    let now = time.elapsed_secs_f64();
    for (bytes, addr) in net.drain() {
        let Some(msg) = decode::<ClientMsg>(&bytes) else {
            continue;
        };
        match msg {
            ClientMsg::Hello { version, name } => {
                if version != PROTOCOL_VERSION {
                    let reason = "Version mismatch - make sure everyone runs the same build".into();
                    net.send_to(&ServerMsg::Reject { reason }, addr);
                    continue;
                }
                if let Some(conn) = net.clients.get(&addr) {
                    // Our Welcome was probably lost; send it again.
                    net.send_to(&ServerMsg::Welcome { id: conn.id }, addr);
                    continue;
                }
                if roster.0.len() >= MAX_PLAYERS {
                    let reason = "The game is full".into();
                    net.send_to(&ServerMsg::Reject { reason }, addr);
                    continue;
                }
                let id = (1..=u8::MAX).find(|id| !roster.0.contains_key(id)).unwrap();
                let name: String = name.chars().take(MAX_NAME_LEN).collect();
                info!("{name} joined from {addr} as player {id}");
                roster.0.insert(id, PlayerInfo::new(id, name));
                net.clients.insert(addr, Conn { id, last_heard: now });
                net.send_to(&ServerMsg::Welcome { id }, addr);
            }
            ClientMsg::Input {
                pos,
                yaw,
                pitch,
                shots: fired,
            } => {
                let Some(conn) = net.clients.get_mut(&addr) else {
                    continue;
                };
                conn.last_heard = now;
                let id = conn.id;
                let Some(p) = roster.0.get_mut(&id) else {
                    continue;
                };
                if pos.iter().chain([yaw, pitch].iter()).any(|v| !v.is_finite()) {
                    continue;
                }
                p.pos = pos;
                p.yaw = yaw;
                p.pitch = pitch;
                if p.alive {
                    // Cap per packet so a client can't fire faster than the gun allows.
                    for (origin, dir) in fired.into_iter().take(4) {
                        let (o, d) = (Vec3::from_array(origin), Vec3::from_array(dir));
                        if o.is_finite() && d.is_finite() {
                            shots.0.push((id, o, d));
                        }
                    }
                }
            }
            ClientMsg::Bye => {
                if let Some(conn) = net.clients.remove(&addr) {
                    info!("player {} left", conn.id);
                    roster.0.remove(&conn.id);
                }
            }
        }
    }

    // Drop players we haven't heard from in a while.
    let stale: Vec<SocketAddr> = net
        .clients
        .iter()
        .filter(|(_, c)| now - c.last_heard > TIMEOUT_SECS)
        .map(|(a, _)| *a)
        .collect();
    for addr in stale {
        if let Some(conn) = net.clients.remove(&addr) {
            info!("player {} timed out", conn.id);
            roster.0.remove(&conn.id);
        }
    }
}

fn host_send(
    time: Res<Time>,
    mut net: ResMut<Net>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    mut events: ResMut<TracerEvents>,
    things: Query<(&Replicated, &Transform, Option<&EnemyLook>)>,
) {
    net.timer += time.delta_secs();
    if net.timer < SNAPSHOT_INTERVAL {
        return;
    }
    net.timer = 0.0;
    let tracers = std::mem::take(&mut events.0);
    if net.clients.is_empty() {
        return;
    }
    net.tick += 1;
    let snapshot = ServerMsg::Snapshot(Snapshot {
        tick: net.tick,
        wave: state.wave,
        game_over: state.game_over,
        started: state.started,
        players: roster.0.values().cloned().collect(),
        entities: things
            .iter()
            .map(|(r, t, look)| NetEntity {
                id: r.id,
                kind: r.kind,
                pos: t.translation.to_array(),
                yaw: t.rotation.to_euler(EulerRot::YXZ).0,
                flash: look.is_some_and(|l| l.flash > 0.0),
            })
            .collect(),
        tracers,
    });
    let bytes = encode(&snapshot);
    for addr in net.clients.keys() {
        let _ = net.socket.send_to(&bytes, addr);
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client-side copies of the host's enemies and projectiles, by network id.
#[derive(Resource, Default)]
struct Puppets(HashMap<u32, Entity>);

/// Where the host last said this entity is; we glide towards it.
#[derive(Component)]
struct Puppet {
    target: Vec3,
    yaw: f32,
}

fn client_send(
    time: Res<Time>,
    mut net: ResMut<Net>,
    session: Res<Session>,
    mut shots: ResMut<ShotQueue>,
    player: Single<(&Transform, &LocalPlayer)>,
) {
    let Some(server) = net.server else { return };
    if !session.connected {
        // Keep knocking until the host answers.
        net.timer -= time.delta_secs();
        if net.timer <= 0.0 {
            net.timer = 0.5;
            let hello = ClientMsg::Hello {
                version: PROTOCOL_VERSION,
                name: session.my_name.clone(),
            };
            net.send_to(&hello, server);
        }
        shots.0.clear();
        return;
    }
    let (tf, p) = *player;
    let input = ClientMsg::Input {
        pos: (tf.translation - Vec3::Y * EYE_HEIGHT).to_array(),
        yaw: p.yaw,
        pitch: p.pitch,
        shots: shots
            .0
            .drain(..)
            .map(|(_, o, d)| (o.to_array(), d.to_array()))
            .collect(),
    };
    net.send_to(&input, server);
}

fn client_receive(
    mut commands: Commands,
    time: Res<Time>,
    mut net: ResMut<Net>,
    mut session: ResMut<Session>,
    mut roster: ResMut<Roster>,
    mut state: ResMut<MatchState>,
    mut tracers: ResMut<Tracers>,
    mut puppets: ResMut<Puppets>,
    assets: Res<GameAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut existing: Query<(&mut Puppet, Option<&mut EnemyLook>)>,
) {
    let now = time.elapsed_secs_f64();
    let server = net.server;
    let mut latest: Option<Snapshot> = None;
    for (bytes, addr) in net.drain() {
        if Some(addr) != server {
            continue;
        }
        let Some(msg) = decode::<ServerMsg>(&bytes) else {
            continue;
        };
        match msg {
            ServerMsg::Welcome { id } => {
                if !session.connected {
                    session.my_id = id;
                    session.connected = true;
                    session.status = format!("Connected to {}", server.unwrap());
                    net.last_heard = now;
                }
            }
            ServerMsg::Reject { reason } => {
                session.status = format!("Can't join: {reason}");
            }
            ServerMsg::Snapshot(s) => {
                if !session.connected {
                    continue;
                }
                net.last_heard = now;
                // Ignore packets that arrive out of order.
                if s.tick > net.last_tick && latest.as_ref().is_none_or(|l| s.tick > l.tick) {
                    latest = Some(s);
                }
            }
        }
    }

    if session.connected && now - net.last_heard > TIMEOUT_SECS {
        session.status = "Lost connection to the host".into();
    } else if session.connected && session.status.starts_with("Lost") {
        session.status = format!("Connected to {}", server.unwrap());
    }

    let Some(snap) = latest else { return };
    net.last_tick = snap.tick;
    state.wave = snap.wave;
    state.game_over = snap.game_over;
    state.started = snap.started;

    // Take the host's word on everything except where we are standing.
    let mine = roster.0.get(&session.my_id).map(|p| (p.pos, p.yaw, p.pitch));
    roster.0 = snap.players.into_iter().map(|p| (p.id, p)).collect();
    if let (Some((pos, yaw, pitch)), Some(me)) = (mine, roster.0.get_mut(&session.my_id)) {
        me.pos = pos;
        me.yaw = yaw;
        me.pitch = pitch;
    }

    for (shooter, start, end) in snap.tracers {
        if shooter != session.my_id {
            let start = Vec3::from_array(start) - Vec3::Y * 0.2;
            tracers.0.push((start, Vec3::from_array(end), 0.06));
        }
    }

    let mut seen = std::collections::HashSet::new();
    for ent in snap.entities {
        seen.insert(ent.id);
        let pos = Vec3::from_array(ent.pos);
        if let Some(&entity) = puppets.0.get(&ent.id) {
            if let Ok((mut puppet, look)) = existing.get_mut(entity) {
                puppet.target = pos;
                puppet.yaw = ent.yaw;
                if let (true, Some(mut look)) = (ent.flash, look) {
                    look.flash = look.flash.max(0.05);
                }
            }
        } else {
            let entity = spawn_replicated(&mut commands, &assets, &mut materials, ent.id, ent.kind, pos);
            commands.entity(entity).insert(Puppet {
                target: pos,
                yaw: ent.yaw,
            });
            puppets.0.insert(ent.id, entity);
        }
    }
    puppets.0.retain(|id, entity| {
        let keep = seen.contains(id);
        if !keep {
            commands.entity(*entity).despawn();
        }
        keep
    });
}

fn move_puppets(time: Res<Time>, mut puppets: Query<(&Puppet, &mut Transform)>) {
    let blend = 1.0 - (-15.0 * time.delta_secs()).exp();
    for (p, mut tf) in &mut puppets {
        if tf.translation.distance(p.target) > 4.0 {
            tf.translation = p.target;
        } else {
            tf.translation = tf.translation.lerp(p.target, blend);
        }
        let target_rot = Quat::from_rotation_y(p.yaw);
        tf.rotation = tf.rotation.slerp(target_rot, blend);
    }
}

/// Tell the host we're leaving so our character disappears right away.
fn say_goodbye(net: Res<Net>, mut exit: EventReader<AppExit>) {
    if exit.read().next().is_some() {
        if let Some(server) = net.server {
            net.send_to(&ClientMsg::Bye, server);
        }
    }
}
