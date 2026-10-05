//! How things look: enemies (zombie-like people), projectiles, power-ups and
//! the other players in your party, with floating name tags.

use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

use crate::data::{Character, PowerUp};
use crate::humanoid;
use crate::models::projectiles;
use crate::player::LocalPlayer;
use crate::rig::Model;
use crate::rig::{Rig, RigAssets};
use crate::sim::enemy_scale;
use crate::{
    AppState, Enemy, EnemyStatus, InGameEntity, NetKind, Phase, Replicated, Roster, Session,
};

pub struct AvatarPlugin;

impl Plugin for AvatarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                dress_new,
                sync_avatars,
                place_name_tags,
                enemy_colors,
                spin,
                tumble,
                spinners,
            )
                .chain()
                .in_set(Phase::Present)
                .run_if(in_state(AppState::InGame)),
        );
    }
}

#[derive(Resource)]
pub struct ReplicatedAssets {
    ball: Handle<Mesh>,
    fireball: Handle<StandardMaterial>,
    grenade: Handle<StandardMaterial>,
    grenade_mesh: Handle<Mesh>,
    spark: Handle<StandardMaterial>,
    pickup_meshes: [Handle<Mesh>; 4],
    pickup_mat: Handle<StandardMaterial>,
    /// Gadget models: (solid, glowing) for the firebomb, turret and coil.
    gadgets: [(Handle<Mesh>, Handle<Mesh>); 3],
    gadget_mat: Handle<StandardMaterial>,
    gadget_glow: Handle<StandardMaterial>,
    /// Ability projectiles by look: (solid, glowing, light colour).
    missiles: Vec<(Handle<Mesh>, Handle<Mesh>, Color)>,
    mine: (Handle<Mesh>, Handle<Mesh>),
    drone: (Handle<Mesh>, Handle<Mesh>),
    rotor: Handle<Mesh>,
}

/// Blaze's firebomb: a bottle with a burning rag.
pub(crate) fn firebomb_kit() -> (crate::kit::Kit, crate::kit::Kit) {
    use crate::kit::{c, Kit};
    let (mut k, mut g) = (Kit::new(), Kit::new());
    let v = Vec3::new;
    k.cyl(
        v(0.0, 0.0, 0.0),
        0.045,
        0.13,
        Quat::IDENTITY,
        c(0.55, 0.25, 0.08),
    );
    k.frustum(
        v(0.0, 0.085, 0.0),
        0.016,
        0.045,
        0.04,
        Quat::IDENTITY,
        c(0.55, 0.25, 0.08),
    );
    k.cyl(
        v(0.0, 0.12, 0.0),
        0.016,
        0.04,
        Quat::IDENTITY,
        c(0.5, 0.22, 0.07),
    );
    k.cyl(
        v(0.0, 0.0, 0.0),
        0.047,
        0.05,
        Quat::IDENTITY,
        c(0.85, 0.8, 0.65),
    );
    k.blob(v(0.0, 0.15, 0.0), v(0.025, 0.03, 0.025), c(0.8, 0.75, 0.6));
    g.blob(v(0.0, 0.18, 0.0), v(0.03, 0.05, 0.03), c(1.0, 0.55, 0.1));
    (k, g)
}

