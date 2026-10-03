//! A small first-person shooter built with Bevy.
//!
//! Controls: mouse to look, WASD to move, Shift to sprint, Space to jump,
//! left click to shoot, R to reload, Esc to release the mouse.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use rand::Rng;
use std::f32::consts::FRAC_PI_2;

// ---------------------------------------------------------------------------
// Tuning
// ---------------------------------------------------------------------------

const EYE_HEIGHT: f32 = 1.6;
const PLAYER_RADIUS: f32 = 0.4;
const WALK_SPEED: f32 = 6.0;
const SPRINT_SPEED: f32 = 9.5;
const JUMP_SPEED: f32 = 6.5;
const GRAVITY: f32 = 18.0;
const MOUSE_SENSITIVITY: f32 = 0.0022;
const MAX_HEALTH: f32 = 100.0;

const MAG_SIZE: u32 = 12;
const FIRE_COOLDOWN: f32 = 0.16;
const RELOAD_TIME: f32 = 1.2;
const GUN_DAMAGE: f32 = 34.0;
const GUN_RANGE: f32 = 100.0;

const ARENA_HALF: f32 = 30.0;
const PLAYER_START: Vec3 = Vec3::new(0.0, 0.0, 12.0);

// ---------------------------------------------------------------------------
// Components and resources
// ---------------------------------------------------------------------------

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Playing,
    GameOver,
}

#[derive(Component)]
struct Player {
    yaw: f32,
    pitch: f32,
    vertical_velocity: f32,
    on_ground: bool,
}

#[derive(Component)]
struct Gun {
    recoil: f32,
    flash_timer: f32,
}

#[derive(Component)]
struct MuzzleFlash;

/// Axis-aligned box collider for static level geometry (half extents).
#[derive(Component)]
struct Collider {
    half: Vec3,
}

#[derive(Clone, Copy, PartialEq)]
enum EnemyKind {
    Grunt,
    Shooter,
}

#[derive(Component)]
struct Enemy {
    kind: EnemyKind,
    health: f32,
    speed: f32,
    attack_timer: f32,
    hit_flash: f32,
    base_color: Color,
}

#[derive(Component)]
struct Projectile {
    velocity: Vec3,
    life: f32,
}

#[derive(Component)]
struct HealthText;
#[derive(Component)]
struct AmmoText;
#[derive(Component)]
struct ScoreText;
#[derive(Component)]
struct CenterText;
#[derive(Component)]
struct DamageOverlay;
#[derive(Component)]
struct GameOverScreen;

#[derive(Resource)]
struct PlayerStats {
    health: f32,
    score: u32,
    kills: u32,
    ammo: u32,
    fire_cooldown: f32,
    reload_timer: f32,
    damage_flash: f32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            health: MAX_HEALTH,
            score: 0,
            kills: 0,
            ammo: MAG_SIZE,
            fire_cooldown: 0.0,
            reload_timer: 0.0,
            damage_flash: 0.0,
        }
    }
}

#[derive(Resource, Default)]
struct Waves {
    wave: u32,
    to_spawn: u32,
    spawn_timer: f32,
    intermission: f32,
    banner_timer: f32,
}

#[derive(Resource)]
struct GameAssets {
    enemy_mesh: Handle<Mesh>,
    eye_mesh: Handle<Mesh>,
    eye_material: Handle<StandardMaterial>,
    projectile_mesh: Handle<Mesh>,
    projectile_material: Handle<StandardMaterial>,
}

