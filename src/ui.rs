//! Menus: the main menu, joining, character select, gun skins (gacha),
//! settings (sliders, display, key remapping), the party screen before a
//! match and the in-game pause menu. Screens are rebuilt whenever what they
//! show changes; buttons carry a `UiAction` that one system carries out.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::abilities::{queue_action, ActionCounter};
use crate::avatars::{spawn_person, ReplicatedAssets};
use crate::config::{key_name, Action, Profile, Settings, RESOLUTIONS};
use crate::data::{roll_skin, skin_def, Character, SKINS};
use crate::game::Overlay;
use crate::humanoid::HumanoidMeshes;
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

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Screen>()
            .init_resource::<Focus>()
            .init_resource::<Rebinding>()
            .init_resource::<SpinResult>()
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
    Skins,
    Settings,
}

/// Which text box is being typed into.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
enum Focus {
    #[default]
    None,
    Name,
    Address,
}

/// The action waiting for a new key, if any.
#[derive(Resource, Default)]
struct Rebinding(Option<Action>);

/// The last gacha pull: (skin, was it new).
#[derive(Resource, Default)]
struct SpinResult(Option<(u8, bool)>);

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
    Spin,
    EquipSkin(u8),
    CycleDisplay,
    CycleResolution(i8),
    Rebind(Action),
    ResetKeys,
    SettingsBack,
    CycleMap(i8),
    ToggleReady,
    StartMatch,
    Leave,
    Resume,
    PauseSettings,
    ChooseUpgrade(u8),
    BackToLobby,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum Slider {
    Fov,
    Sensitivity,
    Volume,
}

impl Slider {
    fn range(self) -> (f32, f32) {
        match self {
            Slider::Fov => (60.0, 120.0),
            Slider::Sensitivity => (0.1, 3.0),
            Slider::Volume => (0.0, 100.0),
        }
    }

    fn get(self, s: &Settings) -> f32 {
        match self {
            Slider::Fov => s.fov,
            Slider::Sensitivity => s.sensitivity,
            Slider::Volume => s.volume,
        }
    }

    fn set(self, s: &mut Settings, v: f32) {
        match self {
            Slider::Fov => s.fov = v.round(),
            Slider::Sensitivity => s.sensitivity = (v * 20.0).round() / 20.0,
            Slider::Volume => s.volume = v.round(),
        }
    }

    fn label(self, s: &Settings) -> String {
        match self {
            Slider::Fov => format!("Field of view: {:.0}", s.fov),
            Slider::Sensitivity => format!("Mouse sensitivity: {:.2}", s.sensitivity),
            Slider::Volume => format!("Volume: {:.0}%  (for when the game has sound)", s.volume),
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
                    height: if centered { Val::Auto } else { Val::Percent(100.0) },
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
    spin: Res<SpinResult>,
    overlay: Res<Overlay>,
    (profile, settings, notice, ready): (Res<Profile>, Res<Settings>, Res<Notice>, Res<LocalReady>),
    (session, roster, state): (Res<Session>, Res<Roster>, Res<MatchState>),
    roots: Query<Entity, With<UiRoot>>,
    mut last: Local<String>,
) {
    let app = app_state.get().clone();
    // Everything the current screen shows, so we only rebuild on changes.
    let sig = match app {
        AppState::Menu => format!(
            "menu {:?} {:?} {:?} {:?} {:?} {} {} {} {} {} {:?} {} {} {} {:?}",
            *screen,
            *focus,
            rebinding.0,
            spin.0,
            profile.character,
            profile.name,
            profile.last_address,
            profile.spins,
            profile.shards,
            profile.skin,
            profile.owned_skins,
            notice.0,
            settings.display.name(),
            settings.resolution,
            settings.keys
        ),
        AppState::Lobby => {
            let players: Vec<_> = roster
                .0
                .values()
                .map(|p| (p.id, p.name.clone(), p.character, p.ready))
                .collect();
            format!(
                "lobby {:?} {} {} {} {:?} {} {:?} {}",
                players,
                state.map,
                session.status,
                session.connected,
                profile.character,
                ready.0,
                session.role,
                profile.skin
            )
        }
        AppState::InGame => format!(
            "game {:?} {:?} {} {} {:?}",
            *overlay,
            rebinding.0,
            settings.display.name(),
            settings.resolution,
            settings.keys
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
            Screen::Skins => skins_screen(&mut commands, &profile, &spin),
            Screen::Settings => panel(&mut commands, false, |p| {
                settings_body(p, &settings, rebinding.0);
            }),
        },
        AppState::Lobby => lobby_screen(&mut commands, &session, &roster, &state, &profile, ready.0),
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
                settings_body(p, &settings, rebinding.0);
            }),
            _ => {}
        },
    }
}

