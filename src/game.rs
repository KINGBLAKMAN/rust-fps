//! Match flow: loading the map when a match starts and clearing it after,
//! travelling between the maps of a run, the in-game menus (pause, level-up
//! picks), cursor locking, the mystery box and teleporter visuals, and
//! handing out gacha spins at the end.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::config::{Action, InputExt, Profile, Settings};
use crate::data::{spins_for_round, ROUNDS_PER_PREMIUM_QUARTER};
use crate::maps::{
    spawn_map, BoxGlow, BoxLid, BoxPillar, CurrentMap, ExtractionBeacon, MysteryBox, BOX_HALF,
};
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
            .init_resource::<LoadedStage>()
            .add_systems(OnEnter(AppState::InGame), start_match)
            .add_systems(OnExit(AppState::InGame), end_match)
            .add_systems(OnEnter(AppState::Travel), |mut next: ResMut<NextState<AppState>>| {
                next.set(AppState::InGame)
            })
            .add_systems(
                Update,
                (travel, menu_keys, cursor_control, award_spins)
                    .chain()
                    .before(Phase::Local)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                (box_visuals, teleporter_visuals)
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
    /// The sandbox tools (F1 in sandbox mode).
    Sandbox,
}

/// What the last match earned, for the end screen.
#[derive(Resource, Default)]
pub struct MatchResult {
    pub awarded: bool,
    pub spins: u32,
    /// Quarter premium spins earned this match.
    pub premium_quarters: u32,
    pub round: u32,
    pub new_best: bool,
    /// Career XP earned, and career level before and after.
    pub xp: u32,
    pub levels: (u32, u32),
    /// The character played and their level before and after.
    pub character: crate::data::Character,
    pub char_levels: (u32, u32),
}

pub fn match_ended(state: &MatchState) -> bool {
    state.game_over || state.won
}

/// The map of the run that is built (the host moves `MatchState::stage` on
/// when the team takes the teleporter).
#[derive(Resource, Default)]
pub struct LoadedStage(u8);

/// Leaves the match for a frame when the run moves to the next map, so it
/// is rebuilt there (players keep everything; it lives in the Roster).
fn travel(
    state: Res<MatchState>,
    loaded: Res<LoadedStage>,
    mut next: ResMut<NextState<AppState>>,
) {
    if state.started && state.stage != loaded.0 {
        next.set(AppState::Travel);
    }
}

#[derive(Component)]
struct BoxGun;

/// One of the things that can float out of the box (a gun, or None for the
/// teddy bear).
#[derive(Component)]
struct BoxGunModel(Option<u8>);

pub fn start_match(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    state: Res<MatchState>,
    mut overlay: ResMut<Overlay>,
    mut paused: ResMut<Paused>,
    mut result: ResMut<MatchResult>,
    mut clear: ResMut<ClearColor>,
    guns: Res<crate::gunmodels::GunAssets>,
    mut ambient: ResMut<AmbientLight>,
    camera: Single<Entity, With<crate::player::LocalPlayer>>,
    mut loaded: ResMut<LoadedStage>,
) {
    loaded.0 = state.stage;
    let layout = spawn_map(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        state.map,
        state.night,
    );
    clear.0 = crate::graphics::sky_colors(layout.sky, state.night).0;
    // Night: dim blue ambient and a flashlight on your head.
    *ambient = if state.night {
        AmbientLight {
            color: Color::srgb(0.55, 0.65, 1.0),
            // Up from 45: the film tone mapping darkens the low end.
            brightness: 70.0,
            ..default()
        }
    } else {
        // Low, so shapes keep their shading; the fill light and the painted
        // material's cool shadow side do the rest. Nearly white so rooms
        // the sun can't reach keep their colours.
        AmbientLight {
            color: Color::srgb(0.96, 0.97, 1.0),
            brightness: 140.0,
            ..default()
        }
    };
    if state.night {
        let torch = commands
            .spawn((
                InGameEntity,
                SpotLight {
                    intensity: 900_000.0,
                    color: Color::srgb(1.0, 0.95, 0.85),
                    range: 40.0,
                    outer_angle: 0.5,
                    inner_angle: 0.3,
                    shadows_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.25, -0.2, 0.0),
            ))
            .id();
        commands.entity(*camera).add_child(torch);
    }
    commands.insert_resource(NavGrid::new(layout.half));
    crate::strips::spawn_wall_guns(&mut commands, &mut materials, &guns, &layout);
    commands.insert_resource(CurrentMap(layout));
    *overlay = Overlay::None;
    paused.0 = false;
    *result = MatchResult::default();

    // What floats out of the mystery box: every gun (one shown at a time)
    // and a teddy bear for when the box flies away.
    let glow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.9, 1.0),
        emissive: LinearRgba::rgb(0.6, 1.2, 2.0),
        ..default()
    });
    let teddy = meshes.add(teddy_kit().build_or_empty());
    let teddy_mat = materials.add(crate::kit::vertex_material(0.95, 0.0));
    commands
        .spawn((
            InGameEntity,
            BoxGun,
            Transform::default(),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            for id in 0..crate::data::GUNS.len() as u8 {
                p.spawn((
                    BoxGunModel(Some(id)),
                    Transform::from_scale(Vec3::splat(1.3)),
                    Visibility::Hidden,
                ))
                .with_children(|g| {
                    crate::gunmodels::spawn_gun(
                        g,
                        &guns,
                        id,
                        crate::data::Attach::NONE,
                        glow.clone(),
                        false,
                        None,
                    )
                });
            }
            p.spawn((
                BoxGunModel(None),
                Mesh3d(teddy),
                MeshMaterial3d(teddy_mat),
                Transform::default(),
                Visibility::Hidden,
            ));
        });
}

