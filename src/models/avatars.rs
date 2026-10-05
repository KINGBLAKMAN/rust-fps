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
                toss,
                blink,
                hover,
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
    gadget_mat: Handle<StandardMaterial>,
    gadget_glow: Handle<StandardMaterial>,
    /// Ability projectiles by look: (solid, glowing, light colour).
    missiles: Vec<(Handle<Mesh>, Handle<Mesh>, Color)>,
    drone: (Handle<Mesh>, Handle<Mesh>),
    rotor: Handle<Mesh>,
    wraith: (Handle<Mesh>, Handle<Mesh>),
    /// Blinking red lights on sticky bombs and claymores.
    blinker: Handle<StandardMaterial>,
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
        gadget_mat: materials.add(crate::kit::vertex_material(0.5, 0.3)),
        gadget_glow: materials.add(crate::kit::glow_material(3.0)),
        missiles: (0..=crate::sim::powers::look::LAST)
            .map(|l| {
                let (k, g, light) = projectiles::missile_kit(l);
                (meshes.add(k.build_or_empty()), meshes.add(g.build_or_empty()), light)
            })
            .collect(),
        drone: {
            let (k, g) = projectiles::drone_kit();
            (meshes.add(k.build_or_empty()), meshes.add(g.build_or_empty()))
        },
        rotor: meshes.add(projectiles::rotor_kit().build_or_empty()),
        wraith: {
            let (k, g) = projectiles::wraith_kit();
            (meshes.add(k.build_or_empty()), meshes.add(g.build_or_empty()))
        },
        blinker: glow(&mut materials, Color::srgb(1.0, 0.1, 0.05), 20.0),
    });
}

/// Per-enemy materials so hits, burning and slows can tint each one.
#[derive(Component)]
struct EnemyLook(Handle<StandardMaterial>, f32, Color, LinearRgba);

#[derive(Component)]
struct Spin;

/// Thrown things tumble through the air.
#[derive(Component)]
struct Tumble;

/// A thrown gadget: tumbles while flying, then lies still once it has
/// landed (or stuck to something).
#[derive(Component, Default)]
struct Toss {
    last_y: f32,
    still: f32,
    settled: bool,
}

/// Blinks a light on and off this many times a second.
#[derive(Component)]
struct Blink(f32);

/// A gentle hover bob (spectral warriors), with a phase offset.
#[derive(Component)]
struct Hover(f32);

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

/// Spins a part about its own axes (drone rotors).
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

fn toss(
    time: Res<Time>,
    roots: Query<&Transform, Without<Toss>>,
    mut q: Query<(&mut Transform, &mut Toss, &ChildOf)>,
) {
    let dt = time.delta_secs();
    for (mut tf, mut t, parent) in &mut q {
        if t.settled {
            continue;
        }
        let Ok(root) = roots.get(parent.parent()) else {
            continue;
        };
        let y = root.translation.y;
        if (y - t.last_y).abs() < 1e-4 {
            t.still += dt;
        } else {
            t.still = 0.0;
        }
        t.last_y = y;
        // Thrown things come to rest at 0.1 m.
        if y < 0.13 || t.still > 0.25 {
            t.settled = true;
            tf.rotation = Quat::IDENTITY;
        } else {
            tf.rotate(Quat::from_euler(EulerRot::XYZ, dt * 9.0, dt * 4.0, 0.0));
        }
    }
}