fn text_field(p: &mut ChildSpawnerCommands, value: &str, focused: bool, action: UiAction, placeholder: &str) {
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
        BorderColor(if focused { ACCENT } else { Color::srgb(0.3, 0.32, 0.4) }),
        BorderRadius::all(Val::Px(6.0)),
    ))
    .with_children(|b| {
        b.spawn((
            Text::new(shown),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(if value.is_empty() && !focused { DIM } else { Color::WHITE }),
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
        text_field(p, &profile.name, focus == Focus::Name, UiAction::FocusName, "Player");
        p.spawn(Node {
            height: Val::Px(6.0),
            ..default()
        });
        button(p, "Play Solo", UiAction::PlaySolo);
        button(p, "Host a Party", UiAction::Host);
        button(p, "Join a Party", UiAction::OpenJoin);
        button(
            p,
            format!("Characters  ({})", profile.character.name()),
            UiAction::OpenCharacters,
        );
        button(
            p,
            format!("Gun Skins  ({} spins)", profile.spins),
            UiAction::OpenSkins,
        );
        button(p, "Settings", UiAction::OpenSettings);
        button(p, "Quit", UiAction::Quit);
        p.spawn(Node {
            height: Val::Px(6.0),
            ..default()
        });
        label(
            p,
            format!(
                "Best round: {}    Extractions: {}",
                profile.best_round, profile.extractions
            ),
            16.0,
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
        let keys = [Action::Ability1, Action::Ability2, Action::Ultimate];
        for c in Character::ALL {
            let selected = profile.character == c;
            p.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.12, 0.13, 0.19, 0.9)),
                BorderColor(if selected { ACCENT } else { Color::NONE }),
                BorderRadius::all(Val::Px(8.0)),
            ))
            .with_children(|card| {
                label(card, c.name(), 26.0, c.suit_color().lighter(0.15));
                label(card, c.tagline(), 15.0, DIM);
                for (i, (name, desc)) in c.abilities().iter().enumerate() {
                    label(
                        card,
                        format!("[{}] {name}: {desc}", key_name(settings.key(keys[i]))),
                        15.0,
                        Color::WHITE,
                    );
                }
                button_sized(
                    card,
                    if selected { "Selected" } else { "Select" },
                    UiAction::SelectCharacter(c),
                    Some(140.0),
                    selected,
                );
            });
        }
        label(
            p,
            "Abilities get stronger as you level up in a match.",
            15.0,
            DIM,
        );
        button(p, "Back", UiAction::BackToMain);
    });
}

fn skins_screen(commands: &mut Commands, profile: &Profile, spin: &SpinResult) {
    panel(commands, false, |p| {
        title(p, "Gun skins");
        label(
            p,
            format!(
                "Spins: {}    Duplicate shards: {}/3",
                profile.spins, profile.shards
            ),
            20.0,
            Color::WHITE,
        );
        let all = profile.owned_skins.len() >= SKINS.len();
        if all {
            label(p, "You own every skin!", 16.0, ACCENT);
        } else if profile.spins > 0 {
            button(p, "Spin!", UiAction::Spin);
        } else {
            label(p, "No spins left - earn more by surviving rounds.", 16.0, WARN);
        }
        if let Some((id, new)) = spin.0 {
            let s = skin_def(id);
            let what = if new {
                "NEW!".to_string()
            } else {
                "Duplicate (+1 shard)".to_string()
            };
            label(
                p,
                format!("You got: {} ({}) {what}", s.name, s.rarity.name()),
                20.0,
                s.rarity.color(),
            );
        }
        label(p, "Click a skin you own to equip it:", 15.0, DIM);
        p.spawn(Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|g| {
            for (i, s) in SKINS.iter().enumerate() {
                let id = i as u8;
                let owned = profile.owned_skins.contains(&id);
                let equipped = profile.skin == id;
                let c = Color::srgb(s.color[0], s.color[1], s.color[2]);
                g.spawn((
                    Button,
                    UiAction::EquipSkin(id),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(6.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(BUTTON),
                    BorderColor(if equipped { ACCENT } else { s.rarity.color().with_alpha(0.4) }),
                    BorderRadius::all(Val::Px(6.0)),
                ))
                .with_children(|b| {
                    b.spawn((
                        Node {
                            width: Val::Px(60.0),
                            height: Val::Px(18.0),
                            ..default()
                        },
                        BackgroundColor(if owned { c } else { Color::srgb(0.2, 0.2, 0.22) }),
                        BorderRadius::all(Val::Px(3.0)),
                    ));
                    let name = if owned { s.name.to_string() } else { "???".to_string() };
                    label(b, name, 15.0, if owned { Color::WHITE } else { DIM });
                    label(b, s.rarity.name(), 12.0, s.rarity.color());
                });
            }
        });
        label(
            p,
            "Earn spins by how far you get: round 5 = 1 spin, round 10 = 3,\nround 15 = 6, round 20 = 10... Extracting doubles them.\nOdds: Common 55%, Rare 30%, Epic 12%, Legendary 3%.",
            14.0,
            DIM,
        );
        button(p, "Back", UiAction::BackToMain);
    });
}

fn settings_body(p: &mut ChildSpawnerCommands, settings: &Settings, rebinding: Option<Action>) {
    label(p, "SETTINGS", 34.0, ACCENT);
    slider(p, Slider::Fov, settings);
    slider(p, Slider::Sensitivity, settings);
    slider(p, Slider::Volume, settings);
    row(p, |r| {
        label(r, "Display:", 16.0, Color::WHITE);
        button_sized(r, settings.display.name(), UiAction::CycleDisplay, Some(150.0), false);
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
    label(p, "Controls (click one, then press the new key):", 16.0, Color::WHITE);
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
                BorderColor(if rebinding == Some(a) { ACCENT } else { Color::NONE }),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_children(|b| label(b, text, 14.0, Color::WHITE));
        }
    });
    label(
        p,
        "Fixed: mouse to aim and shoot, 1 / 2 or scroll to switch guns, Esc for menu.",
        13.0,
        DIM,
    );
    row(p, |r| {
        button_sized(r, "Reset controls", UiAction::ResetKeys, Some(210.0), false);
        button_sized(r, "Back", UiAction::SettingsBack, Some(120.0), false);
    });
}