/// Tinker's sentry: tripod, ammo box, twin barrels and a sensor eye.
pub(crate) fn turret_kit() -> (crate::kit::Kit, crate::kit::Kit) {
    use crate::kit::{c, Kit};
    let (mut k, mut g) = (Kit::new(), Kit::new());
    let v = Vec3::new;
    let yellow = c(0.95, 0.75, 0.12);
    let dark = c(0.15, 0.15, 0.17);
    let steel = c(0.55, 0.57, 0.6);
    for i in 0..3 {
        let a = i as f32 * std::f32::consts::TAU / 3.0 + 0.5;
        k.cyl_between(
            v(0.0, 0.55, 0.0),
            v(a.cos() * 0.5, 0.0, a.sin() * 0.5),
            0.025,
            dark,
        );
        k.cyl(
            v(a.cos() * 0.5, 0.02, a.sin() * 0.5),
            0.05,
            0.04,
            Quat::IDENTITY,
            dark,
        );
    }
    k.cyl(v(0.0, 0.6, 0.0), 0.08, 0.12, Quat::IDENTITY, steel);
    k.cuboid(v(0.0, 0.82, 0.05), v(0.32, 0.26, 0.4), yellow);
    k.cuboid(v(0.0, 0.96, 0.05), v(0.26, 0.04, 0.34), c(0.85, 0.65, 0.08));
    k.cuboid(v(0.22, 0.76, 0.1), v(0.12, 0.16, 0.2), c(0.3, 0.36, 0.22));
    for x in [-0.07, 0.07] {
        k.cyl_z(v(x, 0.84, -0.35), 0.028, 0.45, dark);
        k.cyl_z(v(x, 0.84, -0.58), 0.04, 0.06, steel);
    }
    k.cuboid(v(0.0, 0.84, -0.17), v(0.24, 0.12, 0.06), steel);
    k.cuboid(v(-0.2, 0.84, 0.05), v(0.06, 0.12, 0.3), dark);
    g.cyl_z(v(0.0, 0.93, -0.16), 0.035, 0.02, c(0.3, 0.9, 1.0));
    g.cuboid(v(0.0, 0.97, 0.25), v(0.18, 0.012, 0.02), c(0.3, 0.9, 1.0));
    (k, g)
}

/// Tinker's tesla coil: a base, a column wound with copper and a charged ball.
pub(crate) fn coil_kit() -> (crate::kit::Kit, crate::kit::Kit) {
    use crate::kit::{c, Kit};
    let (mut k, mut g) = (Kit::new(), Kit::new());
    let v = Vec3::new;
    let dark = c(0.18, 0.18, 0.2);
    let copper = c(0.85, 0.48, 0.22);
    k.cyl(v(0.0, 0.08, 0.0), 0.5, 0.16, Quat::IDENTITY, dark);
    k.cyl(
        v(0.0, 0.2, 0.0),
        0.38,
        0.08,
        Quat::IDENTITY,
        c(0.95, 0.75, 0.12),
    );
    for i in 0..4 {
        let a = i as f32 * std::f32::consts::FRAC_PI_2;
        k.cuboid_rot(
            v(a.cos() * 0.45, 0.12, a.sin() * 0.45),
            v(0.2, 0.12, 0.12),
            Quat::from_rotation_y(-a),
            dark,
        );
    }
    k.cyl(
        v(0.0, 1.2, 0.0),
        0.1,
        2.0,
        Quat::IDENTITY,
        c(0.35, 0.35, 0.38),
    );
    for i in 0..12 {
        k.torus(
            v(0.0, 0.5 + i as f32 * 0.12, 0.0),
            0.025,
            0.15,
            Quat::IDENTITY,
            copper,
        );
    }
    k.torus(
        v(0.0, 2.25, 0.0),
        0.06,
        0.32,
        Quat::IDENTITY,
        c(0.6, 0.62, 0.66),
    );
    g.sphere(v(0.0, 2.5, 0.0), 0.22, c(0.55, 0.85, 1.0));
    (k, g)
}

