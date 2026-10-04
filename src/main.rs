//! Rust FPS: a co-op wave-survival shooter built with Bevy.
//!
//! Start the game and use the menus, or skip straight to a party from the
//! command line with `host` or `join <address>`. See README.md.

/// The game's version, from Cargo.toml (major.minor, e.g. "v4.0").
pub const VERSION: &str = concat!(
    "v",
    env!("CARGO_PKG_VERSION_MAJOR"),
    ".",
    env!("CARGO_PKG_VERSION_MINOR")
);

mod abilities;
mod audio;
mod config;
mod data;
mod emotes;
mod fx;
mod game;
mod graphics;
mod hud;
mod maps;
mod models;
// Shorter paths for the most used pieces of the map and model folders.
use fx::auras;
use maps::{nav, props, strips};
use models::{avatars, gunmodels, hands, humanoid, kit, skins};
mod net;
mod physics;
mod pings;
mod player;
mod progression;
mod rig;
mod sim;
mod ui;
mod viewmodel;
mod weapons;
mod zombies;

use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use data::{Character, Upgrade, STARTER_GUN};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const EYE_HEIGHT: f32 = 1.6;
pub const CROUCH_EYE_HEIGHT: f32 = 1.0;
pub const PLAYER_RADIUS: f32 = 0.4;
pub const BASE_HEALTH: f32 = 100.0;
pub const MAX_PLAYERS: usize = 8;
pub const START_POINTS: u32 = 500;

// ---------------------------------------------------------------------------
// App states
// ---------------------------------------------------------------------------

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Menu,
    /// The party screen before a match.
    Lobby,
    InGame,
}

/// Entities that only exist during a match; removed when it ends.
#[derive(Component)]
pub struct InGameEntity;

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
    /// Connection info shown in the lobby and HUD.
    pub status: String,
    /// True once a client has been accepted by the host.
    pub connected: bool,
    /// Start the match as soon as the lobby opens (command line `--start`).
    pub autostart: bool,
}

impl Session {
    pub fn is_authority(&self) -> bool {
        self.role != Role::Client
    }
}

