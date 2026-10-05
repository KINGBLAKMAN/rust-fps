//! Menus: the main menu, joining, character select, gun skins (gacha),
//! settings (sliders, display, key remapping), the party screen before a
//! match and the in-game pause menu. Screens are rebuilt whenever what they
//! show changes; buttons carry a `UiAction` that one system carries out.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::abilities::{queue_action, ActionCounter};
use crate::avatars::spawn_person;
use crate::config::quarters_text;
use crate::config::{key_name, Action, Profile, Settings, RESOLUTIONS};
use crate::data::{MAX_CHAR_LEVEL, 
    crate_odds, gun_def, roll_crate, skin_def, Attach, Character, CRATES, PREMIUM_TRADE_COST,
    ROUNDS_PER_PREMIUM_QUARTER,
};
use crate::game::Overlay;
use crate::maps::{map_name, MAP_NAMES};
use crate::net::{LocalReady, Notice, PartyRequest, DEFAULT_PORT, MAX_NAME_LEN};
use crate::sim::new_match;
use crate::{ActionQueue, AppState, MatchState, PlayerAction, Role, Roster, Session};

pub const PANEL: Color = Color::srgba(0.06, 0.07, 0.11, 0.92);
pub const ACCENT: Color = Color::srgb(1.0, 0.78, 0.25);
const BUTTON: Color = Color::srgb(0.16, 0.18, 0.26);
const BUTTON_HOVER: Color = Color::srgb(0.24, 0.27, 0.38);
const BUTTON_PRESS: Color = Color::srgb(0.34, 0.38, 0.52);
const DIM: Color = Color::srgb(0.68, 0.72, 0.8);
const WARN: Color = Color::srgb(1.0, 0.55, 0.35);
const PREMIUM: Color = Color::srgb(1.0, 0.55, 0.9);

mod carousel;
mod loadout;
mod sandbox;
use loadout::{guide_screen, loadout_screen};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Screen>()
            .init_resource::<Focus>()
            .init_resource::<Rebinding>()
            .init_resource::<SkinShop>()
            .init_resource::<sandbox::Tools>()
            .init_resource::<carousel::CrateSpin>()
            .add_systems(
                Update,
                (
                    autostart,
                    text_input,
                    rebind_keys,
                    menu_back_key,
                    handle_buttons,
                    drag_sliders,
                    rebuild_ui,
                    update_sliders,
                    button_colors,
                    menu_scene,
                    carousel::carousel,
                )
                    .chain(),
            );
    }
}

/// Which page of the main menu is showing.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    #[default]
    Main,
    Join,
    Characters,
    Crates,
    GunSkins,
    Settings,
    Loadout,
    /// The attachment guide, on a tab (0 all, then each slot).
    Guide(u8),
}

/// Which text box is being typed into.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
enum Focus {
    #[default]
    None,
    Name,
    Address,
}

/// The action waiting for a new key, if any, and the settings tab shown
/// (0 general, 1 audio, 2 graphics, 3 controls).
#[derive(Resource, Default)]
struct Rebinding(Option<Action>, u8);

/// The big turning gun on the crates and gun skins screens.
#[derive(Component)]
struct ShowcaseGun;