fn glow(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    power: f32,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::from(color) * power,
        ..default()
    })
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(ReplicatedAssets {
        ball: meshes.add(Sphere::new(1.0).mesh().ico(2).unwrap()),
        fireball: glow(&mut materials, Color::srgb(1.0, 0.45, 0.1), 10.0),
        grenade: materials.add(crate::kit::vertex_material(0.5, 0.2)),
        grenade_mesh: meshes.add({
            let mut k = crate::kit::Kit::new();
            crate::gunmodels::grenade_kit(&mut k, Vec3::ZERO);
            k.build_or_empty()
        }),
        spark: glow(&mut materials, Color::srgb(1.0, 0.7, 0.2), 12.0),
        pickup_meshes: PowerUp::ALL.map(|p| meshes.add(powerup_kit(p).build_or_empty())),
        pickup_mat: materials.add(StandardMaterial {
            emissive: LinearRgba::rgb(0.25, 0.25, 0.25),
            ..crate::kit::vertex_material(0.4, 0.3)
        }),
        gadgets: [firebomb_kit(), turret_kit(), coil_kit()].map(|(k, g)| {
            (
                meshes.add(k.build_or_empty()),
                meshes.add(g.build_or_empty()),
            )
        }),
        gadget_mat: materials.add(crate::kit::vertex_material(0.5, 0.3)),
        gadget_glow: materials.add(crate::kit::glow_material(3.0)),
        missiles: (0..=crate::sim::powers::look::GRAV)
            .map(|l| {
                let (k, g, light) = projectiles::missile_kit(l);
                (meshes.add(k.build_or_empty()), meshes.add(g.build_or_empty()), light)
            })
            .collect(),
        mine: {
            let (k, g) = projectiles::mine_kit();
            (meshes.add(k.build_or_empty()), meshes.add(g.build_or_empty()))
        },
        drone: {
            let (k, g) = projectiles::drone_kit();
            (meshes.add(k.build_or_empty()), meshes.add(g.build_or_empty()))
        },
        rotor: meshes.add(projectiles::rotor_kit().build_or_empty()),
    });
}

/// Per-enemy materials so hits, burning and slows can tint each one.
#[derive(Component)]
struct EnemyLook(Handle<StandardMaterial>, f32);

#[derive(Component)]
struct Spin;

/// Thrown things tumble through the air.
#[derive(Component)]
struct Tumble;