/// A teddy bear, shown when the box decides to fly away.
fn teddy_kit() -> crate::kit::Kit {
    use crate::kit::c;
    let mut k = crate::kit::Kit::new();
    let fur = c(0.55, 0.35, 0.18);
    let light = c(0.8, 0.62, 0.42);
    let v = Vec3::new;
    k.blob(v(0.0, 0.0, 0.0), v(0.2, 0.24, 0.17), fur);
    k.blob(v(0.0, 0.0, 0.12), v(0.12, 0.15, 0.07), light);
    k.sphere(v(0.0, 0.33, 0.0), 0.15, fur);
    k.blob(v(0.0, 0.3, 0.12), v(0.07, 0.055, 0.06), light);
    k.sphere(v(0.0, 0.32, 0.18), 0.022, c(0.05, 0.03, 0.02));
    for s in [-1.0f32, 1.0] {
        k.sphere(v(s * 0.1, 0.45, 0.0), 0.06, fur);
        k.sphere(v(s * 0.1, 0.45, 0.03), 0.035, light);
        k.sphere(v(s * 0.055, 0.37, 0.13), 0.018, c(0.02, 0.02, 0.02));
        k.capsule_between(v(s * 0.17, 0.1, 0.0), v(s * 0.28, -0.05, 0.08), 0.06, fur);
        k.capsule_between(v(s * 0.1, -0.18, 0.02), v(s * 0.13, -0.3, 0.12), 0.07, fur);
    }
    k.cuboid(v(0.0, 0.2, 0.1), v(0.22, 0.04, 0.05), c(0.8, 0.1, 0.15));
    k
}