fn blink(time: Res<Time>, mut q: Query<(&Blink, &mut Visibility)>) {
    let t = time.elapsed_secs();
    for (b, mut vis) in &mut q {
        let on = (t * b.0).fract() < 0.35;
        let want = if on { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != want {
            *vis = want;
        }
    }
}

fn hover(time: Res<Time>, mut q: Query<(&Hover, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (h, mut tf) in &mut q {
        let a = t * 2.2 + h.0;
        tf.translation.y = 0.1 + a.sin() * 0.06;
        tf.rotation = Quat::from_rotation_z((a * 0.5).sin() * 0.04);
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
        NetKind::Grunt | NetKind::Shooter | NetKind::Brute | NetKind::Boss(_) => {
            let model = match kind {
                NetKind::Grunt => Model::Walker((root.index() % 3) as u8),
                NetKind::Shooter => Model::Spitter,
                _ => Model::Brute,
            };
            // Each zombie gets its own copy of the body material so hits,
            // burning and slows can tint just that one.
            let mut look = crate::kit::vertex_material(0.75, 0.05);
            // Bosses are huge Brutes: rust red for a map boss, bruise purple
            // with a glow for the final one.
            match kind {
                NetKind::Boss(0) => look.base_color = Color::srgb(1.0, 0.62, 0.55),
                NetKind::Boss(_) => {
                    look.base_color = Color::srgb(0.78, 0.6, 1.0);
                    look.emissive = LinearRgba::rgb(0.08, 0.02, 0.14);
                }
                _ => {}
            }
            if let NetKind::Boss(level) = kind {
                let color = if level == 0 {
                    Color::srgb(1.0, 0.35, 0.2)
                } else {
                    Color::srgb(0.7, 0.3, 1.0)
                };
                commands.entity(root).with_children(|p| {
                    p.spawn((
                        PointLight {
                            intensity: 60_000.0,
                            color,
                            range: 9.0,
                            ..default()
                        },
                        Transform::from_xyz(0.0, 1.4, 0.6),
                    ));
                });
            }
            let (base, base_glow) = (look.base_color, look.emissive);
            let body = materials.add(look);
            commands.entity(root).insert((
                Enemy,
                EnemyStatus::default(),
                EnemyLook(body.clone(), 0.0, base, base_glow),
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
        NetKind::Missile(look) => {
            use crate::sim::powers::look as L;
            let Some((solid, glowing, light)) = assets.missiles.get(look as usize).cloned() else {
                return;
            };
            // Thrown gadgets tumble until they land; the dart flies straight
            // and the claymore is set down.
            let thrown = matches!(look, L::STICKY | L::MEDKIT | L::FLASK | L::TRAP);
            let (power, range) = match look {
                L::DART => (8_000.0, 3.0),
                L::MEDKIT => (10_000.0, 3.0),
                L::FLASK => (20_000.0, 4.0),
                _ => (0.0, 1.0),
            };
            let led = match look {
                L::STICKY => Some(projectiles::STICKY_LED),
                L::CLAYMORE => Some(projectiles::CLAYMORE_LED),
                _ => None,
            };
            commands.entity(root).with_children(|p| {
                let mut e = p.spawn((
                    Mesh3d(solid),
                    MeshMaterial3d(assets.gadget_mat.clone()),
                    Transform::default(),
                ));
                if thrown {
                    e.insert(Toss::default());
                }
                e.with_children(|m| {
                    m.spawn((Mesh3d(glowing), MeshMaterial3d(assets.gadget_glow.clone())));
                    if let Some(at) = led {
                        m.spawn((
                            Blink(if look == L::STICKY { 4.0 } else { 1.5 }),
                            Mesh3d(assets.ball.clone()),
                            MeshMaterial3d(assets.blinker.clone()),
                            Transform::from_translation(at).with_scale(Vec3::splat(0.013)),
                            Visibility::default(),
                        ))
                        .with_child((
                            PointLight {
                                intensity: 2_500.0,
                                color: light,
                                range: 2.0,
                                ..default()
                            },
                            // Undo the bulb's scale.
                            Transform::from_scale(Vec3::splat(1.0 / 0.013)),
                        ));
                    }
                });
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
        NetKind::Wraith => {
            let (solid, glowing) = assets.wraith.clone();
            commands.entity(root).with_children(|p| {
                p.spawn((
                    Hover((root.index() % 7) as f32),
                    Mesh3d(solid),
                    MeshMaterial3d(assets.gadget_mat.clone()),
                    Transform::default(),
                ))
                .with_child((Mesh3d(glowing), MeshMaterial3d(assets.gadget_glow.clone())));
                p.spawn((
                    PointLight {
                        intensity: 25_000.0,
                        color: Color::srgb(0.3, 1.0, 0.8),
                        range: 5.0,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 1.3, -0.3),
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
        } else if status.marked {
            // Hunter's Mark: a pulsing red glow.
            let pulse = 0.6 + 0.4 * (time.elapsed_secs() * 6.0).sin().abs();
            (
                Color::srgb(1.0, 0.55, 0.5),
                LinearRgba::rgb(0.7, 0.04, 0.02) * pulse,
            )
        } else if status.poisoned {
            (
                Color::srgb(0.65, 0.95, 0.4),
                LinearRgba::rgb(0.12, 0.35, 0.02),
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
            (look.2, look.3)
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