/// A short-lived bullet tracer drawn with gizmos.
#[derive(Resource, Default)]
struct Tracers(Vec<(Vec3, Vec3, f32)>);

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rust FPS".into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .insert_resource(ClearColor(Color::srgb(0.45, 0.62, 0.85)))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 350.0,
            ..default()
        })
        .init_resource::<PlayerStats>()
        .init_resource::<Waves>()
        .init_resource::<Tracers>()
        .add_systems(Startup, (setup_level, setup_player, setup_hud))
        .add_systems(OnEnter(GameState::Playing), reset_game)
        .add_systems(OnEnter(GameState::GameOver), show_game_over)
        .add_systems(OnExit(GameState::GameOver), hide_game_over)
        .add_systems(
            Update,
            (
                mouse_look,
                player_movement,
                shoot,
                reload,
                waves,
                enemy_ai,
                projectiles,
                enemy_hit_flash,
                check_death,
            )
                .chain()
                // The game is paused while the mouse isn't captured.
                .run_if(in_state(GameState::Playing).and(is_cursor_locked)),
        )
        // After shooting, so the click that captures the mouse doesn't fire.
        .add_systems(
            Update,
            grab_cursor
                .after(check_death)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, (animate_gun, draw_tracers, update_hud))
        .add_systems(Update, restart.run_if(in_state(GameState::GameOver)))
        .run();
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Floor
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(ARENA_HALF * 2.0, ARENA_HALF * 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.32, 0.36, 0.30),
            perceptual_roughness: 0.95,
            ..default()
        })),
    ));

    // Floor grid stripes for a sense of motion
    let stripe_mat = materials.add(Color::srgb(0.27, 0.30, 0.25));
    let stripe_mesh = meshes.add(Cuboid::new(ARENA_HALF * 2.0, 0.01, 0.08));
    for i in -6..=6 {
        let p = i as f32 * 5.0;
        commands.spawn((
            Mesh3d(stripe_mesh.clone()),
            MeshMaterial3d(stripe_mat.clone()),
            Transform::from_xyz(0.0, 0.005, p),
        ));
        commands.spawn((
            Mesh3d(stripe_mesh.clone()),
            MeshMaterial3d(stripe_mat.clone()),
            Transform::from_xyz(p, 0.005, 0.0).with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
        ));
    }

    let wall_mat = materials.add(Color::srgb(0.55, 0.52, 0.48));
    let pillar_mat = materials.add(Color::srgb(0.42, 0.45, 0.55));
    let crate_mat = materials.add(Color::srgb(0.62, 0.44, 0.24));

    let mut spawn_box = |pos: Vec3, size: Vec3, mat: &Handle<StandardMaterial>| {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(pos),
            Collider { half: size / 2.0 },
        ));
    };

    // Outer walls
    let h = 4.0;
    let len = ARENA_HALF * 2.0 + 2.0;
    spawn_box(Vec3::new(0.0, h / 2.0, -ARENA_HALF - 0.5), Vec3::new(len, h, 1.0), &wall_mat);
    spawn_box(Vec3::new(0.0, h / 2.0, ARENA_HALF + 0.5), Vec3::new(len, h, 1.0), &wall_mat);
    spawn_box(Vec3::new(-ARENA_HALF - 0.5, h / 2.0, 0.0), Vec3::new(1.0, h, len), &wall_mat);
    spawn_box(Vec3::new(ARENA_HALF + 0.5, h / 2.0, 0.0), Vec3::new(1.0, h, len), &wall_mat);

    // Pillars
    for (x, z) in [
        (-12.0, -12.0),
        (12.0, -12.0),
        (-12.0, 12.0),
        (12.0, 12.0),
        (0.0, -18.0),
        (-20.0, 0.0),
        (20.0, 0.0),
    ] {
        spawn_box(Vec3::new(x, 2.5, z), Vec3::new(2.0, 5.0, 2.0), &pillar_mat);
    }

    // Low cover walls
    spawn_box(Vec3::new(-6.0, 0.75, -4.0), Vec3::new(6.0, 1.5, 0.6), &wall_mat);
    spawn_box(Vec3::new(7.0, 0.75, 3.0), Vec3::new(0.6, 1.5, 6.0), &wall_mat);
    spawn_box(Vec3::new(0.0, 0.75, 22.0), Vec3::new(8.0, 1.5, 0.6), &wall_mat);

    // Crates (can be jumped on)
    for (x, z, s) in [
        (-3.0, 6.0, 1.2),
        (-1.8, 6.0, 1.0),
        (4.0, -10.0, 1.4),
        (15.0, 18.0, 1.2),
        (-17.0, -20.0, 1.4),
        (22.0, -15.0, 1.0),
        (-24.0, 16.0, 1.2),
        (-8.0, 14.0, 1.0),
    ] {
        spawn_box(Vec3::new(x, s / 2.0, z), Vec3::splat(s), &crate_mat);
    }
    // A stacked crate
    spawn_box(Vec3::new(-3.0, 1.2 + 0.5, 6.0), Vec3::splat(1.0), &crate_mat);

    // Lights
    commands.spawn((
        DirectionalLight {
            illuminance: 9000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(15.0, 30.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Shared enemy assets
    commands.insert_resource(GameAssets {
        enemy_mesh: meshes.add(Capsule3d::new(0.5, 1.0)),
        eye_mesh: meshes.add(Sphere::new(0.1)),
        eye_material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 0.6),
            emissive: LinearRgba::rgb(6.0, 6.0, 2.0),
            ..default()
        }),
        projectile_mesh: meshes.add(Sphere::new(0.2)),
        projectile_material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.5, 0.1),
            emissive: LinearRgba::rgb(12.0, 4.0, 0.5),
            ..default()
        }),
    });
}

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let gun_body = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.12, 0.14),
        metallic: 0.6,
        perceptual_roughness: 0.4,
        ..default()
    });
    let gun_accent = materials.add(Color::srgb(0.7, 0.35, 0.1));

    commands
        .spawn((
            Player {
                yaw: 0.0,
                pitch: 0.0,
                vertical_velocity: 0.0,
                on_ground: true,
            },
            Camera3d::default(),
            Projection::from(PerspectiveProjection {
                fov: 75f32.to_radians(),
                near: 0.05,
                ..default()
            }),
            Transform::from_translation(PLAYER_START + Vec3::Y * EYE_HEIGHT),
        ))
        .with_children(|cam| {
            cam.spawn((
                Gun {
                    recoil: 0.0,
                    flash_timer: 0.0,
                },
                Transform::from_xyz(0.28, -0.24, -0.55).with_scale(Vec3::splat(0.75)),
                Visibility::default(),
            ))
            .with_children(|gun| {
                gun.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.1, 0.12, 0.5))),
                    MeshMaterial3d(gun_body.clone()),
                ));
                gun.spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.03, 0.25))),
                    MeshMaterial3d(gun_body.clone()),
                    Transform::from_xyz(0.0, 0.02, -0.33)
                        .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                ));
                gun.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.08, 0.16, 0.08))),
                    MeshMaterial3d(gun_accent.clone()),
                    Transform::from_xyz(0.0, -0.12, 0.12)
                        .with_rotation(Quat::from_rotation_x(-0.3)),
                ));
                gun.spawn((
                    MuzzleFlash,
                    PointLight {
                        intensity: 0.0,
                        color: Color::srgb(1.0, 0.8, 0.4),
                        range: 12.0,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.02, -0.5),
                ));
            });
        });
}

