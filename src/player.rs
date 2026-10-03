//! The local player: camera, movement, the gun and its effects.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use std::f32::consts::FRAC_PI_2;

use crate::physics::{collect_boxes, ground_height, resolve_collisions, trace_shot};
use crate::{
    cursor_locked, spawn_point, Collider, Enemy, MatchState, Phase, Roster, Session, ShotQueue,
    Tracers, ARENA_HALF, EYE_HEIGHT, PLAYER_RADIUS,
};

const WALK_SPEED: f32 = 6.0;
const SPRINT_SPEED: f32 = 9.5;
const JUMP_SPEED: f32 = 6.5;
const GRAVITY: f32 = 18.0;
const MOUSE_SENSITIVITY: f32 = 0.0022;

pub const MAG_SIZE: u32 = 12;
const FIRE_COOLDOWN: f32 = 0.16;
pub const RELOAD_TIME: f32 = 1.2;
pub const GUN_RANGE: f32 = 100.0;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Weapon>()
            .add_systems(Startup, setup_player)
            .add_systems(
                Update,
                (
                    respawn,
                    mouse_look,
                    player_movement,
                    shoot,
                    reload,
                    // After shooting, so the click that captures the mouse doesn't fire.
                    grab_cursor,
                    sync_to_roster,
                )
                    .chain()
                    .in_set(Phase::Local),
            )
            .add_systems(Update, (animate_gun, draw_tracers).in_set(Phase::Present));
    }
}

#[derive(Component)]
pub struct LocalPlayer {
    pub yaw: f32,
    pub pitch: f32,
    vertical_velocity: f32,
    on_ground: bool,
}

#[derive(Resource)]
pub struct Weapon {
    pub ammo: u32,
    fire_cooldown: f32,
    pub reload_timer: f32,
    recoil: f32,
    flash_timer: f32,
    /// Last `spawn_seq` we teleported for; starts unset so we always spawn.
    last_spawn_seq: Option<u32>,
}

impl Default for Weapon {
    fn default() -> Self {
        Self {
            ammo: MAG_SIZE,
            fire_cooldown: 0.0,
            reload_timer: 0.0,
            recoil: 0.0,
            flash_timer: 0.0,
            last_spawn_seq: None,
        }
    }
}

#[derive(Component)]
struct Gun;

