//! What you see of yourself in first person: your gun (in your skin), gloved
//! hands and sleeved forearms, all animated: idle breathing, sway when you
//! look around, walk bob, sprint pose, recoil, muzzle flash, raising a new
//! gun, magazine reloads (or loading shells one at a time), shotgun pumps,
//! and the left hand holding and throwing grenades or casting abilities.
//!
//! The whole rig is drawn at 40% size, closer to the camera. It looks exactly
//! the same on screen but no longer pokes through walls you stand next to.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

use crate::abilities::CastState;
use crate::data::{gun_def, Character, GunClass};
use crate::gunmodels::{grenade_kit, spawn_gun, GunAssets, GunMag, GunPump, Support};
use crate::hands::{forearm_kit, hand_kit, HandPose};
use crate::kit::{c, glow_material, vertex_material, Kit};
use crate::player::LocalPlayer;
use crate::weapons::Loadout;
use crate::{AppState, Phase, Roster, Session};

/// Size the rig is drawn at (see the module notes).
const RIG_SCALE: f32 = 0.4;

pub struct ViewModelPlugin;

impl Plugin for ViewModelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewAnim>()
            .init_resource::<ViewMuzzle>()
            .add_systems(Startup, spawn_rig.after(crate::player::spawn_camera))
            .add_systems(Update, (rebuild, animate, muzzle_light).chain().in_set(Phase::Present))
            .add_systems(OnEnter(AppState::InGame), |mut a: ResMut<ViewAnim>| a.shown = None);
    }
}

/// Where the muzzle is relative to the camera (for tracers).
#[derive(Resource)]
pub struct ViewMuzzle(pub Vec3);

impl Default for ViewMuzzle {
    fn default() -> Self {
        Self(Vec3::new(0.1, -0.08, -0.35))
    }
}

#[derive(Resource, Default)]
struct ViewAnim {
    shown: Option<(u8, u8, Character)>,
    equip: f32,
    last_shots: u32,
    since_shot: f32,
    flash_roll: f32,
    sway: Vec2,
    bob: f32,
    sprint: f32,
    crouch: f32,
    busy: f32,
    /// Muzzle light: position, on, colour.
    light: (Vec3, bool, Color),
}

#[derive(Component)]
struct ViewRoot;

/// 0 = the gun in the right hand, 1 = the second gun of a dual pair.
#[derive(Component)]
struct GunPivot(u8);

#[derive(Component)]
struct Hand(f32);

#[derive(Component)]
struct HandMesh(HandPose);

#[derive(Component)]
struct Forearm(f32);

#[derive(Component)]
struct HeldGrenade;

#[derive(Component)]
struct HeldOrb;

#[derive(Component)]
struct Flash;

#[derive(Component)]
struct FlashLight;

#[derive(Resource)]
struct RigAssets {
    skin_mat: Handle<StandardMaterial>,
    glow_mat: Handle<StandardMaterial>,
    orb_mat: Handle<StandardMaterial>,
    flash: Handle<Mesh>,
    grenade: Handle<Mesh>,
    orb: Handle<Mesh>,
}

fn flash_kit() -> Kit {
    let mut k = Kit::new();
    let fwd = Quat::from_rotation_x(-PI / 2.0);
    k.cone(Vec3::new(0.0, 0.0, -0.05), 0.022, 0.1, fwd, c(1.0, 0.75, 0.3));
    k.cone(Vec3::new(0.0, 0.0, -0.03), 0.035, 0.05, fwd, c(1.0, 0.9, 0.55));
    for i in 0..4 {
        let r = Quat::from_rotation_z(i as f32 * PI / 4.0);
        k.cuboid_rot(Vec3::new(0.0, 0.0, -0.02), Vec3::new(0.09, 0.006, 0.02), r, c(1.0, 0.65, 0.2));
    }
    k.sphere(Vec3::ZERO, 0.02, c(1.0, 0.95, 0.75));
    k
}

