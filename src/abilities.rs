//! Local input for abilities and interacting. The host checks cooldowns and
//! does the actual work (see sim.rs); Dash moves you right away here.
//!
//! Each ability has a cast mode (Settings):
//! - Instant: casts the moment you press the key.
//! - Quick: hold the key to aim (with a preview), release to cast.
//! - Confirm: press to aim, left-click to cast, right-click or press the key
//!   again to cancel.
//! Grenades are held in your hand while aiming and cook: the longer you hold,
//! the sooner they go off after landing. Hold too long and you throw it.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::f32::consts::FRAC_PI_2;

use crate::config::{Action, CastMode, InputExt, Settings};
use crate::data::Character;
use crate::game::Paused;
use crate::physics::{collect_boxes, line_of_sight, ray_world};
use crate::player::{can_act, LocalPlayer};
use crate::{ActionQueue, AppState, Collider, MatchState, Phase, PlayerAction, Roster, Session};

/// Grenade fuse in seconds and how long it can be cooked in the hand.
pub const GRENADE_FUSE: f32 = 1.8;
pub const MAX_COOK: f32 = 1.5;
pub const GRENADE_SPEED: f32 = 16.0;
pub const GRENADE_LIFT: f32 = 3.0;
pub const GRENADE_GRAVITY: f32 = 18.0;

pub struct AbilityPlugin;

impl Plugin for AbilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionCounter>()
            .init_resource::<CastState>()
            .add_systems(
                Update,
                use_abilities
                    .after(crate::player::movement)
                    .before(crate::weapons::fire)
                    .in_set(Phase::Local),
            )
            .add_systems(
                Update,
                draw_previews
                    .in_set(Phase::Present)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnEnter(AppState::InGame), |mut c: ResMut<CastState>| {
                *c = CastState::default()
            });
    }
}

/// Numbers our actions so the host handles each exactly once.
#[derive(Resource, Default)]
pub struct ActionCounter(pub u32);

/// What the local player is doing with their abilities (for the hands and
/// the aim previews).
#[derive(Resource, Default)]
pub struct CastState {
    /// Ability slot being aimed or held.
    pub aiming: Option<u8>,
    /// Seconds it has been held (grenades cook).
    pub held: f32,
    /// The last cast: (slot, seconds since).
    pub cast: Option<(u8, f32)>,
    /// A Confirm-mode ability is waiting for a click, so the gun holds fire.
    pub blocks_fire: bool,
    /// The click that confirmed a cast is still held down.
    swallow_click: bool,
}

pub fn queue_action(
    session: &Session,
    counter: &mut ActionCounter,
    queue: &mut ActionQueue,
    action: PlayerAction,
) {
    counter.0 += 1;
    queue.0.push((session.my_id, counter.0, action));
}

fn is_grenade(character: Character, slot: u8) -> bool {
    character == Character::Striker && slot == 1
}

const SLOTS: [(u8, Action); 3] = [(0, Action::Ability1), (1, Action::Ability2), (2, Action::Ultimate)];

fn dash_direction(keys: &ButtonInput<KeyCode>, settings: &Settings, yaw: f32) -> Vec3 {
    let fwd = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(-fwd.z, 0.0, fwd.x);
    let mut dir = Vec3::ZERO;
    if keys.held(settings, Action::Forward) {
        dir += fwd;
    }
    if keys.held(settings, Action::Back) {
        dir -= fwd;
    }
    if keys.held(settings, Action::Right) {
        dir += right;
    }
    if keys.held(settings, Action::Left) {
        dir -= right;
    }
    if dir == Vec3::ZERO {
        fwd
    } else {
        dir.normalize()
    }
}