fn lobby_screen(
    commands: &mut Commands,
    session: &Session,
    roster: &Roster,
    state: &MatchState,
    profile: &Profile,
    ready: bool,
) {
    let authority = session.is_authority();
    panel(commands, false, |p| {
        let heading = match session.role {
            Role::Solo => "Solo game",
            Role::Host => "Your party",
            Role::Client => "Party",
        };
        title(p, heading);
        if !session.status.is_empty() {
            label(p, session.status.clone(), 16.0, Color::srgb(0.5, 0.9, 1.0));
        }
        if session.role == Role::Client && !session.connected {
            label(p, "Waiting for the host to answer...", 16.0, DIM);
        }
        if session.role != Role::Solo {
            label(p, format!("Players ({}):", roster.0.len()), 17.0, Color::WHITE);
            for pl in roster.0.values() {
                let host = if pl.id == 0 { " (host)" } else { "" };
                let you = if pl.id == session.my_id { " - you" } else { "" };
                let r = if pl.id == 0 || pl.ready { "Ready" } else { "Not ready" };
                label(
                    p,
                    format!("  {}{host}{you}  -  {}  -  {r}", pl.name, pl.character.name()),
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
                    Some(140.0),
                    profile.character == c,
                );
            }
        });
        let ab = profile.character.abilities();
        label(
            p,
            format!("{}, {}, ultimate: {}", ab[0].0, ab[1].0, ab[2].0),
            14.0,
            DIM,
        );
        label(
            p,
            format!("Gun skin: {} (change it in Gun Skins on the main menu)", skin_def(profile.skin).name),
            14.0,
            DIM,
        );
        label(p, "Map:", 17.0, Color::WHITE);
        if authority {
            row(p, |r| {
                button_sized(r, "<", UiAction::CycleMap(-1), Some(40.0), false);
                label(r, map_name(state.map), 20.0, ACCENT);
                button_sized(r, ">", UiAction::CycleMap(1), Some(40.0), false);
            });
        } else {
            label(p, format!("{} (the host picks)", map_name(state.map)), 18.0, ACCENT);
        }
        let blurb = match state.map {
            0 => "Container stacks, cranes and narrow lanes.",
            1 => "Open lawns, trees, a pond and a bandstand.",
            _ => "Houses, fences and a cul-de-sac.",
        };
        label(p, blurb, 14.0, DIM);
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
                    format!("{ready_n} of {others} friends ready. Friends can also join mid-match."),
                    15.0,
                    DIM,
                );
            }
            button(p, "Start match", UiAction::StartMatch);
        } else if session.connected {
            button(
                p,
                if ready { "Ready! (click to undo)" } else { "Ready up" },
                UiAction::ToggleReady,
            );
            label(p, "The host starts the match.", 15.0, DIM);
        }
        button(
            p,
            match session.role {
                Role::Solo => "Back",
                Role::Host => "Close party",
                Role::Client => "Leave party",
            },
            UiAction::Leave,
        );
    });
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// Starts the match straight away when launched with `--start`.
fn autostart(
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
    mut focus: ResMut<Focus>,
    mut profile: ResMut<Profile>,
    mut requests: EventWriter<PartyRequest>,
) {
    if *focus == Focus::None {
        events.clear();
        return;
    }
    for ev in events.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        let (value, max) = match *focus {
            Focus::Name => (&mut profile.name, MAX_NAME_LEN),
            Focus::Address => (&mut profile.last_address, 64),
            Focus::None => return,
        };
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
    mut spin: ResMut<SpinResult>,
    (mut profile, mut settings, mut ready): (ResMut<Profile>, ResMut<Settings>, ResMut<LocalReady>),
    (session, mut roster, mut state): (Res<Session>, ResMut<Roster>, ResMut<MatchState>),
    (mut overlay, mut counter, mut actions): (ResMut<Overlay>, ResMut<ActionCounter>, ResMut<ActionQueue>),
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
            UiAction::OpenSkins => {
                spin.0 = None;
                *screen = Screen::Skins;
            }
            UiAction::OpenSettings => *screen = Screen::Settings,
            UiAction::BackToMain => *screen = Screen::Main,
            UiAction::Quit => {
                exit.write(AppExit::Success);
            }
            UiAction::FocusName => *focus = Focus::Name,
            UiAction::FocusAddress => *focus = Focus::Address,
            UiAction::SelectCharacter(c) => profile.character = c,
            UiAction::Spin => {
                if profile.spins == 0 || profile.owned_skins.len() >= SKINS.len() {
                    continue;
                }
                profile.spins -= 1;
                let id = roll_skin(&mut rand::thread_rng());
                let new = !profile.owned_skins.contains(&id);
                if new {
                    profile.owned_skins.push(id);
                    profile.owned_skins.sort();
                } else {
                    profile.shards += 1;
                    if profile.shards >= 3 {
                        profile.shards = 0;
                        profile.spins += 1;
                    }
                }
                spin.0 = Some((id, new));
            }
            UiAction::EquipSkin(id) => {
                if profile.owned_skins.contains(&id) {
                    profile.skin = id;
                }
            }
            UiAction::CycleDisplay => settings.display = settings.display.next(),
            UiAction::CycleResolution(d) => {
                let n = RESOLUTIONS.len() as i32;
                settings.resolution = ((settings.resolution as i32 + d as i32).rem_euclid(n)) as usize;
            }
            UiAction::Rebind(a) => rebinding.0 = Some(a),
            UiAction::ResetKeys => settings.reset_keys(),
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
                queue_action(&session, &mut counter, &mut actions, PlayerAction::Choose(i));
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
    mut buttons: Query<(&Interaction, &mut BackgroundColor), (With<UiAction>, Changed<Interaction>)>,
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
    assets: Res<ReplicatedAssets>,
    meshes: Res<HumanoidMeshes>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scene: Query<Entity, With<MenuScene>>,
    mut turn: Query<(&mut Transform, &Turntable)>,
    mut last: Local<String>,
) {
    // Who to show: you in the menu, the whole party in the lobby.
    let people: Vec<(Character, u8)> = match app_state.get() {
        AppState::InGame => Vec::new(),
        AppState::Menu => vec![(profile.character, profile.skin)],
        AppState::Lobby => {
            let mut v: Vec<(Character, u8)> = vec![(profile.character, profile.skin)];
            v.extend(
                roster
                    .0
                    .values()
                    .filter(|p| p.id != session.my_id)
                    .map(|p| (p.character, p.skin)),
            );
            v
        }
    };
    let sig = format!("{people:?}");
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
            for (i, (c, skin)) in people.iter().enumerate() {
                let n = people.len() as f32;
                let spacing = if n > 4.0 { 1.0 } else { 1.4 };
                let x = 2.6 + (i as f32 - (n - 1.0) / 2.0) * spacing;
                let z = -5.5 - (i % 2) as f32 * 0.4 * (n > 4.0) as u8 as f32;
                // Face the camera (at the origin).
                let base = x.atan2(z);
                let tf = Transform::from_xyz(x, 0.0, z).with_rotation(Quat::from_rotation_y(base));
                let e = spawn_person(&mut commands, &meshes, &mut materials, &assets, *c, *skin, tf);
                commands.entity(e).insert((MenuScene, Turntable(base, i as f32)));
            }
        }
    }
    let t = time.elapsed_secs();
    for (mut tf, Turntable(base, offset)) in &mut turn {
        let a = base + 0.4 * (t * 0.6 + offset).sin();
        tf.rotation = Quat::from_rotation_y(a);
    }
}