fn spawn_rig(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera: Single<Entity, With<LocalPlayer>>,
) {
    let mut grenade = Kit::new();
    grenade_kit(&mut grenade, Vec3::ZERO);
    let assets = RigAssets {
        skin_mat: materials.add(vertex_material(0.75, 0.0)),
        glow_mat: materials.add(glow_material(1.0)),
        orb_mat: materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.85),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        flash: meshes.add(flash_kit().build_or_empty()),
        grenade: meshes.add(grenade.build_or_empty()),
        orb: meshes.add(Sphere::new(0.03).mesh().ico(2).unwrap()),
    };
    commands.entity(*camera).with_children(|cam| {
        cam.spawn((
            ViewRoot,
            Transform::from_scale(Vec3::splat(RIG_SCALE)),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((GunPivot(0), Transform::default(), Visibility::default()));
            root.spawn((GunPivot(1), Transform::default(), Visibility::default()));
            for side in [1.0, -1.0] {
                root.spawn((Hand(side), Transform::default(), Visibility::default()))
                    .with_children(|h| {
                        if side < 0.0 {
                            h.spawn((
                                HeldGrenade,
                                Mesh3d(assets.grenade.clone()),
                                MeshMaterial3d(assets.skin_mat.clone()),
                                Transform::from_xyz(0.0, -0.005, -0.012),
                                Visibility::Hidden,
                                NotShadowCaster,
                            ));
                            h.spawn((
                                HeldOrb,
                                Mesh3d(assets.orb.clone()),
                                MeshMaterial3d(assets.orb_mat.clone()),
                                Transform::from_xyz(0.0, 0.0, -0.01),
                                Visibility::Hidden,
                                NotShadowCaster,
                            ));
                        }
                    });
                root.spawn((Forearm(side), Transform::default(), Visibility::default()));
            }
            root.spawn((
                FlashLight,
                PointLight {
                    intensity: 0.0,
                    color: Color::srgb(1.0, 0.8, 0.4),
                    range: 10.0,
                    ..default()
                },
                Transform::default(),
            ));
        });
    });
    commands.insert_resource(assets);
}

