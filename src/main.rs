//! A small co-op wave-survival first-person shooter built with Bevy.
//!
//! Run with no arguments to play solo, `host` to host a game, or
//! `join <address>` to join one. See README.md for details.
//!
//! Controls: mouse to look, WASD to move, Shift to sprint, Space to jump,
//! left click to shoot, R to reload, Esc to release the mouse.

mod avatars;
mod hud;
mod level;
mod net;
mod physics;
mod player;
mod sim;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Tuning
// ---------------------------------------------------------------------------

pub const EYE_HEIGHT: f32 = 1.6;
pub const PLAYER_RADIUS: f32 = 0.4;
pub const MAX_HEALTH: f32 = 100.0;
pub const ARENA_HALF: f32 = 30.0;
pub const PLAYER_START: Vec3 = Vec3::new(0.0, 0.0, 12.0);
pub const MAX_PLAYERS: usize = 8;

/// Where a player (re)spawns. Players are spread out in a line.
pub fn spawn_point(id: u8) -> Vec3 {
    let col = (id % 4) as f32;
    let row = (id / 4) as f32;
    PLAYER_START + Vec3::new(col * 2.0 - 3.0, 0.0, row * 2.0)
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// Single player, no networking.
    Solo,
    /// Runs the game simulation and accepts players.
    Host,
    /// Mirrors the host's game.
    Client,
}

#[derive(Resource)]
pub struct Session {
    pub role: Role,
    pub my_id: u8,
    pub my_name: String,
    /// Connection info shown at the top of the screen.
    pub status: String,
    /// True once a client has been accepted by the host.
    pub connected: bool,
}

impl Session {
    /// The host (or solo player) owns enemies, damage, score and waves.
    pub fn is_authority(&self) -> bool {
        self.role != Role::Client
    }
}

/// Everything about a player that the whole game needs to know.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: u8,
    pub name: String,
    /// Feet position.
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub score: u32,
    pub kills: u32,
    pub alive: bool,
    /// Bumped by the host whenever this player should teleport to spawn.
    pub spawn_seq: u32,
}

impl PlayerInfo {
    pub fn new(id: u8, name: String) -> Self {
        Self {
            id,
            name,
            pos: spawn_point(id).to_array(),
            yaw: 0.0,
            pitch: 0.0,
            health: MAX_HEALTH,
            score: 0,
            kills: 0,
            alive: true,
            spawn_seq: 0,
        }
    }

    pub fn feet(&self) -> Vec3 {
        Vec3::from_array(self.pos)
    }

    pub fn damage(&mut self, amount: f32) {
        if !self.alive {
            return;
        }
        self.health -= amount;
        if self.health <= 0.0 {
            self.health = 0.0;
            self.alive = false;
        }
    }
}

/// All players in the game, keyed by id. On the host this is the truth; on
/// clients it is a copy received from the host.
#[derive(Resource, Default)]
pub struct Roster(pub BTreeMap<u8, PlayerInfo>);

impl Roster {
    pub fn me<'a>(&'a self, session: &Session) -> Option<&'a PlayerInfo> {
        self.0.get(&session.my_id)
    }
}

#[derive(Resource, Default)]
pub struct MatchState {
    pub wave: u32,
    /// Waves begin once the host clicks into the game.
    pub started: bool,
    pub game_over: bool,
    // Host-only bookkeeping
    pub to_spawn: u32,
    pub spawn_timer: f32,
    pub intermission: f32,
    pub next_net_id: u32,
}

/// Shots fired this frame: (shooter id, origin, direction). The host resolves
/// them; clients send them to the host.
#[derive(Resource, Default)]
pub struct ShotQueue(pub Vec<(u8, Vec3, Vec3)>);

/// Short-lived bullet tracers drawn with gizmos: (start, end, time left).
#[derive(Resource, Default)]
pub struct Tracers(pub Vec<(Vec3, Vec3, f32)>);

/// Axis-aligned box collider for static level geometry (half extents).
#[derive(Component)]
pub struct Collider {
    pub half: Vec3,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum NetKind {
    Grunt,
    Shooter,
    Projectile,
}

/// An entity the host replicates to clients (enemies and projectiles).
#[derive(Component)]
pub struct Replicated {
    pub id: u32,
    pub kind: NetKind,
}

/// Marker for enemies (on the host and on clients), used for hit tests.
#[derive(Component)]
pub struct Enemy;

/// Enemy appearance; `flash` > 0 makes it flash white after being hit.
#[derive(Component)]
pub struct EnemyLook {
    pub base_color: Color,
    pub flash: f32,
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Receive network messages.
    NetIn,
    /// Local player input, movement and shooting.
    Local,
    /// Host-side game simulation.
    Sim,
    /// Send network messages.
    NetOut,
    /// Visuals and HUD.
    Present,
}

pub fn cursor_locked(window: &Window) -> bool {
    window.cursor_options.grab_mode != CursorGrabMode::None
}

fn is_authority(session: Res<Session>) -> bool {
    session.is_authority()
}

/// The simulation always runs when hosting (other people are playing), but
/// pauses in solo mode while the mouse is released.
fn sim_running(session: Res<Session>, window: Single<&Window, With<PrimaryWindow>>) -> bool {
    match session.role {
        Role::Solo => cursor_locked(&window),
        Role::Host => true,
        Role::Client => false,
    }
}

fn main() {
    let (session, net) = match net::from_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}\n");
            eprintln!("usage:");
            eprintln!("  rust-fps                         play solo");
            eprintln!("  rust-fps host [port] [--name N]   host a co-op game (default port {})", net::DEFAULT_PORT);
            eprintln!("  rust-fps join <address> [--name N] join a game, e.g. join 192.168.1.20");
            std::process::exit(1);
        }
    };
    let title = match session.role {
        Role::Solo => "Rust FPS".to_string(),
        Role::Host => "Rust FPS (host)".to_string(),
        Role::Client => "Rust FPS (client)".to_string(),
    };

    let mut roster = Roster::default();
    if session.is_authority() {
        roster.0.insert(0, PlayerInfo::new(0, session.my_name.clone()));
    }

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title,
            ..default()
        }),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.45, 0.62, 0.85)))
    .insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 350.0,
        ..default()
    })
    .insert_resource(session)
    .insert_resource(roster)
    .insert_resource(MatchState {
        intermission: 1.5,
        ..default()
    })
    .init_resource::<ShotQueue>()
    .init_resource::<Tracers>()
    .configure_sets(
        Update,
        (
            Phase::NetIn,
            Phase::Local,
            Phase::Sim.run_if(sim_running),
            Phase::NetOut,
            Phase::Present,
        )
            .chain(),
    )
    .add_plugins((
        level::LevelPlugin,
        player::PlayerPlugin,
        sim::SimPlugin,
        avatars::AvatarPlugin,
        hud::HudPlugin,
    ))
    .add_systems(Update, sim::restart.run_if(is_authority));

    if let Some(net) = net {
        app.insert_resource(net).add_plugins(net::NetPlugin);
    }
    app.run();
}
