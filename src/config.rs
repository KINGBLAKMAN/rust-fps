//! Settings (FOV, sensitivity, keys, display) and the player profile (name,
//! character, skins, gacha spins). Both are saved as JSON in the user's data
//! folder: %APPDATA%\RustFPS on Windows, ~/.config/rust-fps elsewhere.

use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, VideoModeSelection, WindowMode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::PathBuf;

use crate::data::Character;

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(load::<Settings>("settings.json"))
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
    SwapWeapon,
    Upgrades,
    Scoreboard,
}

impl Action {
    pub const ALL: [Action; 15] = [
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
        Action::SwapWeapon,
        Action::Upgrades,
        Action::Scoreboard,
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
            Action::SwapWeapon => "Swap weapon",
            Action::Upgrades => "Pick level-up upgrade",
            Action::Scoreboard => "Scoreboard",
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
            Action::SwapWeapon => KeyCode::KeyV,
            Action::Upgrades => KeyCode::KeyB,
            Action::Scoreboard => KeyCode::Tab,
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
    pub volume: f32,
    pub display: DisplayMode,
    pub resolution: usize,
    pub keys: Vec<(Action, KeyCode)>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            fov: 80.0,
            sensitivity: 1.0,
            volume: 80.0,
            display: DisplayMode::Windowed,
            resolution: 0,
            keys: Action::ALL.iter().map(|a| (*a, a.default_key())).collect(),
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
    pub best_round: u32,
    pub extractions: u32,
    pub last_address: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "Player".into(),
            character: Character::Striker,
            spins: 1,
            owned_skins: vec![0],
            shards: 0,
            skin: 0,
            best_round: 0,
            extractions: 0,
            last_address: String::new(),
        }
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
