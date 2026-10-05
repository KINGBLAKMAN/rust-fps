//! Settings (FOV, sensitivity, keys, display) and the player profile (name,
//! character, skins, gacha spins). Both are saved as JSON in the user's data
//! folder: %APPDATA%\RustFPS on Windows, ~/.config/rust-fps elsewhere.

use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, VideoModeSelection, WindowMode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::data::{Ability, Character};

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(load::<Settings>("settings.json").with_all_keys())
            .insert_resource(load::<Profile>("profile.json"))
            .add_systems(Startup, apply_display)
            .add_systems(Update, (apply_display, save_on_change));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Action {
    Forward,
    Back,
    Left,
    Right,
    Jump,
    Sprint,
    Crouch,
    Reload,
    Interact,
    Ability1,
    Ability2,
    Ultimate,
    WeaponAbility1,
    WeaponAbility2,
    SwapWeapon,
    Upgrades,
    Scoreboard,
    Emote,
    Ping,
    Melee,
}

impl Action {
    pub const ALL: [Action; 20] = [
        Action::Forward,
        Action::Back,
        Action::Left,
        Action::Right,
        Action::Jump,
        Action::Sprint,
        Action::Crouch,
        Action::Reload,
        Action::Interact,
        Action::Ability1,
        Action::Ability2,
        Action::Ultimate,
        Action::WeaponAbility1,
        Action::WeaponAbility2,
        Action::SwapWeapon,
        Action::Upgrades,
        Action::Scoreboard,
        Action::Emote,
        Action::Ping,
        Action::Melee,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Action::Forward => "Move forward",
            Action::Back => "Move back",
            Action::Left => "Move left",
            Action::Right => "Move right",
            Action::Jump => "Jump",
            Action::Sprint => "Sprint",
            Action::Crouch => "Crouch / slide",
            Action::Reload => "Reload",
            Action::Interact => "Interact / buy",
            Action::Ability1 => "Ability 1",
            Action::Ability2 => "Ability 2",
            Action::Ultimate => "Ultimate",
            Action::WeaponAbility1 => "Weapon ability 1",
            Action::WeaponAbility2 => "Weapon ability 2",
            Action::SwapWeapon => "Swap weapon",
            Action::Upgrades => "Pick level-up upgrade",
            Action::Scoreboard => "Scoreboard",
            Action::Emote => "Emotes",
            Action::Ping => "Ping",
            Action::Melee => "Melee",
        }
    }

    fn default_key(self) -> KeyCode {
        match self {
            Action::Forward => KeyCode::KeyW,
            Action::Back => KeyCode::KeyS,
            Action::Left => KeyCode::KeyA,
            Action::Right => KeyCode::KeyD,
            Action::Jump => KeyCode::Space,
            Action::Sprint => KeyCode::ShiftLeft,
            Action::Crouch => KeyCode::KeyC,
            Action::Reload => KeyCode::KeyR,
            Action::Interact => KeyCode::KeyF,
            Action::Ability1 => KeyCode::KeyQ,
            Action::Ability2 => KeyCode::KeyE,
            Action::Ultimate => KeyCode::KeyX,
            Action::WeaponAbility1 => KeyCode::Digit3,
            Action::WeaponAbility2 => KeyCode::Digit4,
            Action::SwapWeapon => KeyCode::KeyT,
            Action::Upgrades => KeyCode::KeyB,
            Action::Scoreboard => KeyCode::Tab,
            Action::Emote => KeyCode::KeyG,
            Action::Ping => KeyCode::KeyZ,
            Action::Melee => KeyCode::KeyV,
        }
    }
}

/// Short, readable name for a key ("KeyW" -> "W").
pub fn key_name(key: KeyCode) -> String {
    let s = format!("{key:?}");
    if let Some(rest) = s.strip_prefix("Key") {
        return rest.to_string();
    }
    if let Some(rest) = s.strip_prefix("Digit") {
        return rest.to_string();
    }
    match key {
        KeyCode::ShiftLeft => "L-Shift".into(),
        KeyCode::ShiftRight => "R-Shift".into(),
        KeyCode::ControlLeft => "L-Ctrl".into(),
        KeyCode::ControlRight => "R-Ctrl".into(),
        KeyCode::AltLeft => "L-Alt".into(),
        _ => s,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum DisplayMode {
    #[default]
    Windowed,
    Borderless,
    Fullscreen,
}

impl DisplayMode {
    pub fn name(self) -> &'static str {
        match self {
            DisplayMode::Windowed => "Windowed",
            DisplayMode::Borderless => "Borderless",
            DisplayMode::Fullscreen => "Fullscreen",
        }
    }

    pub fn next(self) -> Self {
        match self {
            DisplayMode::Windowed => DisplayMode::Borderless,
            DisplayMode::Borderless => DisplayMode::Fullscreen,
            DisplayMode::Fullscreen => DisplayMode::Windowed,
        }
    }
}

/// How an ability key casts.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum CastMode {
    /// Casts the moment you press the key.
    Instant,
    /// Hold the key to aim, release to cast.
    #[default]
    Quick,
    /// Press to aim, left-click to cast, right-click to cancel.
    Confirm,
}