/// The crates and gun skins screens: which crate is open, the last pull
/// (skin, was it new), the skin being previewed, any message, and the gun
/// picked in Gun Skins (`ALL_GUNS` for the default finish).
#[derive(Resource, Default, Debug)]
struct SkinShop {
    crate_id: usize,
    gun: u8,
    last: Option<(u8, bool)>,
    preview: Option<u8>,
    message: Option<String>,
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub enum UiAction {
    PlaySolo,
    Host,
    OpenJoin,
    Connect,
    OpenCharacters,
    OpenSkins,
    OpenSettings,
    BackToMain,
    Quit,
    FocusName,
    FocusAddress,
    SelectCharacter(Character),
    /// Put an ability in slot 0, 1 or 2 (ultimate) of this character's kit.
    PickAbility(u8, crate::data::Ability),
    SelectCrate(u8),
    OpenCrate,
    TradePremium,
    PreviewSkin(u8),
    OpenGunSkins,
    SelectGun(u8),
    /// Put a skin on a gun (`ALL_GUNS` for the default finish); 255 takes
    /// it off.
    ApplySkin(u8, u8),
    CycleDisplay,
    CycleResolution(i8),
    Rebind(Action),
    CycleCast(u8),
    ResetKeys,
    SettingsBack,
    SettingsTab(u8),
    CycleGraphics(u8),
    CycleMap(i8),
    ToggleNight,
    ToggleReady,
    StartMatch,
    Leave,
    Resume,
    PauseSettings,
    ChooseUpgrade(u8),
    BackToLobby,
    OpenLoadout,
    /// Bring `gun` in slot 0 (primary) or 1 (secondary).
    LoadoutGun(u8, u8),
    /// On the gun in a slot, fit attachment `id` in an attachment slot (0
    /// takes it off).
    LoadoutAttach(u8, u8, u8),
    OpenGuide(u8),
    /// Show or hide the host's internet address.
    RevealIp,
    PlaySandbox,
    /// Something locked: clicking does nothing.
    Locked,
    SandboxSpawn(u8, u8),
    SandboxToggle(u8),
    SandboxPoints,
    SandboxLevel,
    SandboxRound,
    SandboxKill,
    SandboxGun(u8),
    SandboxAttach(u8, u8),
    SandboxGive,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum Slider {
    Fov,
    Sensitivity,
    Volume,
    Guns,
    Enemies,
    Movement,
    Effects,
    Interface,
}

impl Slider {
    fn range(self) -> (f32, f32) {
        match self {
            Slider::Fov => (60.0, 120.0),
            Slider::Sensitivity => (0.1, 3.0),
            _ => (0.0, 100.0),
        }
    }

    fn get(self, s: &Settings) -> f32 {
        match self {
            Slider::Fov => s.fov,
            Slider::Sensitivity => s.sensitivity,
            Slider::Volume => s.volume,
            Slider::Guns => s.vol_guns,
            Slider::Enemies => s.vol_enemies,
            Slider::Movement => s.vol_movement,
            Slider::Effects => s.vol_effects,
            Slider::Interface => s.vol_interface,
        }
    }

    fn set(self, s: &mut Settings, v: f32) {
        match self {
            Slider::Fov => s.fov = v.round(),
            Slider::Sensitivity => s.sensitivity = (v * 20.0).round() / 20.0,
            Slider::Volume => s.volume = v.round(),
            Slider::Guns => s.vol_guns = v.round(),
            Slider::Enemies => s.vol_enemies = v.round(),
            Slider::Movement => s.vol_movement = v.round(),
            Slider::Effects => s.vol_effects = v.round(),
            Slider::Interface => s.vol_interface = v.round(),
        }
    }

    fn label(self, s: &Settings) -> String {
        match self {
            Slider::Fov => format!("Field of view: {:.0}", s.fov),
            Slider::Sensitivity => format!("Mouse sensitivity: {:.2}", s.sensitivity),
            Slider::Volume => format!("Master volume: {:.0}%", s.volume),
            Slider::Guns => format!("Guns and hits: {:.0}%", s.vol_guns),
            Slider::Enemies => format!("Zombies: {:.0}%", s.vol_enemies),
            Slider::Movement => format!("Footsteps and movement: {:.0}%", s.vol_movement),
            Slider::Effects => format!("Abilities and explosions: {:.0}%", s.vol_effects),
            Slider::Interface => format!("Box, rounds, menus and music: {:.0}%", s.vol_interface),
        }
    }
}

#[derive(Component)]
struct SliderFill(Slider);

#[derive(Component)]
struct SliderLabel(Slider);

/// The root of whatever menu is on screen.
#[derive(Component)]
struct UiRoot;

/// Highlighted (selected) buttons.
#[derive(Component)]
struct Selected;

/// The 3D characters shown behind the menus.
#[derive(Component)]
struct MenuScene;

#[derive(Component)]
struct Turntable(f32, f32);

// ---------------------------------------------------------------------------
// Building blocks
// ---------------------------------------------------------------------------

fn label(p: &mut ChildSpawnerCommands, value: impl Into<String>, size: f32, color: Color) {
    p.spawn((
        Text::new(value),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
    ));
}

/// A clickable button that does `action`.
pub fn button(p: &mut ChildSpawnerCommands, value: impl Into<String>, action: UiAction) {
    button_sized(p, value, action, None, false);
}

fn button_sized(
    p: &mut ChildSpawnerCommands,
    value: impl Into<String>,
    action: UiAction,
    width: Option<f32>,
    selected: bool,
) {
    let mut e = p.spawn((
        Button,
        action,
        Node {
            width: width.map_or(Val::Auto, Val::Px),
            min_width: Val::Px(if width.is_some() { 0.0 } else { 260.0 }),
            padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(BUTTON),
        BorderColor(if selected { ACCENT } else { Color::NONE }),
        BorderRadius::all(Val::Px(6.0)),
    ));
    if selected {
        e.insert(Selected);
    }
    e.with_children(|b| {
        b.spawn((
            Text::new(value),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::WHITE),
            TextLayout::new_with_justify(JustifyText::Center),
        ));
    });
}

fn row(p: &mut ChildSpawnerCommands, f: impl FnOnce(&mut ChildSpawnerCommands)) {
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(8.0),
        flex_wrap: FlexWrap::Wrap,
        row_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(f);
}

fn slider(p: &mut ChildSpawnerCommands, kind: Slider, settings: &Settings) {
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(4.0),
        ..default()
    })
    .with_children(|c| {
        c.spawn((
            SliderLabel(kind),
            Text::new(kind.label(settings)),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
        c.spawn((
            Button,
            kind,
            RelativeCursorPosition::default(),
            Node {
                width: Val::Px(380.0),
                height: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(BUTTON),
            BorderRadius::all(Val::Px(8.0)),
        ))
        .with_children(|t| {
            t.spawn((
                SliderFill(kind),
                Node {
                    width: Val::Percent(50.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(ACCENT),
                BorderRadius::all(Val::Px(8.0)),
                Pickable::IGNORE,
            ));
        });
    });
}

/// A full-height panel down the left side (menus) or a centred box (in game).
fn panel(commands: &mut Commands, centered: bool, f: impl FnOnce(&mut ChildSpawnerCommands)) {
    let outer = if centered {
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        }
    } else {
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        }
    };
    let bg = if centered {
        Color::srgba(0.0, 0.0, 0.0, 0.5)
    } else {
        Color::NONE
    };
    commands
        .spawn((UiRoot, outer, BackgroundColor(bg), GlobalZIndex(10)))
        .with_children(|o| {
            o.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(9.0),
                    padding: UiRect::all(Val::Px(28.0)),
                    min_width: Val::Px(480.0),
                    height: if centered {
                        Val::Auto
                    } else {
                        Val::Percent(100.0)
                    },
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderRadius::all(Val::Px(if centered { 10.0 } else { 0.0 })),
            ))
            .with_children(f);
        });
}

fn title(p: &mut ChildSpawnerCommands, sub: &str) {
    label(p, "RUST FPS", 52.0, Color::srgb(0.9, 0.2, 0.15));
    label(p, sub, 20.0, ACCENT);
}

// ---------------------------------------------------------------------------
// Screens
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn rebuild_ui(
    mut commands: Commands,
    app_state: Res<State<AppState>>,
    screen: Res<Screen>,
    focus: Res<Focus>,
    rebinding: Res<Rebinding>,
    spin: Res<SkinShop>,
    overlay: Res<Overlay>,
    (profile, settings, notice, ready): (Res<Profile>, Res<Settings>, Res<Notice>, Res<LocalReady>),
    (session, roster, state): (Res<Session>, Res<Roster>, Res<MatchState>),
    roots: Query<Entity, With<UiRoot>>,
    mut last: Local<String>,
    tools: Res<sandbox::Tools>,
    public_ip: Res<crate::net::PublicIp>,
) {
    let app = app_state.get().clone();
    // Everything the current screen shows, so we only rebuild on changes.
    let sig = match app {
        AppState::Menu => format!(
            "menu {:?} {:?} {:?} {:?} {:?} {} {} {} {} {} {:?} {} {} {} {:?} {:?} {:?} {} {:?}",
            *screen,
            *focus,
            (
                rebinding.0,
                rebinding.1,
                settings.shadows,
                settings.antialias,
                settings.bloom,
                settings.ambient_occlusion,
                settings.outlines,
                settings.camera_shake,
                settings.particles
            ),
            *spin,
            (profile.character, profile.kit(profile.character), profile.char_level(profile.character)),
            profile.name,
            profile.last_address,
            profile.spins,
            profile.shards,
            profile.skin,
            profile.owned_skins,
            notice.0,
            settings.display.name(),
            settings.resolution,
            settings.keys,
            settings.cast_modes,
            profile.gun_skins,
            profile.premium_quarters,
            (
                profile.round_bank,
                profile.career_xp,
                profile.class_loadout(profile.character),
            )
        ),
        AppState::Lobby => {
            let players: Vec<_> = roster
                .0
                .values()
                .map(|p| (p.id, p.name.clone(), p.character, p.ready))
                .collect();
            format!(
                "lobby {:?} {} {} {} {} {:?} {} {:?} {} {:?}",
                players,
                state.map,
                state.night,
                session.status,
                session.connected,
                (profile.character, profile.kit(profile.character)),
                ready.0,
                session.role,
                profile.skin,
                (
                    public_ip.visible,
                    &public_ip.ip,
                    &public_ip.error,
                    public_ip.looking()
                )
            )
        }
        AppState::Travel => "travel".to_string(),
        AppState::InGame => format!(
            "game {:?} {:?} {:?} {:?} {} {} {:?} {:?}",
            *overlay,
            (state.sandbox, state.round),
            *tools,
            (
                rebinding.0,
                rebinding.1,
                settings.shadows,
                settings.antialias,
                settings.bloom,
                settings.ambient_occlusion,
                settings.outlines,
                settings.camera_shake,
                settings.particles
            ),
            settings.display.name(),
            settings.resolution,
            settings.keys,
            settings.cast_modes
        ),
    };
    if *last == sig {
        return;
    }
    *last = sig;
    for e in &roots {
        commands.entity(e).despawn();
    }

    match app {
        AppState::Menu => match *screen {
            Screen::Main => main_screen(&mut commands, &profile, &notice, *focus),
            Screen::Join => join_screen(&mut commands, &profile, &notice, *focus),
            Screen::Characters => character_screen(&mut commands, &profile, &settings),
            Screen::Crates => crates_screen(&mut commands, &profile, &spin),
            Screen::GunSkins => gun_skins_screen(&mut commands, &profile, &spin),
            Screen::Settings => panel(&mut commands, false, |p| {
                settings_body(p, &settings, rebinding.0, rebinding.1);
            }),
            Screen::Loadout => loadout_screen(&mut commands, &profile),
            Screen::Guide(tab) => guide_screen(&mut commands, &profile, tab),
        },
        AppState::Lobby => lobby_screen(
            &mut commands,
            &session,
            &roster,
            &state,
            &profile,
            ready.0,
            &public_ip,
        ),
        AppState::Travel => {}
        AppState::InGame => match *overlay {
            Overlay::Pause => {
                let solo = session.role == Role::Solo;
                panel(&mut commands, true, |p| {
                    label(p, if solo { "PAUSED" } else { "MENU" }, 40.0, ACCENT);
                    if !solo {
                        label(p, "The match keeps going while this is open!", 15.0, WARN);
                    }
                    button(p, "Resume", UiAction::Resume);
                    button(p, "Settings", UiAction::PauseSettings);
                    button(
                        p,
                        if session.role == Role::Host {
                            "End party and quit to menu"
                        } else {
                            "Quit to main menu"
                        },
                        UiAction::Leave,
                    );
                });
            }
            Overlay::Settings => panel(&mut commands, true, |p| {
                settings_body(p, &settings, rebinding.0, rebinding.1);
            }),
            Overlay::Sandbox => sandbox::panel(&mut commands, &state, &tools),
            _ => {}
        },
    }
}

fn text_field(
    p: &mut ChildSpawnerCommands,
    value: &str,
    focused: bool,
    action: UiAction,
    placeholder: &str,
) {
    let shown = if value.is_empty() && !focused {
        placeholder.to_string()
    } else if focused {
        format!("{value}_")
    } else {
        value.to_string()
    };
    p.spawn((
        Button,
        action,
        Node {
            width: Val::Px(340.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.11, 0.16)),
        BorderColor(if focused {
            ACCENT
        } else {
            Color::srgb(0.3, 0.32, 0.4)
        }),
        BorderRadius::all(Val::Px(6.0)),
    ))
    .with_children(|b| {
        b.spawn((
            Text::new(shown),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(if value.is_empty() && !focused {
                DIM
            } else {
                Color::WHITE
            }),
        ));
    });
}

fn main_screen(commands: &mut Commands, profile: &Profile, notice: &Notice, focus: Focus) {
    panel(commands, false, |p| {
        title(p, "Co-op wave survival");
        p.spawn(Node {
            height: Val::Px(8.0),
            ..default()
        });
        label(p, "Your name (click to change)", 15.0, DIM);
        text_field(
            p,
            &profile.name,
            focus == Focus::Name,
            UiAction::FocusName,
            "Player",
        );
        p.spawn(Node {
            height: Val::Px(6.0),
            ..default()
        });
        button(p, "Play Solo", UiAction::PlaySolo);
        button(p, "Host a Party", UiAction::Host);
        button(p, "Join a Party", UiAction::OpenJoin);
        let brought = profile.class_loadout(profile.character);
        button(
            p,
            format!(
                "Loadout  ({} + {})",
                gun_def(brought[0].0).name,
                gun_def(brought[1].0).name
            ),
            UiAction::OpenLoadout,
        );
        button(
            p,
            format!("Characters  ({})", profile.character.name()),
            UiAction::OpenCharacters,
        );
        button(
            p,
            format!(
                "Gun Crates  ({} spin{})",
                profile.spins,
                if profile.spins == 1 { "" } else { "s" }
            ),
            UiAction::OpenSkins,
        );
        button(p, "Gun Skins", UiAction::OpenGunSkins);
        row(p, |r| {
            button_sized(
                r,
                "Attachment Guide",
                UiAction::OpenGuide(0),
                Some(344.0),
                false,
            );
            button_sized(r, "Sandbox", UiAction::PlaySandbox, Some(344.0), false);
        });
        row(p, |r| {
            button_sized(r, "Settings", UiAction::OpenSettings, Some(344.0), false);
            button_sized(r, "Quit", UiAction::Quit, Some(344.0), false);
        });
        p.spawn(Node {
            height: Val::Px(6.0),
            ..default()
        });
        let (level, into, need) = crate::progression::career(profile.career_xp);
        label(
            p,
            format!(
                "Career level {level} ({into}/{need} XP)    Best round: {}    Runs won: {}    Version {}",
                profile.best_round, profile.extractions, crate::VERSION
            ),
            15.0,
            DIM,
        );
        if !notice.0.is_empty() {
            label(p, notice.0.clone(), 16.0, WARN);
        }
    });
}

fn join_screen(commands: &mut Commands, profile: &Profile, notice: &Notice, focus: Focus) {
    panel(commands, false, |p| {
        title(p, "Join a party");
        label(p, "Host's address", 15.0, DIM);
        text_field(
            p,
            &profile.last_address,
            focus == Focus::Address,
            UiAction::FocusAddress,
            "e.g. 192.168.1.20",
        );
        label(
            p,
            format!(
                "The host's party screen shows the address to type.\nSame Wi-Fi works out of the box. Over the internet the host\nneeds to forward UDP port {DEFAULT_PORT} (or use a VPN like Tailscale)."
            ),
            15.0,
            DIM,
        );
        button(p, "Connect", UiAction::Connect);
        button(p, "Back", UiAction::BackToMain);
        if !notice.0.is_empty() {
            label(p, notice.0.clone(), 16.0, WARN);
        }
    });
}

fn character_screen(commands: &mut Commands, profile: &Profile, settings: &Settings) {
    panel(commands, false, |p| {
        title(p, "Choose your character");
        row(p, |r| {
            for c in Character::ALL {
                let level = profile.char_level(c).0;
                button_sized(
                    r,
                    format!("{}  Lv {level}", c.name()),
                    UiAction::SelectCharacter(c),
                    Some(150.0),
                    profile.character == c,
                );
            }
        });
        let c = profile.character;
        let (level, into, need) = profile.char_level(c);
        let kit = profile.kit(c);
        let keys = [Action::Ability1, Action::Ability2, Action::Ultimate];
        p.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(14.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.13, 0.19, 0.9)),
            BorderColor(ACCENT),
            BorderRadius::all(Val::Px(8.0)),
        ))
        .with_children(|card| {
            row(card, |r| {
                label(r, c.name(), 30.0, c.suit_color().lighter(0.2));
                label(r, c.tagline(), 16.0, DIM);
            });
            let xp = if level >= MAX_CHAR_LEVEL {
                format!("Level {level} (max)")
            } else {
                format!("Level {level}   {into} / {need} XP to level {}", level + 1)
            };
            label(card, xp, 16.0, Color::srgb(0.55, 0.85, 1.0));
            bar(card, if level >= MAX_CHAR_LEVEL { 1.0 } else { into as f32 / need as f32 });
            // Pick an ability for each key. Locked ones show their level.
            row(card, |cols| {
                for slot in 0..3usize {
                    cols.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.0),
                        width: Val::Px(250.0),
                        ..default()
                    })
                    .with_children(|col| {
                        let head = if slot == 2 { "Ultimate" } else { "Ability" };
                        label(
                            col,
                            format!("{head} [{}]", key_name(settings.key(keys[slot]))),
                            16.0,
                            ACCENT,
                        );
                        for a in c.pool().into_iter().filter(|a| a.is_ult() == (slot == 2)) {
                            let unlock = a.def().unlock;
                            if unlock > level {
                                button_sized(
                                    col,
                                    format!("{}  (Lv {unlock})", a.name()),
                                    UiAction::Locked,
                                    Some(250.0),
                                    false,
                                );
                            } else {
                                button_sized(
                                    col,
                                    a.name(),
                                    UiAction::PickAbility(slot as u8, a),
                                    Some(250.0),
                                    kit[slot] == a,
                                );
                            }
                        }
                    });
                }
            });
            for (slot, a) in kit.iter().enumerate() {
                label(
                    card,
                    format!("[{}] {}: {}", key_name(settings.key(keys[slot])), a.name(), a.def().desc),
                    15.0,
                    Color::WHITE,
                );
            }
        });
        label(
            p,
            "Play matches as a character to level them up and unlock new abilities. Abilities also get stronger as you level up in a match.",
            15.0,
            DIM,
        );
        button(p, "Back", UiAction::BackToMain);
    });
}

