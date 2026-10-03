//! Co-op networking over plain UDP.
//!
//! The host runs the whole simulation. Clients send their position, aim, the
//! shots they fire and their actions (buy, abilities, upgrade picks); the
//! host sends back a snapshot of the party, the match and every enemy about
//! 30 times a second. Clients move their own character locally, so movement
//! feels instant even with some lag. Actions are numbered and resent until
//! the host confirms them, so a dropped packet never loses a purchase.

use bevy::prelude::*;
use bincode::Options;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket};

use crate::avatars::{spawn_replicated, ReplicatedAssets};
use crate::config::Profile;
use crate::data::Character;
use crate::fx::{Fx, FxOutbox, FxQueue};
use crate::humanoid::HumanoidMeshes;
use crate::{
    ActionQueue, AppState, EnemyStatus, MatchState, NetKind, Phase, PlayerAction, PlayerInfo,
    Replicated, Role, Roster, Session, Shot, ShotQueue, MAX_PLAYERS,
};

pub const DEFAULT_PORT: u16 = 7777;
/// Bump when the message format changes so old builds can't join.
const PROTOCOL_VERSION: u32 = 6;
const SNAPSHOT_INTERVAL: f32 = 1.0 / 30.0;
const SEND_INTERVAL: f32 = 1.0 / 60.0;
const TIMEOUT_SECS: f64 = 10.0;
const CONNECT_SECS: f64 = 8.0;
const MAX_PACKET: u64 = 60_000;
pub const MAX_NAME_LEN: usize = 16;

// ---------------------------------------------------------------------------
// Command line
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum LaunchMode {
    Menu,
    Solo,
    Host(u16),
    Join(String),
}

/// What the command line asked for (see README): `solo`, `host [port]`,
/// `join <address>`, plus `--name <name>`, `--map <0-2>` and `--start`.
#[derive(Resource, Clone, Debug)]
pub struct Launch {
    pub mode: LaunchMode,
    pub name: Option<String>,
    pub map: Option<u8>,
    pub start: bool,
    pub error: Option<String>,
}

pub fn parse_args() -> Launch {
    let mut launch = Launch {
        mode: LaunchMode::Menu,
        name: None,
        map: None,
        start: false,
        error: None,
    };
    let mut args = std::env::args().skip(1).peekable();
    let mut positional = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--name" => launch.name = args.next(),
            "--map" => launch.map = args.next().and_then(|m| m.parse().ok()).map(|m: u8| m.min(2)),
            "--start" => launch.start = true,
            _ => positional.push(a),
        }
    }
    launch.mode = match positional.first().map(String::as_str) {
        None => LaunchMode::Menu,
        Some("solo") => LaunchMode::Solo,
        Some("host") => match positional.get(1).map(|p| p.parse::<u16>()) {
            None => LaunchMode::Host(DEFAULT_PORT),
            Some(Ok(p)) => LaunchMode::Host(p),
            Some(Err(_)) => {
                launch.error = Some(format!("Bad port: {}", positional[1]));
                LaunchMode::Menu
            }
        },
        Some("join") => match positional.get(1) {
            Some(addr) => LaunchMode::Join(addr.clone()),
            None => {
                launch.error = Some("join needs an address, e.g. join 192.168.1.20".into());
                LaunchMode::Menu
            }
        },
        Some(other) => {
            launch.error = Some(format!("Unknown command: {other}"));
            LaunchMode::Menu
        }
    };
    launch
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
enum ClientMsg {
    Hello {
        version: u32,
        name: String,
        character: Character,
        skin: u8,
    },
    Update(ClientUpdate),
    Bye,
}

#[derive(Serialize, Deserialize)]
struct ClientUpdate {
    pos: [f32; 3],
    yaw: f32,
    pitch: f32,
    stance: u8,
    emote: u8,
    emote_seq: u8,
    active_slot: u8,
    character: Character,
    skin: u8,
    gun_skins: Vec<u8>,
    ready: bool,
    shots: Vec<Shot>,
    actions: Vec<(u32, PlayerAction)>,
}