fn end_match(
    mut commands: Commands,
    things: Query<Entity, With<InGameEntity>>,
    mut clear: ResMut<ClearColor>,
    mut ambient: ResMut<AmbientLight>,
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
    ambient.brightness = 350.0;
    ambient.color = Color::WHITE;
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
            Overlay::Pause | Overlay::Upgrades | Overlay::Sandbox => Overlay::None,
        };
    } else if keys.just_pressed(KeyCode::F1) && state.sandbox.on {
        *overlay = match *overlay {
            Overlay::None => Overlay::Sandbox,
            Overlay::Sandbox => Overlay::None,
            other => other,
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
/// more for every 5 rounds. Every 20 rounds survived (over all matches)
/// also earns a quarter of a premium spin.
fn award_spins(
    state: Res<MatchState>,
    mut result: ResMut<MatchResult>,
    mut profile: ResMut<Profile>,
    session: Res<crate::Session>,
    roster: Res<crate::Roster>,
) {
    if result.awarded || !match_ended(&state) {
        return;
    }
    // The round you were on only counts if you got through it.
    let survived = if state.won {
        state.round
    } else {
        state.round.saturating_sub(1)
    };
    let spins = spins_for_round(survived);
    let new_best = survived > profile.best_round;
    profile.spins += spins;
    profile.round_bank += survived;
    let premium_quarters = profile.round_bank / ROUNDS_PER_PREMIUM_QUARTER;
    profile.round_bank %= ROUNDS_PER_PREMIUM_QUARTER;
    profile.premium_quarters += premium_quarters;
    if new_best {
        profile.best_round = survived;
    }
    if state.won {
        profile.extractions += 1;
    }
    let kills = roster.me(&session).map_or(0, |p| p.kills);
    let cleared = state.stage as u32 + state.won as u32;
    let xp = crate::progression::match_xp(survived, kills, cleared, state.won);
    let before = crate::progression::career(profile.career_xp).0;
    profile.career_xp += xp;
    let after = crate::progression::career(profile.career_xp).0;
    // The same XP goes to the character you played.
    let character = profile.character;
    let char_before = profile.char_level(character).0;
    *profile.char_xp.entry(character).or_insert(0) += xp;
    let char_after = profile.char_level(character).0;
    *result = MatchResult {
        awarded: true,
        spins,
        premium_quarters,
        round: survived,
        new_best,
        xp,
        levels: (before, after),
        character,
        char_levels: (char_before, char_after),
    };
}

#[allow(clippy::too_many_arguments)]
fn box_visuals(
    time: Res<Time>,
    state: Res<MatchState>,
    map: Res<CurrentMap>,
    mut last_spot: Local<Option<Vec3>>,
    mut lid_angle: Local<f32>,
    mut boxes: Query<
        (&mut Transform, &mut Visibility),
        (With<MysteryBox>, Without<BoxGun>, Without<BoxLid>),
    >,
    mut lids: Query<
        &mut Transform,
        (
            With<BoxLid>,
            Without<MysteryBox>,
            Without<BoxGun>,
            Without<BoxGunModel>,
        ),
    >,
    mut glow: Query<&mut PointLight, With<BoxGlow>>,
    gun: Single<
        (&mut Transform, &mut Visibility),
        (With<BoxGun>, Without<MysteryBox>, Without<BoxGunModel>),
    >,
    mut models: Query<(&BoxGunModel, &mut Visibility), (Without<BoxGun>, Without<MysteryBox>)>,
    mut pillars: Query<
        (&mut Visibility, &mut Transform),
        (
            With<BoxPillar>,
            Without<BoxGun>,
            Without<MysteryBox>,
            Without<BoxGunModel>,
            Without<BoxLid>,
        ),
    >,
) {
    let spot = map.0.box_spots[(state.box_spot as usize).min(4)];
    let t = time.elapsed_secs();
    let dt = time.delta_secs();
    // Flying away: rise and spin off the old spot, then drop onto the new.
    let (pos, spin, show_box) = match state.box_state {
        BoxState::Moving { time } => {
            let from = last_spot.unwrap_or(spot);
            if time > 2.0 {
                let k = 4.0 - time;
                (
                    from + Vec3::Y * (BOX_HALF.y + k * k * 4.0),
                    k * k * 3.0,
                    true,
                )
            } else {
                (
                    spot + Vec3::Y * (BOX_HALF.y + time * time * 5.0),
                    time * 4.0,
                    time < 1.9,
                )
            }
        }
        _ => {
            *last_spot = Some(spot);
            (spot + Vec3::Y * BOX_HALF.y, 0.0, true)
        }
    };
    for (mut tf, mut vis) in &mut boxes {
        tf.translation = pos;
        tf.rotation = Quat::from_rotation_y(spin);
        *vis = if show_box {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let open = matches!(
        state.box_state,
        BoxState::Rolling { .. } | BoxState::Offer { .. }
    );
    // The beam pulses so it catches the eye.
    let pulse = 1.0 + 0.25 * (t * 2.5).sin();
    for (mut vis, mut tf) in &mut pillars {
        *vis = if open || !matches!(state.box_state, BoxState::Idle) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        tf.scale = Vec3::new(pulse, 1.0, pulse);
    }
    let target = if open { -1.25 } else { 0.0 };
    *lid_angle += (target - *lid_angle) * (1.0 - (-8.0 * dt).exp());
    for mut tf in &mut lids {
        tf.rotation = Quat::from_rotation_x(*lid_angle);
    }
    for mut light in &mut glow {
        light.color = match state.box_state {
            BoxState::Rolling { .. } => Color::srgb(1.0, 0.85, 0.3),
            BoxState::Offer { .. } => Color::srgb(0.4, 1.0, 0.5),
            _ => Color::srgb(0.4, 0.7, 1.0),
        };
        light.intensity = 60_000.0 + 20_000.0 * (t * 3.0).sin() + if open { 80_000.0 } else { 0.0 };
    }

    let (mut gtf, mut gvis) = gun.into_inner();
    // Which model floats above the box, and how high.
    let (shown, height) = match state.box_state {
        BoxState::Rolling { time, .. } => {
            // Cycles through guns, slowing down as it "decides".
            let rate = 4.0 + time * 4.0;
            let k = (t * rate).floor() as usize;
            (
                Some(Some(((k * 7) % 23) as u8)),
                0.9 + (3.0 - time).clamp(0.0, 3.0) * 0.15,
            )
        }
        BoxState::Offer { gun, .. } => (Some(Some(gun)), 1.35),
        BoxState::Moving { time } if time > 2.6 => (Some(None), 0.9 + (4.0 - time) * 0.5),
        _ => (None, 0.0),
    };
    *gvis = if shown.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if let Some(which) = shown {
        let base = if matches!(state.box_state, BoxState::Moving { .. }) {
            last_spot.unwrap_or(spot)
        } else {
            spot
        };
        gtf.translation = base + Vec3::Y * height;
        gtf.rotation = Quat::from_rotation_y(t * 1.5);
        for (model, mut vis) in &mut models {
            *vis = if model.0 == which {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn teleporter_visuals(
    time: Res<Time>,
    state: Res<MatchState>,
    mut beacon: Query<(&mut Visibility, &mut Transform), With<ExtractionBeacon>>,
) {
    for (mut vis, mut tf) in &mut beacon {
        *vis = if state.teleport && !match_ended(&state) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        tf.rotation = Quat::from_rotation_y(time.elapsed_secs() * 0.6);
    }
}