impl Default for Session {
    fn default() -> Self {
        Self {
            role: Role::Solo,
            my_id: 0,
            status: String::new(),
            connected: true,
            autostart: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared game state (replicated from the host)
// ---------------------------------------------------------------------------

/// Everything about a player that the whole party needs to know.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: u8,
    pub name: String,
    pub character: Character,
    /// The two abilities and the ultimate this player brought.
    pub kit: [data::Ability; 3],
    /// Their level with this character (shown in the party).
    pub char_level: u8,
    pub skin: u8,
    /// Gun-specific skins per gun id (255 for none).
    pub gun_skins: Vec<u8>,
    /// The gun (and attachments) this player brought for a wall board.
    pub loadout: Option<(u8, data::Attach)>,
    pub ready: bool,

    /// Feet position.
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    /// 0 standing, 1 crouching, 2 sliding.
    pub stance: u8,
    /// Emote playing (0 none) and a counter that restarts it.
    pub emote: u8,
    pub emote_seq: u8,
    /// Bumped by the host when a Supply Drop refills this player's ammo.
    pub supply_seq: u8,

    pub health: f32,
    pub alive: bool,
    /// Bumped by the host whenever this player should teleport to spawn.
    pub spawn_seq: u32,

    pub points: u32,
    pub score: u32,
    pub kills: u32,

    pub guns: [Option<u8>; 2],
    /// Attachments on each gun.
    pub attach: [data::Attach; 2],
    pub active_slot: u8,
    pub perks: u8,

    pub level: u32,
    pub xp: u32,
    /// Level-up rewards waiting to be picked (one set of options at a time).
    pub choices: Vec<Upgrade>,
    pub pending_picks: u8,
    pub tiers: [u8; 3],
    pub gun_elements: u8,
    pub ability_elements: u8,

    pub cooldowns: [f32; 2],
    pub ult_charge: f32,
    pub overdrive: f32,
    /// Combat Stim: faster movement and fire while above zero.
    pub stim: f32,
    /// Thousand Cuts: gone from sight, untouchable, while above zero.
    pub vanish: f32,
    /// Highest action number the host has handled (for resending).
    pub action_ack: u32,
}

impl PlayerInfo {
    /// The skin this player shows on `gun`.
    pub fn skin_for(&self, gun: u8) -> u8 {
        crate::data::skin_for(self.skin, &self.gun_skins, gun)
    }

    pub fn new(id: u8, name: String, character: Character, skin: u8) -> Self {
        Self {
            id,
            name,
            character,
            kit: character.default_kit(),
            char_level: 1,
            skin,
            gun_skins: Vec::new(),
            loadout: None,
            ready: false,
            pos: [0.0; 3],
            yaw: 0.0,
            pitch: 0.0,
            stance: 0,
            emote: 0,
            emote_seq: 0,
            supply_seq: 0,
            health: BASE_HEALTH,
            alive: true,
            spawn_seq: 0,
            points: START_POINTS,
            score: 0,
            kills: 0,
            guns: [Some(STARTER_GUN), None],
            attach: [data::Attach::NONE; 2],
            active_slot: 0,
            perks: 0,
            level: 1,
            xp: 0,
            choices: Vec::new(),
            pending_picks: 0,
            tiers: [0; 3],
            gun_elements: 0,
            ability_elements: 0,
            cooldowns: [0.0; 2],
            ult_charge: 0.0,
            overdrive: 0.0,
            stim: 0.0,
            vanish: 0.0,
            action_ack: 0,
        }
    }

    /// Back to a fresh start for a new match (keeps name and loadout picks).
    pub fn reset_for_match(&mut self) {
        let mut keep = PlayerInfo::new(self.id, self.name.clone(), self.character, self.skin);
        keep.gun_skins = std::mem::take(&mut self.gun_skins);
        keep.kit = self.kit;
        keep.char_level = self.char_level;
        let seq = self.spawn_seq;
        let ack = self.action_ack;
        *self = keep;
        self.spawn_seq = seq + 1;
        self.action_ack = ack;
        self.ready = false;
    }

    pub fn feet(&self) -> Vec3 {
        Vec3::from_array(self.pos)
    }

    pub fn max_health(&self) -> f32 {
        if data::has_perk(self.perks, data::Perk::Juggernaut) {
            200.0
        } else {
            BASE_HEALTH
        }
    }

    pub fn damage(&mut self, amount: f32) {
        if !self.alive || self.vanish > 0.0 {
            return;
        }
        self.health -= amount;
        if self.health <= 0.0 {
            self.health = 0.0;
            self.alive = false;
            // Perks are lost when you go down.
            self.perks = 0;
            self.overdrive = 0.0;
            self.stim = 0.0;
        }
    }
}

/// All players, keyed by id. On the host this is the truth; on clients it
/// is a copy received from the host.
#[derive(Resource, Default)]
pub struct Roster(pub BTreeMap<u8, PlayerInfo>);

impl Roster {
    pub fn me<'a>(&'a self, session: &Session) -> Option<&'a PlayerInfo> {
        self.0.get(&session.my_id)
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub enum BoxState {
    #[default]
    Idle,
    Rolling {
        player: u8,
        time: f32,
    },
    Offer {
        player: u8,
        gun: u8,
        attach: data::Attach,
        time: f32,
    },
    Moving {
        time: f32,
    },
}

#[derive(Resource, Clone, Debug, Default, Serialize, Deserialize)]
pub struct MatchState {
    pub map: u8,
    pub round: u32,
    pub started: bool,
    pub game_over: bool,
    pub extracted: bool,
    pub intermission: f32,
    pub to_spawn: u32,
    pub spawn_timer: f32,
    pub next_net_id: u32,

    pub insta_kill: f32,
    pub double_points: f32,
    pub max_ammo_seq: u32,
    /// Last power-up grabbed, for the on-screen banner.
    pub powerup_seq: u32,
    pub last_powerup: Option<data::PowerUp>,

    pub box_spot: u8,
    pub box_state: BoxState,
    pub box_uses: u32,
    pub box_move_after: u32,

    /// Seconds left to extract (0 = extraction not available).
    pub extraction: f32,
    /// How long the whole team has been standing in the zone.
    pub extract_hold: f32,
    /// Areas opened by buying doors (bit 1 north, 2 south, 3 east, 4 west).
    pub doors: u8,
    /// Night version of the map (picked by the host in the lobby).
    pub night: bool,
    pub sandbox: Sandbox,
}

/// Sandbox mode: a solo practice match with tools (F1) to spawn zombies,
/// try any gun and attachments, and switch waves, damage and cooldowns off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sandbox {
    pub on: bool,
    /// Rounds run on their own (off: only the zombies you spawn).
    pub waves: bool,
    /// No damage taken and endless ammo.
    pub god: bool,
    /// No ability cooldowns and the ultimate always ready.
    pub free_abilities: bool,
}

/// A sandbox tool used by a player (only works in sandbox mode).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum DevCmd {
    /// Zombies in front of `at`: kind 0 walker, 1 spitter, 2 brute, 3 crawler.
    Spawn {
        kind: u8,
        count: u8,
        at: [f32; 3],
        dir: [f32; 3],
    },
    Gun {
        gun: u8,
        attach: data::Attach,
    },
    /// 0 waves, 1 god mode, 2 free abilities.
    Toggle(u8),
    Points,
    NextRound,
    KillAll,
    LevelUp,
}