impl CastMode {
    pub fn name(self) -> &'static str {
        match self {
            CastMode::Instant => "Instant",
            CastMode::Quick => "Quick",
            CastMode::Confirm => "Confirm",
        }
    }

    pub fn next(self) -> Self {
        match self {
            CastMode::Instant => CastMode::Quick,
            CastMode::Quick => CastMode::Confirm,
            CastMode::Confirm => CastMode::Instant,
        }
    }
}

pub const RESOLUTIONS: [(u32, u32); 6] = [
    (1280, 720),
    (1366, 768),
    (1600, 900),
    (1920, 1080),
    (2560, 1440),
    (3840, 2160),
];

#[derive(Resource, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub fov: f32,
    pub sensitivity: f32,
    /// Master volume, then one per sound group (all 0-100).
    pub volume: f32,
    pub vol_guns: f32,
    pub vol_enemies: f32,
    pub vol_movement: f32,
    pub vol_effects: f32,
    pub vol_interface: f32,
    pub display: DisplayMode,
    pub resolution: usize,
    /// Graphics: shadow quality (0 off, 1 normal, 2 high), smoothed edges,
    /// glow on bright things, and soft shading in corners.
    pub shadows: u8,
    pub antialias: bool,
    pub bloom: bool,
    pub ambient_occlusion: bool,
    /// Thin drawn outlines on characters, zombies and nearby props.
    pub outlines: bool,
    /// The camera shakes from explosions and Brute slams.
    pub camera_shake: bool,
    /// Dust, pollen or ash drifting in the air.
    pub particles: bool,
    pub keys: Vec<(Action, KeyCode)>,
    /// Cast mode for ability 1, ability 2 and the ultimate.
    pub cast_modes: [CastMode; 3],
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            fov: 80.0,
            sensitivity: 1.0,
            volume: 80.0,
            vol_guns: 100.0,
            vol_enemies: 100.0,
            vol_movement: 100.0,
            vol_effects: 100.0,
            vol_interface: 100.0,
            display: DisplayMode::Windowed,
            resolution: 0,
            shadows: 2,
            antialias: true,
            bloom: true,
            ambient_occlusion: false,
            outlines: true,
            camera_shake: true,
            particles: true,
            keys: Action::ALL.iter().map(|a| (*a, a.default_key())).collect(),
            cast_modes: [CastMode::Quick; 3],
        }
    }
}

impl Settings {
    pub fn key(&self, action: Action) -> KeyCode {
        self.keys
            .iter()
            .find(|(a, _)| *a == action)
            .map(|(_, k)| *k)
            .unwrap_or_else(|| action.default_key())
    }

    pub fn set_key(&mut self, action: Action, key: KeyCode) {
        // A key can only do one thing: swap with whoever had it.
        let old = self.key(action);
        for (a, k) in self.keys.iter_mut() {
            if *k == key && *a != action {
                *k = old;
            }
        }
        match self.keys.iter_mut().find(|(a, _)| *a == action) {
            Some(entry) => entry.1 = key,
            None => self.keys.push((action, key)),
        }
    }

    /// Settings saved by an older version miss newer actions: give each one
    /// its default key, or a free key if that one is taken.
    fn with_all_keys(mut self) -> Self {
        for action in Action::ALL {
            if self.keys.iter().any(|(a, _)| *a == action) {
                continue;
            }
            let taken = |k: KeyCode, keys: &[(Action, KeyCode)]| keys.iter().any(|(_, x)| *x == k);
            let key = [
                action.default_key(),
                KeyCode::KeyT,
                KeyCode::KeyH,
                KeyCode::KeyN,
                KeyCode::KeyM,
            ]
            .into_iter()
            .find(|k| !taken(*k, &self.keys))
            .unwrap_or(action.default_key());
            self.keys.push((action, key));
        }
        self
    }

    pub fn reset_keys(&mut self) {
        self.keys = Settings::default().keys;
    }
}

/// Convenience for reading bound keys in systems.
pub trait InputExt {
    fn held(&self, settings: &Settings, action: Action) -> bool;
    fn tapped(&self, settings: &Settings, action: Action) -> bool;
}

impl InputExt for ButtonInput<KeyCode> {
    fn held(&self, settings: &Settings, action: Action) -> bool {
        self.pressed(settings.key(action))
    }
    fn tapped(&self, settings: &Settings, action: Action) -> bool {
        self.just_pressed(settings.key(action))
    }
}

