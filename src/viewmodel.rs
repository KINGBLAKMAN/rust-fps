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
use crate::data::{gun_def, Ability, CastStyle, Character, GunClass};
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
            .add_systems(
                Update,
                (rebuild, animate, muzzle_light)
                    .chain()
                    .in_set(Phase::Present),
            )
            .add_systems(OnEnter(AppState::InGame), |mut a: ResMut<ViewAnim>| {
                a.shown = None
            });
    }
}

/// Where the muzzle is relative to the camera (for tracers).
#[derive(Resource)]
pub struct ViewMuzzle(pub Vec3, pub Option<Vec3>);

impl Default for ViewMuzzle {
    fn default() -> Self {
        Self(Vec3::new(0.1, -0.08, -0.35), None)
    }
}

#[derive(Resource, Default)]
struct ViewAnim {
    shown: Option<(u8, u8, Character, crate::data::Attach)>,
    equip: f32,
    last_shots: u32,
    since_shot: f32,
    flash_roll: f32,
    sway: Vec2,
    bob: f32,
    sprint: f32,
    crouch: f32,
    busy: f32,
    glowing: bool,
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

/// Something held in a hand that only shows at certain times.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Prop {
    Grenade,
    Orb,
    Knife,
    Firebomb,
    /// Tinker's gadgets: 0 turret, 1 supply crate, 2 tesla coil.
    Gadget(u8),
    /// Ronin's katana (right hand) and its scabbard (left hand).
    Katana,
    Saya,
    /// Valkyrie's spear (right hand).
    Spear,
    /// A glowing edge along the katana while Iaido charges.
    Edge,
}

#[derive(Component)]
struct Flash(u8);

#[derive(Component)]
struct FlashLight;

#[derive(Resource)]
struct RigAssets {
    skin_mat: Handle<StandardMaterial>,
    glow_mat: Handle<StandardMaterial>,
    orb_mat: Handle<StandardMaterial>,
    edge: Handle<Mesh>,
    edge_mat: Handle<StandardMaterial>,
    flash: Handle<Mesh>,
    grenade: Handle<Mesh>,
    orb: Handle<Mesh>,
    knife: Handle<Mesh>,
    firebomb: (Handle<Mesh>, Handle<Mesh>),
    gadgets: [Handle<Mesh>; 3],
    katana: Handle<Mesh>,
    saya: Handle<Mesh>,
    spear: Handle<Mesh>,
    spear_mat: Handle<StandardMaterial>,
}

/// Ronin's katana: wrapped hilt, round guard and a long, slightly curved
/// blade with a pale edge, pointing forward (-Z) from the fist.
fn katana_kit() -> Kit {
    let mut k = Kit::fine();
    let wrap = c(0.12, 0.05, 0.05);
    let ray = c(0.85, 0.82, 0.75);
    let steel = c(0.78, 0.8, 0.84);
    let edge = c(0.95, 0.96, 1.0);
    let gold = c(0.75, 0.6, 0.25);
    // Hilt: ray skin under a diamond wrap, gold collar and cap.
    k.cyl_z(Vec3::new(0.0, 0.0, 0.07), 0.016, 0.24, ray);
    for i in 0..7 {
        let z = -0.03 + i as f32 * 0.03;
        k.cuboid_rot(
            Vec3::new(0.0, 0.0, z),
            Vec3::new(0.036, 0.008, 0.012),
            Quat::from_rotation_z(0.6),
            wrap,
        );
        k.cuboid_rot(
            Vec3::new(0.0, 0.0, z),
            Vec3::new(0.036, 0.008, 0.012),
            Quat::from_rotation_z(-0.6),
            wrap,
        );
    }
    k.cyl_z(Vec3::new(0.0, 0.0, 0.195), 0.019, 0.02, gold);
    k.cyl_z(Vec3::new(0.0, 0.0, -0.055), 0.018, 0.02, gold);
    // Guard.
    k.cyl_z(Vec3::new(0.0, 0.0, -0.07), 0.045, 0.01, c(0.15, 0.13, 0.12));
    k.cyl_z(Vec3::new(0.0, 0.0, -0.082), 0.016, 0.015, gold);
    // Blade in four segments, each tilted a little more for the curve.
    let mut at = Vec3::new(0.0, 0.0, -0.09);
    for i in 0..4 {
        let tilt = 0.035 * (i as f32 + 0.5);
        let dir = Quat::from_rotation_x(tilt) * Vec3::NEG_Z;
        let len = 0.17;
        let mid = at + dir * len / 2.0;
        let rot = Quat::from_rotation_x(tilt);
        let w = 0.03 - i as f32 * 0.002;
        k.cuboid_rot(mid, Vec3::new(0.006, w, len + 0.004), rot, steel);
        k.cuboid_rot(
            mid + rot * Vec3::new(0.0, -w / 2.0, 0.0),
            Vec3::new(0.003, 0.008, len + 0.004),
            rot,
            edge,
        );
        at += dir * len;
    }
    let tip = Quat::from_rotation_x(0.16);
    k.wedge(
        at + tip * Vec3::new(0.0, -0.004, -0.025),
        Vec3::new(0.006, 0.026, 0.05),
        tip * Quat::from_rotation_y(PI / 2.0),
        edge,
    );
    k
}