fn setup_hud(mut commands: Commands) {
    let font = |size: f32| TextFont {
        font_size: size,
        ..default()
    };

    // Red damage vignette (full screen)
    commands.spawn((
        DamageOverlay,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.8, 0.0, 0.0, 0.0)),
    ));

    // Crosshair
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|p| {
            p.spawn((
                Node {
                    width: Val::Px(18.0),
                    height: Val::Px(2.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::WHITE),
            ));
            p.spawn((
                Node {
                    width: Val::Px(2.0),
                    height: Val::Px(18.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::WHITE),
            ));
        });

    commands.spawn((
        HealthText,
        Text::new(""),
        font(32.0),
        TextColor(Color::srgb(0.4, 1.0, 0.4)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            bottom: Val::Px(16.0),
            ..default()
        },
    ));
    commands.spawn((
        AmmoText,
        Text::new(""),
        font(32.0),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            bottom: Val::Px(16.0),
            ..default()
        },
    ));
    commands.spawn((
        ScoreText,
        Text::new(""),
        font(26.0),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(14.0),
            ..default()
        },
    ));
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            top: Val::Percent(30.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|p| {
            p.spawn((
                CenterText,
                Text::new(""),
                font(40.0),
                TextColor(Color::srgb(1.0, 0.9, 0.4)),
                TextLayout::new_with_justify(JustifyText::Center),
            ));
        });
}

// ---------------------------------------------------------------------------
// Game flow
// ---------------------------------------------------------------------------

fn reset_game(
    mut commands: Commands,
    mut stats: ResMut<PlayerStats>,
    mut waves: ResMut<Waves>,
    mut tracers: ResMut<Tracers>,
    player: Single<(&mut Transform, &mut Player)>,
    enemies: Query<Entity, Or<(With<Enemy>, With<Projectile>)>>,
) {
    *stats = PlayerStats::default();
    *waves = Waves {
        intermission: 1.5,
        ..default()
    };
    tracers.0.clear();
    for e in &enemies {
        commands.entity(e).despawn();
    }
    let (mut tf, mut p) = player.into_inner();
    tf.translation = PLAYER_START + Vec3::Y * EYE_HEIGHT;
    p.yaw = 0.0;
    p.pitch = 0.0;
    p.vertical_velocity = 0.0;
    tf.rotation = Quat::IDENTITY;
}