/// Power-up models: a nuke bomb, a skull, a big "x2" and an ammo crate.
fn powerup_kit(kind: PowerUp) -> crate::kit::Kit {
    use crate::kit::c;
    use std::f32::consts::FRAC_PI_2;
    let mut k = crate::kit::Kit::new();
    let v = Vec3::new;
    match kind {
        PowerUp::Nuke => {
            let body = c(0.25, 0.28, 0.22);
            k.blob(v(0.0, 0.0, 0.0), v(0.22, 0.22, 0.38), body);
            k.cyl_z(v(0.0, 0.0, 0.0), 0.225, 0.08, c(0.95, 0.75, 0.1));
            k.cone(
                v(0.0, 0.0, 0.42),
                0.12,
                0.14,
                Quat::from_rotation_x(FRAC_PI_2),
                body,
            );
            for i in 0..4 {
                let r = Quat::from_rotation_z(i as f32 * FRAC_PI_2);
                k.cuboid_rot(
                    r * v(0.0, 0.16, 0.42),
                    v(0.02, 0.18, 0.16),
                    r,
                    c(0.2, 0.2, 0.2),
                );
            }
            // Radiation trefoil on each side.
            for s in [-1.0, 1.0] {
                k.cyl(
                    v(s * 0.2, 0.0, -0.08),
                    0.1,
                    0.02,
                    Quat::from_rotation_z(FRAC_PI_2),
                    c(0.95, 0.8, 0.1),
                );
                for i in 0..3 {
                    let a = i as f32 * 2.094 + 0.52;
                    k.cuboid_rot(
                        v(s * 0.212, a.sin() * 0.05, -0.08 + a.cos() * 0.05),
                        v(0.01, 0.06, 0.05),
                        Quat::from_rotation_x(-a),
                        c(0.08, 0.08, 0.08),
                    );
                }
            }
        }
        PowerUp::InstaKill => {
            let bone = c(0.92, 0.9, 0.82);
            k.blob(v(0.0, 0.08, 0.0), v(0.24, 0.24, 0.26), bone);
            k.cuboid(v(0.0, -0.12, -0.08), v(0.24, 0.14, 0.16), bone);
            for s in [-1.0, 1.0] {
                k.blob(
                    v(s * 0.09, 0.05, -0.2),
                    v(0.06, 0.07, 0.04),
                    c(0.05, 0.02, 0.02),
                );
            }
            k.cone(
                v(0.0, -0.04, -0.235),
                0.03,
                0.05,
                Quat::from_rotation_x(FRAC_PI_2),
                c(0.05, 0.02, 0.02),
            );
            for i in 0..5 {
                k.cuboid(
                    v(-0.08 + i as f32 * 0.04, -0.14, -0.165),
                    v(0.03, 0.05, 0.01),
                    c(0.98, 0.97, 0.9),
                );
            }
        }
        PowerUp::DoublePoints => {
            let g = c(0.25, 0.95, 0.35);
            let t = Vec2::new(0.06, 0.06);
            k.beam(v(-0.32, -0.15, 0.0), v(-0.12, 0.15, 0.0), t, g);
            k.beam(v(-0.32, 0.15, 0.0), v(-0.12, -0.15, 0.0), t, g);
            k.beam(v(0.02, 0.12, 0.0), v(0.1, 0.17, 0.0), t, g);
            k.beam(v(0.1, 0.17, 0.0), v(0.24, 0.12, 0.0), t, g);
            k.beam(v(0.24, 0.12, 0.0), v(0.24, 0.03, 0.0), t, g);
            k.beam(v(0.24, 0.03, 0.0), v(0.02, -0.15, 0.0), t, g);
            k.beam(v(0.0, -0.15, 0.0), v(0.28, -0.15, 0.0), t, g);
        }
        PowerUp::MaxAmmo => {
            let olive = c(0.3, 0.36, 0.22);
            k.cuboid(v(0.0, -0.08, 0.0), v(0.5, 0.26, 0.3), olive);
            k.cuboid(v(0.0, 0.06, 0.0), v(0.52, 0.04, 0.32), c(0.22, 0.27, 0.16));
            k.cuboid(v(0.0, -0.08, -0.152), v(0.3, 0.08, 0.01), c(0.95, 0.8, 0.2));
            k.cuboid(v(0.0, 0.1, 0.0), v(0.16, 0.03, 0.05), c(0.1, 0.1, 0.1));
            for i in 0..5 {
                let x = -0.16 + i as f32 * 0.08;
                k.cyl(
                    v(x, 0.16, 0.0),
                    0.022,
                    0.16,
                    Quat::IDENTITY,
                    c(0.8, 0.62, 0.25),
                );
                k.cone(
                    v(x, 0.27, 0.0),
                    0.022,
                    0.06,
                    Quat::IDENTITY,
                    c(0.75, 0.45, 0.25),
                );
            }
        }
    }
    k
}

/// Spins a part about its own axes (kunai end over end, rotors, rings).
#[derive(Component)]
struct Spinner(Vec3);

fn spinners(time: Res<Time>, mut q: Query<(&Spinner, &mut Transform)>) {
    let dt = time.delta_secs();
    for (s, mut tf) in &mut q {
        let r = s.0 * dt;
        tf.rotate_local(Quat::from_euler(EulerRot::XYZ, r.x, r.y, r.z));
    }
}

fn tumble(time: Res<Time>, mut q: Query<&mut Transform, With<Tumble>>) {
    let dt = time.delta_secs();
    for mut tf in &mut q {
        tf.rotate(Quat::from_euler(EulerRot::XYZ, dt * 9.0, dt * 4.0, 0.0));
    }
}

/// Spawns the visible model for something the host replicates. Used by the
/// host's simulation and by clients when a new entity appears.
pub fn spawn_replicated(
    commands: &mut Commands,
    assets: &ReplicatedAssets,
    rigs: &RigAssets,
    materials: &mut Assets<StandardMaterial>,
    id: u32,
    kind: NetKind,
    pos: Vec3,
) -> Entity {
    let root = commands
        .spawn((
            InGameEntity,
            Replicated { id, kind },
            Transform::from_translation(pos),
            Visibility::default(),
        ))
        .id();
    dress(commands, assets, rigs, materials, root, kind, pos);
    root
}