/// Swaps in the right gun, skin, gloves and sleeves when any of them change.
fn rebuild(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    session: Res<Session>,
    roster: Res<Roster>,
    loadout: Res<Loadout>,
    guns: Res<GunAssets>,
    rig: Res<RigAssets>,
    mut anim: ResMut<ViewAnim>,
    pivots: Query<(Entity, &GunPivot)>,
    hands: Query<(Entity, &Hand)>,
    arms: Query<(Entity, &Forearm)>,
    hand_meshes: Query<Entity, With<HandMesh>>,
) {
    let Some(me) = roster.me(&session) else { return };
    let Some(gun) = loadout.current().map(|g| g.id) else { return };
    let key = (gun, me.skin_for(gun), me.character);
    if anim.shown == Some(key) {
        return;
    }
    let character_changed = anim.shown.is_none_or(|s| s.2 != me.character);
    let gun_changed = anim.shown.is_none_or(|s| s.0 != gun);
    anim.shown = Some(key);
    if gun_changed {
        anim.equip = 0.0;
    }
    let def = guns.gun(gun);
    let skin = guns.skin(me.skin_for(gun));
    for (e, pivot) in &pivots {
        commands.entity(e).despawn_related::<Children>();
        if pivot.0 == 1 && !def.rig.dual {
            continue;
        }
        commands.entity(e).with_children(|p| {
            spawn_gun(p, &guns, gun, skin.clone(), true);
            if pivot.0 == 0 {
                p.spawn((
                    Flash,
                    Mesh3d(rig.flash.clone()),
                    MeshMaterial3d(rig.glow_mat.clone()),
                    Transform::from_translation(def.rig.muzzle),
                    Visibility::Hidden,
                    NotShadowCaster,
                ));
            }
        });
    }
    if character_changed {
        for e in &hand_meshes {
            commands.entity(e).despawn();
        }
        let trim = me.character.trim_color();
        for (e, hand) in &hands {
            commands.entity(e).with_children(|h| {
                for pose in HandPose::ALL {
                    h.spawn((
                        HandMesh(pose),
                        Mesh3d(meshes.add(hand_kit(hand.0, pose, trim).build_or_empty())),
                        MeshMaterial3d(rig.skin_mat.clone()),
                        Transform::default(),
                        Visibility::Hidden,
                        NotShadowCaster,
                    ));
                }
            });
        }
        let arm = meshes.add(forearm_kit(me.character.suit_color(), trim).build_or_empty());
        for (e, _) in &arms {
            commands.entity(e).despawn_related::<Children>();
            commands.entity(e).with_children(|a| {
                a.spawn((
                    Mesh3d(arm.clone()),
                    MeshMaterial3d(rig.skin_mat.clone()),
                    Transform::default(),
                    NotShadowCaster,
                ));
            });
        }
    }
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 0 → 1 between `a` and `b`.
fn ramp(x: f32, a: f32, b: f32) -> f32 {
    ease((x - a) / (b - a))
}

fn approach(v: f32, target: f32, rate: f32) -> f32 {
    v + (target - v) * (1.0 - (-rate).exp())
}

fn mirror(q: Quat) -> Quat {
    Quat::from_xyzw(q.x, -q.y, -q.z, q.w)
}

fn blend(a: Transform, b: Transform, t: f32) -> Transform {
    Transform {
        translation: a.translation.lerp(b.translation, t),
        rotation: a.rotation.slerp(b.rotation, t),
        scale: Vec3::ONE,
    }
}

fn at(pos: Vec3, rot: Quat) -> Transform {
    Transform::from_translation(pos).with_rotation(rot)
}

/// Ability colour for the orb in the left hand.
fn ability_color(character: Character, slot: u8) -> Color {
    match (character, slot) {
        (Character::Striker, 2) => c(1.0, 0.5, 0.1),
        (Character::Warden, 0) => c(0.35, 1.0, 0.5),
        (Character::Warden, 1) => c(0.55, 0.85, 1.0),
        (Character::Warden, _) => c(1.0, 0.25, 0.2),
        _ => c(1.0, 1.0, 1.0),
    }
}

#[allow(clippy::too_many_arguments)]
fn animate(
    time: Res<Time>,
    motion: Res<AccumulatedMouseMotion>,
    state: Res<State<AppState>>,
    session: Res<Session>,
    roster: Res<Roster>,
    loadout: Res<Loadout>,
    cast: Res<CastState>,
    guns: Res<GunAssets>,
    rig_assets: Res<RigAssets>,
    mut anim: ResMut<ViewAnim>,
    mut muzzle: ResMut<ViewMuzzle>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player: Single<&LocalPlayer>,
    mut parts: ParamSet<(
        Query<(&GunPivot, &mut Transform, &mut Visibility), Without<ViewRoot>>,
        Query<(Entity, &Hand, &mut Transform)>,
        Query<(&Forearm, &mut Transform)>,
        Query<(&HandMesh, &ChildOf, &mut Visibility)>,
        Query<(&mut Transform, &mut Visibility), With<Flash>>,
        Query<(&mut Transform, Has<GunPump>), Or<(With<GunMag>, With<GunPump>)>>,
        Query<(&mut Visibility, &mut Transform, Has<HeldGrenade>), Or<(With<HeldGrenade>, With<HeldOrb>)>>,
        Query<&mut Visibility, With<ViewRoot>>,
    )>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let me = roster.me(&session);
    let gun = loadout.current().map(|g| g.id);
    let show = *state.get() == AppState::InGame && me.is_some_and(|m| m.alive) && gun.is_some();
    for mut vis in &mut parts.p7() {
        *vis = if show { Visibility::Inherited } else { Visibility::Hidden };
    }
    let (Some(me), Some(gun), true) = (me, gun, show) else { return };
    let rig = guns.gun(gun).rig;
    let def = gun_def(gun);
    let character = me.character;

    // --- Timers and blends -------------------------------------------------
    anim.equip = (anim.equip + dt / 0.35).min(1.0);
    if loadout.shots != anim.last_shots {
        anim.last_shots = loadout.shots;
        anim.since_shot = 0.0;
        anim.flash_roll = rand::thread_rng().gen_range(0.0..PI);
    } else {
        anim.since_shot += dt;
    }
    let firing = anim.since_shot < 0.3;
    let sprint_target = if player.sprinting && player.on_ground && !firing { 1.0 } else { 0.0 };
    anim.sprint = approach(anim.sprint, sprint_target, dt * 9.0);
    let crouch_target = if player.crouching || player.sliding > 0.0 { 1.0 } else { 0.0 };
    anim.crouch = approach(anim.crouch, crouch_target, dt * 8.0);
    let look = -motion.delta * 0.0012;
    let look = look.clamp(Vec2::splat(-0.08), Vec2::splat(0.08));
    anim.sway = anim.sway.lerp(look, 1.0 - (-dt * 9.0).exp());
    let speed = player.horizontal_speed();
    let amp = if player.on_ground { (speed / 6.0).min(1.4) } else { 0.0 };
    anim.bob += dt * (4.0 + speed * 1.2).min(16.0) * amp.min(1.0);

    // Is the left hand busy with an ability?
    let cast_anim = cast.cast.filter(|(_, s)| *s < 0.55);
    let cast_slot = cast.aiming.or(cast_anim.map(|(s, _)| s));
    let uses_hand = |slot: u8| !(character == Character::Striker && slot == 0);
    let busy = cast_slot.is_some_and(uses_hand);
    anim.busy = approach(anim.busy, if busy { 1.0 } else { 0.0 }, dt * 12.0);

    // --- Gun pose ----------------------------------------------------------
    let short = rig.length < 0.4;
    let mut pos = if short {
        Vec3::new(0.13, -0.155, -0.33)
    } else {
        Vec3::new(0.15, -0.172, -0.24)
    };
    let mut rot = Quat::from_rotation_y(if short { 0.12 } else { 0.045 });
    if rig.dual {
        pos.x = 0.15;
    }
    // Breathing and walk bob.
    pos.y += (t * 1.7).sin() * 0.002;
    pos.x += anim.bob.sin() * 0.007 * amp;
    pos.y -= anim.bob.cos().abs() * 0.009 * amp;
    rot *= Quat::from_rotation_z(anim.bob.sin() * 0.015 * amp);
    // Sway lags behind the mouse.
    pos.x += anim.sway.x * 0.15;
    pos.y -= anim.sway.y * 0.15;
    rot *= Quat::from_euler(EulerRot::YXZ, anim.sway.x, anim.sway.y, anim.sway.x * 0.8);
    // In the air the gun floats up a little.
    if !player.on_ground {
        pos.y += (player.vel.y * -0.002).clamp(-0.01, 0.015);
    }
    // Sprint: muzzle swings down and in.
    let sp = ease(anim.sprint);
    pos += Vec3::new(-0.03, -0.045, 0.03) * sp;
    rot *= Quat::from_euler(EulerRot::YXZ, 0.65 * sp, -0.3 * sp, 0.35 * sp);
    // Crouch / slide cant.
    rot *= Quat::from_rotation_z(0.12 * anim.crouch);
    // Recoil.
    let kick = match def.class {
        GunClass::Sniper => 1.7,
        GunClass::Shotgun => 1.5,
        GunClass::Pistol => 1.1,
        GunClass::Wonder => 1.2,
        GunClass::Lmg => 0.5,
        _ => 0.7,
    };
    let r = loadout.recoil;
    pos.z += r * 0.045 * kick;
    pos.y += r * 0.006 * kick;
    rot *= Quat::from_rotation_x(r * 0.09 * kick);
    // Raising a new gun.
    let e = ease(anim.equip);
    pos.y -= (1.0 - e) * 0.3;
    rot *= Quat::from_rotation_x(-(1.0 - e) * 1.0);
    // Left hand off the gun: lower it a touch.
    pos.y -= anim.busy * 0.015;
    // Reload.
    let reload = (loadout.reload > 0.0 && loadout.reload_total > 0.0)
        .then(|| 1.0 - loadout.reload / loadout.reload_total);
    let rl = reload.map(|p| ramp(p, 0.0, 0.1) * (1.0 - ramp(p, 0.88, 1.0))).unwrap_or(0.0);
    if rig.dual {
        pos.y -= 0.18 * rl;
        rot *= Quat::from_rotation_x(-0.9 * rl);
    } else if gun == 12 {
        // Break the double barrel open: muzzle drops.
        pos += Vec3::new(-0.03, 0.02, 0.0) * rl;
        rot *= Quat::from_euler(EulerRot::YXZ, -0.2 * rl, -0.45 * rl, -0.3 * rl);
    } else {
        pos += Vec3::new(-0.035, -0.02, 0.02) * rl;
        rot *= Quat::from_euler(EulerRot::YXZ, -0.2 * rl, 0.22 * rl, -0.55 * rl);
    }
    let gun_tf = at(pos, rot);
    let gun2_tf = at(Vec3::new(-pos.x, pos.y, pos.z), mirror(rot));

    // --- Magazine and pump -------------------------------------------------
    let mut mag_offset = 0.0f32;
    let mut mag_hidden = false;
    if let Some(p) = reload {
        if rig.single_load && gun != 12 {
            // Revolver cylinder swings out and stays out while loading.
            mag_offset = 0.04 * ramp(p, 0.08, 0.18) * (1.0 - ramp(p, 0.84, 0.92));
        } else {
            if p > 0.12 && p < 0.3 {
                mag_offset = 0.3 * ramp(p, 0.12, 0.3);
            } else if (0.3..0.55).contains(&p) {
                mag_hidden = true;
            } else if (0.55..0.82).contains(&p) {
                mag_offset = 0.25 * (1.0 - ramp(p, 0.55, 0.78));
            }
        }
    }
    let pump = if rig.pump_travel > 0.0 {
        let s = anim.since_shot;
        let shot = if (0.12..0.5).contains(&s) { ((s - 0.12) / 0.38 * PI).sin() } else { 0.0 };
        let after_reload = reload.map(|p| if p > 0.86 { ((p - 0.86) / 0.14 * PI).sin() } else { 0.0 });
        rig.pump_travel * shot.max(after_reload.unwrap_or(0.0))
    } else {
        0.0
    };

    // --- Left hand ---------------------------------------------------------
    let grip_rot = Quat::from_rotation_x(-rig.grip_tilt);
    let grip_hand = at(grip_rot * Vec3::new(0.0, -0.045, 0.004), grip_rot);
    let right_tf = gun_tf * grip_hand;
    let (support_pose, support_local) = match rig.support_style {
        Support::Under => (HandPose::Under, at(rig.support + Vec3::new(0.0, 0.032, pump), Quat::IDENTITY)),
        Support::Vertical => (
            HandPose::Grip { trigger: false },
            at(rig.support, Quat::from_rotation_x(-0.1)),
        ),
        Support::Cup => (
            HandPose::Grip { trigger: false },
            at(rig.support + Vec3::new(0.0, 0.0, -0.005), grip_rot * Quat::from_rotation_y(-0.2)),
        ),
    };
    let mut left_pose = support_pose;
    let mut left_tf = if rig.dual { gun2_tf * grip_hand } else { gun_tf * support_local };
    if rig.dual {
        left_pose = HandPose::Grip { trigger: true };
    }

    // Reloads move the left hand to the magazine and away for a new one.
    if let (Some(p), false) = (reload, rig.dual) {
        let mag_tf = |off: f32| gun_tf * at(rig.mag_pos + rig.mag_out * (off + 0.07), Quat::IDENTITY);
        let away = at(Vec3::new(-0.12, -0.42, -0.15), Quat::from_rotation_x(0.6));
        let back = left_tf;
        if rig.single_load {
            // Feed rounds in one at a time.
            let port = gun_tf
                * at(
                    rig.mag_pos + if gun == 18 { Vec3::new(-0.06, -0.02, 0.0) } else { Vec3::new(0.0, -0.05, 0.0) },
                    Quat::IDENTITY,
                );
            let below = at(port.translation + Vec3::new(-0.04, -0.14, 0.06), port.rotation);
            let cycles = if gun == 12 { 1.0 } else { 3.0 };
            if (0.12..0.86).contains(&p) {
                let u = (p - 0.12) / 0.74 * cycles;
                let k = (u.fract() * PI).sin();
                left_tf = blend(below, port, k);
                left_pose = HandPose::Hold;
            } else {
                let k = if p < 0.12 { ramp(p, 0.0, 0.12) } else { 1.0 - ramp(p, 0.86, 1.0) };
                left_tf = blend(back, below, k);
                if k > 0.5 {
                    left_pose = HandPose::Hold;
                }
            }
        } else {
            left_pose = HandPose::Hold;
            left_tf = if p < 0.12 {
                blend(back, mag_tf(0.0), ramp(p, 0.0, 0.12))
            } else if p < 0.3 {
                blend(mag_tf(mag_offset), away, ramp(p, 0.18, 0.3))
            } else if p < 0.55 {
                away
            } else if p < 0.82 {
                blend(away, mag_tf(mag_offset), ramp(p, 0.55, 0.62))
            } else {
                left_pose = support_pose;
                blend(mag_tf(0.0), back, ramp(p, 0.82, 0.95))
            };
            if p >= 0.82 && p < 0.88 {
                left_pose = HandPose::Hold;
            }
        }
    }

    // Abilities take over the left hand.
    let mut grenade_visible = false;
    let mut orb: Option<(Color, f32)> = None;
    if busy {
        let slot = cast_slot.unwrap_or(0);
        let ready = at(Vec3::new(-0.13, -0.13, -0.32), Quat::from_rotation_x(0.15));
        let grenade = character == Character::Striker && slot == 1;
        let color = ability_color(character, slot);
        let mut hand = ready;
        let mut pose = HandPose::Hold;
        if let Some((_, s)) = cast_anim.filter(|_| cast.aiming.is_none()) {
            if grenade {
                // Wind up, throw, then bring the hand back down.
                let wind = at(Vec3::new(-0.1, -0.04, -0.12), Quat::from_rotation_x(0.9));
                let out = at(Vec3::new(-0.03, -0.02, -0.62), Quat::from_rotation_x(-0.5));
                hand = if s < 0.1 {
                    blend(left_tf, ready, ramp(s, 0.0, 0.1))
                } else if s < 0.2 {
                    blend(ready, wind, ramp(s, 0.1, 0.2))
                } else if s < 0.3 {
                    blend(wind, out, ramp(s, 0.2, 0.3))
                } else {
                    blend(out, left_tf, ramp(s, 0.3, 0.55))
                };
                grenade_visible = s < 0.26;
                pose = if s < 0.26 { HandPose::Hold } else { HandPose::Open };
            } else {
                // Push the palm out (heal, nova, overdrive) or raise it to
                // the sky (orbital strike).
                let push = if character == Character::Warden && slot == 2 {
                    at(Vec3::new(-0.09, 0.02, -0.42), Quat::from_rotation_x(0.3))
                } else {
                    at(Vec3::new(-0.06, -0.08, -0.52), Quat::IDENTITY)
                };
                hand = if s < 0.12 {
                    blend(ready, push, ramp(s, 0.0, 0.12))
                } else {
                    blend(push, left_tf, ramp(s, 0.3, 0.55))
                };
                pose = if s < 0.4 { HandPose::Open } else { support_pose };
                if s < 0.2 {
                    orb = Some((color, 1.0 + s * 8.0));
                }
            }
        } else {
            // Holding / aiming: grenade cooking or an orb gathering energy.
            let shake = if grenade { (cast.held * 2.5).min(1.0) } else { 0.3 };
            hand.translation += Vec3::new((t * 40.0).sin(), (t * 33.0).cos(), 0.0) * 0.002 * shake;
            if grenade {
                grenade_visible = true;
            } else {
                orb = Some((color, 0.8 + 0.15 * (t * 8.0).sin()));
            }
        }
        left_tf = blend(left_tf, hand, anim.busy);
        if anim.busy > 0.4 {
            left_pose = pose;
        }
    }

    // --- Apply -------------------------------------------------------------
    for (pivot, mut tf, mut vis) in &mut parts.p0() {
        if pivot.0 == 0 {
            *tf = gun_tf;
        } else {
            *tf = gun2_tf;
            *vis = if rig.dual && anim.busy < 0.5 { Visibility::Inherited } else { Visibility::Hidden };
        }
    }
    // Place the hands and remember which pose each one shows.
    let mut hand_poses: Vec<(Entity, HandPose)> = Vec::with_capacity(2);
    let mut wrists = [Vec3::ZERO; 2];
    for (entity, hand, mut tf) in &mut parts.p1() {
        let (htf, pose, i) = if hand.0 > 0.0 {
            (right_tf, HandPose::Grip { trigger: true }, 0)
        } else {
            (left_tf, left_pose, 1)
        };
        *tf = htf;
        wrists[i] = htf.transform_point(pose.wrist(hand.0));
        hand_poses.push((entity, pose));
    }
    for (arm, mut tf) in &mut parts.p2() {
        let i = if arm.0 > 0.0 { 0 } else { 1 };
        let wrist = wrists[i];
        let shoulder = if arm.0 > 0.0 { Vec3::new(0.28, -0.45, 0.25) } else { Vec3::new(-0.32, -0.5, 0.08) };
        let d = shoulder - wrist;
        let len = d.length().max(0.05);
        *tf = Transform::from_translation(wrist)
            .with_rotation(Quat::from_rotation_arc(Vec3::Y, d / len))
            .with_scale(Vec3::new(1.0, len, 1.0));
    }
    for (mesh, parent, mut vis) in &mut parts.p3() {
        let shown = hand_poses
            .iter()
            .any(|(e, pose)| *e == parent.parent() && *pose == mesh.0);
        *vis = if shown { Visibility::Inherited } else { Visibility::Hidden };
    }

    // Muzzle flash.
    let flash_on = anim.since_shot < 0.045 && loadout.shots > 0;
    for (mut tf, mut vis) in &mut parts.p4() {
        *vis = if flash_on { Visibility::Inherited } else { Visibility::Hidden };
        let s = if def.class == GunClass::Shotgun || def.class == GunClass::Lmg { 1.4 } else { 1.0 };
        *tf = Transform::from_translation(rig.muzzle)
            .with_rotation(Quat::from_rotation_z(anim.flash_roll))
            .with_scale(Vec3::splat(s));
    }
    let muzzle_root = gun_tf.transform_point(rig.muzzle);
    muzzle.0 = muzzle_root * RIG_SCALE;
    anim.light = (
        muzzle_root + Vec3::new(0.0, 0.05, 0.0),
        flash_on,
        if def.rare { rig.glow_color } else { Color::srgb(1.0, 0.8, 0.4) },
    );
    for (mut tf, is_pump) in &mut parts.p5() {
        if is_pump {
            tf.translation = Vec3::new(0.0, 0.0, pump);
        } else {
            tf.translation = rig.mag_pos + rig.mag_out * mag_offset;
            tf.scale = if mag_hidden { Vec3::ZERO } else { Vec3::ONE };
        }
    }
    for (mut vis, mut tf, is_grenade) in &mut parts.p6() {
        if is_grenade {
            *vis = if grenade_visible { Visibility::Inherited } else { Visibility::Hidden };
            continue;
        }
        match orb {
            Some((color, size)) => {
                *vis = Visibility::Inherited;
                tf.scale = Vec3::splat(size);
                if let Some(m) = materials.get_mut(&rig_assets.orb_mat) {
                    m.base_color = color.with_alpha(0.85);
                }
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

fn muzzle_light(anim: Res<ViewAnim>, mut lights: Query<(&mut Transform, &mut PointLight), With<FlashLight>>) {
    for (mut tf, mut light) in &mut lights {
        tf.translation = anim.light.0;
        light.intensity = if anim.light.1 { 60_000.0 } else { 0.0 };
        light.color = anim.light.2;
    }
}