fn check_death(stats: Res<PlayerStats>, mut next: ResMut<NextState<GameState>>) {
    if stats.health <= 0.0 {
        next.set(GameState::GameOver);
    }
}

fn show_game_over(
    mut commands: Commands,
    stats: Res<PlayerStats>,
    waves: Res<Waves>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;
    commands
        .spawn((
            GameOverScreen,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("YOU DIED"),
                TextFont {
                    font_size: 72.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.15, 0.15)),
            ));
            p.spawn((
                Text::new(format!(
                    "Score: {}   Kills: {}   Wave: {}",
                    stats.score, stats.kills, waves.wave
                )),
                TextFont {
                    font_size: 32.0,
                    ..default()
                },
            ));
            p.spawn((
                Text::new("Press Enter to play again"),
                TextFont {
                    font_size: 26.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

fn hide_game_over(mut commands: Commands, screens: Query<Entity, With<GameOverScreen>>) {
    for e in &screens {
        commands.entity(e).despawn();
    }
}

fn restart(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::Enter) {
        next.set(GameState::Playing);
    }
}

// ---------------------------------------------------------------------------
// Player
// ---------------------------------------------------------------------------

fn grab_cursor(
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}

fn cursor_locked(window: &Window) -> bool {
    window.cursor_options.grab_mode != CursorGrabMode::None
}

fn is_cursor_locked(window: Single<&Window, With<PrimaryWindow>>) -> bool {
    cursor_locked(&window)
}

fn mouse_look(
    motion: Res<AccumulatedMouseMotion>,
    window: Single<&Window, With<PrimaryWindow>>,
    player: Single<(&mut Transform, &mut Player)>,
) {
    if !cursor_locked(&window) {
        return;
    }
    let (mut tf, mut p) = player.into_inner();
    p.yaw -= motion.delta.x * MOUSE_SENSITIVITY;
    p.pitch = (p.pitch - motion.delta.y * MOUSE_SENSITIVITY).clamp(-1.5, 1.5);
    tf.rotation = Quat::from_euler(EulerRot::YXZ, p.yaw, p.pitch, 0.0);
}

/// Pushes a circle (in XZ) out of every box it overlaps, ignoring boxes whose
/// top is below `feet_y` (so you can stand on crates).
fn resolve_collisions(pos: &mut Vec3, radius: f32, feet_y: f32, colliders: &[(Vec3, Vec3)]) {
    for (center, half) in colliders {
        let top = center.y + half.y;
        if feet_y >= top - 0.05 {
            continue;
        }
        let closest = Vec2::new(
            pos.x.clamp(center.x - half.x, center.x + half.x),
            pos.z.clamp(center.z - half.z, center.z + half.z),
        );
        let p = Vec2::new(pos.x, pos.z);
        let diff = p - closest;
        let dist = diff.length();
        if dist < radius {
            let push = if dist > 1e-4 {
                diff / dist * (radius - dist)
            } else {
                // Center is inside the box: push out along the shallowest axis.
                let dx = half.x - (pos.x - center.x).abs();
                let dz = half.z - (pos.z - center.z).abs();
                if dx < dz {
                    Vec2::new((dx + radius) * (pos.x - center.x).signum(), 0.0)
                } else {
                    Vec2::new(0.0, (dz + radius) * (pos.z - center.z).signum())
                }
            };
            pos.x += push.x;
            pos.z += push.y;
        }
    }
}

/// Highest box top under the circle that the feet are above (0 = floor).
fn ground_height(pos: Vec3, radius: f32, feet_y: f32, colliders: &[(Vec3, Vec3)]) -> f32 {
    let mut ground: f32 = 0.0;
    for (center, half) in colliders {
        let top = center.y + half.y;
        let inside_x = (pos.x - center.x).abs() < half.x + radius * 0.7;
        let inside_z = (pos.z - center.z).abs() < half.z + radius * 0.7;
        if inside_x && inside_z && feet_y >= top - 0.3 {
            ground = ground.max(top);
        }
    }
    ground
}

fn collider_list(colliders: &Query<(&Transform, &Collider), Without<Player>>) -> Vec<(Vec3, Vec3)> {
    colliders.iter().map(|(t, c)| (t.translation, c.half)).collect()
}

fn player_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    player: Single<(&mut Transform, &mut Player)>,
    colliders: Query<(&Transform, &Collider), Without<Player>>,
) {
    let dt = time.delta_secs();
    let (mut tf, mut p) = player.into_inner();
    let boxes = collider_list(&colliders);

    let forward = Vec3::new(-p.yaw.sin(), 0.0, -p.yaw.cos());
    let right = Vec3::new(forward.z * -1.0, 0.0, forward.x);
    let mut wish = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        wish += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        wish -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        wish += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        wish -= right;
    }
    let speed = if keys.pressed(KeyCode::ShiftLeft) {
        SPRINT_SPEED
    } else {
        WALK_SPEED
    };

    let mut feet = tf.translation - Vec3::Y * EYE_HEIGHT;
    feet += wish.normalize_or_zero() * speed * dt;

    if p.on_ground && keys.just_pressed(KeyCode::Space) {
        p.vertical_velocity = JUMP_SPEED;
        p.on_ground = false;
    }
    p.vertical_velocity -= GRAVITY * dt;
    feet.y += p.vertical_velocity * dt;

    let feet_y = feet.y;
    resolve_collisions(&mut feet, PLAYER_RADIUS, feet_y, &boxes);
    feet.x = feet.x.clamp(-ARENA_HALF + PLAYER_RADIUS, ARENA_HALF - PLAYER_RADIUS);
    feet.z = feet.z.clamp(-ARENA_HALF + PLAYER_RADIUS, ARENA_HALF - PLAYER_RADIUS);

    let ground = ground_height(feet, PLAYER_RADIUS, feet.y, &boxes);
    if feet.y <= ground {
        feet.y = ground;
        p.vertical_velocity = 0.0;
        p.on_ground = true;
    } else {
        p.on_ground = feet.y - ground < 0.05;
    }

    tf.translation = feet + Vec3::Y * EYE_HEIGHT;
}