/// Host side: projectiles and gadgets the simulation spawns get their models.
fn dress_new(
    mut commands: Commands,
    assets: Res<ReplicatedAssets>,
    rigs: Res<RigAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    new: Query<(Entity, &Replicated, &Transform), (Added<Replicated>, Without<Children>)>,
) {
    for (e, r, tf) in &new {
        commands.entity(e).insert(Visibility::default());
        dress(
            &mut commands,
            &assets,
            &rigs,
            &mut materials,
            e,
            r.kind,
            tf.translation,
        );
    }
}

/// Adds the model for a replicated thing to `root`.
fn dress(
    commands: &mut Commands,
    assets: &ReplicatedAssets,
    rigs: &RigAssets,
    materials: &mut Assets<StandardMaterial>,
    root: Entity,
    kind: NetKind,
    pos: Vec3,
) {
    match kind {
        NetKind::Grunt | NetKind::Shooter | NetKind::Brute => {
            let model = match kind {
                NetKind::Grunt => Model::Walker((root.index() % 3) as u8),
                NetKind::Shooter => Model::Spitter,
                _ => Model::Brute,
            };
            // Each zombie gets its own copy of the body material so hits,
            // burning and slows can tint just that one.
            let body = materials.add(crate::kit::vertex_material(0.75, 0.05));
            commands.entity(root).insert((
                Enemy,
                EnemyStatus::default(),
                EnemyLook(body.clone(), 0.0),
                Transform::from_translation(pos).with_scale(Vec3::splat(enemy_scale(kind))),
            ));
            crate::rig::spawn_rig_with(commands, rigs, root, model, None, Some(body));
        }
        NetKind::Fireball => {
            commands.entity(root).with_children(|p| {
                p.spawn((
                    Mesh3d(assets.ball.clone()),
                    MeshMaterial3d(assets.fireball.clone()),
                    Transform::from_scale(Vec3::splat(0.25)),
                ));
            });
        }
        NetKind::Grenade => {
            commands.entity(root).with_children(|p| {
                p.spawn((
                    Tumble,
                    Mesh3d(assets.grenade_mesh.clone()),
                    MeshMaterial3d(assets.grenade.clone()),
                    Transform::from_scale(Vec3::splat(1.8)),
                ))
                .with_children(|g| {
                    // Burning fuse.
                    g.spawn((
                        Mesh3d(assets.ball.clone()),
                        MeshMaterial3d(assets.spark.clone()),
                        Transform::from_xyz(0.0, 0.058, 0.0).with_scale(Vec3::splat(0.008)),
                    ));
                });
                p.spawn((
                    PointLight {
                        intensity: 8_000.0,
                        color: Color::srgb(1.0, 0.6, 0.2),
                        range: 3.0,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.15, 0.0),
                ));
            });
        }
        NetKind::Firebomb | NetKind::Turret | NetKind::Coil => {
            let i = match kind {
                NetKind::Firebomb => 0,
                NetKind::Turret => 1,
                _ => 2,
            };
            let (solid, glow) = assets.gadgets[i].clone();
            commands.entity(root).with_children(|p| {
                let scale = if i == 0 { 1.4 } else { 1.0 };
                let mut e = p.spawn((
                    Mesh3d(solid),
                    MeshMaterial3d(assets.gadget_mat.clone()),
                    Transform::from_scale(Vec3::splat(scale)),
                ));
                if i == 0 {
                    e.insert(Tumble);
                }
                e.with_child((Mesh3d(glow), MeshMaterial3d(assets.gadget_glow.clone())));
                let (color, height, power) = match i {
                    0 => (Color::srgb(1.0, 0.5, 0.15), 0.2, 6_000.0),
                    1 => (Color::srgb(0.3, 0.9, 1.0), 1.0, 4_000.0),
                    _ => (Color::srgb(0.5, 0.8, 1.0), 2.5, 60_000.0),
                };
                p.spawn((
                    PointLight {
                        intensity: power,
                        color,
                        range: if i == 2 { 9.0 } else { 3.0 },
                        ..default()
                    },
                    Transform::from_xyz(0.0, height, 0.0),
                ));
            });
        }
        NetKind::Missile(look) => {
            use crate::sim::powers::look as L;
            let Some((solid, glowing, light)) = assets.missiles.get(look as usize).cloned() else {
                return;
            };
            let spin = match look {
                L::KUNAI => Vec3::X * -22.0,
                L::CRYO => Vec3::new(1.5, 3.0, 0.0),
                L::GRAV => Vec3::new(4.0, 7.0, 0.0),
                L::ROCKET => Vec3::Z * 10.0,
                L::SMOKE | L::BOMBLET => Vec3::new(8.0, 0.0, 5.0),
                _ => Vec3::ZERO,
            };
            let (power, range) = match look {
                0..=2 => (60_000.0 * (1.0 + look as f32), 8.0),
                L::FIREBALL => (120_000.0, 9.0),
                L::CRYO => (60_000.0, 7.0),
                L::BOMBLET | L::SMOKE => (0.0, 1.0),
                _ => (15_000.0, 4.0),
            };
            commands.entity(root).with_children(|p| {
                p.spawn((
                    Spinner(spin),
                    Mesh3d(solid),
                    MeshMaterial3d(assets.gadget_mat.clone()),
                    Transform::default(),
                ))
                .with_child((Mesh3d(glowing), MeshMaterial3d(assets.gadget_glow.clone())));
                if power > 0.0 {
                    p.spawn((
                        PointLight {
                            intensity: power,
                            color: light,
                            range,
                            ..default()
                        },
                        Transform::default(),
                    ));
                }
            });
        }
        NetKind::Mine => {
            let (solid, glowing) = assets.mine.clone();
            commands.entity(root).with_children(|p| {
                p.spawn((
                    Mesh3d(solid),
                    MeshMaterial3d(assets.gadget_mat.clone()),
                    Transform::default(),
                ))
                .with_child((Mesh3d(glowing), MeshMaterial3d(assets.gadget_glow.clone())));
                p.spawn((
                    PointLight {
                        intensity: 3_000.0,
                        color: Color::srgb(1.0, 0.2, 0.1),
                        range: 2.5,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.25, 0.0),
                ));
            });
        }
        NetKind::Drone => {
            let (solid, glowing) = assets.drone.clone();
            commands.entity(root).with_children(|p| {
                p.spawn((
                    Mesh3d(solid),
                    MeshMaterial3d(assets.gadget_mat.clone()),
                    Transform::default(),
                ))
                .with_child((Mesh3d(glowing), MeshMaterial3d(assets.gadget_glow.clone())));
                for (i, at) in projectiles::rotor_spots().into_iter().enumerate() {
                    let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
                    p.spawn((
                        Spinner(Vec3::Y * 40.0 * dir),
                        Mesh3d(assets.rotor.clone()),
                        MeshMaterial3d(assets.gadget_mat.clone()),
                        Transform::from_translation(at),
                    ));
                }
                p.spawn((
                    PointLight {
                        intensity: 6_000.0,
                        color: Color::srgb(0.3, 1.0, 0.85),
                        range: 4.0,
                        ..default()
                    },
                    Transform::from_xyz(0.0, -0.2, -0.2),
                ));
            });
        }
        NetKind::PowerUp(kind) => {
            let i = PowerUp::ALL.iter().position(|p| *p == kind).unwrap_or(0);
            commands.entity(root).with_children(|p| {
                p.spawn((
                    Spin,
                    Mesh3d(assets.pickup_meshes[i].clone()),
                    MeshMaterial3d(assets.pickup_mat.clone()),
                    Transform::default(),
                ));
                p.spawn((
                    PointLight {
                        intensity: 30_000.0,
                        color: kind.color(),
                        range: 5.0,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.8, 0.0),
                ));
            });
        }
    }
}