#[derive(Serialize, Deserialize)]
enum ServerMsg {
    Welcome { id: u8 },
    Reject { reason: String },
    Snapshot(Snapshot),
    Closed,
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
    tick: u32,
    state: MatchState,
    players: Vec<PlayerInfo>,
    entities: Vec<NetEntity>,
    fx: Vec<Fx>,
}

#[derive(Serialize, Deserialize)]
struct NetEntity {
    id: u32,
    kind: NetKind,
    pos: [f32; 3],
    yaw: f32,
    /// 1 = hit flash, 2 = burning, 4 = slowed.
    flags: u8,
}

/// Size-limited encoding so a bad packet can't make us allocate gigabytes.
fn codec() -> impl Options {
    bincode::DefaultOptions::new().with_limit(MAX_PACKET)
}

fn encode<T: Serialize>(msg: &T) -> Vec<u8> {
    codec().serialize(msg).unwrap_or_default()
}

fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Option<T> {
    codec().deserialize(bytes).ok()
}

// ---------------------------------------------------------------------------
// Connection state
// ---------------------------------------------------------------------------

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
    started_at: Option<f64>,
    last_heard: f64,
    timer: f32,
    tick: u32,
    last_tick: u32,
    /// Client only: actions not yet confirmed by the host.
    pending: Vec<(u32, PlayerAction)>,
    shots: Vec<Shot>,
}

impl Net {
    fn new(socket: UdpSocket, server: Option<SocketAddr>) -> std::io::Result<Self> {
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            server,
            clients: HashMap::new(),
            started_at: None,
            last_heard: 0.0,
            timer: 0.0,
            tick: 0,
            last_tick: 0,
            pending: Vec::new(),
            shots: Vec::new(),
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

/// Best guess at this machine's LAN address (no packets are actually sent).
pub fn lan_ip() -> Option<IpAddr> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    Some(s.local_addr().ok()?.ip())
}

/// A message for the main menu (why a connection ended, errors).
#[derive(Resource, Default)]
pub struct Notice(pub String);

/// Whether this player has pressed Ready in the party screen.
#[derive(Resource, Default)]
pub struct LocalReady(pub bool);

/// Requests from the menus.
#[derive(Event, Clone, Debug)]
pub enum PartyRequest {
    Solo,
    Host(u16),
    Join(String),
    /// Leave the party (or solo game) and go back to the main menu.
    Leave(Option<String>),
}

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Puppets>()
            .init_resource::<Notice>()
            .init_resource::<LocalReady>()
            .add_event::<PartyRequest>()
            .add_systems(Startup, apply_launch)
            .add_systems(Update, handle_requests.before(Phase::NetIn))
            .add_systems(
                Update,
                (host_receive.run_if(is_host), client_receive.run_if(is_client))
                    .in_set(Phase::NetIn)
                    .run_if(resource_exists::<Net>),
            )
            .add_systems(
                Update,
                (
                    sync_own_choices,
                    host_send.run_if(is_host.and(resource_exists::<Net>)),
                    client_send.run_if(is_client.and(resource_exists::<Net>)),
                    clear_outbox.run_if(not(is_host)),
                )
                    .chain()
                    .in_set(Phase::NetOut),
            )
            .add_systems(
                Update,
                move_puppets
                    .in_set(Phase::Present)
                    .run_if(is_client.and(in_state(AppState::InGame))),
            )
            .add_systems(OnExit(AppState::InGame), forget_puppets)
            .add_systems(Last, say_goodbye.run_if(resource_exists::<Net>));
    }
}

fn is_host(session: Res<Session>) -> bool {
    session.role == Role::Host
}

fn is_client(session: Res<Session>) -> bool {
    session.role == Role::Client
}