// ---------------------------------------------------------------------------
// Shooting
// ---------------------------------------------------------------------------

/// Ray vs axis-aligned box (slab method). Returns hit distance.
fn ray_aabb(origin: Vec3, dir: Vec3, center: Vec3, half: Vec3) -> Option<f32> {
    let min = center - half;
    let max = center + half;
    let inv = dir.recip();
    let t1 = (min - origin) * inv;
    let t2 = (max - origin) * inv;
    let tmin = t1.min(t2).max_element();
    let tmax = t1.max(t2).min_element();
    (tmax >= tmin.max(0.0)).then_some(tmin.max(0.0))
}

/// Ray vs sphere. Returns hit distance.
fn ray_sphere(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = origin - center;
    let b = oc.dot(dir);
    let c = oc.length_squared() - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    (t >= 0.0).then_some(t)
}

/// Enemies are capsules; approximate with a stack of spheres.
fn ray_enemy(origin: Vec3, dir: Vec3, enemy_pos: Vec3) -> Option<f32> {
    [0.55, 1.0, 1.45]
        .iter()
        .filter_map(|y| ray_sphere(origin, dir, enemy_pos + Vec3::Y * (y - 1.0), 0.52))
        .min_by(|a, b| a.total_cmp(b))
}

fn shoot(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut stats: ResMut<PlayerStats>,
    waves: Res<Waves>,
    mut tracers: ResMut<Tracers>,
    camera: Single<&Transform, With<Player>>,
    mut gun: Single<&mut Gun>,
    mut enemies: Query<(Entity, &Transform, &mut Enemy)>,
    colliders: Query<(&Transform, &Collider), Without<Player>>,
) {
    stats.fire_cooldown -= time.delta_secs();
    if !cursor_locked(&window) || !mouse.pressed(MouseButton::Left) {
        return;
    }
    if stats.fire_cooldown > 0.0 || stats.reload_timer > 0.0 {
        return;
    }
    if stats.ammo == 0 {
        stats.reload_timer = RELOAD_TIME;
        return;
    }
    stats.ammo -= 1;
    stats.fire_cooldown = FIRE_COOLDOWN;
    gun.recoil = 1.0;
    gun.flash_timer = 0.05;

    let origin = camera.translation;
    let dir = camera.forward().as_vec3();

    let mut nearest = GUN_RANGE;
    for (tf, c) in &colliders {
        if let Some(t) = ray_aabb(origin, dir, tf.translation, c.half) {
            nearest = nearest.min(t);
        }
    }
    // Floor
    if dir.y < 0.0 {
        nearest = nearest.min(-origin.y / dir.y);
    }

    let mut hit: Option<(f32, Entity)> = None;
    for (e, tf, _) in &enemies {
        if let Some(t) = ray_enemy(origin, dir, tf.translation) {
            if t < nearest && hit.is_none_or(|(best, _)| t < best) {
                hit = Some((t, e));
            }
        }
    }

    let end_dist = hit.map(|(t, _)| t).unwrap_or(nearest);
    // Start the tracer at the gun muzzle rather than the eye.
    let muzzle = origin + camera.rotation * Vec3::new(0.28, -0.2, -1.0);
    tracers.0.push((muzzle, origin + dir * end_dist, 0.06));

    if let Some((_, entity)) = hit {
        if let Ok((_, _, mut enemy)) = enemies.get_mut(entity) {
            enemy.health -= GUN_DAMAGE;
            enemy.hit_flash = 0.1;
            if enemy.health <= 0.0 {
                stats.score += match enemy.kind {
                    EnemyKind::Grunt => 100,
                    EnemyKind::Shooter => 150,
                } * waves.wave.max(1);
                stats.kills += 1;
                commands.entity(entity).despawn();
            }
        }
    }
}