fn enemy_colors(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut enemies: Query<(&mut EnemyStatus, &mut EnemyLook)>,
) {
    let dt = time.delta_secs();
    for (mut status, mut look) in &mut enemies {
        status.flash -= dt;
        // A hit lights them up warm-white, fading quickly (`look.1`).
        if status.flash > 0.0 {
            look.1 = 1.0;
        } else {
            look.1 = (look.1 - 7.0 * dt).max(0.0);
        }
        let (tint, glow) = if status.stunned {
            // Frozen stiff (or choking in smoke): pale and glowing.
            (
                Color::srgb(0.75, 0.9, 1.0),
                LinearRgba::rgb(0.12, 0.25, 0.45),
            )
        } else if status.burning {
            let flicker = 0.6 + 0.4 * (time.elapsed_secs() * 12.0).sin().abs();
            (
                Color::srgb(1.0, 0.75, 0.6),
                LinearRgba::rgb(0.5, 0.15, 0.02) * flicker,
            )
        } else if status.slowed {
            (
                Color::srgb(0.6, 0.85, 1.0),
                LinearRgba::rgb(0.02, 0.08, 0.18),
            )
        } else {
            (Color::WHITE, LinearRgba::BLACK)
        };
        let glow = glow + LinearRgba::rgb(0.45, 0.42, 0.40) * look.1;
        // Only touch the material when it changes (every change is re-sent
        // to the GPU).
        let same = materials
            .get(&look.0)
            .is_some_and(|m| m.base_color == tint && m.emissive == glow);
        if !same {
            if let Some(m) = materials.get_mut(&look.0) {
                m.base_color = tint;
                m.emissive = glow;
            }
        }
    }
}