#[derive(Component)]
struct MuzzleFlash;

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
            LocalPlayer {
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
            Transform::from_translation(crate::PLAYER_START + Vec3::Y * EYE_HEIGHT),
        ))
        .with_children(|cam| {
            cam.spawn((
                Gun,
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

/// Can the local player act right now?
fn can_act(session: &Session, roster: &Roster, state: &MatchState) -> bool {
    !state.game_over && roster.me(session).is_none_or(|me| me.alive)
}

/// Teleports to the spawn point whenever the host bumps our spawn counter
/// (game start, restart, revive).
fn respawn(
    session: Res<Session>,
    roster: Res<Roster>,
    mut weapon: ResMut<Weapon>,
    player: Single<(&mut Transform, &mut LocalPlayer)>,
) {
    let Some(me) = roster.me(&session) else {
        return;
    };
    if weapon.last_spawn_seq == Some(me.spawn_seq) {
        return;
    }
    weapon.last_spawn_seq = Some(me.spawn_seq);
    weapon.ammo = MAG_SIZE;
    weapon.reload_timer = 0.0;
    let (mut tf, mut p) = player.into_inner();
    tf.translation = spawn_point(session.my_id) + Vec3::Y * EYE_HEIGHT;
    p.yaw = 0.0;
    p.pitch = 0.0;
    p.vertical_velocity = 0.0;
    tf.rotation = Quat::IDENTITY;
}

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

fn mouse_look(
    motion: Res<AccumulatedMouseMotion>,
    window: Single<&Window, With<PrimaryWindow>>,
    player: Single<(&mut Transform, &mut LocalPlayer)>,
) {
    if !cursor_locked(&window) {
        return;
    }
    let (mut tf, mut p) = player.into_inner();
    p.yaw -= motion.delta.x * MOUSE_SENSITIVITY;
    p.pitch = (p.pitch - motion.delta.y * MOUSE_SENSITIVITY).clamp(-1.5, 1.5);
    tf.rotation = Quat::from_euler(EulerRot::YXZ, p.yaw, p.pitch, 0.0);
}

fn player_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    window: Single<&Window, With<PrimaryWindow>>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    player: Single<(&mut Transform, &mut LocalPlayer)>,
    colliders: Query<(&Transform, &Collider), Without<LocalPlayer>>,
) {
    // Solo mode is paused while the mouse is free.
    if session.role == crate::Role::Solo && !cursor_locked(&window) {
        return;
    }
    let dt = time.delta_secs();
    let (mut tf, mut p) = player.into_inner();
    let boxes = collect_boxes(colliders.iter());
    let active = can_act(&session, &roster, &state) && cursor_locked(&window);

    let forward = Vec3::new(-p.yaw.sin(), 0.0, -p.yaw.cos());
    let right = Vec3::new(-forward.z, 0.0, forward.x);
    let mut wish = Vec3::ZERO;
    if active {
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
    }
    let speed = if keys.pressed(KeyCode::ShiftLeft) {
        SPRINT_SPEED
    } else {
        WALK_SPEED
    };

    let mut feet = tf.translation - Vec3::Y * EYE_HEIGHT;
    feet += wish.normalize_or_zero() * speed * dt;

    if active && p.on_ground && keys.just_pressed(KeyCode::Space) {
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

/// Copies our position into the roster so the host (and other players) see it.
fn sync_to_roster(
    session: Res<Session>,
    mut roster: ResMut<Roster>,
    player: Single<(&Transform, &LocalPlayer)>,
) {
    let (tf, p) = *player;
    if let Some(me) = roster.0.get_mut(&session.my_id) {
        me.pos = (tf.translation - Vec3::Y * EYE_HEIGHT).to_array();
        me.yaw = p.yaw;
        me.pitch = p.pitch;
    }
}

fn shoot(
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    mut weapon: ResMut<Weapon>,
    mut tracers: ResMut<Tracers>,
    mut shots: ResMut<ShotQueue>,
    camera: Single<&Transform, With<LocalPlayer>>,
    enemies: Query<(Entity, &Transform), With<Enemy>>,
    colliders: Query<(&Transform, &Collider), Without<LocalPlayer>>,
) {
    weapon.fire_cooldown -= time.delta_secs();
    if !cursor_locked(&window)
        || !mouse.pressed(MouseButton::Left)
        || !can_act(&session, &roster, &state)
        || weapon.fire_cooldown > 0.0
        || weapon.reload_timer > 0.0
    {
        return;
    }
    if weapon.ammo == 0 {
        weapon.reload_timer = RELOAD_TIME;
        return;
    }
    weapon.ammo -= 1;
    weapon.fire_cooldown = FIRE_COOLDOWN;
    weapon.recoil = 1.0;
    weapon.flash_timer = 0.05;

    let origin = camera.translation;
    let dir = camera.forward().as_vec3();
    let boxes = collect_boxes(colliders.iter());
    let (dist, _) = trace_shot(
        origin,
        dir,
        GUN_RANGE,
        &boxes,
        enemies.iter().map(|(e, t)| (e, t.translation)),
    );
    // Start the tracer at the gun muzzle rather than the eye.
    let muzzle = origin + camera.rotation * Vec3::new(0.28, -0.2, -1.0);
    tracers.0.push((muzzle, origin + dir * dist, 0.06));

    // The host decides what was actually hit.
    shots.0.push((session.my_id, origin, dir));
}

fn reload(time: Res<Time>, keys: Res<ButtonInput<KeyCode>>, mut weapon: ResMut<Weapon>) {
    if keys.just_pressed(KeyCode::KeyR) && weapon.reload_timer <= 0.0 && weapon.ammo < MAG_SIZE {
        weapon.reload_timer = RELOAD_TIME;
    }
    if weapon.reload_timer > 0.0 {
        weapon.reload_timer -= time.delta_secs();
        if weapon.reload_timer <= 0.0 {
            weapon.reload_timer = 0.0;
            weapon.ammo = MAG_SIZE;
        }
    }
}

fn animate_gun(
    time: Res<Time>,
    session: Res<Session>,
    roster: Res<Roster>,
    mut weapon: ResMut<Weapon>,
    mut gun: Single<(&mut Transform, &mut Visibility), With<Gun>>,
    mut flash: Single<&mut PointLight, With<MuzzleFlash>>,
) {
    let dt = time.delta_secs();
    weapon.recoil = (weapon.recoil - dt * 8.0).max(0.0);
    weapon.flash_timer -= dt;

    // Dip the gun down while reloading.
    let reload_dip = if weapon.reload_timer > 0.0 {
        (weapon.reload_timer / RELOAD_TIME * std::f32::consts::PI).sin() * 0.25
    } else {
        0.0
    };
    let (tf, vis) = &mut *gun;
    tf.translation = Vec3::new(0.28, -0.24 - reload_dip, -0.55 + weapon.recoil * 0.08);
    tf.rotation = Quat::from_rotation_x(weapon.recoil * 0.15 - reload_dip * 2.0);
    let alive = roster.me(&session).is_none_or(|me| me.alive);
    **vis = if alive {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    flash.intensity = if weapon.flash_timer > 0.0 { 60_000.0 } else { 0.0 };
}

fn draw_tracers(time: Res<Time>, mut tracers: ResMut<Tracers>, mut gizmos: Gizmos) {
    let dt = time.delta_secs();
    for (a, b, life) in tracers.0.iter_mut() {
        gizmos.line(*a, *b, Color::srgb(1.0, 0.9, 0.5));
        *life -= dt;
    }
    tracers.0.retain(|(_, _, life)| *life > 0.0);
}