fn reload(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut stats: ResMut<PlayerStats>,
) {
    if keys.just_pressed(KeyCode::KeyR) && stats.reload_timer <= 0.0 && stats.ammo < MAG_SIZE {
        stats.reload_timer = RELOAD_TIME;
    }
    if stats.reload_timer > 0.0 {
        stats.reload_timer -= time.delta_secs();
        if stats.reload_timer <= 0.0 {
            stats.reload_timer = 0.0;
            stats.ammo = MAG_SIZE;
        }
    }
}

fn animate_gun(
    time: Res<Time>,
    stats: Res<PlayerStats>,
    gun: Single<(&mut Gun, &mut Transform)>,
    mut flash: Single<&mut PointLight, With<MuzzleFlash>>,
) {
    let dt = time.delta_secs();
    let (mut gun, mut tf) = gun.into_inner();
    gun.recoil = (gun.recoil - dt * 8.0).max(0.0);
    gun.flash_timer -= dt;

    // Dip the gun down while reloading.
    let reload_dip = if stats.reload_timer > 0.0 {
        (stats.reload_timer / RELOAD_TIME * std::f32::consts::PI).sin() * 0.25
    } else {
        0.0
    };
    tf.translation = Vec3::new(0.28, -0.24 - reload_dip, -0.55 + gun.recoil * 0.08);
    tf.rotation = Quat::from_rotation_x(gun.recoil * 0.15 - reload_dip * 2.0);
    tf.scale = Vec3::splat(0.75);

    flash.intensity = if gun.flash_timer > 0.0 { 60_000.0 } else { 0.0 };
}

fn draw_tracers(time: Res<Time>, mut tracers: ResMut<Tracers>, mut gizmos: Gizmos) {
    let dt = time.delta_secs();
    for (a, b, life) in tracers.0.iter_mut() {
        gizmos.line(*a, *b, Color::srgb(1.0, 0.9, 0.5));
        *life -= dt;
    }
    tracers.0.retain(|(_, _, life)| *life > 0.0);
}

// ---------------------------------------------------------------------------
// Enemies
// ---------------------------------------------------------------------------