#[allow(clippy::too_many_arguments)]
fn use_abilities(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<Settings>,
    window: Single<&Window, With<PrimaryWindow>>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    paused: Res<Paused>,
    mut counter: ResMut<ActionCounter>,
    mut queue: ResMut<ActionQueue>,
    mut cast: ResMut<CastState>,
    mut fx: ResMut<crate::fx::FxQueue>,
    player: Single<(&Transform, &mut LocalPlayer)>,
    mut local_cd: Local<[f32; 3]>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for cd in local_cd.iter_mut() {
        *cd -= dt;
    }
    if let Some((_, t)) = cast.cast.as_mut() {
        *t += dt;
    }
    let Some(me) = roster.me(&session) else {
        return;
    };
    if !can_act(&session, &roster, &state, &paused, &window) {
        // Menus and going down cancel aiming (a cooked grenade is dropped
        // safely back in your pocket).
        cast.aiming = None;
        cast.blocks_fire = false;
        return;
    }
    let (cam, mut p) = player.into_inner();

    if keys.tapped(&settings, Action::Interact) {
        queue_action(&session, &mut counter, &mut queue, PlayerAction::Interact);
    }

    let ready = |slot: u8| {
        if slot == 2 {
            me.ult_charge >= 100.0
        } else {
            me.cooldowns[slot as usize] <= 0.0
        }
    };

    // Decide whether to cast this frame.
    let mut fire_slot = None;
    if let Some(slot) = cast.aiming {
        cast.held += dt;
        let key = SLOTS[slot as usize].1;
        let mode = settings.cast_modes[slot as usize];
        let cancel = match mode {
            CastMode::Confirm => mouse.just_pressed(MouseButton::Right) || keys.tapped(&settings, key),
            _ => false,
        };
        let release = match mode {
            CastMode::Quick => !keys.held(&settings, key),
            CastMode::Confirm => mouse.just_pressed(MouseButton::Left),
            CastMode::Instant => true,
        };
        if cancel || !ready(slot) {
            cast.aiming = None;
        } else if release || (is_grenade(me.character, slot) && cast.held >= MAX_COOK) {
            fire_slot = Some(slot);
        }
    } else {
        for (slot, action) in SLOTS {
            if !keys.tapped(&settings, action) || !ready(slot) || local_cd[slot as usize] > 0.0 {
                continue;
            }
            if settings.cast_modes[slot as usize] == CastMode::Instant {
                fire_slot = Some(slot);
            } else {
                cast.aiming = Some(slot);
                cast.held = 0.0;
            }
            break;
        }
    }
    if !mouse.pressed(MouseButton::Left) {
        cast.swallow_click = false;
    }
    cast.blocks_fire = cast.swallow_click
        || cast
            .aiming
            .is_some_and(|s| settings.cast_modes[s as usize] == CastMode::Confirm);

    let Some(slot) = fire_slot else { return };
    // The local timer stops double-firing while the host catches up.
    local_cd[slot as usize] = 0.5;
    let cook = if is_grenade(me.character, slot) && cast.aiming == Some(slot) {
        cast.held.min(MAX_COOK)
    } else {
        0.0
    };
    cast.aiming = None;
    cast.cast = Some((slot, 0.0));
    // A Confirm click shouldn't also fire the gun.
    if settings.cast_modes[slot as usize] == CastMode::Confirm {
        cast.swallow_click = true;
        cast.blocks_fire = true;
    }
    if me.character == Character::Striker && slot == 0 {
        p.dash_dir = dash_direction(&keys, &settings, p.yaw);
        p.dash_time = 0.18 + 0.03 * me.tiers[0] as f32;
        let to = p.feet + p.dash_dir * 22.0 * p.dash_time;
        fx.0.push(crate::fx::Fx::Dash {
            player: session.my_id,
            a: p.feet.to_array(),
            b: to.to_array(),
        });
    }
    queue_action(
        &session,
        &mut counter,
        &mut queue,
        PlayerAction::Ability {
            slot,
            origin: cam.translation.to_array(),
            dir: cam.forward().as_vec3().to_array(),
            cook,
        },
    );
}

