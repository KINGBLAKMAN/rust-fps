//! Match flow: loading the map when a match starts and clearing it after,
//! the in-game menus (pause, level-up picks), cursor locking, the mystery
//! box and extraction visuals, and handing out gacha spins at the end.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::config::{Action, InputExt, Profile, Settings};
use crate::data::{gun_def, spins_for_round};
use crate::maps::{spawn_map, BoxGlow, CurrentMap, ExtractionBeacon, MysteryBox, BOX_HALF};
use crate::nav::NavGrid;
use crate::{
    cursor_locked, set_cursor_lock, AppState, BoxState, InGameEntity, MatchState, Phase, Roster,
    Session,
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Paused>()
            .init_resource::<Overlay>()
            .init_resource::<MatchResult>()
            .add_systems(OnEnter(AppState::InGame), start_match)
            .add_systems(OnExit(AppState::InGame), end_match)
            .add_systems(
                Update,
                (menu_keys, cursor_control, award_spins)
                    .chain()
                    .before(Phase::Local)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                (box_visuals, extraction_visuals)
                    .in_set(Phase::Present)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

/// True while an in-game menu is open. Solo games freeze; in a party the
/// world keeps going but your character stands still.
#[derive(Resource, Default)]
pub struct Paused(pub bool);

/// Which in-game menu is open.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Overlay {
    #[default]
    None,
    Pause,
    Settings,
    Upgrades,
}

/// What the last match earned, for the end screen.
#[derive(Resource, Default)]
pub struct MatchResult {
    pub awarded: bool,
    pub spins: u32,
    pub round: u32,
    pub extracted: bool,
    pub new_best: bool,
}

pub fn match_ended(state: &MatchState) -> bool {
    state.game_over || state.extracted
}

#[derive(Component)]
struct BoxGun;

fn start_match(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<MatchState>,
    mut overlay: ResMut<Overlay>,
    mut paused: ResMut<Paused>,
    mut result: ResMut<MatchResult>,
    mut clear: ResMut<ClearColor>,
) {
    let layout = spawn_map(&mut commands, &mut meshes, &mut materials, state.map);
    clear.0 = layout.sky;
    commands.insert_resource(NavGrid::new(layout.half));
    commands.insert_resource(CurrentMap(layout));
    *overlay = Overlay::None;
    paused.0 = false;
    *result = MatchResult::default();

    // The gun that floats out of the mystery box.
    commands.spawn((
        InGameEntity,
        BoxGun,
        Mesh3d(meshes.add(Cuboid::new(0.12, 0.18, 0.8))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.85, 1.0),
            emissive: LinearRgba::rgb(1.5, 3.0, 5.0),
            ..default()
        })),
        Transform::default(),
        Visibility::Hidden,
    ));
}

fn end_match(
    mut commands: Commands,
    things: Query<Entity, With<InGameEntity>>,
    mut clear: ResMut<ClearColor>,
    mut overlay: ResMut<Overlay>,
    mut paused: ResMut<Paused>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    for e in &things {
        if let Ok(mut ec) = commands.get_entity(e) {
            ec.try_despawn();
        }
    }
    commands.remove_resource::<CurrentMap>();
    commands.remove_resource::<NavGrid>();
    clear.0 = Color::srgb(0.05, 0.06, 0.09);
    *overlay = Overlay::None;
    paused.0 = false;
    set_cursor_lock(&mut window, false);
}

fn menu_keys(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    mut overlay: ResMut<Overlay>,
    mut paused: ResMut<Paused>,
) {
    if match_ended(&state) {
        *overlay = Overlay::None;
    } else if keys.just_pressed(KeyCode::Escape) {
        *overlay = match *overlay {
            Overlay::None => Overlay::Pause,
            Overlay::Settings => Overlay::Pause,
            Overlay::Pause | Overlay::Upgrades => Overlay::None,
        };
    } else if keys.tapped(&settings, Action::Upgrades) {
        let has_choices = roster.me(&session).is_some_and(|m| !m.choices.is_empty());
        *overlay = match *overlay {
            Overlay::None if has_choices => Overlay::Upgrades,
            Overlay::Upgrades => Overlay::None,
            other => other,
        };
    }
    // Close the picker once everything is picked.
    if *overlay == Overlay::Upgrades && roster.me(&session).is_none_or(|m| m.choices.is_empty()) {
        *overlay = Overlay::None;
    }
    paused.0 = *overlay != Overlay::None;
}