fn waves(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<GameAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut waves: ResMut<Waves>,
    mut stats: ResMut<PlayerStats>,
    enemies: Query<(), With<Enemy>>,
    player: Single<&Transform, With<Player>>,
) {
    let dt = time.delta_secs();
    waves.banner_timer -= dt;

    if waves.to_spawn == 0 && enemies.is_empty() {
        if waves.intermission <= 0.0 {
            waves.intermission = 3.0;
            if waves.wave > 0 {
                stats.health = (stats.health + 25.0).min(MAX_HEALTH);
            }
        }
        waves.intermission -= dt;
        if waves.intermission <= 0.0 {
            waves.wave += 1;
            waves.to_spawn = 3 + waves.wave * 2;
            waves.spawn_timer = 0.0;
            waves.banner_timer = 2.5;
        }
        return;
    }

    if waves.to_spawn == 0 {
        return;
    }
    waves.spawn_timer -= dt;
    if waves.spawn_timer > 0.0 {
        return;
    }
    waves.spawn_timer = (1.2 - waves.wave as f32 * 0.08).max(0.4);
    waves.to_spawn -= 1;

    let mut rng = rand::thread_rng();
    let spawn_points = [
        Vec3::new(-26.0, 0.0, -26.0),
        Vec3::new(26.0, 0.0, -26.0),
        Vec3::new(-26.0, 0.0, 26.0),
        Vec3::new(26.0, 0.0, 26.0),
        Vec3::new(0.0, 0.0, -26.0),
    ];
    // Pick a spawn point that isn't right next to the player.
    let player_pos = player.translation;
    let candidates: Vec<Vec3> = spawn_points
        .iter()
        .copied()
        .filter(|p| p.distance(player_pos.with_y(0.0)) > 12.0)
        .collect();
    let base = candidates[rng.gen_range(0..candidates.len())];
    let pos = base + Vec3::new(rng.gen_range(-2.0..2.0), 1.0, rng.gen_range(-2.0..2.0));

    let kind = if waves.wave >= 2 && rng.gen_bool(0.35) {
        EnemyKind::Shooter
    } else {
        EnemyKind::Grunt
    };
    let (color, health, speed) = match kind {
        EnemyKind::Grunt => (
            Color::srgb(0.8, 0.15, 0.12),
            100.0,
            3.2 + waves.wave as f32 * 0.25,
        ),
        EnemyKind::Shooter => (Color::srgb(0.5, 0.2, 0.75), 70.0, 2.5),
    };

    commands
        .spawn((
            Enemy {
                kind,
                health,
                speed: speed.min(7.0),
                attack_timer: 1.0,
                hit_flash: 0.0,
                base_color: color,
            },
            Mesh3d(assets.enemy_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.6,
                ..default()
            })),
            Transform::from_translation(pos),
        ))
        .with_children(|e| {
            for x in [-0.18, 0.18] {
                e.spawn((
                    Mesh3d(assets.eye_mesh.clone()),
                    MeshMaterial3d(assets.eye_material.clone()),
                    Transform::from_xyz(x, 0.45, -0.42),
                ));
            }
        });
}

fn enemy_ai(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<GameAssets>,
    mut stats: ResMut<PlayerStats>,
    player: Single<&Transform, With<Player>>,
    mut enemies: Query<(Entity, &mut Transform, &mut Enemy), Without<Player>>,
    colliders: Query<(&Transform, &Collider), (Without<Player>, Without<Enemy>)>,
) {
    let dt = time.delta_secs();
    let boxes: Vec<(Vec3, Vec3)> = colliders.iter().map(|(t, c)| (t.translation, c.half)).collect();
    let player_eye = player.translation;
    let player_feet = player_eye - Vec3::Y * EYE_HEIGHT;

    // Snapshot positions for separation.
    let positions: Vec<(Entity, Vec3)> = enemies.iter().map(|(e, t, _)| (e, t.translation)).collect();

    for (entity, mut tf, mut enemy) in &mut enemies {
        let to_player = (player_feet - tf.translation).with_y(0.0);
        let dist = to_player.length();
        let dir = to_player.normalize_or_zero();

        // Face the player.
        if dist > 0.01 {
            let target = Vec3::new(player_feet.x, tf.translation.y, player_feet.z);
            tf.look_at(target, Vec3::Y);
        }

        let desired_dist = match enemy.kind {
            EnemyKind::Grunt => 1.2,
            EnemyKind::Shooter => 12.0,
        };
        let mut velocity = Vec3::ZERO;
        if dist > desired_dist {
            velocity = dir * enemy.speed;
        } else if enemy.kind == EnemyKind::Shooter && dist < desired_dist - 3.0 {
            velocity = -dir * enemy.speed * 0.6;
        }

        // Separation from other enemies.
        for (other, pos) in &positions {
            if *other == entity {
                continue;
            }
            let away = (tf.translation - *pos).with_y(0.0);
            let d = away.length();
            if d < 1.3 && d > 1e-3 {
                velocity += away / d * (1.3 - d) * 6.0;
            }
        }

        let mut pos = tf.translation + velocity * dt;
        let feet_y = pos.y - 1.0;
        resolve_collisions(&mut pos, 0.5, feet_y, &boxes);
        tf.translation = pos;

        enemy.attack_timer -= dt;
        match enemy.kind {
            EnemyKind::Grunt => {
                if dist < 1.8 && enemy.attack_timer <= 0.0 {
                    enemy.attack_timer = 0.9;
                    stats.health -= 12.0;
                    stats.damage_flash = 0.4;
                }
            }
            EnemyKind::Shooter => {
                if dist < 30.0 && enemy.attack_timer <= 0.0 {
                    enemy.attack_timer = 2.2;
                    let start = tf.translation + Vec3::Y * 0.45 + dir * 0.6;
                    let aim = (player_eye - Vec3::Y * 0.4 - start).normalize_or_zero();
                    commands.spawn((
                        Projectile {
                            velocity: aim * 14.0,
                            life: 4.0,
                        },
                        Mesh3d(assets.projectile_mesh.clone()),
                        MeshMaterial3d(assets.projectile_material.clone()),
                        Transform::from_translation(start),
                    ));
                }
            }
        }
    }
}