/// A thin progress bar.
fn bar(p: &mut ChildSpawnerCommands, frac: f32) {
    p.spawn((
        Node {
            width: Val::Px(400.0),
            height: Val::Px(8.0),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
        BorderRadius::all(Val::Px(4.0)),
    ))
    .with_child((
        Node {
            width: Val::Percent(frac.clamp(0.0, 1.0) * 100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.55, 0.85, 1.0)),
        BorderRadius::all(Val::Px(4.0)),
    ));
}

fn crates_screen(commands: &mut Commands, profile: &Profile, shop: &SkinShop) {
    let c = &CRATES[shop.crate_id];
    panel(commands, false, |p| {
        label(p, "GUN CRATES", 34.0, ACCENT);
        label(
            p,
            format!(
                "Spins: {}    Premium spins: {}    Duplicate shards: {}/3",
                profile.spins,
                quarters_text(profile.premium_quarters),
                profile.shards
            ),
            18.0,
            Color::WHITE,
        );
        row(p, |r| {
            button_sized(
                r,
                format!("Trade {PREMIUM_TRADE_COST} spins for 1 premium spin"),
                UiAction::TradePremium,
                Some(330.0),
                false,
            );
            label(
                r,
                format!(
                    "Also: +1/4 premium spin every\n{ROUNDS_PER_PREMIUM_QUARTER} rounds survived ({}/{ROUNDS_PER_PREMIUM_QUARTER})",
                    profile.round_bank
                ),
                13.0,
                DIM,
            );
        });
        row(p, |r| {
            for (i, cr) in CRATES.iter().enumerate() {
                let selected = i == shop.crate_id;
                r.spawn((
                    Button,
                    UiAction::SelectCrate(i as u8),
                    Node {
                        width: Val::Px(104.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(4.0), Val::Px(5.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(BUTTON),
                    BorderColor(if selected {
                        ACCENT
                    } else if cr.premium {
                        PREMIUM.with_alpha(0.6)
                    } else {
                        Color::NONE
                    }),
                    BorderRadius::all(Val::Px(6.0)),
                ))
                .with_children(|b| {
                    label(b, cr.name, 14.0, Color::WHITE);
                    let owned = cr
                        .skins
                        .iter()
                        .filter(|s| profile.owned_skins.contains(s))
                        .count();
                    let tag = if cr.premium { "PREMIUM" } else { "Regular" };
                    label(
                        b,
                        format!("{tag}  {owned}/{}", cr.skins.len()),
                        11.0,
                        if cr.premium { PREMIUM } else { DIM },
                    );
                });
            }
        });
        label(
            p,
            c.blurb,
            14.0,
            if c.premium { PREMIUM } else { Color::WHITE },
        );
        let odds: Vec<String> = crate_odds(shop.crate_id)
            .into_iter()
            .map(|(r, pct)| format!("{} {:.0}%", r.name(), pct))
            .collect();
        label(p, format!("Odds: {}", odds.join(", ")), 13.0, DIM);
        let all = c.skins.iter().all(|s| profile.owned_skins.contains(s));
        let (cost, have) = if c.premium {
            ("1 premium spin", profile.premium_quarters >= 4)
        } else {
            ("1 spin", profile.spins > 0)
        };
        if all {
            label(p, "You own everything in this crate!", 16.0, ACCENT);
        } else if have {
            button(p, format!("Open {} ({cost})", c.name), UiAction::OpenCrate);
        } else if c.premium {
            label(
                p,
                "No premium spins - trade spins for one or keep surviving rounds.",
                15.0,
                WARN,
            );
        } else {
            label(
                p,
                "No spins left - earn more by surviving rounds.",
                15.0,
                WARN,
            );
        }
        if let Some((id, new)) = shop.last {
            let s = skin_def(id);
            let what = if new {
                "NEW!"
            } else if c.premium {
                "Duplicate (+1/4 premium spin)"
            } else {
                "Duplicate (+1 shard)"
            };
            label(
                p,
                format!("You got: {} ({}) {what}", s.name, s.rarity.name()),
                18.0,
                s.rarity.color(),
            );
        } else if let Some(msg) = &shop.message {
            label(p, msg.clone(), 15.0, WARN);
        }
        p.spawn(Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
            column_gap: Val::Px(6.0),
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|g| {
            for &id in c.skins {
                let owned = profile.owned_skins.contains(&id);
                let s = skin_def(id);
                let fits = s.gun.map_or("All guns", |g| gun_def(g).name);
                skin_tile(g, id, owned, shop.preview == Some(id), fits, UiAction::PreviewSkin(id));
            }
        });
        label(
            p,
            "Click a skin you own to see it on the right. Put skins on your guns in\nGun Skins on the main menu. Round 5 = 1 spin, round 10 = 3,\nround 15 = 6, round 20 = 10.",
            12.0,
            DIM,
        );
        button(p, "Back", UiAction::BackToMain);
    });
}

/// One skin in a grid: a colour swatch, its name and a line underneath.
fn skin_tile(
    g: &mut ChildSpawnerCommands,
    id: u8,
    owned: bool,
    selected: bool,
    under: &str,
    action: UiAction,
) {
    let s = skin_def(id);
    g.spawn((
        Button,
        action,
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(4.0)),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(BUTTON),
        BorderColor(if selected {
            ACCENT
        } else {
            s.rarity.color().with_alpha(0.4)
        }),
        BorderRadius::all(Val::Px(6.0)),
    ))
    .with_children(|b| {
        // A two-tone swatch for patterned skins.
        b.spawn(Node {
            flex_direction: FlexDirection::Row,
            ..default()
        })
        .with_children(|sw| {
            let rgb = |c: [f32; 3]| Color::srgb(c[0], c[1], c[2]);
            let locked = Color::srgb(0.2, 0.2, 0.22);
            let parts = if s.gun.is_some() {
                [s.color, s.accent]
            } else {
                [s.color, s.color]
            };
            for part in parts {
                sw.spawn((
                    Node {
                        width: Val::Px(28.0),
                        height: Val::Px(12.0),
                        ..default()
                    },
                    BackgroundColor(if owned { rgb(part) } else { locked }),
                ));
            }
        });
        let name = if owned { s.name } else { "???" };
        label(b, name, 13.0, if owned { Color::WHITE } else { DIM });
        label(
            b,
            format!("{} - {under}", s.rarity.name()),
            10.0,
            s.rarity.color(),
        );
    });
}

/// Stands in for a gun in Gun Skins: the default finish every gun uses
/// unless it has its own skin.
pub const ALL_GUNS: u8 = 254;

/// Gun Skins: pick a gun, then put one of your skins on it.
fn gun_skins_screen(commands: &mut Commands, profile: &Profile, shop: &SkinShop) {
    let gun = shop.gun;
    panel(commands, false, |p| {
        label(p, "GUN SKINS", 34.0, ACCENT);
        label(
            p,
            "Pick a gun, then click a skin to put it on. It shows in game, for you and your party.",
            14.0,
            DIM,
        );
        p.spawn(Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::flex(5, 1.0),
            column_gap: Val::Px(4.0),
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|g| {
            let guns = std::iter::once(ALL_GUNS).chain(0..crate::data::GUNS.len() as u8);
            for id in guns {
                let (name, skin) = if id == ALL_GUNS {
                    ("Default finish", profile.skin)
                } else {
                    (gun_def(id).name, profile.skin_for(id))
                };
                let s = skin_def(skin);
                g.spawn((
                    Button,
                    UiAction::SelectGun(id),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(3.0), Val::Px(3.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(BUTTON),
                    BorderColor(if id == gun { ACCENT } else { Color::NONE }),
                    BorderRadius::all(Val::Px(5.0)),
                ))
                .with_children(|b| {
                    label(b, name, 12.0, Color::WHITE);
                    label(b, s.name, 10.0, s.rarity.color());
                });
            }
        });
        let (title, current) = if gun == ALL_GUNS {
            ("Default finish (guns with no skin of their own)".to_string(), profile.skin)
        } else {
            (format!("Skins for the {}", gun_def(gun).name), profile.skin_for(gun))
        };
        label(p, title, 18.0, ACCENT);
        // Skins that fit: the default finish takes skins for every gun; a
        // gun also takes its own.
        let fits: Vec<u8> = (0..crate::data::SKINS.len() as u8)
            .filter(|&id| profile.owned_skins.contains(&id))
            .filter(|&id| {
                if gun == ALL_GUNS {
                    skin_def(id).gun.is_none()
                } else {
                    crate::data::skin_fits(id, gun)
                }
            })
            .collect();
        let own = gun != ALL_GUNS && profile.gun_skins.get(gun as usize).is_some_and(|&s| crate::data::skin_fits(s, gun));
        p.spawn(Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::flex(5, 1.0),
            column_gap: Val::Px(5.0),
            row_gap: Val::Px(5.0),
            ..default()
        })
        .with_children(|g| {
            if gun != ALL_GUNS {
                g.spawn((
                    Button,
                    UiAction::ApplySkin(gun, 255),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        padding: UiRect::all(Val::Px(4.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(BUTTON),
                    BorderColor(if own { Color::NONE } else { ACCENT }),
                    BorderRadius::all(Val::Px(6.0)),
                ))
                .with_children(|b| {
                    label(b, "Default", 13.0, Color::WHITE);
                    label(b, skin_def(profile.skin).name, 10.0, DIM);
                });
            }
            for id in fits {
                let made_for = if skin_def(id).gun.is_some() { "Made for it" } else { "All guns" };
                let on = if gun == ALL_GUNS { profile.skin == id } else { own && current == id };
                skin_tile(g, id, true, on, made_for, UiAction::ApplySkin(gun, id));
            }
        });
        if !profile.owned_skins.iter().any(|&id| id != 0) {
            label(p, "Open Gun Crates to win more skins.", 14.0, WARN);
        }
        button(p, "Back", UiAction::BackToMain);
    });
}

fn settings_body(
    p: &mut ChildSpawnerCommands,
    settings: &Settings,
    rebinding: Option<Action>,
    tab: u8,
) {
    label(p, "SETTINGS", 34.0, ACCENT);
    row(p, |r| {
        for (i, name) in ["General", "Audio", "Graphics", "Controls"]
            .iter()
            .enumerate()
        {
            button_sized(
                r,
                *name,
                UiAction::SettingsTab(i as u8),
                Some(120.0),
                tab == i as u8,
            );
        }
    });
    match tab {
        1 => {
            slider(p, Slider::Volume, settings);
            slider(p, Slider::Guns, settings);
            slider(p, Slider::Enemies, settings);
            slider(p, Slider::Movement, settings);
            slider(p, Slider::Effects, settings);
            slider(p, Slider::Interface, settings);
        }
        2 => {
            let shadows = ["Off", "Normal", "High"][settings.shadows.min(2) as usize];
            let on = |b: bool| if b { "On" } else { "Off" };
            for (name, value, action, help) in [
                (
                    "Shadows",
                    shadows,
                    UiAction::CycleGraphics(0),
                    "High: sharper shadows that reach further.",
                ),
                (
                    "Anti-aliasing",
                    on(settings.antialias),
                    UiAction::CycleGraphics(1),
                    "Smooths jagged edges.",
                ),
                (
                    "Bloom",
                    on(settings.bloom),
                    UiAction::CycleGraphics(2),
                    "Lights, muzzle flashes and beams glow.",
                ),
                (
                    "Ambient occlusion",
                    on(settings.ambient_occlusion),
                    UiAction::CycleGraphics(3),
                    "Soft shadows in corners and creases (slower; turns anti-aliasing off).",
                ),
                (
                    "Outlines",
                    on(settings.outlines),
                    UiAction::CycleGraphics(4),
                    "Thin drawn lines round characters, zombies and nearby props.",
                ),
                (
                    "Camera shake",
                    on(settings.camera_shake),
                    UiAction::CycleGraphics(5),
                    "Explosions and Brute slams shake the view.",
                ),
                (
                    "Air particles",
                    on(settings.particles),
                    UiAction::CycleGraphics(6),
                    "Dust, pollen or ash drifting in the air.",
                ),
            ] {
                row(p, |r| {
                    label(r, format!("{name}:"), 16.0, Color::WHITE);
                    button_sized(r, value, action, Some(100.0), false);
                    label(r, help, 13.0, DIM);
                });
            }
        }
        3 => {
            label(
                p,
                "Controls (click one, then press the new key):",
                16.0,
                Color::WHITE,
            );
            p.spawn(Node {
                display: Display::Grid,
                grid_template_columns: vec![
                    GridTrack::px(150.0),
                    GridTrack::px(100.0),
                    GridTrack::px(150.0),
                    GridTrack::px(100.0),
                ],
                column_gap: Val::Px(6.0),
                row_gap: Val::Px(4.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|g| {
                for a in Action::ALL {
                    label(g, a.label(), 14.0, DIM);
                    let text = if rebinding == Some(a) {
                        "press a key".to_string()
                    } else {
                        key_name(settings.key(a))
                    };
                    g.spawn((
                        Button,
                        UiAction::Rebind(a),
                        Node {
                            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                            justify_content: JustifyContent::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(BUTTON),
                        BorderColor(if rebinding == Some(a) {
                            ACCENT
                        } else {
                            Color::NONE
                        }),
                        BorderRadius::all(Val::Px(4.0)),
                    ))
                    .with_children(|b| label(b, text, 14.0, Color::WHITE));
                }
            });
            label(
                p,
                "Fixed: mouse to shoot, right mouse to aim, 1 / 2 or scroll to switch guns, Esc for menu.",
                13.0,
                DIM,
            );
            row(p, |r| {
                label(r, "Casting:", 16.0, Color::WHITE);
                for (i, name) in ["Ability 1", "Ability 2", "Ultimate"].iter().enumerate() {
                    let text = format!("{name}: {}", settings.cast_modes[i].name());
                    button_sized(r, &text, UiAction::CycleCast(i as u8), Some(170.0), false);
                }
            });
            label(
                p,
                "Instant: casts on press. Quick: hold to aim, release to cast. Confirm: press to aim, click to cast, right-click to cancel.",
                13.0,
                DIM,
            );
            button_sized(p, "Reset controls", UiAction::ResetKeys, Some(210.0), false);
        }
        _ => {
            slider(p, Slider::Fov, settings);
            slider(p, Slider::Sensitivity, settings);
            row(p, |r| {
                label(r, "Display:", 16.0, Color::WHITE);
                button_sized(
                    r,
                    settings.display.name(),
                    UiAction::CycleDisplay,
                    Some(150.0),
                    false,
                );
                label(r, "Resolution:", 16.0, Color::WHITE);
                button_sized(r, "<", UiAction::CycleResolution(-1), Some(36.0), false);
                let (w, h) = RESOLUTIONS[settings.resolution.min(RESOLUTIONS.len() - 1)];
                label(r, format!("{w}x{h}"), 16.0, Color::WHITE);
                button_sized(r, ">", UiAction::CycleResolution(1), Some(36.0), false);
            });
            label(
                p,
                "Resolution applies in Windowed mode; Borderless and Fullscreen use your screen's.",
                13.0,
                DIM,
            );
        }
    }
    button_sized(p, "Back", UiAction::SettingsBack, Some(120.0), false);
}

fn lobby_screen(
    commands: &mut Commands,
    session: &Session,
    roster: &Roster,
    state: &MatchState,
    profile: &Profile,
    ready: bool,
    public_ip: &crate::net::PublicIp,
) {
    let authority = session.is_authority();
    panel(commands, false, |p| {
        let heading = match session.role {
            Role::Solo if state.sandbox.on => "Sandbox",
            Role::Solo => "Solo game",
            Role::Host => "Your party",
            Role::Client => "Party",
        };
        title(p, heading);
        if !session.status.is_empty() {
            label(p, session.status.clone(), 16.0, Color::srgb(0.5, 0.9, 1.0));
        }
        if session.role == Role::Host {
            row(p, |r| {
                label(r, "Over the internet:", 16.0, Color::WHITE);
                let text = if !public_ip.visible {
                    "Click to reveal".to_string()
                } else if let Some(a) = public_ip.address() {
                    a
                } else if public_ip.looking() {
                    "Looking it up...".to_string()
                } else {
                    "Couldn't look it up - click to retry".to_string()
                };
                button_sized(r, text, UiAction::RevealIp, Some(300.0), public_ip.visible);
            });
            if public_ip.visible {
                label(
                    p,
                    format!(
                        "Keep this private. Friends outside your Wi-Fi need you to forward\nUDP port {} on your router (or use a VPN like Tailscale).",
                        public_ip.port
                    ),
                    13.0,
                    DIM,
                );
            }
        }
        if session.role == Role::Client && !session.connected {
            label(p, "Waiting for the host to answer...", 16.0, DIM);
        }
        if session.role != Role::Solo {
            label(
                p,
                format!("Players ({}):", roster.0.len()),
                17.0,
                Color::WHITE,
            );
            for pl in roster.0.values() {
                let host = if pl.id == 0 { " (host)" } else { "" };
                let you = if pl.id == session.my_id { " - you" } else { "" };
                let r = if pl.id == 0 || pl.ready {
                    "Ready"
                } else {
                    "Not ready"
                };
                label(
                    p,
                    format!(
                        "  {}{host}{you}  -  {}  -  {r}",
                        pl.name,
                        pl.character.name()
                    ),
                    16.0,
                    if pl.ready || pl.id == 0 {
                        Color::srgb(0.5, 1.0, 0.6)
                    } else {
                        DIM
                    },
                );
            }
        }
        label(p, "Character:", 17.0, Color::WHITE);
        row(p, |r| {
            for c in Character::ALL {
                button_sized(
                    r,
                    c.name(),
                    UiAction::SelectCharacter(c),
                    Some(104.0),
                    profile.character == c,
                );
            }
        });
        let ab = profile.kit(profile.character);
        label(
            p,
            format!(
                "Level {} - {}, {}, ultimate: {} (change in Characters)",
                profile.char_level(profile.character).0,
                ab[0].name(),
                ab[1].name(),
                ab[2].name()
            ),
            14.0,
            DIM,
        );
        label(
            p,
            format!(
                "Gun skins: {} on most guns, {} with their own (change them in Gun Skins)",
                skin_def(profile.skin).name,
                (0..crate::data::GUNS.len() as u8)
                    .filter(|&g| profile.skin_for(g) != profile.skin)
                    .count()
            ),
            14.0,
            DIM,
        );
        label(
            p,
            format!(
                "A run is {} maps: {} rounds on each, then its boss and a teleporter to the next. Start on:",
                crate::data::STAGES,
                crate::data::ROUNDS_PER_STAGE
            ),
            15.0,
            DIM,
        );
        if authority {
            row(p, |r| {
                button_sized(r, "<", UiAction::CycleMap(-1), Some(40.0), false);
                label(r, map_name(state.map), 20.0, ACCENT);
                button_sized(r, ">", UiAction::CycleMap(1), Some(40.0), false);
            });
        } else {
            label(
                p,
                format!("{} (the host picks)", map_name(state.map)),
                18.0,
                ACCENT,
            );
        }
        let time = if state.night { "Night" } else { "Day" };
        if authority {
            row(p, |r| {
                label(r, "Time:", 17.0, Color::WHITE);
                button_sized(r, time, UiAction::ToggleNight, Some(110.0), state.night);
            });
        } else {
            label(p, format!("Time: {time}"), 17.0, Color::WHITE);
        }
        let blurb = match state.map {
            0 => "Container stacks, cranes and narrow lanes.",
            1 => "Open lawns, trees, a pond and a bandstand.",
            _ => "Houses, fences and a cul-de-sac.",
        };
        label(p, blurb, 14.0, DIM);
        if state.sandbox.on {
            label(
                p,
                "No waves until you start them. Press F1 in the match for the sandbox tools.",
                15.0,
                ACCENT,
            );
        }
        p.spawn(Node {
            height: Val::Px(6.0),
            ..default()
        });
        if authority {
            if session.role == Role::Host {
                let others = roster.0.values().filter(|p| p.id != 0).count();
                let ready_n = roster.0.values().filter(|p| p.id != 0 && p.ready).count();
                label(
                    p,
                    format!(
                        "{ready_n} of {others} friends ready. Friends can also join mid-match."
                    ),
                    15.0,
                    DIM,
                );
            }
        }
        let leave = match session.role {
            Role::Solo => "Back",
            Role::Host => "Close party",
            Role::Client => "Leave party",
        };
        row(p, |r| {
            if authority {
                button_sized(r, "Start match", UiAction::StartMatch, Some(344.0), false);
            } else if session.connected {
                let text = if ready {
                    "Ready! (click to undo)"
                } else {
                    "Ready up"
                };
                button_sized(r, text, UiAction::ToggleReady, Some(344.0), ready);
            }
            button_sized(r, leave, UiAction::Leave, Some(344.0), false);
        });
        if !authority && session.connected {
            label(p, "The host starts the match.", 15.0, DIM);
        }
    });
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// Starts the match straight away when launched with `--start`.
pub fn autostart(
    app_state: Res<State<AppState>>,
    mut session: ResMut<Session>,
    mut state: ResMut<MatchState>,
    mut roster: ResMut<Roster>,
    mut next: ResMut<NextState<AppState>>,
) {
    if *app_state.get() == AppState::Lobby && session.autostart && session.is_authority() {
        session.autostart = false;
        let map = state.map;
        new_match(&mut state, &mut roster, map);
        next.set(AppState::InGame);
    }
}

fn text_input(
    mut events: EventReader<KeyboardInput>,
    mut ctrl: Local<bool>,
    mut focus: ResMut<Focus>,
    mut profile: ResMut<Profile>,
    mut requests: EventWriter<PartyRequest>,
) {
    for ev in events.read() {
        // Ctrl is tracked from the events in order: a quick Ctrl+V can press
        // and release both keys within one frame.
        if matches!(ev.key_code, KeyCode::ControlLeft | KeyCode::ControlRight) {
            *ctrl = ev.state == ButtonState::Pressed;
            continue;
        }
        if ev.state != ButtonState::Pressed || *focus == Focus::None {
            continue;
        }
        let (value, max) = match *focus {
            Focus::Name => (&mut profile.name, MAX_NAME_LEN),
            Focus::Address => (&mut profile.last_address, 64),
            Focus::None => return,
        };
        // Ctrl+V pastes; other Ctrl shortcuts type nothing.
        if *ctrl {
            if ev.key_code == KeyCode::KeyV {
                let pasted = arboard::Clipboard::new().and_then(|mut c| c.get_text()).unwrap_or_default();
                for ch in pasted.trim().chars() {
                    let ok = ch.is_ascii_graphic() || (ch == ' ' && *focus == Focus::Name);
                    if ok && value.chars().count() < max {
                        value.push(ch);
                    }
                }
            }
            continue;
        }
        match &ev.logical_key {
            Key::Enter => {
                if *focus == Focus::Address {
                    requests.write(PartyRequest::Join(profile.last_address.clone()));
                }
                *focus = Focus::None;
                return;
            }
            Key::Escape => {
                *focus = Focus::None;
                return;
            }
            Key::Backspace => {
                value.pop();
            }
            Key::Space if *focus == Focus::Name && value.chars().count() < max => value.push(' '),
            Key::Character(s) => {
                for ch in s.chars() {
                    let ok = ch.is_ascii_graphic() || (ch == ' ' && *focus == Focus::Name);
                    if ok && value.chars().count() < max {
                        value.push(ch);
                    }
                }
            }
            _ => {}
        }
    }
}

fn rebind_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut rebinding: ResMut<Rebinding>,
    mut settings: ResMut<Settings>,
) {
    let Some(action) = rebinding.0 else {
        return;
    };
    let Some(key) = keys.get_just_pressed().next().copied() else {
        return;
    };
    if key != KeyCode::Escape {
        settings.set_key(action, key);
    }
    rebinding.0 = None;
}

fn menu_back_key(
    keys: Res<ButtonInput<KeyCode>>,
    app_state: Res<State<AppState>>,
    focus: Res<Focus>,
    rebinding: Res<Rebinding>,
    mut screen: ResMut<Screen>,
) {
    if *app_state.get() == AppState::Menu
        && *focus == Focus::None
        && rebinding.0.is_none()
        && keys.just_pressed(KeyCode::Escape)
    {
        *screen = Screen::Main;
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    buttons: Query<(&Interaction, &UiAction), Changed<Interaction>>,
    mut screen: ResMut<Screen>,
    mut focus: ResMut<Focus>,
    mut rebinding: ResMut<Rebinding>,
    mut spin: ResMut<SkinShop>,
    (mut profile, mut settings, mut ready): (ResMut<Profile>, ResMut<Settings>, ResMut<LocalReady>),
    (session, mut roster, mut state): (Res<Session>, ResMut<Roster>, ResMut<MatchState>),
    (mut overlay, mut counter, mut actions): (
        ResMut<Overlay>,
        ResMut<ActionCounter>,
        ResMut<ActionQueue>,
    ),
    mut crate_spin: ResMut<carousel::CrateSpin>,
    (mut tools, mut public_ip): (ResMut<sandbox::Tools>, ResMut<crate::net::PublicIp>),
    mut next: ResMut<NextState<AppState>>,
    mut requests: EventWriter<PartyRequest>,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        // Clicking anything else stops typing.
        if !matches!(action, UiAction::FocusName | UiAction::FocusAddress) {
            *focus = Focus::None;
        }
        match *action {
            UiAction::PlaySolo => {
                requests.write(PartyRequest::Solo);
            }
            UiAction::Host => {
                requests.write(PartyRequest::Host(DEFAULT_PORT));
            }
            UiAction::OpenJoin => {
                *screen = Screen::Join;
                *focus = Focus::Address;
            }
            UiAction::Connect => {
                requests.write(PartyRequest::Join(profile.last_address.clone()));
            }
            UiAction::OpenCharacters => *screen = Screen::Characters,
            UiAction::OpenLoadout => *screen = Screen::Loadout,
            UiAction::OpenGuide(tab) => *screen = Screen::Guide(tab),
            UiAction::RevealIp => public_ip.toggle(),
            UiAction::Locked => {}
            UiAction::LoadoutGun(slot, gun) => {
                let c = profile.character;
                let mut picked = profile.class_loadout(c).map(|(g, _)| g);
                picked[slot as usize & 1] = gun;
                profile.class_guns.insert(c, picked);
            }
            UiAction::LoadoutAttach(gun_slot, slot, id) => {
                let (gun, a) = profile.class_loadout(profile.character)[gun_slot as usize & 1];
                let (o, m, u, x) = (a.optic(), a.muzzle(), a.under(), a.ext_mag());
                let a = match slot {
                    0 => Attach::new(id, m, u, x),
                    1 => Attach::new(o, id, u, x),
                    2 => Attach::new(o, m, id, x),
                    _ => Attach::new(o, m, u, id > 0),
                };
                profile.gun_attach.insert(gun, a.0);
            }
            UiAction::PlaySandbox => {
                requests.write(PartyRequest::Sandbox);
            }
            UiAction::SandboxGun(g) => {
                tools.gun = g;
                tools.attach = crate::progression::allowed_attach(u32::MAX, g, tools.attach);
            }
            UiAction::SandboxAttach(slot, id) => {
                let a = tools.attach;
                let (o, m, u, x) = (a.optic(), a.muzzle(), a.under(), a.ext_mag());
                tools.attach = match slot {
                    0 => Attach::new(id, m, u, x),
                    1 => Attach::new(o, id, u, x),
                    2 => Attach::new(o, m, id, x),
                    _ => Attach::new(o, m, u, id > 0),
                };
            }
            UiAction::SandboxSpawn(..)
            | UiAction::SandboxToggle(_)
            | UiAction::SandboxPoints
            | UiAction::SandboxLevel
            | UiAction::SandboxRound
            | UiAction::SandboxKill
            | UiAction::SandboxGive => {
                use crate::DevCmd;
                let me = roster.me(&session);
                let cmd = match *action {
                    UiAction::SandboxSpawn(kind, count) => {
                        let Some(me) = me else { continue };
                        let dir = Vec3::new(-me.yaw.sin(), 0.0, -me.yaw.cos());
                        DevCmd::Spawn {
                            kind,
                            count,
                            at: me.feet().to_array(),
                            dir: dir.to_array(),
                        }
                    }
                    UiAction::SandboxToggle(n) => DevCmd::Toggle(n),
                    UiAction::SandboxPoints => DevCmd::Points,
                    UiAction::SandboxLevel => DevCmd::LevelUp,
                    UiAction::SandboxRound => DevCmd::NextRound,
                    UiAction::SandboxKill => DevCmd::KillAll,
                    _ => DevCmd::Gun {
                        gun: tools.gun,
                        attach: tools.attach,
                    },
                };
                queue_action(&session, &mut counter, &mut actions, PlayerAction::Dev(cmd));
            }
            UiAction::OpenSkins => {
                spin.last = None;
                spin.message = None;
                spin.preview = None;
                *screen = Screen::Crates;
            }
            UiAction::OpenGunSkins => *screen = Screen::GunSkins,
            UiAction::SelectGun(g) => spin.gun = g,
            UiAction::OpenSettings => *screen = Screen::Settings,
            UiAction::BackToMain => *screen = Screen::Main,
            UiAction::Quit => {
                exit.write(AppExit::Success);
            }
            UiAction::FocusName => *focus = Focus::Name,
            UiAction::FocusAddress => *focus = Focus::Address,
            UiAction::SelectCharacter(c) => profile.character = c,
            UiAction::PickAbility(slot, a) => {
                let c = profile.character;
                let mut kit = profile.kit(c);
                let slot = (slot as usize).min(2);
                // Picking the one in the other key swaps them over.
                if slot < 2 && kit[1 - slot] == a {
                    kit[1 - slot] = kit[slot];
                }
                kit[slot] = a;
                if crate::data::valid_kit(c, profile.char_level(c).0, kit) {
                    profile.kits.insert(c, kit);
                }
            }
            UiAction::SelectCrate(i) => {
                spin.crate_id = (i as usize).min(CRATES.len() - 1);
                spin.last = None;
                spin.message = None;
            }
            UiAction::OpenCrate => {
                if crate_spin.spinning() {
                    continue;
                }
                let c = &CRATES[spin.crate_id];
                if c.skins.iter().all(|s| profile.owned_skins.contains(s)) {
                    continue;
                }
                if c.premium {
                    if profile.premium_quarters < 4 {
                        continue;
                    }
                    profile.premium_quarters -= 4;
                } else {
                    if profile.spins == 0 {
                        continue;
                    }
                    profile.spins -= 1;
                }
                let id = roll_crate(spin.crate_id, &mut rand::thread_rng());
                let new = !profile.owned_skins.contains(&id);
                if new {
                    profile.owned_skins.push(id);
                    profile.owned_skins.sort();
                } else if c.premium {
                    profile.premium_quarters += 1;
                } else {
                    profile.shards += 1;
                    if profile.shards >= 3 {
                        profile.shards = 0;
                        profile.spins += 1;
                    }
                }
                // The win shows once the carousel stops on it.
                spin.last = None;
                spin.message = None;
                crate_spin.start(spin.crate_id, id, new);
            }
            UiAction::TradePremium => {
                spin.last = None;
                if profile.spins >= PREMIUM_TRADE_COST {
                    profile.spins -= PREMIUM_TRADE_COST;
                    profile.premium_quarters += 4;
                    spin.message = Some("Traded for 1 premium spin.".into());
                } else {
                    spin.message = Some(format!(
                        "You need {PREMIUM_TRADE_COST} spins to trade for a premium spin."
                    ));
                }
            }
            UiAction::PreviewSkin(id) => {
                if profile.owned_skins.contains(&id) {
                    spin.preview = Some(id);
                }
            }
            UiAction::ApplySkin(gun, id) => {
                if id != 255 && !profile.owned_skins.contains(&id) {
                    continue;
                }
                if gun == ALL_GUNS {
                    if skin_def(id).gun.is_none() {
                        profile.skin = id;
                    }
                } else if id == 255 || crate::data::skin_fits(id, gun) {
                    let gun = gun as usize;
                    if profile.gun_skins.len() <= gun {
                        profile.gun_skins.resize(gun + 1, 255);
                    }
                    profile.gun_skins[gun] = id;
                }
            }
            UiAction::CycleDisplay => settings.display = settings.display.next(),
            UiAction::CycleResolution(d) => {
                let n = RESOLUTIONS.len() as i32;
                settings.resolution =
                    ((settings.resolution as i32 + d as i32).rem_euclid(n)) as usize;
            }
            UiAction::Rebind(a) => rebinding.0 = Some(a),
            UiAction::SettingsTab(t) => {
                rebinding.0 = None;
                rebinding.1 = t;
            }
            UiAction::CycleGraphics(which) => match which {
                0 => settings.shadows = (settings.shadows + 1) % 3,
                1 => {
                    settings.antialias = !settings.antialias;
                    if settings.antialias {
                        settings.ambient_occlusion = false;
                    }
                }
                2 => settings.bloom = !settings.bloom,
                4 => settings.outlines = !settings.outlines,
                5 => settings.camera_shake = !settings.camera_shake,
                6 => settings.particles = !settings.particles,
                _ => {
                    settings.ambient_occlusion = !settings.ambient_occlusion;
                    if settings.ambient_occlusion {
                        settings.antialias = false;
                    }
                }
            },
            UiAction::ResetKeys => settings.reset_keys(),
            UiAction::CycleCast(i) => {
                let m = &mut settings.cast_modes[i as usize];
                *m = m.next();
            }
            UiAction::SettingsBack => {
                rebinding.0 = None;
                if *overlay == Overlay::Settings {
                    *overlay = Overlay::Pause;
                } else {
                    *screen = Screen::Main;
                }
            }
            UiAction::CycleMap(d) => {
                if session.is_authority() {
                    let n = MAP_NAMES.len() as i32;
                    state.map = (state.map as i32 + d as i32).rem_euclid(n) as u8;
                }
            }
            UiAction::ToggleNight => {
                if session.is_authority() {
                    state.night = !state.night;
                }
            }
            UiAction::ToggleReady => ready.0 = !ready.0,
            UiAction::StartMatch => {
                if session.is_authority() {
                    let map = state.map;
                    new_match(&mut state, &mut roster, map);
                    next.set(AppState::InGame);
                }
            }
            UiAction::Leave => {
                *screen = Screen::Main;
                requests.write(PartyRequest::Leave(None));
            }
            UiAction::Resume => *overlay = Overlay::None,
            UiAction::PauseSettings => *overlay = Overlay::Settings,
            UiAction::ChooseUpgrade(i) => {
                queue_action(
                    &session,
                    &mut counter,
                    &mut actions,
                    PlayerAction::Choose(i),
                );
            }
            UiAction::BackToLobby => {
                if session.is_authority() {
                    *state = MatchState::new(state.map);
                    ready.0 = false;
                    next.set(AppState::Lobby);
                }
            }
        }
    }
}

fn drag_sliders(
    sliders: Query<(&Interaction, &RelativeCursorPosition, &Slider)>,
    mut settings: ResMut<Settings>,
) {
    for (interaction, cursor, kind) in &sliders {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(pos) = cursor.normalized else {
            continue;
        };
        let (lo, hi) = kind.range();
        let v = lo + (hi - lo) * pos.x.clamp(0.0, 1.0);
        if (kind.get(&settings) - v).abs() > 1e-3 {
            kind.set(&mut settings, v);
        }
    }
}

fn update_sliders(
    settings: Res<Settings>,
    mut fills: Query<(&SliderFill, &mut Node)>,
    mut labels: Query<(&SliderLabel, &mut Text)>,
) {
    for (SliderFill(kind), mut node) in &mut fills {
        let (lo, hi) = kind.range();
        let t = ((kind.get(&settings) - lo) / (hi - lo)).clamp(0.0, 1.0);
        node.width = Val::Percent(t * 100.0);
    }
    for (SliderLabel(kind), mut text) in &mut labels {
        let l = kind.label(&settings);
        if text.0 != l {
            text.0 = l;
        }
    }
}

fn button_colors(
    mut buttons: Query<
        (&Interaction, &mut BackgroundColor),
        (With<UiAction>, Changed<Interaction>),
    >,
) {
    for (interaction, mut bg) in &mut buttons {
        bg.0 = match interaction {
            Interaction::Pressed => BUTTON_PRESS,
            Interaction::Hovered => BUTTON_HOVER,
            Interaction::None => BUTTON,
        };
    }
}

// ---------------------------------------------------------------------------
// 3D characters behind the menus
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn menu_scene(
    mut commands: Commands,
    time: Res<Time>,
    app_state: Res<State<AppState>>,
    session: Res<Session>,
    roster: Res<Roster>,
    profile: Res<Profile>,
    rigs: Res<crate::rig::RigAssets>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scene: Query<Entity, With<MenuScene>>,
    mut turn: Query<(&mut Transform, &Turntable), Without<ShowcaseGun>>,
    mut showcase: Query<&mut Transform, With<ShowcaseGun>>,
    (screen, shop, guns): (Res<Screen>, Res<SkinShop>, Res<crate::gunmodels::GunAssets>),
    mut last: Local<String>,
) {
    const MENU_GUN: u8 = 6;
    // Who to show: you in the menu, the whole party in the lobby, each with
    // (character, skin, gun).
    let mut showcase_skin = None;
    let people: Vec<(Character, u8, u8)> = match app_state.get() {
        AppState::InGame | AppState::Travel => Vec::new(),
        AppState::Menu => {
            let mut gun = MENU_GUN;
            let mut skin = profile.skin_for(MENU_GUN);
            if *screen == Screen::Crates {
                if let Some(id) = shop.preview {
                    gun = skin_def(id).gun.unwrap_or(MENU_GUN);
                    skin = id;
                    showcase_skin = Some((gun, id));
                }
            }
            if *screen == Screen::GunSkins {
                // The picked gun in the skin it will show in game.
                if shop.gun == ALL_GUNS {
                    showcase_skin = Some((MENU_GUN, profile.skin));
                } else {
                    gun = shop.gun;
                    skin = profile.skin_for(gun);
                    showcase_skin = Some((gun, skin));
                }
            }
            vec![(profile.character, skin, gun)]
        }
        AppState::Lobby => {
            let mut v = vec![(profile.character, profile.skin_for(MENU_GUN), MENU_GUN)];
            v.extend(
                roster
                    .0
                    .values()
                    .filter(|p| p.id != session.my_id)
                    .map(|p| (p.character, p.skin_for(MENU_GUN), MENU_GUN)),
            );
            v
        }
    };
    let sig = format!("{people:?} {showcase_skin:?}");
    if *last != sig {
        *last = sig;
        for e in &scene {
            commands.entity(e).despawn();
        }
        if !people.is_empty() {
            commands.spawn((
                MenuScene,
                DirectionalLight {
                    illuminance: 9000.0,
                    shadows_enabled: true,
                    ..default()
                },
                Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::new(2.0, 0.0, -5.0), Vec3::Y),
            ));
            commands.spawn((
                MenuScene,
                Mesh3d(mesh_assets.add(Cylinder::new(3.4, 0.1))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.12, 0.13, 0.17),
                    perceptual_roughness: 0.9,
                    ..default()
                })),
                Transform::from_xyz(2.6, -0.05, -5.8),
            ));
            if let Some((gun, skin)) = showcase_skin {
                // The previewed skin up close, slowly turning.
                commands
                    .spawn((
                        MenuScene,
                        ShowcaseGun,
                        Transform::from_xyz(3.5, 1.3, -3.6).with_scale(Vec3::splat(2.0)),
                        Visibility::default(),
                    ))
                    .with_children(|p| {
                        crate::gunmodels::spawn_gun(
                            p,
                            &guns,
                            gun,
                            crate::data::Attach::NONE,
                            guns.skin(skin),
                            false,
                            Some(crate::outline::Outline::Figure),
                        );
                    });
            }
            for (i, (c, skin, gun)) in people.iter().enumerate() {
                let n = people.len() as f32;
                let spacing = if n > 4.0 { 1.0 } else { 1.4 };
                let x = 2.6 + (i as f32 - (n - 1.0) / 2.0) * spacing;
                let z = -5.5 - (i % 2) as f32 * 0.4 * (n > 4.0) as u8 as f32;
                // Face the camera (at the origin).
                let base = x.atan2(z);
                let tf = Transform::from_xyz(x, 0.0, z).with_rotation(Quat::from_rotation_y(base));
                let (e, _) = spawn_person(&mut commands, &rigs, *c, *skin, *gun, tf);
                commands
                    .entity(e)
                    .insert((MenuScene, Turntable(base, i as f32)));
            }
        }
    }
    let t = time.elapsed_secs();
    for mut tf in &mut showcase {
        tf.rotation = Quat::from_rotation_y(t * 0.7) * Quat::from_rotation_x(-0.2);
    }
    for (mut tf, Turntable(base, offset)) in &mut turn {
        let a = base + 0.4 * (t * 0.6 + offset).sin();
        tf.rotation = Quat::from_rotation_y(a);
    }
}