fn apply_launch(
    launch: Res<crate::net::Launch>,
    mut profile: ResMut<Profile>,
    mut session: ResMut<Session>,
    mut state: ResMut<MatchState>,
    mut notice: ResMut<Notice>,
    mut requests: EventWriter<PartyRequest>,
) {
    if let Some(name) = &launch.name {
        let name: String = name.chars().take(MAX_NAME_LEN).collect();
        if !name.trim().is_empty() {
            profile.name = name;
        }
    }
    if let Some(e) = &launch.error {
        notice.0 = e.clone();
    }
    session.autostart = launch.start;
    match &launch.mode {
        LaunchMode::Menu => {}
        LaunchMode::Solo => {
            requests.write(PartyRequest::Solo);
        }
        LaunchMode::Host(port) => {
            requests.write(PartyRequest::Host(*port));
        }
        LaunchMode::Join(addr) => {
            requests.write(PartyRequest::Join(addr.clone()));
        }
    }
    if let Some(map) = launch.map {
        state.map = map;
    }
}

fn resolve(target: &str) -> Result<SocketAddr, String> {
    let target = target.trim();
    if target.is_empty() {
        return Err("Type the host's address first".into());
    }
    let with_port = if target.contains(':') {
        target.to_string()
    } else {
        format!("{target}:{DEFAULT_PORT}")
    };
    with_port
        .to_socket_addrs()
        .map_err(|e| format!("Can't find {with_port}: {e}"))?
        .find(SocketAddr::is_ipv4)
        .ok_or_else(|| format!("No IPv4 address for {with_port}"))
}

fn handle_requests(
    mut commands: Commands,
    mut requests: EventReader<PartyRequest>,
    time: Res<Time>,
    net: Option<Res<Net>>,
    profile: Res<Profile>,
    mut session: ResMut<Session>,
    mut roster: ResMut<Roster>,
    mut state: ResMut<MatchState>,
    mut ready: ResMut<LocalReady>,
    mut notice: ResMut<Notice>,
    mut next: ResMut<NextState<AppState>>,
) {
    for req in requests.read() {
        let map = state.map;
        let me = || PlayerInfo {
            gun_skins: profile.gun_skins.clone(),
            ..PlayerInfo::new(0, profile.name.clone(), profile.character, profile.skin)
        };
        ready.0 = false;
        match req {
            PartyRequest::Solo => {
                *session = Session {
                    role: Role::Solo,
                    autostart: session.autostart,
                    ..default()
                };
                roster.0 = [(0, me())].into();
                *state = MatchState::new(map);
                notice.0.clear();
                next.set(AppState::Lobby);
            }
            PartyRequest::Host(port) => {
                let socket = match UdpSocket::bind(("0.0.0.0", *port)) {
                    Ok(s) => s,
                    Err(e) => {
                        notice.0 = format!("Can't host on port {port}: {e}");
                        continue;
                    }
                };
                let Ok(n) = Net::new(socket, None) else {
                    notice.0 = "Couldn't set up networking".into();
                    continue;
                };
                let ip = lan_ip().map(|ip| ip.to_string()).unwrap_or_else(|| "<your IP>".into());
                let address = if *port == DEFAULT_PORT {
                    ip
                } else {
                    format!("{ip}:{port}")
                };
                *session = Session {
                    role: Role::Host,
                    status: format!("Hosting - friends join with address {address}"),
                    autostart: session.autostart,
                    ..default()
                };
                info!("{}", session.status);
                commands.insert_resource(n);
                roster.0 = [(0, me())].into();
                *state = MatchState::new(map);
                notice.0.clear();
                next.set(AppState::Lobby);
            }
            PartyRequest::Join(target) => {
                let addr = match resolve(target) {
                    Ok(a) => a,
                    Err(e) => {
                        notice.0 = e;
                        continue;
                    }
                };
                let socket = match UdpSocket::bind("0.0.0.0:0") {
                    Ok(s) => s,
                    Err(e) => {
                        notice.0 = format!("Can't open a network socket: {e}");
                        continue;
                    }
                };
                let Ok(mut n) = Net::new(socket, Some(addr)) else {
                    notice.0 = "Couldn't set up networking".into();
                    continue;
                };
                n.started_at = Some(time.elapsed_secs_f64());
                commands.insert_resource(n);
                *session = Session {
                    role: Role::Client,
                    my_id: 0,
                    status: format!("Connecting to {addr}..."),
                    connected: false,
                    autostart: false,
                };
                roster.0.clear();
                *state = MatchState::new(map);
                notice.0.clear();
                next.set(AppState::Lobby);
            }
            PartyRequest::Leave(reason) => {
                if let Some(net) = &net {
                    if let Some(server) = net.server {
                        net.send_to(&ClientMsg::Bye, server);
                    }
                    for addr in net.clients.keys() {
                        for _ in 0..3 {
                            net.send_to(&ServerMsg::Closed, *addr);
                        }
                    }
                }
                commands.remove_resource::<Net>();
                *session = Session::default();
                roster.0.clear();
                *state = MatchState::new(map);
                notice.0 = reason.clone().unwrap_or_default();
                next.set(AppState::Menu);
            }
        }
    }
}