impl MatchState {
    pub fn new(map: u8) -> Self {
        Self {
            map,
            intermission: 3.0,
            box_move_after: 6,
            ..default()
        }
    }
}

/// A shot fired by a player: the host decides what it hit.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Shot {
    pub origin: [f32; 3],
    pub dir: [f32; 3],
    pub gun: u8,
}

#[derive(Resource, Default)]
pub struct ShotQueue(pub Vec<(u8, Shot)>);

/// Things a player asks the host to do.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PlayerAction {
    Interact,
    /// A knife slash from `origin` facing `dir`.
    Melee {
        origin: [f32; 3],
        dir: [f32; 3],
    },
    Ability {
        slot: u8,
        origin: [f32; 3],
        dir: [f32; 3],
        /// How far a held ability was charged (0 to 1).
        charge: f32,
    },
    Choose(u8),
    /// Mark a spot (or an enemy by net id; u32::MAX for none).
    Ping {
        pos: [f32; 3],
        target: u32,
    },
    Dev(DevCmd),
}

/// Actions waiting for the host: (player, sequence number, action).
#[derive(Resource, Default)]
pub struct ActionQueue(pub Vec<(u8, u32, PlayerAction)>);

/// Axis-aligned box collider for static level geometry (half extents).
#[derive(Component, Clone, Copy)]
pub struct Collider {
    pub half: Vec3,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum NetKind {
    Grunt,
    Shooter,
    Brute,
    Fireball,
    Grenade,
    PowerUp(data::PowerUp),
    Firebomb,
    Turret,
    Coil,
    /// An ability projectile (see `sim::powers::Look`).
    Missile(u8),
    Mine,
    Drone,
}

/// An entity the host replicates to clients (enemies, projectiles, pickups).
#[derive(Component)]
pub struct Replicated {
    pub id: u32,
    pub kind: NetKind,
}

/// Marker for enemies (on the host and on clients), used for hit tests.
#[derive(Component)]
pub struct Enemy;

/// Visual status shown on enemies: hit flash, burning, slowed.
#[derive(Component, Default)]
pub struct EnemyStatus {
    pub flash: f32,
    pub burning: bool,
    pub slowed: bool,
    /// Frozen or stunned in place.
    pub stunned: bool,
    /// Legs shot out.
    pub crawler: bool,
    /// Mid-swing (or spitting).
    pub attacking: bool,
}

// ---------------------------------------------------------------------------
// Scheduling
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

pub fn set_cursor_lock(window: &mut Window, locked: bool) {
    window.cursor_options.grab_mode = if locked {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    window.cursor_options.visible = !locked;
}

/// Solo games pause while a menu is open; hosted games keep running.
fn sim_running(
    session: Res<Session>,
    paused: Res<game::Paused>,
    state: Res<State<AppState>>,
) -> bool {
    *state.get() == AppState::InGame
        && match session.role {
            Role::Solo => !paused.0,
            Role::Host => true,
            Role::Client => false,
        }
}

fn main() {
    let launch = net::parse_args();

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: format!("Rust FPS {VERSION}"),
            ..default()
        }),
        ..default()
    }))
    .init_state::<AppState>()
    .insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.09)))
    .insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 350.0,
        ..default()
    })
    .insert_resource(launch)
    .init_resource::<Session>()
    .init_resource::<Roster>()
    .insert_resource(MatchState::new(0))
    .init_resource::<ShotQueue>()
    .init_resource::<ActionQueue>()
    .configure_sets(
        Update,
        (
            Phase::NetIn,
            Phase::Local.run_if(in_state(AppState::InGame)),
            Phase::Sim.run_if(sim_running),
            Phase::NetOut,
            Phase::Present,
        )
            .chain(),
    )
    .add_plugins((
        config::ConfigPlugin,
        ui::UiPlugin,
        net::NetPlugin,
        game::GamePlugin,
        player::PlayerPlugin,
        weapons::WeaponPlugin,
        abilities::AbilityPlugin,
        sim::SimPlugin,
        avatars::AvatarPlugin,
        humanoid::HumanoidPlugin,
        fx::FxPlugin,
        hud::HudPlugin,
        gunmodels::GunModelPlugin,
        viewmodel::ViewModelPlugin,
    ))
    .add_plugins(audio::AudioPlugin)
    .add_plugins((
        rig::RigPlugin,
        zombies::ZombiePlugin,
        auras::AuraPlugin,
        progression::ProgressionPlugin,
        graphics::GraphicsPlugin,
        emotes::EmotePlugin,
        pings::PingPlugin,
    ))
    .run();
}