/// The mouse is captured while playing and freed for menus, the end screen
/// and when you switch to another window (click to grab it again).
fn cursor_control(
    overlay: Res<Overlay>,
    state: Res<MatchState>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut focus_events: EventReader<bevy::window::WindowFocused>,
    mut away: Local<bool>,
    mut paused: ResMut<Paused>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    for ev in focus_events.read() {
        *away = !ev.focused;
    }
    if mouse.just_pressed(MouseButton::Left) {
        *away = false;
    }
    let want = *overlay == Overlay::None && !match_ended(&state) && !*away;
    if want != cursor_locked(&window) {
        set_cursor_lock(&mut window, want);
    }
    // Clicking away from the window pauses a solo game too.
    if *away {
        paused.0 = true;
    }
}

/// Gacha spins are earned by how far you get: nothing below round 5, then
/// more for every 5 rounds, doubled if the team extracts.
fn award_spins(
    state: Res<MatchState>,
    mut result: ResMut<MatchResult>,
    mut profile: ResMut<Profile>,
) {
    if result.awarded || !match_ended(&state) {
        return;
    }
    // The round you were on only counts if you got through it.
    let survived = if state.extracted {
        state.round
    } else {
        state.round.saturating_sub(1)
    };
    let spins = spins_for_round(survived, state.extracted);
    let new_best = survived > profile.best_round;
    profile.spins += spins;
    if new_best {
        profile.best_round = survived;
    }
    if state.extracted {
        profile.extractions += 1;
    }
    *result = MatchResult {
        awarded: true,
        spins,
        round: survived,
        extracted: state.extracted,
        new_best,
    };
}

fn box_visuals(
    time: Res<Time>,
    state: Res<MatchState>,
    map: Res<CurrentMap>,
    mut boxes: Query<(&mut Transform, &mut Visibility), (With<MysteryBox>, Without<BoxGun>)>,
    mut glow: Query<&mut PointLight, With<BoxGlow>>,
    gun: Single<
        (&mut Transform, &mut Visibility, &MeshMaterial3d<StandardMaterial>),
        (With<BoxGun>, Without<MysteryBox>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let spot = map.0.box_spots[(state.box_spot as usize).min(4)];
    let moving = matches!(state.box_state, BoxState::Moving { .. });
    for (mut tf, mut vis) in &mut boxes {
        tf.translation = spot + Vec3::Y * BOX_HALF.y;
        *vis = if moving { Visibility::Hidden } else { Visibility::Inherited };
    }
    let t = time.elapsed_secs();
    for mut light in &mut glow {
        light.color = match state.box_state {
            BoxState::Rolling { .. } => Color::srgb(1.0, 0.85, 0.3),
            BoxState::Offer { .. } => Color::srgb(0.4, 1.0, 0.5),
            _ => Color::srgb(0.4, 0.7, 1.0),
        };
        light.intensity = 60_000.0 + 20_000.0 * (t * 3.0).sin();
    }

    let (mut gtf, mut gvis, mat) = gun.into_inner();
    let (show, height, color) = match state.box_state {
        BoxState::Rolling { time, .. } => {
            // Flicker through colours while it "decides".
            let k = (t * 12.0).floor();
            let c = Color::hsl((k * 47.0) % 360.0, 0.8, 0.6);
            (true, 1.1 + (3.0 - time).clamp(0.0, 3.0) * 0.12, c)
        }
        BoxState::Offer { gun, .. } => {
            let c = if gun_def(gun).rare {
                Color::srgb(0.3, 1.0, 0.4)
            } else {
                Color::srgb(0.9, 0.9, 1.0)
            };
            (true, 1.5, c)
        }
        _ => (false, 0.0, Color::WHITE),
    };
    *gvis = if show { Visibility::Inherited } else { Visibility::Hidden };
    if show {
        gtf.translation = spot + Vec3::Y * height;
        gtf.rotation = Quat::from_rotation_y(t * 2.5);
        if let Some(m) = materials.get_mut(&mat.0) {
            m.base_color = color;
            m.emissive = LinearRgba::from(color) * 3.0;
        }
    }
}

fn extraction_visuals(
    state: Res<MatchState>,
    mut beacon: Query<&mut Visibility, With<ExtractionBeacon>>,
) {
    for mut vis in &mut beacon {
        *vis = if state.extraction > 0.0 && !match_ended(&state) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