/// Before a match, your character, skin and Ready flag come from the menus.
fn sync_own_choices(
    session: Res<Session>,
    profile: Res<Profile>,
    ready: Res<LocalReady>,
    state: Res<MatchState>,
    mut roster: ResMut<Roster>,
) {
    if state.started || session.role == Role::Client {
        return;
    }
    if let Some(me) = roster.0.get_mut(&session.my_id) {
        if me.character != profile.character {
            me.character = profile.character;
        }
        if me.skin != profile.skin {
            me.skin = profile.skin;
        }
        if me.gun_skins != profile.gun_skins {
            me.gun_skins = profile.gun_skins.clone();
        }
        if me.name != profile.name {
            me.name = profile.name.clone();
        }
        me.ready = ready.0 || session.role == Role::Host;
    }
}

/// Effects only need to be kept for clients when hosting.
fn clear_outbox(mut out: ResMut<FxOutbox>) {
    out.0.clear();
}

// ---------------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------------

fn host_receive(
    time: Res<Time>,
    mut net: ResMut<Net>,
    mut roster: ResMut<Roster>,
    state: Res<MatchState>,
    mut shots: ResMut<ShotQueue>,
    mut actions: ResMut<ActionQueue>,
) {
    let now = time.elapsed_secs_f64();
    for (bytes, addr) in net.drain() {
        let Some(msg) = decode::<ClientMsg>(&bytes) else {
            continue;
        };
        match msg {
            ClientMsg::Hello {
                version,
                name,
                character,
                skin,
            } => {
                if version != PROTOCOL_VERSION {
                    let reason = format!("Version mismatch - the host runs {}; everyone needs the same version", crate::VERSION);
                    net.send_to(&ServerMsg::Reject { reason }, addr);
                    continue;
                }
                if let Some(conn) = net.clients.get(&addr) {
                    // Our Welcome was probably lost; send it again.
                    net.send_to(&ServerMsg::Welcome { id: conn.id }, addr);
                    continue;
                }
                if roster.0.len() >= MAX_PLAYERS {
                    let reason = "The party is full".into();
                    net.send_to(&ServerMsg::Reject { reason }, addr);
                    continue;
                }
                let Some(id) = (1..=u8::MAX).find(|id| !roster.0.contains_key(id)) else {
                    continue;
                };
                let mut name: String = name.chars().take(MAX_NAME_LEN).collect();
                if name.trim().is_empty() {
                    name = format!("Player {id}");
                }
                info!("{name} joined from {addr} as player {id}");
                let skin = skin.min(crate::data::SKINS.len() as u8 - 1);
                roster.0.insert(id, PlayerInfo::new(id, name, character, skin));
                net.clients.insert(addr, Conn { id, last_heard: now });
                net.send_to(&ServerMsg::Welcome { id }, addr);
            }
            ClientMsg::Update(u) => {
                let Some(conn) = net.clients.get_mut(&addr) else {
                    continue;
                };
                conn.last_heard = now;
                let id = conn.id;
                let Some(p) = roster.0.get_mut(&id) else {
                    continue;
                };
                if !state.started {
                    p.character = u.character;
                    p.skin = u.skin.min(crate::data::SKINS.len() as u8 - 1);
                    p.gun_skins = u.gun_skins.into_iter().take(crate::data::GUNS.len()).collect();
                    p.ready = u.ready;
                }
                if u.pos.iter().chain([u.yaw, u.pitch].iter()).all(|v| v.is_finite()) {
                    p.pos = u.pos;
                    p.yaw = u.yaw;
                    p.pitch = u.pitch;
                }
                p.stance = u.stance.min(2);
                p.emote = u.emote.min(crate::rig::EMOTES.len() as u8);
                p.emote_seq = u.emote_seq;
                p.active_slot = u.active_slot.min(1);
                // Cap per packet so a client can't fire faster than any gun allows.
                for shot in u.shots.into_iter().take(32) {
                    shots.0.push((id, shot));
                }
                for (seq, action) in u.actions.into_iter().take(32) {
                    actions.0.push((id, seq, action));
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
    mut out: ResMut<FxOutbox>,
    things: Query<(&Replicated, &Transform, Option<&EnemyStatus>)>,
) {
    net.timer += time.delta_secs();
    if net.timer < SNAPSHOT_INTERVAL {
        return;
    }
    net.timer = 0.0;
    let fx = std::mem::take(&mut out.0);
    if net.clients.is_empty() {
        return;
    }
    net.tick += 1;
    let snapshot = ServerMsg::Snapshot(Snapshot {
        tick: net.tick,
        state: state.clone(),
        players: roster.0.values().cloned().collect(),
        entities: things
            .iter()
            .map(|(r, t, status)| NetEntity {
                id: r.id,
                kind: r.kind,
                pos: t.translation.to_array(),
                yaw: t.rotation.to_euler(EulerRot::YXZ).0,
                flags: status.map_or(0, |s| {
                    (s.flash > 0.0) as u8 | (s.burning as u8) << 1 | (s.slowed as u8) << 2
                }),
            })
            .collect(),
        fx,
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

fn forget_puppets(mut puppets: ResMut<Puppets>) {
    puppets.0.clear();
}

fn client_send(
    time: Res<Time>,
    mut net: ResMut<Net>,
    session: Res<Session>,
    profile: Res<Profile>,
    ready: Res<LocalReady>,
    roster: Res<Roster>,
    mut shots: ResMut<ShotQueue>,
    mut actions: ResMut<ActionQueue>,
) {
    let Some(server) = net.server else { return };
    // Collect everything fired or done since the last packet.
    net.shots.extend(shots.0.drain(..).map(|(_, s)| s));
    net.pending.extend(actions.0.drain(..).map(|(_, seq, a)| (seq, a)));

    net.timer -= time.delta_secs();
    if net.timer > 0.0 {
        return;
    }
    if !session.connected {
        // Keep knocking until the host answers.
        net.timer = 0.5;
        net.shots.clear();
        net.pending.clear();
        let hello = ClientMsg::Hello {
            version: PROTOCOL_VERSION,
            name: profile.name.clone(),
            character: profile.character,
            skin: profile.skin,
        };
        net.send_to(&hello, server);
        return;
    }
    net.timer = SEND_INTERVAL;
    let Some(me) = roster.me(&session) else {
        return;
    };
    let ack = me.action_ack;
    net.pending.retain(|(seq, _)| *seq > ack);
    let update = ClientMsg::Update(ClientUpdate {
        pos: me.pos,
        yaw: me.yaw,
        pitch: me.pitch,
        stance: me.stance,
        emote: me.emote,
        emote_seq: me.emote_seq,
        active_slot: me.active_slot,
        character: profile.character,
        skin: profile.skin,
        gun_skins: profile.gun_skins.clone(),
        ready: ready.0,
        shots: std::mem::take(&mut net.shots),
        actions: net.pending.iter().take(16).copied().collect(),
    });
    net.send_to(&update, server);
}

fn client_receive(
    mut commands: Commands,
    time: Res<Time>,
    mut net: ResMut<Net>,
    mut session: ResMut<Session>,
    mut roster: ResMut<Roster>,
    mut state: ResMut<MatchState>,
    mut fx: ResMut<FxQueue>,
    mut puppets: ResMut<Puppets>,
    app_state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut requests: EventWriter<PartyRequest>,
    assets: Res<ReplicatedAssets>,
    meshes: Res<HumanoidMeshes>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut existing: Query<(&mut Puppet, Option<&mut EnemyStatus>)>,
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
                    net.timer = 0.0;
                }
            }
            ServerMsg::Reject { reason } => {
                requests.write(PartyRequest::Leave(Some(format!("Can't join: {reason}"))));
                return;
            }
            ServerMsg::Closed => {
                requests.write(PartyRequest::Leave(Some("The host closed the party.".into())));
                return;
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

    if !session.connected {
        if net.started_at.is_some_and(|t| now - t > CONNECT_SECS) {
            let addr = server.map(|s| s.to_string()).unwrap_or_default();
            requests.write(PartyRequest::Leave(Some(format!(
                "Couldn't reach a host at {addr}. Check the address, that they are hosting, and their firewall."
            ))));
        }
        return;
    }
    if now - net.last_heard > TIMEOUT_SECS {
        requests.write(PartyRequest::Leave(Some("Lost connection to the host.".into())));
        return;
    }

    let Some(snap) = latest else { return };
    net.last_tick = snap.tick;

    // Take the host's word on everything except where we are standing.
    let mine = roster
        .0
        .get(&session.my_id)
        .map(|p| (p.pos, p.yaw, p.pitch, p.stance, p.active_slot, p.emote, p.emote_seq));
    roster.0 = snap.players.into_iter().map(|p| (p.id, p)).collect();
    if let (Some((pos, yaw, pitch, stance, slot, emote, emote_seq)), Some(me)) = (mine, roster.0.get_mut(&session.my_id)) {
        me.emote = emote;
        me.emote_seq = emote_seq;
        me.pos = pos;
        me.yaw = yaw;
        me.pitch = pitch;
        me.stance = stance;
        me.active_slot = slot;
    }
    *state = snap.state;

    // Follow the host between the party screen and the match.
    let in_game = *app_state.get() == AppState::InGame;
    if *app_state.get() == AppState::Lobby && state.started && !state.game_over && !state.extracted {
        next.set(AppState::InGame);
        return;
    }
    if in_game && !state.started {
        next.set(AppState::Lobby);
        return;
    }
    if !in_game {
        return;
    }

    for f in snap.fx {
        match &f {
            Fx::Tracer { shooter: who, .. } | Fx::Dash { player: who, .. } if *who == session.my_id => {
                continue;
            }
            _ => {}
        }
        fx.0.push(f);
    }

    let mut seen = HashSet::new();
    for ent in snap.entities {
        seen.insert(ent.id);
        let pos = Vec3::from_array(ent.pos);
        if let Some(&entity) = puppets.0.get(&ent.id) {
            if let Ok((mut puppet, status)) = existing.get_mut(entity) {
                puppet.target = pos;
                puppet.yaw = ent.yaw;
                if let Some(mut status) = status {
                    if ent.flags & 1 != 0 {
                        status.flash = status.flash.max(0.06);
                    }
                    status.burning = ent.flags & 2 != 0;
                    status.slowed = ent.flags & 4 != 0;
                }
            }
        } else {
            let entity =
                spawn_replicated(&mut commands, &assets, &meshes, &mut materials, ent.id, ent.kind, pos);
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
            if let Ok(mut e) = commands.get_entity(*entity) {
                e.try_despawn();
            }
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

/// Tell the others we're leaving so nobody waits for a timeout.
fn say_goodbye(net: Res<Net>, mut exit: EventReader<AppExit>) {
    if exit.read().next().is_some() {
        if let Some(server) = net.server {
            net.send_to(&ClientMsg::Bye, server);
        }
        for addr in net.clients.keys() {
            net.send_to(&ServerMsg::Closed, *addr);
        }
    }
}