#[derive(Resource, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub character: Character,
    pub spins: u32,
    pub owned_skins: Vec<u8>,
    /// Duplicate pulls; three make a free spin.
    pub shards: u32,
    pub skin: u8,
    /// Equipped gun-specific skin per gun id (255 for none).
    pub gun_skins: Vec<u8>,
    /// Premium spins, counted in quarters (4 = one spin).
    pub premium_quarters: u32,
    /// Rounds survived towards the next quarter premium spin.
    pub round_bank: u32,
    pub best_round: u32,
    pub extractions: u32,
    pub last_address: String,
    /// Career experience from every match (see progression.rs).
    pub career_xp: u32,
    /// Each class's chosen guns (primary, secondary).
    pub class_guns: HashMap<Character, [u8; 2]>,
    /// Attachments picked for each gun (by gun id).
    pub gun_attach: HashMap<u8, u8>,
    /// XP earned with each character (their character level).
    pub char_xp: HashMap<Character, u32>,
    /// The abilities picked for each character.
    /// Renamed from `kits` in v12, when every ability changed.
    pub class_kits: HashMap<Character, [Ability; 3]>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "Player".into(),
            character: Character::Bulwark,
            spins: 1,
            owned_skins: vec![0],
            shards: 0,
            skin: 0,
            gun_skins: Vec::new(),
            premium_quarters: 0,
            round_bank: 0,
            best_round: 0,
            extractions: 0,
            last_address: String::new(),
            career_xp: 0,
            class_guns: HashMap::new(),
            gun_attach: HashMap::new(),
            char_xp: HashMap::new(),
            class_kits: HashMap::new(),
        }
    }
}

impl Profile {
    /// A class's guns (primary, secondary) and their attachments, falling
    /// back to the first choices for anything not picked or not unlocked.
    pub fn class_loadout(&self, c: Character) -> [(u8, crate::data::Attach); 2] {
        let level = crate::progression::career(self.career_xp).0;
        let defaults = c.default_guns();
        let picked = self.class_guns.get(&c).copied().unwrap_or(defaults);
        std::array::from_fn(|slot| {
            let gun = if c.has_gun(slot, picked[slot])
                && crate::progression::gun_unlocked(level, picked[slot])
            {
                picked[slot]
            } else {
                defaults[slot]
            };
            let attach = crate::data::Attach(self.gun_attach.get(&gun).copied().unwrap_or(0));
            (gun, crate::progression::allowed_attach(level, gun, attach))
        })
    }

    /// The skin this player shows on `gun`.
    pub fn skin_for(&self, gun: u8) -> u8 {
        crate::data::skin_for(self.skin, &self.gun_skins, gun)
    }

    /// Level with a character: (level, XP into it, XP needed for the next).
    pub fn char_level(&self, c: Character) -> (u32, u32, u32) {
        crate::data::char_level(self.char_xp.get(&c).copied().unwrap_or(0))
    }

    /// The kit picked for a character, falling back to the starting kit if
    /// the saved one isn't allowed (yet).
    pub fn kit(&self, c: Character) -> [Ability; 3] {
        let level = self.char_level(c).0;
        self.class_kits
            .get(&c)
            .copied()
            .filter(|k| crate::data::valid_kit(c, level, *k))
            .unwrap_or(c.default_kit())
    }
}

/// "1 3/4" style text for a count of quarters.
pub fn quarters_text(q: u32) -> String {
    let (whole, part) = (q / 4, q % 4);
    let frac = ["", "1/4", "1/2", "3/4"][part as usize];
    match (whole, part) {
        (0, 0) => "0".into(),
        (0, _) => frac.into(),
        (_, 0) => whole.to_string(),
        _ => format!("{whole} {frac}"),
    }
}

fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("RUST_FPS_DATA") {
        return PathBuf::from(dir);
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata).join("RustFPS");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("rust-fps");
    }
    PathBuf::from(".")
}

fn load<T: DeserializeOwned + Default>(file: &str) -> T {
    std::fs::read_to_string(data_dir().join(file))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save<T: Serialize>(file: &str, value: &T) {
    let dir = data_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    if let Ok(json) = serde_json::to_string_pretty(value) {
        if let Err(e) = std::fs::write(dir.join(file), json) {
            warn!("couldn't save {file}: {e}");
        }
    }
}

fn save_on_change(settings: Res<Settings>, profile: Res<Profile>) {
    if settings.is_changed() && !settings.is_added() {
        save("settings.json", &*settings);
    }
    if profile.is_changed() && !profile.is_added() {
        save("profile.json", &*profile);
    }
}

/// Applies window mode and resolution whenever they change.
fn apply_display(
    settings: Res<Settings>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut last: Local<Option<(DisplayMode, usize)>>,
) {
    let current = (settings.display, settings.resolution);
    if *last == Some(current) {
        return;
    }
    *last = Some(current);
    let (w, h) = RESOLUTIONS[settings.resolution.min(RESOLUTIONS.len() - 1)];
    window.mode = match settings.display {
        DisplayMode::Windowed => WindowMode::Windowed,
        DisplayMode::Borderless => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        DisplayMode::Fullscreen => {
            WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current)
        }
    };
    if settings.display == DisplayMode::Windowed {
        window.resolution.set(w as f32, h as f32);
    }
}