fn spin(time: Res<Time>, mut q: Query<&mut Transform, With<Spin>>) {
    let t = time.elapsed_secs();
    for mut tf in &mut q {
        tf.rotation = Quat::from_rotation_y(t * 2.0) * Quat::from_rotation_x(0.4);
        tf.translation.y = (t * 3.0).sin() * 0.15;
    }
}

// ---------------------------------------------------------------------------
// Other players
// ---------------------------------------------------------------------------

#[derive(Component)]
pub(crate) struct Avatar {
    pub(crate) id: u8,
    tag: Entity,
    character: Character,
    skin: u8,
    mount: Option<Entity>,
    emote_seq: u8,
}

#[derive(Component)]
struct NameTag;

/// Builds a player's character model, holding `gun` in `skin`.
pub fn spawn_person(
    commands: &mut Commands,
    rigs: &RigAssets,
    character: Character,
    skin: u8,
    gun: u8,
    transform: Transform,
) -> (Entity, Option<Entity>) {
    let root = commands.spawn((transform, Visibility::default())).id();
    let mount = crate::rig::spawn_rig(commands, rigs, root, character, Some((gun, skin)));
    (root, mount)
}

#[allow(clippy::too_many_arguments)]
fn sync_avatars(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<Session>,
    roster: Res<Roster>,
    rigs: Res<RigAssets>,
    local: Single<&LocalPlayer>,
    mut avatars: Query<(
        Entity,
        &mut Avatar,
        &mut Transform,
        &mut Rig,
        &mut Visibility,
    )>,
    mut tags: Query<&mut Text, With<NameTag>>,
    mut mounts: Query<&mut humanoid::GunMount>,
) {
    let blend = 1.0 - (-15.0 * time.delta_secs()).exp();
    let mut have = Vec::new();

    for (entity, mut avatar, mut tf, mut rig, mut vis) in &mut avatars {
        let p = roster
            .0
            .get(&avatar.id)
            .filter(|p| p.character == avatar.character && p.skin == avatar.skin);
        let Some(p) = p else {
            commands.entity(avatar.tag).despawn();
            commands.entity(entity).despawn();
            continue;
        };
        have.push(avatar.id);
        let mine = p.id == session.my_id;
        if let Some(mut m) = avatar.mount.and_then(|e| mounts.get_mut(e).ok()) {
            let gun = p.guns[(p.active_slot as usize).min(1)].or(p.guns[0]);
            if m.want != gun {
                m.want = gun;
            }
            let skin = gun.map_or(p.skin, |g| p.skin_for(g));
            if m.skin != skin {
                m.skin = skin;
            }
            let slot = if p.guns[(p.active_slot as usize).min(1)].is_some() {
                (p.active_slot as usize).min(1)
            } else {
                0
            };
            let attach = p.attach[slot];
            if m.attach != attach {
                m.attach = attach;
            }
        }
        // Your own model only shows while the camera pulls out for an emote.
        let (feet, yaw, pitch, stance, emote, seq) = if mine {
            let l = *local;
            let e = l.emote.map_or(0, |e| e.0);
            (l.feet, l.yaw, l.pitch, l.stance(), e, l.emote_seq)
        } else {
            (p.feet(), p.yaw, p.pitch, p.stance, p.emote, p.emote_seq)
        };
        // Thousand Cuts: gone from sight while the cuts land.
        let shown = (!mine || local.cam_out > 0.05) && p.vanish <= 0.0;
        let want = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        if seq != avatar.emote_seq {
            avatar.emote_seq = seq;
            rig.emote = 0;
        }
        if p.alive {
            rig.play(emote);
        } else {
            rig.play(0);
        }
        rig.pitch = pitch;
        rig.stance = if p.alive { stance } else { 0 };
        // The model faces -Z, the same as yaw 0.
        let (target_pos, target_rot) = if p.alive {
            (feet, Quat::from_rotation_y(yaw))
        } else {
            // Downed players lie on the floor.
            (
                feet + Vec3::Y * 0.2,
                Quat::from_rotation_y(yaw) * Quat::from_rotation_x(-FRAC_PI_2),
            )
        };
        if mine || tf.translation.distance(target_pos) > 4.0 {
            tf.translation = target_pos;
        } else {
            tf.translation = tf.translation.lerp(target_pos, blend);
        }
        tf.rotation = if mine {
            target_rot
        } else {
            tf.rotation.slerp(target_rot, blend)
        };
        if let Ok(mut text) = tags.get_mut(avatar.tag) {
            let label = if mine {
                String::new()
            } else if p.alive {
                format!("{} [{}] {:.0} HP", p.name, p.level, p.health)
            } else {
                format!("{} (down)", p.name)
            };
            if text.0 != label {
                text.0 = label;
            }
        }
    }

    for p in roster.0.values() {
        if have.contains(&p.id) {
            continue;
        }
        let tag = commands
            .spawn((
                InGameEntity,
                NameTag,
                Text::new(p.name.clone()),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.9, 1.0)),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Visibility::Hidden,
            ))
            .id();
        let (root, mount) = spawn_person(
            &mut commands,
            &rigs,
            p.character,
            p.skin,
            p.guns[0].unwrap_or(0),
            Transform::from_translation(p.feet()),
        );
        commands.entity(root).insert((
            InGameEntity,
            Avatar {
                id: p.id,
                tag,
                character: p.character,
                skin: p.skin,
                mount,
                emote_seq: p.emote_seq,
            },
        ));
    }
}

/// Projects each avatar's head into screen space to position its name.
fn place_name_tags(
    camera: Single<(&Camera, &GlobalTransform), With<LocalPlayer>>,
    avatars: Query<(&Avatar, &Transform)>,
    mut tags: Query<(&mut Node, &mut Visibility, &ComputedNode), With<NameTag>>,
) {
    let (cam, cam_tf) = *camera;
    for (avatar, tf) in &avatars {
        let Ok((mut node, mut vis, computed)) = tags.get_mut(avatar.tag) else {
            continue;
        };
        let head = tf.translation + Vec3::Y * 2.25;
        let in_front = cam_tf.forward().dot(head - cam_tf.translation()) > 0.0;
        match cam.world_to_viewport(cam_tf, head) {
            Ok(screen) if in_front => {
                let size = computed.size() * computed.inverse_scale_factor();
                node.left = Val::Px(screen.x - size.x / 2.0);
                node.top = Val::Px(screen.y - size.y);
                *vis = Visibility::Inherited;
            }
            _ => *vis = Visibility::Hidden,
        }
    }
}
