//! Local input for abilities and interacting. The host checks cooldowns and
//! does the actual work (see sim.rs); Dash moves you right away here.
//!
//! Each ability has a cast mode (Settings):
//! - Instant: casts the moment you press the key.
//! - Quick: hold the key to aim (with a preview), release to cast.
//! - Confirm: press to aim, left-click to cast, right-click or press the key
//!   again to cancel.
//! Grenades sit in your hand, pin pulled, while you aim; the fuse only starts
//! when it leaves your hand, so you can hold one as long as you like.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::config::{Action, CastMode, InputExt, Settings};
use crate::data::Ability;
use crate::game::Paused;
use crate::player::{can_act, LocalPlayer};
use crate::{ActionQueue, AppState, MatchState, Phase, PlayerAction, Roster, Session};

mod preview;

/// Grenade fuse in seconds, from the throw.
pub const GRENADE_FUSE: f32 = 1.8;
pub const GRENADE_SPEED: f32 = 16.0;
pub const GRENADE_LIFT: f32 = 3.0;
pub const GRENADE_GRAVITY: f32 = 18.0;
/// How far the Ranger's grapple reaches.
pub const GRAPPLE_RANGE: f32 = 30.0;
/// Speed of every dash (matches player.rs).
pub const DASH_SPEED: f32 = 22.0;

/// Seconds of holding to fully charge a charged ability (none since v12).
pub const FULL_CHARGE: f32 = 1.0;

/// How long a movement ability carries you at `DASH_SPEED`.
pub fn move_time(ability: Ability, tier: f32) -> f32 {
    let base = match ability {
        Ability::ShieldCharge => 0.32,
        Ability::WraithStep => 0.3,
        Ability::BlastJump => 0.36,
        _ => 0.2,
    };
    base + 0.03 * tier
}

pub struct AbilityPlugin;

impl Plugin for AbilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionCounter>()
            .init_resource::<CastState>()
            .add_systems(
                Update,
                (use_abilities, use_weapon_abilities)
                    .after(crate::player::movement)
                    .before(crate::weapons::fire)
                    .in_set(Phase::Local),
            )
            .add_systems(
                Update,
                preview::draw_previews
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
    /// Seconds it has been held.
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

const SLOTS: [(u8, Action); 3] = [
    (0, Action::Ability1),
    (1, Action::Ability2),
    (2, Action::Ultimate),
];

pub(crate) fn dash_direction(keys: &ButtonInput<KeyCode>, settings: &Settings, yaw: f32) -> Vec3 {
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

/// Keys 3 and 4 (by default): the class's two weapon abilities. The host
/// checks the cooldown.
#[allow(clippy::too_many_arguments)]
fn use_weapon_abilities(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    window: Single<&Window, With<PrimaryWindow>>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    paused: Res<Paused>,
    menu: Res<crate::emotes::EmoteMenu>,
    mut counter: ResMut<ActionCounter>,
    mut queue: ResMut<ActionQueue>,
) {
    if menu.open || !can_act(&session, &roster, &state, &paused, &window) {
        return;
    }
    let Some(me) = roster.me(&session) else {
        return;
    };
    for (i, action) in [Action::WeaponAbility1, Action::WeaponAbility2]
        .into_iter()
        .enumerate()
    {
        if keys.tapped(&settings, action) && me.weapon_cd[i] <= 0.0 {
            queue_action(&session, &mut counter, &mut queue, PlayerAction::WeaponAbility(i as u8));
        }
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
    colliders: Query<(&Transform, &crate::Collider), Without<LocalPlayer>>,
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
        // Menus and going down cancel aiming (a held grenade goes
        // back in your pocket).
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
            me.charges[slot as usize] > 0
        }
    };

    // Decide whether to cast this frame.
    let mut fire_slot = None;
    if let Some(slot) = cast.aiming {
        cast.held += dt;
        let key = SLOTS[slot as usize].1;
        // Charged abilities always work by holding and letting go.
        let mode = if me.kit[slot as usize].charges() {
            CastMode::Quick
        } else {
            settings.cast_modes[slot as usize]
        };
        let cancel = match mode {
            CastMode::Confirm => {
                mouse.just_pressed(MouseButton::Right) || keys.tapped(&settings, key)
            }
            _ => false,
        };
        let release = match mode {
            CastMode::Quick => !keys.held(&settings, key),
            CastMode::Confirm => mouse.just_pressed(MouseButton::Left),
            CastMode::Instant => true,
        };
        if cancel || !ready(slot) {
            cast.aiming = None;
        } else if release {
            fire_slot = Some(slot);
        }
    } else {
        for (slot, action) in SLOTS {
            if !keys.tapped(&settings, action) || !ready(slot) || local_cd[slot as usize] > 0.0 {
                continue;
            }
            let charges = me.kit[slot as usize].charges();
            if settings.cast_modes[slot as usize] == CastMode::Instant && !charges {
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
        || cast.aiming.is_some_and(|s| {
            settings.cast_modes[s as usize] == CastMode::Confirm || me.kit[s as usize].charges()
        });

    let Some(slot) = fire_slot else { return };
    let ability = me.kit[slot as usize];
    let charge = if ability.charges() {
        (cast.held / FULL_CHARGE).min(1.0)
    } else {
        0.0
    };
    // The local timer stops double-firing while the host catches up.
    local_cd[slot as usize] = 0.5;
    cast.aiming = None;
    cast.cast = Some((slot, 0.0));
    // A Confirm click shouldn't also fire the gun.
    if settings.cast_modes[slot as usize] == CastMode::Confirm && !ability.charges() {
        cast.swallow_click = true;
        cast.blocks_fire = true;
    }
    let tier = me.tiers[slot as usize] as f32;
    let ahead = cam
        .forward()
        .as_vec3()
        .with_y(0.0)
        .normalize_or(Vec3::NEG_Z);
    if ability.is_dash() {
        p.dash_dir = ahead;
        p.dash_time = move_time(ability, tier);
        match ability {
            Ability::BlastJump => {
                // Blown up into the air and forward.
                p.vel.y = 11.0;
                p.on_ground = false;
            }
            Ability::Grapple => {
                // Zip toward the spot you aimed at, lifted if it's higher.
                let look = cam.forward().as_vec3();
                let boxes = crate::physics::collect_boxes(colliders.iter());
                let dist = crate::physics::ray_world(cam.translation, look, GRAPPLE_RANGE, &boxes);
                let target = cam.translation + look * (dist - 0.6).max(1.0);
                let flat = (target - p.feet).with_y(0.0);
                p.dash_dir = flat.normalize_or(ahead);
                p.dash_time = (flat.length() / DASH_SPEED).clamp(0.12, 1.2);
                let rise = (target.y - p.feet.y).max(0.0);
                p.vel.y = 5.0 + rise * 2.2;
                p.on_ground = false;
            }
            _ => {}
        }
        let to = p.feet + p.dash_dir * DASH_SPEED * p.dash_time;
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
            charge,
        },
    );
}