/// Tinker's supply drop: a small green crate with yellow bands and a
/// beacon light.
fn supply_kit() -> (Kit, Kit) {
    let (mut k, mut g) = (Kit::new(), Kit::new());
    k.cuboid(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.3, 0.18, 0.22),
        c(0.25, 0.35, 0.2),
    );
    for x in [-0.1, 0.1] {
        k.cuboid(
            Vec3::new(x, 0.0, 0.0),
            Vec3::new(0.03, 0.185, 0.225),
            c(0.95, 0.75, 0.12),
        );
    }
    k.cuboid(
        Vec3::new(0.0, 0.1, 0.0),
        Vec3::new(0.08, 0.02, 0.04),
        c(0.15, 0.15, 0.17),
    );
    g.cuboid(
        Vec3::new(0.0, 0.0, 0.112),
        Vec3::new(0.06, 0.06, 0.005),
        c(1.0, 1.0, 1.0),
    );
    g.cuboid(
        Vec3::new(0.0, 0.0, 0.113),
        Vec3::new(0.05, 0.015, 0.005),
        c(0.9, 0.15, 0.1),
    );
    g.cuboid(
        Vec3::new(0.0, 0.0, 0.113),
        Vec3::new(0.015, 0.05, 0.005),
        c(0.9, 0.15, 0.1),
    );
    (k, g)
}

/// Black lacquered scabbard with a gold mouth and cord.
fn saya_kit() -> Kit {
    let mut k = Kit::fine();
    k.blob(
        Vec3::new(0.0, 0.0, -0.35),
        Vec3::new(0.016, 0.026, 0.36),
        c(0.06, 0.05, 0.06),
    );
    k.cyl_z(Vec3::new(0.0, 0.0, 0.0), 0.024, 0.02, c(0.75, 0.6, 0.25));
    k.torus(
        Vec3::new(0.0, 0.0, -0.1),
        0.004,
        0.024,
        Quat::from_rotation_x(PI / 2.0),
        c(0.5, 0.1, 0.1),
    );
    k
}

/// Combat knife held in the fist, blade forward.
fn knife_kit() -> Kit {
    let mut k = Kit::fine();
    let grip = c(0.08, 0.08, 0.085);
    let steel = c(0.72, 0.74, 0.78);
    k.cyl_z(Vec3::new(0.0, 0.0, 0.0), 0.013, 0.1, grip);
    for i in 0..4 {
        k.torus(
            Vec3::new(0.0, 0.0, 0.035 - i as f32 * 0.022),
            0.003,
            0.0125,
            Quat::from_rotation_x(PI / 2.0),
            c(0.03, 0.03, 0.03),
        );
    }
    k.sphere(Vec3::new(0.0, 0.0, 0.052), 0.015, c(0.3, 0.3, 0.32));
    k.cuboid(
        Vec3::new(0.0, 0.0, -0.054),
        Vec3::new(0.055, 0.012, 0.008),
        c(0.25, 0.25, 0.27),
    );
    // Blade: a long flat wedge with a darker spine and a clipped point.
    k.cuboid(
        Vec3::new(0.0, 0.004, -0.13),
        Vec3::new(0.004, 0.026, 0.14),
        steel,
    );
    k.cuboid(
        Vec3::new(0.0, 0.016, -0.12),
        Vec3::new(0.005, 0.005, 0.12),
        c(0.35, 0.36, 0.38),
    );
    k.wedge(
        Vec3::new(0.0, 0.004, -0.215),
        Vec3::new(0.004, 0.026, 0.03),
        Quat::from_rotation_y(PI / 2.0),
        steel,
    );
    k
}