fn flat_circle(gizmos: &mut Gizmos, center: Vec3, radius: f32, color: Color) {
    let iso = Isometry3d::new(center + Vec3::Y * 0.06, Quat::from_rotation_x(FRAC_PI_2));
    gizmos.circle(iso, radius, color).resolution(48);
    let iso = Isometry3d::new(center + Vec3::Y * 0.08, Quat::from_rotation_x(FRAC_PI_2));
    gizmos.circle(iso, radius * 0.985, color).resolution(48);
}

/// Shows where an ability will land while you aim it.
fn draw_previews(
    time: Res<Time>,
    cast: Res<CastState>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    session: Res<Session>,
    roster: Res<Roster>,
    player: Single<(&Transform, &LocalPlayer)>,
    colliders: Query<(&Transform, &Collider)>,
    mut gizmos: Gizmos,
) {
    let Some(slot) = cast.aiming else { return };
    let Some(me) = roster.me(&session) else { return };
    let (cam, p) = player.into_inner();
    let t = time.elapsed_secs();
    let pulse = 0.75 + 0.25 * (t * 6.0).sin();
    let tier = me.tiers[slot as usize] as f32;
    let feet = p.feet;
    let forward = cam.forward().as_vec3();
    match (me.character, slot) {
        (Character::Striker, 0) => {
            // Dash: arrow along the ground.
            let dir = dash_direction(&keys, &settings, p.yaw);
            let len = 22.0 * (0.18 + 0.03 * tier);
            let a = feet + Vec3::Y * 0.1;
            let b = a + dir * len;
            let color = Color::srgba(0.4, 0.8, 1.0, pulse);
            gizmos.line(a, b, color);
            let side = Vec3::new(-dir.z, 0.0, dir.x) * 0.5;
            gizmos.line(b, b - dir * 0.8 + side, color);
            gizmos.line(b, b - dir * 0.8 - side, color);
        }
        (Character::Striker, 1) => {
            // Grenade: the arc it will fly and where it lands.
            let boxes = collect_boxes(colliders.iter());
            let mut pos = cam.translation + forward * 0.6;
            let mut vel = forward * GRENADE_SPEED + Vec3::Y * GRENADE_LIFT;
            let mut pts = vec![pos];
            let step = 0.025;
            for _ in 0..160 {
                vel.y -= GRENADE_GRAVITY * step;
                let next = pos + vel * step;
                if next.y < 0.1 || !line_of_sight(pos, next, &boxes) {
                    break;
                }
                pos = next;
                pts.push(pos);
            }
            let cooked = (cast.held / MAX_COOK).min(1.0);
            let color = Color::srgb(1.0, 0.6 - 0.45 * cooked, 0.15);
            gizmos.linestrip(pts, color);
            flat_circle(&mut gizmos, pos.with_y(0.0), 4.0 + 0.5 * tier, color.with_alpha(pulse));
        }
        (Character::Striker, _) => {
            flat_circle(&mut gizmos, feet, 1.2, Color::srgba(1.0, 0.5, 0.1, pulse));
        }
        (Character::Warden, 0) => {
            flat_circle(&mut gizmos, feet, 8.0, Color::srgba(0.3, 1.0, 0.5, pulse));
        }
        (Character::Warden, 1) => {
            flat_circle(&mut gizmos, feet, 7.0, Color::srgba(0.5, 0.85, 1.0, pulse));
        }
        (Character::Warden, _) => {
            let boxes = collect_boxes(colliders.iter());
            let dist = ray_world(cam.translation, forward, 80.0, &boxes);
            let target = (cam.translation + forward * dist).with_y(0.0);
            let r = 9.0 + tier;
            let color = Color::srgba(1.0, 0.25, 0.2, pulse);
            flat_circle(&mut gizmos, target, r, color);
            flat_circle(&mut gizmos, target, r * 0.5, color);
            gizmos.line(target, target + Vec3::Y * 25.0, color);
            gizmos.line(target - Vec3::X * r, target + Vec3::X * r, color);
            gizmos.line(target - Vec3::Z * r, target + Vec3::Z * r, color);
        }
    }
}