fn projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut stats: ResMut<PlayerStats>,
    player: Single<&Transform, With<Player>>,
    mut shots: Query<(Entity, &mut Transform, &mut Projectile), Without<Player>>,
    colliders: Query<(&Transform, &Collider), (Without<Player>, Without<Projectile>)>,
) {
    let dt = time.delta_secs();
    let player_body = player.translation - Vec3::Y * 0.7;
    for (e, mut tf, mut shot) in &mut shots {
        tf.translation += shot.velocity * dt;
        shot.life -= dt;

        let hit_player = tf.translation.distance(player_body) < 0.9;
        let hit_wall = tf.translation.y < 0.0
            || colliders.iter().any(|(ct, c)| {
                let d = (tf.translation - ct.translation).abs();
                d.x < c.half.x && d.y < c.half.y && d.z < c.half.z
            });
        if hit_player {
            stats.health -= 10.0;
            stats.damage_flash = 0.4;
        }
        if hit_player || hit_wall || shot.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

fn enemy_hit_flash(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut enemies: Query<(&mut Enemy, &MeshMaterial3d<StandardMaterial>)>,
) {
    for (mut enemy, mat) in &mut enemies {
        if enemy.hit_flash <= 0.0 {
            continue;
        }
        enemy.hit_flash -= time.delta_secs();
        if let Some(m) = materials.get_mut(&mat.0) {
            m.base_color = if enemy.hit_flash > 0.0 {
                Color::WHITE
            } else {
                enemy.base_color
            };
        }
    }
}

// ---------------------------------------------------------------------------
// HUD
// ---------------------------------------------------------------------------

fn update_hud(
    time: Res<Time>,
    mut stats: ResMut<PlayerStats>,
    waves: Res<Waves>,
    window: Single<&Window, With<PrimaryWindow>>,
    state: Res<State<GameState>>,
    mut health: Single<(&mut Text, &mut TextColor), With<HealthText>>,
    mut ammo: Single<&mut Text, (With<AmmoText>, Without<HealthText>)>,
    mut score: Single<&mut Text, (With<ScoreText>, Without<HealthText>, Without<AmmoText>)>,
    mut center: Single<
        &mut Text,
        (With<CenterText>, Without<HealthText>, Without<AmmoText>, Without<ScoreText>),
    >,
    mut overlay: Single<&mut BackgroundColor, With<DamageOverlay>>,
) {
    let hp = stats.health.max(0.0);
    health.0 .0 = format!("HP {:.0}", hp);
    health.1 .0 = if hp > 60.0 {
        Color::srgb(0.4, 1.0, 0.4)
    } else if hp > 30.0 {
        Color::srgb(1.0, 0.85, 0.3)
    } else {
        Color::srgb(1.0, 0.3, 0.3)
    };

    ammo.0 = if stats.reload_timer > 0.0 {
        "RELOADING...".into()
    } else {
        format!("{} / {}", stats.ammo, MAG_SIZE)
    };
    score.0 = format!("Score {}   Kills {}   Wave {}", stats.score, stats.kills, waves.wave);

    center.0 = if *state.get() != GameState::Playing {
        String::new()
    } else if !cursor_locked(&window) {
        "PAUSED - click to play".into()
    } else if waves.banner_timer > 0.0 {
        format!("WAVE {}", waves.wave)
    } else if stats.ammo == 0 && stats.reload_timer <= 0.0 {
        "Press R to reload".into()
    } else {
        String::new()
    };

    stats.damage_flash = (stats.damage_flash - time.delta_secs()).max(0.0);
    overlay.0 = Color::srgba(0.8, 0.0, 0.0, stats.damage_flash * 0.8);
}