fn flash_kit() -> Kit {
    let mut k = Kit::new();
    let fwd = Quat::from_rotation_x(-PI / 2.0);
    k.cone(
        Vec3::new(0.0, 0.0, -0.05),
        0.022,
        0.1,
        fwd,
        c(1.0, 0.75, 0.3),
    );
    k.cone(
        Vec3::new(0.0, 0.0, -0.03),
        0.035,
        0.05,
        fwd,
        c(1.0, 0.9, 0.55),
    );
    for i in 0..4 {
        let r = Quat::from_rotation_z(i as f32 * PI / 4.0);
        k.cuboid_rot(
            Vec3::new(0.0, 0.0, -0.02),
            Vec3::new(0.09, 0.006, 0.02),
            r,
            c(1.0, 0.65, 0.2),
        );
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
        edge: meshes.add(Cuboid::new(0.014, 0.04, 0.7)),
        edge_mat: materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.85, 0.4, 0.8),
            unlit: true,
            alpha_mode: AlphaMode::Add,
            ..default()
        }),
        flash: meshes.add(flash_kit().build_or_empty()),
        grenade: meshes.add(grenade.build_or_empty()),
        orb: meshes.add(Sphere::new(0.03).mesh().ico(2).unwrap()),
        knife: meshes.add(knife_kit().build_or_empty()),
        firebomb: {
            let (k, g) = crate::avatars::firebomb_kit();
            (
                meshes.add(k.build_or_empty()),
                meshes.add(g.build_or_empty()),
            )
        },
        gadgets: [
            crate::avatars::turret_kit(),
            supply_kit(),
            crate::avatars::coil_kit(),
        ]
        .map(|(mut k, g)| {
            k.append(g, Transform::IDENTITY);
            meshes.add(k.build_or_empty())
        }),
        katana: meshes.add(katana_kit().build_or_empty()),
        saya: meshes.add(saya_kit().build_or_empty()),
        spear: meshes.add(crate::fx::spear_kit().build_or_empty()),
        spear_mat: materials.add(StandardMaterial {
            emissive: LinearRgba::rgb(0.25, 0.5, 1.0),
            ..vertex_material(0.3, 0.4)
        }),
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
                        let prop = |h: &mut ChildSpawnerCommands,
                                    prop: Prop,
                                    mesh: &Handle<Mesh>,
                                    mat: &Handle<StandardMaterial>,
                                    tf: Transform| {
                            h.spawn((
                                prop,
                                Mesh3d(mesh.clone()),
                                MeshMaterial3d(mat.clone()),
                                tf,
                                Visibility::Hidden,
                                NotShadowCaster,
                            ))
                            .id()
                        };
                        let skin = &assets.skin_mat;
                        if side < 0.0 {
                            prop(
                                h,
                                Prop::Grenade,
                                &assets.grenade,
                                skin,
                                Transform::from_xyz(0.0, -0.005, -0.012),
                            );
                            prop(
                                h,
                                Prop::Knife,
                                &assets.knife,
                                skin,
                                Transform::from_xyz(0.0, -0.01, -0.01),
                            );
                            prop(
                                h,
                                Prop::Orb,
                                &assets.orb,
                                &assets.orb_mat,
                                Transform::from_xyz(0.0, 0.0, -0.01),
                            );
                            let bomb = prop(
                                h,
                                Prop::Firebomb,
                                &assets.firebomb.0,
                                skin,
                                Transform::from_xyz(0.0, -0.06, -0.012),
                            );
                            h.commands().entity(bomb).with_child((
                                Mesh3d(assets.firebomb.1.clone()),
                                MeshMaterial3d(assets.glow_mat.clone()),
                                NotShadowCaster,
                            ));
                            for (i, scale) in [0.13, 0.22, 0.09].into_iter().enumerate() {
                                prop(
                                    h,
                                    Prop::Gadget(i as u8),
                                    &assets.gadgets[i],
                                    skin,
                                    Transform::from_xyz(0.0, 0.02, -0.03)
                                        .with_scale(Vec3::splat(scale)),
                                );
                            }
                            prop(
                                h,
                                Prop::Saya,
                                &assets.saya,
                                skin,
                                Transform::from_xyz(0.0, 0.0, -0.01),
                            );
                        } else {
                            // The hilt runs up through the fist like a pistol grip.
                            prop(
                                h,
                                Prop::Katana,
                                &assets.katana,
                                skin,
                                Transform::from_xyz(0.0, 0.06, 0.0)
                                    .with_rotation(Quat::from_rotation_x(PI / 2.0)),
                            );
                            prop(
                                h,
                                Prop::Edge,
                                &assets.edge,
                                &assets.edge_mat,
                                Transform::from_xyz(0.0, 0.06 + 0.43, 0.012)
                                    .with_rotation(Quat::from_rotation_x(PI / 2.0 + 0.07)),
                            );
                            prop(
                                h,
                                Prop::Spear,
                                &assets.spear,
                                &assets.spear_mat,
                                Transform::from_xyz(0.0, 0.1, 0.0).with_scale(Vec3::splat(0.7)),
                            );
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
    let Some(me) = roster.me(&session) else {
        return;
    };
    let Some((gun, attach)) = loadout.current().map(|g| (g.id, g.attach)) else {
        return;
    };
    let key = (gun, me.skin_for(gun), me.character, attach);
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
            spawn_gun(p, &guns, gun, attach, skin.clone(), true);
            if pivot.0 == 0 || def.rig.dual {
                p.spawn((
                    Flash(pivot.0),
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

#[allow(clippy::too_many_arguments)]
fn animate(
    time: Res<Time>,
    motion: Res<AccumulatedMouseMotion>,
    state: Res<State<AppState>>,
    session: Res<Session>,
    roster: Res<Roster>,
    loadout: Res<Loadout>,
    cast: Res<CastState>,
    auras: Res<crate::auras::Auras>,
    aim: Res<crate::weapons::Aim>,
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
        Query<(&Flash, &mut Transform, &mut Visibility)>,
        Query<(&mut Transform, Has<GunPump>), Or<(With<GunMag>, With<GunPump>)>>,
        Query<(&Prop, &mut Visibility, &mut Transform)>,
        Query<&mut Visibility, With<ViewRoot>>,
    )>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let me = roster.me(&session);
    let gun = loadout.current().map(|g| g.id);
    let attach = loadout.current().map(|g| g.attach).unwrap_or_default();
    // Hidden while dead, emoting or looking through a scope.
    let show = *state.get() == AppState::InGame
        && me.is_some_and(|m| m.alive)
        && gun.is_some()
        && !player.third_person()
        && !aim.scoped;
    for mut vis in &mut parts.p7() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let (Some(me), Some(gun), true) = (me, gun, show) else {
        return;
    };
    let (rig, sight_y) = crate::gunmodels::fitted_rig(&guns.gun(gun).rig, attach);
    let ads = ease(aim.amount);
    let steady = 1.0 - 0.85 * ads;
    let def = gun_def(gun);

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
    let sprint_target = if player.sprinting && player.on_ground && !firing {
        1.0
    } else {
        0.0
    };
    anim.sprint = approach(anim.sprint, sprint_target, dt * 9.0);
    let crouch_target = if player.crouching || player.sliding > 0.0 {
        1.0
    } else {
        0.0
    };
    anim.crouch = approach(anim.crouch, crouch_target, dt * 8.0);
    let look = -motion.delta * 0.0012;
    let look = look.clamp(Vec2::splat(-0.08), Vec2::splat(0.08));
    anim.sway = anim.sway.lerp(look, 1.0 - (-dt * 9.0).exp());
    let speed = player.horizontal_speed();
    let amp = if player.on_ground {
        (speed / 6.0).min(1.4)
    } else {
        0.0
    };
    anim.bob += dt * (4.0 + speed * 1.2).min(16.0) * amp.min(1.0);

    // Is the left hand busy with an ability?
    let ab = |slot: u8| me.kit[slot as usize];
    let style_of = |slot: u8| ab(slot).style();
    let blade = |slot: u8| matches!(style_of(slot), CastStyle::Sword | CastStyle::Spear);
    let anim_len = |slot: u8| if blade(slot) { 0.7 } else { 0.55 };
    let cast_anim = cast.cast.filter(|(slot, s)| *s < anim_len(*slot));
    // Sword and spear casts use the right hand, so the gun goes down.
    let two_hand = cast_anim.filter(|(slot, _)| cast.aiming.is_none() && blade(*slot));
    // Holding Iaido: the sword comes out and waits, ready to cut.
    let charging = cast.aiming.filter(|s| ab(*s).charges());
    let stow = if charging.is_some() {
        ramp(cast.held, 0.0, 0.1)
    } else {
        two_hand.map_or(0.0, |(_, s)| {
            ramp(s, 0.0, 0.07) * (1.0 - ramp(s, 0.5, 0.68))
        })
    };
    let cast_slot = cast.aiming.or(cast_anim.map(|(s, _)| s));
    let busy = two_hand.is_none()
        && charging.is_none()
        && cast_slot.is_some_and(|s| style_of(s) != CastStyle::Move);
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
    // Aiming down sights: the sight line runs straight through the middle
    // of the screen, with the eye a little behind the rear sight or optic.
    if ads > 0.0 && !rig.dual {
        let eye_z = if short {
            -0.3
        } else if attach.optic() > 0 {
            -(rig.optic_at.z + 0.19)
        } else if rig.builtin_sight.is_some() {
            -(rig.builtin_z + 0.19)
        } else {
            -0.1
        };
        // Tilt the gun so the line from the rear sight to the front post
        // points straight ahead, then put that line through the eye.
        let along = Vec3::new(0.0, -rig.sight_slope, -1.0).normalize();
        let ads_rot = Quat::from_rotation_arc(along, Vec3::NEG_Z);
        let ads_pos = Vec3::new(0.0, 0.0, eye_z) - ads_rot * Vec3::new(0.0, sight_y, 0.0);
        pos = pos.lerp(ads_pos, ads);
        rot = rot.slerp(ads_rot, ads);
    }
    // Breathing and walk bob.
    pos.y += (t * 1.7).sin() * 0.002 * steady;
    pos.x += anim.bob.sin() * 0.007 * amp * steady;
    pos.y -= anim.bob.cos().abs() * 0.009 * amp * steady;
    rot *= Quat::from_rotation_z(anim.bob.sin() * 0.015 * amp * steady);
    // Sway lags behind the mouse.
    pos.x += anim.sway.x * 0.15 * steady;
    pos.y -= anim.sway.y * 0.15 * steady;
    let sw = anim.sway * steady;
    rot *= Quat::from_euler(EulerRot::YXZ, sw.x, sw.y, sw.x * 0.8);
    // In the air the gun floats up a little.
    if !player.on_ground {
        pos.y += (player.vel.y * -0.002).clamp(-0.01, 0.015);
    }
    // Sprint: muzzle swings down and in.
    let sp = ease(anim.sprint);
    pos += Vec3::new(-0.03, -0.045, 0.03) * sp;
    rot *= Quat::from_euler(EulerRot::YXZ, 0.65 * sp, -0.3 * sp, 0.35 * sp);
    // Crouch / slide cant.
    rot *= Quat::from_rotation_z(0.12 * anim.crouch * (1.0 - ads));
    // Recoil.
    let handling = attach.handling(gun);
    let kick = crate::data::recoil(gun).visual;
    let r = loadout.recoil * (0.5 + 0.5 * handling.recoil_up);
    let recoil_at = |k: f32| {
        (
            Vec3::new(0.0, r * 0.006 * kick * k, r * 0.045 * kick * k * (1.0 - 0.4 * ads)),
            Quat::from_rotation_x(r * 0.09 * kick * k * (1.0 - 0.6 * ads)),
        )
    };
    // Dual guns fire together; the left one kicks a little out of step.
    let (recoil_pos, recoil_rot) = recoil_at(1.0);
    let (recoil2_pos, recoil2_rot) = recoil_at(0.85);
    // Raising a new gun.
    let e = ease(anim.equip);
    pos.y -= (1.0 - e) * 0.3;
    rot *= Quat::from_rotation_x(-(1.0 - e) * 1.0);
    // Left hand off the gun: lower it a touch.
    pos.y -= anim.busy * 0.015;
    // Reload.
    let reload = (loadout.reload > 0.0 && loadout.reload_total > 0.0)
        .then(|| 1.0 - loadout.reload / loadout.reload_total);
    let rl = reload
        .map(|p| ramp(p, 0.0, 0.1) * (1.0 - ramp(p, 0.88, 1.0)))
        .unwrap_or(0.0);
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
    // Melee: the gun drops out of the way while the knife comes across.
    let melee = loadout.melee.map(|t| t / crate::weapons::MELEE_TIME);
    if let Some(k) = melee {
        let off = ramp(k, 0.0, 0.15) * (1.0 - ramp(k, 0.6, 1.0));
        pos += Vec3::new(0.06, -0.13, 0.06) * off;
        rot *= Quat::from_euler(EulerRot::YXZ, -0.35 * off, -0.6 * off, -0.5 * off);
    }
    if stow > 0.0 {
        pos += Vec3::new(0.05, -0.3, 0.1) * stow;
        rot *= Quat::from_rotation_x(-0.8 * stow);
    }
    let gun_tf = at(pos + recoil_pos, rot * recoil_rot);
    let gun2_tf = at(
        Vec3::new(-pos.x, pos.y, pos.z) + recoil2_pos,
        mirror(rot) * recoil2_rot,
    );

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
        let shot = if (0.12..0.5).contains(&s) {
            ((s - 0.12) / 0.38 * PI).sin()
        } else {
            0.0
        };
        let after_reload = reload.map(|p| {
            if p > 0.86 {
                ((p - 0.86) / 0.14 * PI).sin()
            } else {
                0.0
            }
        });
        rig.pump_travel * shot.max(after_reload.unwrap_or(0.0))
    } else {
        0.0
    };

    // --- Left hand ---------------------------------------------------------
    let grip_rot = Quat::from_rotation_x(-rig.grip_tilt);
    let grip_hand = at(grip_rot * Vec3::new(0.0, -0.045, 0.004), grip_rot);
    let mut right_tf = gun_tf * grip_hand;
    let mut right_pose = HandPose::Grip { trigger: true };
    let mut right_held = None;
    let (support_pose, support_local) = match rig.support_style {
        Support::Under => (
            HandPose::Under,
            at(rig.support + Vec3::new(0.0, 0.032, pump), Quat::IDENTITY),
        ),
        Support::Vertical => (
            HandPose::Grip { trigger: false },
            at(rig.support, Quat::from_rotation_x(-0.1)),
        ),
        Support::Cup => (
            HandPose::Grip { trigger: false },
            at(
                rig.support + Vec3::new(0.0, 0.0, -0.005),
                grip_rot * Quat::from_rotation_y(-0.2),
            ),
        ),
    };
    let mut left_pose = support_pose;
    let mut left_tf = if rig.dual {
        gun2_tf * grip_hand
    } else {
        gun_tf * support_local
    };
    if rig.dual {
        left_pose = HandPose::Grip { trigger: true };
    }

    // Reloads move the left hand to the magazine and away for a new one.
    if let (Some(p), false) = (reload, rig.dual) {
        let mag_tf =
            |off: f32| gun_tf * at(rig.mag_pos + rig.mag_out * (off + 0.07), Quat::IDENTITY);
        let away = at(Vec3::new(-0.12, -0.42, -0.15), Quat::from_rotation_x(0.6));
        let back = left_tf;
        if rig.single_load {
            // Feed rounds in one at a time.
            let port = gun_tf
                * at(
                    rig.mag_pos
                        + if gun == 18 {
                            Vec3::new(-0.06, -0.02, 0.0)
                        } else {
                            Vec3::new(0.0, -0.05, 0.0)
                        },
                    Quat::IDENTITY,
                );
            let below = at(
                port.translation + Vec3::new(-0.04, -0.14, 0.06),
                port.rotation,
            );
            let cycles = if gun == 12 { 1.0 } else { 3.0 };
            if (0.12..0.86).contains(&p) {
                let u = (p - 0.12) / 0.74 * cycles;
                let k = (u.fract() * PI).sin();
                left_tf = blend(below, port, k);
                left_pose = HandPose::Hold;
            } else {
                let k = if p < 0.12 {
                    ramp(p, 0.0, 0.12)
                } else {
                    1.0 - ramp(p, 0.86, 1.0)
                };
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
    let mut held: Option<Prop> = None;
    let mut orb: Option<(Color, f32)> = None;
    if busy {
        let slot = cast_slot.unwrap_or(0);
        let style = style_of(slot);
        let ready = at(Vec3::new(-0.13, -0.13, -0.32), Quat::from_rotation_x(0.15));
        let ability = ab(slot);
        let color = ability.color();
        // What the hand holds for this ability, if anything.
        let thing = match ability {
            Ability::Firebomb => Some(Prop::Firebomb),
            Ability::KunaiFan => Some(Prop::Knife),
            Ability::Sentry | Ability::CombatDrone => Some(Prop::Gadget(0)),
            Ability::SupplyDrop => Some(Prop::Gadget(1)),
            Ability::TeslaCoil => Some(Prop::Gadget(2)),
            _ if style == CastStyle::Throw => Some(Prop::Grenade),
            _ => None,
        };
        let mut hand = ready;
        let mut pose = HandPose::Hold;
        if let Some((_, s)) = cast_anim.filter(|_| cast.aiming.is_none()) {
            match style {
                CastStyle::Throw => {
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
                    held = thing.filter(|_| s < 0.26);
                    pose = if s < 0.26 {
                        HandPose::Hold
                    } else {
                        HandPose::Open
                    };
                }
                CastStyle::Deploy => {
                    // Set the gadget down in front, then let go.
                    let down = at(Vec3::new(-0.05, -0.38, -0.6), Quat::from_rotation_x(-0.2));
                    hand = if s < 0.06 {
                        blend(left_tf, ready, ramp(s, 0.0, 0.06))
                    } else if s < 0.26 {
                        blend(ready, down, ramp(s, 0.06, 0.26))
                    } else {
                        blend(down, left_tf, ramp(s, 0.3, 0.55))
                    };
                    held = thing.filter(|_| s < 0.28);
                    pose = if s < 0.28 {
                        HandPose::Hold
                    } else {
                        HandPose::Open
                    };
                    if (0.22..0.32).contains(&s) {
                        orb = Some((color, 2.5));
                    }
                }
                _ => {
                    // Push the palm out (heal, nova, overdrive, flames) or
                    // raise it to the sky (orbital strike, Ragnarok).
                    let push = match style {
                        CastStyle::Sky => {
                            at(Vec3::new(-0.09, 0.08, -0.4), Quat::from_rotation_x(0.5))
                        }
                        // Slammed down at the floor.
                        CastStyle::Ground => {
                            at(Vec3::new(-0.04, -0.42, -0.5), Quat::from_rotation_x(-1.2))
                        }
                        _ => at(Vec3::new(-0.06, -0.08, -0.52), Quat::IDENTITY),
                    };
                    hand = if s < 0.12 {
                        blend(ready, push, ramp(s, 0.0, 0.12))
                    } else {
                        blend(push, left_tf, ramp(s, 0.3, 0.55))
                    };
                    pose = if s < 0.4 {
                        HandPose::Open
                    } else {
                        support_pose
                    };
                    if s < 0.25 {
                        orb = Some((color, 1.0 + s * 10.0));
                    }
                }
            }
        } else {
            // Holding / aiming: the thing ready in the hand (steady) or an
            // orb gathering energy.
            if let Some(thing) = thing {
                held = Some(thing);
            } else {
                hand.translation += Vec3::new((t * 40.0).sin(), (t * 33.0).cos(), 0.0) * 0.0006;
                orb = Some((color, 0.8 + 0.15 * (t * 8.0).sin()));
            }
        }
        left_tf = blend(left_tf, hand, anim.busy);
        if anim.busy > 0.4 {
            left_pose = pose;
        } else {
            held = None;
        }
    }

    // Iaido / Blade Storm: draw the katana from the left hip and cut across.
    // Arc Spear: raise the spear over the shoulder and hurl it.
    let mut saya = false;
    let mut edge: Option<(Color, f32)> = None;
    // The fist turned so the blade (up through the grip) points along `dir`.
    let sword = |pos: Vec3, dir: Vec3| at(pos, Quat::from_rotation_arc(Vec3::Y, dir.normalize()));
    let sheath = sword(Vec3::new(-0.18, -0.3, -0.22), Vec3::new(-0.5, -0.3, 0.8));
    // Iaido held ready: blade out to the left, level, edge forward.
    let ready_cut = sword(Vec3::new(0.02, -0.15, -0.42), Vec3::new(-0.75, 0.15, -0.65));
    // The left hand holds the scabbard at the hip.
    let hip = at(
        Vec3::new(-0.2, -0.34, -0.22),
        Quat::from_euler(EulerRot::YXZ, 1.75, 0.12, 0.0),
    );
    if let Some(slot) = charging {
        let k = (cast.held / crate::abilities::FULL_CHARGE).min(1.0);
        let draw = ramp(cast.held, 0.0, 0.07);
        let mut ready = ready_cut;
        // Fully charged: the blade trembles with held power.
        if k >= 1.0 {
            ready.translation +=
                Vec3::new((t * 70.0).sin(), (t * 53.0).cos(), (t * 61.0).sin()) * 0.0025;
        }
        right_tf = if cast.held < 0.07 {
            blend(right_tf, sheath, draw)
        } else {
            blend(sheath, ready, ease(ramp(cast.held, 0.07, 0.22)))
        };
        right_held = Some(Prop::Katana);
        right_pose = HandPose::Grip { trigger: false };
        left_tf = blend(left_tf, hip, ease(draw));
        left_pose = HandPose::Hold;
        saya = true;
        edge = Some((ab(slot).color(), 0.25 + 0.75 * k));
    }
    if let Some((slot, s)) = two_hand {
        let back = right_tf;
        let ease_back = |pose: Transform| blend(pose, back, ramp(s, 0.48, 0.68));
        if style_of(slot) == CastStyle::Sword {
            // The cut sweeps the blade across the screen: drawn from the
            // hip, up on the left, over and down to the right.
            let mut start = sword(Vec3::new(-0.3, -0.06, -0.36), Vec3::new(-0.8, 0.5, -0.3));
            let mut mid = sword(Vec3::new(0.0, -0.06, -0.46), Vec3::new(0.05, 0.85, -0.5));
            let mut finish = sword(Vec3::new(0.3, -0.2, -0.32), Vec3::new(0.9, -0.15, -0.4));
            let ability = ab(slot);
            if ability == Ability::RisingDragon {
                // Low on the right, up through the middle, high overhead.
                start = sword(Vec3::new(0.2, -0.4, -0.3), Vec3::new(0.3, -0.3, -0.9));
                mid = sword(Vec3::new(0.05, -0.05, -0.5), Vec3::new(0.0, 0.6, -0.8));
                finish = sword(Vec3::new(-0.05, 0.25, -0.35), Vec3::new(-0.1, 1.0, 0.3));
            }
            // A charged Iaido cuts straight from where it was held, flat
            // and fast across the whole view.
            let from_ready = ability.charges() && cast.held > 0.1;
            if from_ready {
                start = ready_cut;
                mid = sword(Vec3::new(0.05, -0.12, -0.5), Vec3::new(0.0, 0.2, -1.0));
                finish = sword(Vec3::new(0.38, -0.16, -0.3), Vec3::new(0.95, 0.05, -0.2));
                if s < 0.3 {
                    edge = Some((ability.color(), 1.0 - ramp(s, 0.1, 0.3)));
                }
            }
            let (t0, t1, t2) = if from_ready {
                (0.0, 0.035, 0.08)
            } else {
                (0.08, 0.13, 0.17)
            };
            right_tf = if s < t0 {
                blend(back, sheath, ramp(s, 0.0, t0))
            } else if s < t1 {
                blend(if from_ready { start } else { sheath }, if from_ready { mid } else { start }, ramp(s, t0, t1))
            } else if s < t2 {
                blend(if from_ready { mid } else { start }, if from_ready { finish } else { mid }, ramp(s, t1, t2))
            } else if !from_ready && s < 0.22 {
                blend(mid, finish, ramp(s, 0.17, 0.22))
            } else {
                ease_back(finish)
            };
            right_held = (if from_ready { 0.0 } else { 0.06 }..0.6)
                .contains(&s)
                .then_some(Prop::Katana);
            left_tf = blend(
                left_tf,
                hip,
                ramp(s, 0.0, 0.07) * (1.0 - ramp(s, 0.5, 0.68)),
            );
            if s < 0.6 {
                left_pose = HandPose::Hold;
                saya = true;
            }
        } else {
            // The fist turned so the spear points forward (and a little up).
            let spear = |pitch: f32| Quat::from_rotation_x(pitch - PI / 2.0);
            let raise = at(Vec3::new(0.17, -0.02, -0.08), spear(0.12));
            let release = at(Vec3::new(0.06, -0.08, -0.55), spear(-0.05));
            let after = at(Vec3::new(0.1, -0.3, -0.4), spear(-0.5));
            right_tf = if s < 0.1 {
                blend(back, raise, ramp(s, 0.0, 0.1))
            } else if s < 0.18 {
                blend(raise, release, ramp(s, 0.1, 0.18))
            } else if s < 0.35 {
                blend(release, after, ramp(s, 0.18, 0.35))
            } else {
                ease_back(after)
            };
            right_held = (0.03..0.18).contains(&s).then_some(Prop::Spear);
            // The other hand points the way.
            let point = at(Vec3::new(-0.16, -0.06, -0.5), Quat::from_rotation_x(0.1));
            left_tf = blend(
                left_tf,
                point,
                ramp(s, 0.0, 0.1) * (1.0 - ramp(s, 0.4, 0.6)),
            );
            if s < 0.45 {
                left_pose = HandPose::Open;
            }
            if s < 0.2 {
                orb = Some((ab(slot).color(), 1.0 + s * 6.0));
            }
        }
        if s < 0.6 {
            right_pose = HandPose::Grip { trigger: false };
        }
    }

    // Melee: wind up on the left, slash across, bring the hand back.
    if let Some(k) = melee {
        let wind = at(
            Vec3::new(-0.22, 0.02, -0.26),
            Quat::from_euler(EulerRot::YXZ, 0.75, 0.25, -1.2),
        );
        let cut = at(
            Vec3::new(0.0, -0.05, -0.42),
            Quat::from_euler(EulerRot::YXZ, 0.0, 0.05, -1.45),
        );
        let end = at(
            Vec3::new(0.22, -0.15, -0.32),
            Quat::from_euler(EulerRot::YXZ, -0.85, -0.15, -1.5),
        );
        let hand = if k < 0.2 {
            blend(left_tf, wind, ramp(k, 0.0, 0.2))
        } else if k < 0.3 {
            blend(wind, cut, ramp(k, 0.2, 0.3))
        } else if k < 0.4 {
            blend(cut, end, ramp(k, 0.3, 0.4))
        } else {
            blend(end, left_tf, ramp(k, 0.5, 0.95))
        };
        left_tf = hand;
        left_pose = HandPose::Hold;
        held = (k < 0.8).then_some(Prop::Knife);
        orb = None;
    }

    // --- Apply -------------------------------------------------------------
    for (pivot, mut tf, mut vis) in &mut parts.p0() {
        if pivot.0 == 0 {
            *tf = gun_tf;
            *vis = if stow < 0.6 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        } else {
            *tf = gun2_tf;
            *vis = if rig.dual && anim.busy < 0.5 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    // Place the hands and remember which pose each one shows.
    let mut hand_poses: Vec<(Entity, HandPose)> = Vec::with_capacity(2);
    let mut wrists = [Vec3::ZERO; 2];
    for (entity, hand, mut tf) in &mut parts.p1() {
        let (htf, pose, i) = if hand.0 > 0.0 {
            (right_tf, right_pose, 0)
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
        let shoulder = if arm.0 > 0.0 {
            Vec3::new(0.28, -0.45, 0.25)
        } else {
            Vec3::new(-0.32, -0.5, 0.08)
        };
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
        *vis = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Muzzle flash (a suppressor hides it).
    let flash_on = anim.since_shot < 0.045 && loadout.shots > 0 && handling.flash > 0.0;
    for (flash, mut tf, mut vis) in &mut parts.p4() {
        *vis = if flash_on && (flash.0 == 0 || rig.dual) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let s = handling.flash
            * if def.class == GunClass::Shotgun || def.class == GunClass::Lmg {
                1.4
            } else {
                1.0
            };
        *tf = Transform::from_translation(rig.muzzle)
            .with_rotation(Quat::from_rotation_z(anim.flash_roll))
            .with_scale(Vec3::splat(s));
    }
    let muzzle_root = gun_tf.transform_point(rig.muzzle);
    muzzle.0 = muzzle_root * RIG_SCALE;
    muzzle.1 = rig.dual.then(|| gun2_tf.transform_point(rig.muzzle) * RIG_SCALE);
    anim.light = (
        muzzle_root + Vec3::new(0.0, 0.05, 0.0),
        flash_on,
        if def.rare {
            rig.glow_color
        } else {
            Color::srgb(1.0, 0.8, 0.4)
        },
    );
    for (mut tf, is_pump) in &mut parts.p5() {
        if is_pump {
            tf.translation = Vec3::new(0.0, 0.0, pump);
        } else {
            tf.translation = rig.mag_pos + rig.mag_out * mag_offset;
            let long = if attach.ext_mag() { 1.45 } else { 1.0 };
            tf.scale = if mag_hidden {
                Vec3::ZERO
            } else {
                Vec3::new(1.0, long, 1.0)
            };
        }
    }
    for (prop, mut vis, mut tf) in &mut parts.p6() {
        let shown = match *prop {
            Prop::Orb => {
                if let Some((color, size)) = orb {
                    tf.scale = Vec3::splat(size);
                    if let Some(m) = materials.get_mut(&rig_assets.orb_mat) {
                        m.base_color = color.with_alpha(0.85);
                    }
                }
                orb.is_some()
            }
            Prop::Saya => saya,
            Prop::Edge => {
                if let Some((color, k)) = edge {
                    tf.scale = Vec3::new(1.0 + k, 1.0, 0.4 + 0.6 * k);
                    if let Some(m) = materials.get_mut(&rig_assets.edge_mat) {
                        m.base_color = color.with_alpha(0.9 * k);
                    }
                }
                edge.is_some() && right_held == Some(Prop::Katana)
            }
            p => held == Some(p) || right_held == Some(p),
        };
        let want = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }

    // Your aura makes your hands and arms glow.
    let glow = auras
        .glow(me.id)
        .map(|(c, k)| LinearRgba::from(c) * k * (0.2 + 0.08 * (t * 7.0).sin()));
    if glow.is_some() || anim.glowing {
        anim.glowing = glow.is_some();
        if let Some(m) = materials.get_mut(&rig_assets.skin_mat) {
            m.emissive = glow.unwrap_or(LinearRgba::BLACK);
        }
    }
}

fn muzzle_light(
    anim: Res<ViewAnim>,
    mut lights: Query<(&mut Transform, &mut PointLight), With<FlashLight>>,
) {
    for (mut tf, mut light) in &mut lights {
        tf.translation = anim.light.0;
        light.intensity = if anim.light.1 { 60_000.0 } else { 0.0 };
        light.color = anim.light.2;
    }
}

