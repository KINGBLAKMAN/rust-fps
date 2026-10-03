//! Visual effects: bullet tracers, explosions (fireball, shockwave, smoke,
//! debris, sparks, scorch mark), Heal Pulse (green ring, light column and
//! rising crosses), Frost Nova (ice spikes bursting from the ground, frost and
//! mist), the Orbital Strike (target marker, sky laser, beam and blast), Dash
//! afterimages and lightning arcs. The host broadcasts effects so every
//! player sees them.

use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::f32::consts::{FRAC_PI_2, TAU};

use crate::kit::{c, vertex_material, Kit};
use crate::{AppState, InGameEntity, Phase};

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FxQueue>()
            .init_resource::<FxOutbox>()
            .init_resource::<Lines>()
            .add_systems(
                Update,
                zone_fx.in_set(Phase::Present).run_if(in_state(AppState::InGame)),
            )
            .add_systems(PreStartup, setup)
            .add_systems(
                Update,
                (play, animate, particles, markers, draw_lines)
                    .chain()
                    .in_set(Phase::Present)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnExit(AppState::InGame), clear);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Fx {
    /// Bullet tracer from a player (`shooter`) so they can skip their own.
    Tracer { shooter: u8, a: [f32; 3], b: [f32; 3] },
    Explosion { pos: [f32; 3], radius: f32, color: [f32; 3] },
    Ring { pos: [f32; 3], radius: f32, color: [f32; 3] },
    Lightning { a: [f32; 3], b: [f32; 3] },
    Beam { pos: [f32; 3], radius: f32, delay: f32 },
    Heal { pos: [f32; 3], radius: f32 },
    Nova { pos: [f32; 3], radius: f32 },
    /// A player dashed from `a` to `b` (skipped by the dasher, who plays it
    /// locally straight away).
    Dash { player: u8, a: [f32; 3], b: [f32; 3] },
    /// A sword cut across an arc in front of `pos`.
    Slash { pos: [f32; 3], dir: [f32; 3], radius: f32 },
    /// A cone of flame.
    Cone { pos: [f32; 3], dir: [f32; 3], range: f32 },
    /// A lasting area effect: 0 blade storm, 1 tesla field, 2 fire pool,
    /// 3 inferno. `follow` is a player id (255 stays put).
    Zone { pos: [f32; 3], radius: f32, life: f32, follow: u8, kind: u8 },
    /// A player pinged a spot or an enemy (see pings.rs).
    Ping { player: u8, pos: [f32; 3], target: u32 },
}

/// Effects to show on this machine this frame.
#[derive(Resource, Default)]
pub struct FxQueue(pub Vec<Fx>);

/// Host only: effects to send to clients with the next snapshot.
#[derive(Resource, Default)]
pub struct FxOutbox(pub Vec<Fx>);

/// Short-lived lines: (start, end, color, time left).
#[derive(Resource, Default)]
pub struct Lines(Vec<(Vec3, Vec3, Color, f32)>);

#[derive(Resource)]
pub struct FxAssets {
    ball: Handle<Mesh>,
    disc: Handle<Mesh>,
    beam: Handle<Mesh>,
    ring: Handle<Mesh>,
    cube: Handle<Mesh>,
    spike: Handle<Mesh>,
    cross: Handle<Mesh>,
    marker: Handle<Mesh>,
    ghost: Handle<Mesh>,
    fire: Handle<StandardMaterial>,
    smoke: Handle<StandardMaterial>,
    debris: Handle<StandardMaterial>,
    spark: Handle<StandardMaterial>,
    scorch: Handle<StandardMaterial>,
    ice: Handle<StandardMaterial>,
    frost: Handle<StandardMaterial>,
    mist: Handle<StandardMaterial>,
    heal: Handle<StandardMaterial>,
    heal_soft: Handle<StandardMaterial>,
    beam_core: Handle<StandardMaterial>,
    beam_outer: Handle<StandardMaterial>,
    marker_mat: Handle<StandardMaterial>,
    ghost_mat: Handle<StandardMaterial>,
    blade: Handle<Mesh>,
    blade_mat: Handle<StandardMaterial>,
    tesla_mat: Handle<StandardMaterial>,
}

/// A lasting area effect on screen (see `Fx::Zone`).
#[derive(Component)]
struct ZoneFx {
    life: f32,
    max: f32,
    radius: f32,
    follow: Option<u8>,
    kind: u8,
    emit: f32,
}

/// Spinning blades of the Blade Storm.
#[derive(Component)]
struct StormBlade(Entity, f32);

/// How a one-shot shape changes over its life.
#[derive(Clone, Copy)]
enum Grow {
    /// Sphere growing to this radius.
    Ball(f32),
    /// Flat shape (ring, disc) spreading to this radius.
    Flat(f32),
    /// Spreads quickly to this radius, holds, then shrinks away (frost).
    Sheet(f32),
    /// Keeps its size, shrinks away at the end (scorch marks).
    Hold,
    /// A tall column that thins out.
    Column,
    /// A light that dims.
    Light,
    /// A beam that hits at full width and narrows.
    Column2(f32),
}

/// A growing, fading one-shot shape.
#[derive(Component)]
struct Burst {
    life: f32,
    max: f32,
    grow: Grow,
}

/// A moving particle that changes size over its life.
#[derive(Component)]
struct Particle {
    vel: Vec3,
    life: f32,
    max: f32,
    gravity: f32,
    drag: f32,
    size: (f32, f32),
    /// Fraction of the life spent growing in at the start.
    pop: f32,
    spin: Vec3,
    /// Stays put once it hits the ground.
    lands: bool,
}

/// Orbital strike marker counting down to the blast.
#[derive(Component)]
struct Pending {
    delay: f32,
    pos: Vec3,
    radius: f32,
}

/// An ice spike: shoots up out of the ground, then sinks back.
#[derive(Component)]
struct Spike {
    life: f32,
    max: f32,
    scale: Vec3,
    base: Vec3,
}

fn spike_kit() -> Kit {
    let mut k = Kit::new();
    let ice = c(0.75, 0.92, 1.0);
    k.cone(Vec3::new(0.0, 0.5, 0.0), 0.18, 1.0, Quat::IDENTITY, ice);
    k.cone(Vec3::new(0.08, 0.3, 0.05), 0.1, 0.6, Quat::from_rotation_z(-0.35), c(0.6, 0.85, 1.0));
    k.cone(Vec3::new(-0.07, 0.25, -0.05), 0.08, 0.5, Quat::from_rotation_x(0.4), c(0.85, 0.96, 1.0));
    k
}

fn cross_kit() -> Kit {
    let mut k = Kit::new();
    let g = c(0.4, 1.0, 0.55);
    k.cuboid(Vec3::ZERO, Vec3::new(0.3, 0.1, 0.1), g);
    k.cuboid(Vec3::ZERO, Vec3::new(0.1, 0.3, 0.1), g);
    k
}

/// Orbital target: a ring of segments with corner ticks and a centre dot.
fn marker_kit() -> Kit {
    let mut k = Kit::new();
    let red = c(1.0, 0.2, 0.15);
    for i in 0..12 {
        let a = i as f32 / 12.0 * TAU;
        k.cuboid_rot(
            Vec3::new(a.cos(), 0.0, a.sin()),
            Vec3::new(0.06, 0.02, 0.38),
            Quat::from_rotation_y(-a),
            red,
        );
    }
    for i in 0..4 {
        let a = i as f32 / 4.0 * TAU + 0.785;
        k.cuboid_rot(
            Vec3::new(a.cos() * 0.75, 0.0, a.sin() * 0.75),
            Vec3::new(0.3, 0.02, 0.05),
            Quat::from_rotation_y(-a),
            red,
        );
    }
    k.cyl(Vec3::ZERO, 0.08, 0.02, Quat::IDENTITY, red);
    k
}

fn unlit(materials: &mut Assets<StandardMaterial>, color: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: if color.alpha() < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
        ..default()
    })
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let assets = FxAssets {
        ball: meshes.add(Sphere::new(1.0).mesh().ico(2).unwrap()),
        disc: meshes.add(Cylinder::new(1.0, 0.04).mesh().resolution(32)),
        beam: meshes.add(Cylinder::new(1.0, 60.0).mesh().resolution(20)),
        ring: meshes.add(Torus::new(0.94, 1.0).mesh().major_resolution(40).minor_resolution(6)),
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        spike: meshes.add(spike_kit().build_or_empty()),
        cross: meshes.add(cross_kit().build_or_empty()),
        marker: meshes.add(marker_kit().build_or_empty()),
        ghost: meshes.add(Capsule3d::new(0.35, 1.1)),
        fire: unlit(&mut materials, Color::srgb(1.0, 0.6, 0.18)),
        smoke: materials.add(StandardMaterial {
            base_color: Color::srgba(0.16, 0.15, 0.15, 0.55),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 1.0,
            ..default()
        }),
        debris: materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.1, 0.09),
            perceptual_roughness: 0.9,
            ..default()
        }),
        spark: unlit(&mut materials, Color::srgb(1.0, 0.85, 0.4)),
        scorch: materials.add(StandardMaterial {
            base_color: Color::srgba(0.02, 0.02, 0.02, 0.75),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 1.0,
            ..default()
        }),
        ice: materials.add(StandardMaterial {
            emissive: LinearRgba::rgb(0.1, 0.3, 0.5),
            perceptual_roughness: 0.15,
            ..vertex_material(0.15, 0.0)
        }),
        frost: materials.add(StandardMaterial {
            base_color: Color::srgba(0.8, 0.93, 1.0, 0.55),
            alpha_mode: AlphaMode::Blend,
            emissive: LinearRgba::rgb(0.1, 0.2, 0.3),
            perceptual_roughness: 0.2,
            ..default()
        }),
        mist: unlit(&mut materials, Color::srgba(0.85, 0.95, 1.0, 0.35)),
        heal: unlit(&mut materials, Color::srgb(0.35, 1.0, 0.5)),
        heal_soft: unlit(&mut materials, Color::srgba(0.35, 1.0, 0.5, 0.25)),
        beam_core: unlit(&mut materials, Color::srgb(1.0, 0.95, 0.85)),
        beam_outer: unlit(&mut materials, Color::srgba(1.0, 0.45, 0.15, 0.4)),
        marker_mat: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            ..default()
        }),
        ghost_mat: unlit(&mut materials, Color::srgba(0.4, 0.75, 1.0, 0.3)),
        blade: meshes.add({
            let mut k = Kit::new();
            k.cuboid(Vec3::new(0.0, 0.0, -0.45), Vec3::new(0.02, 0.09, 0.9), c(1.0, 0.85, 0.5));
            k.wedge(Vec3::new(0.0, 0.0, -0.95), Vec3::new(0.09, 0.12, 0.02), Quat::from_rotation_x(-FRAC_PI_2) * Quat::from_rotation_z(FRAC_PI_2), c(1.0, 0.9, 0.6));
            k.cuboid(Vec3::new(0.0, 0.0, 0.02), Vec3::new(0.04, 0.16, 0.04), c(0.9, 0.6, 0.2));
            k.build_or_empty()
        }),
        blade_mat: glow_material(&mut materials, [1.0, 0.8, 0.35], 0.9),
        tesla_mat: glow_material(&mut materials, [0.45, 0.8, 1.0], 0.25),
    };
    commands.insert_resource(assets);
}

pub fn rgb(c: Color) -> [f32; 3] {
    let s = c.to_srgba();
    [s.red, s.green, s.blue]
}

fn glow_material(materials: &mut Assets<StandardMaterial>, color: [f32; 3], alpha: f32) -> Handle<StandardMaterial> {
    let c = Color::srgba(color[0], color[1], color[2], alpha);
    materials.add(StandardMaterial {
        base_color: c,
        emissive: LinearRgba::rgb(color[0], color[1], color[2]) * 6.0,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    })
}

fn rand_dir(rng: &mut impl Rng) -> Vec3 {
    let a = rng.gen_range(0.0..TAU);
    let z: f32 = rng.gen_range(-1.0..1.0);
    let r = (1.0 - z * z).sqrt();
    Vec3::new(r * a.cos(), z, r * a.sin())
}

fn particle(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    mat: &Handle<StandardMaterial>,
    pos: Vec3,
    p: Particle,
) {
    commands.spawn((
        InGameEntity,
        Mesh3d(mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(pos).with_scale(Vec3::splat(p.size.0.max(0.001))),
        NotShadowCaster,
        p,
    ));
}

fn burst(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    mat: Handle<StandardMaterial>,
    tf: Transform,
    life: f32,
    grow: Grow,
) {
    commands.spawn((
        InGameEntity,
        Burst {
            life,
            max: life,
            grow,
        },
        Mesh3d(mesh.clone()),
        MeshMaterial3d(mat),
        tf,
        NotShadowCaster,
    ));
}

fn flash_light(commands: &mut Commands, pos: Vec3, color: Color, intensity: f32, range: f32, life: f32) {
    commands.spawn((
        InGameEntity,
        Burst {
            life,
            max: life,
            grow: Grow::Light,
        },
        PointLight {
            intensity,
            color,
            range,
            ..default()
        },
        Transform::from_translation(pos),
    ));
}

/// Fireball, shockwave, smoke, debris, sparks and a scorch mark.
fn explosion(
    commands: &mut Commands,
    a: &FxAssets,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    radius: f32,
    color: [f32; 3],
) {
    let mut rng = rand::thread_rng();
    let ground = pos.with_y(0.0);
    let col = Color::srgb(color[0], color[1], color[2]);
    burst(
        commands,
        &a.ball,
        glow_material(materials, color, 0.75),
        Transform::from_translation(pos).with_scale(Vec3::splat(0.2)),
        0.4,
        Grow::Ball(radius * 0.8),
    );
    burst(
        commands,
        &a.ring,
        glow_material(materials, color, 0.6),
        Transform::from_translation(ground + Vec3::Y * 0.15).with_scale(Vec3::new(0.3, 3.0, 0.3)),
        0.35,
        Grow::Flat(radius * 1.25),
    );
    flash_light(commands, pos + Vec3::Y, col, 500_000.0, radius * 4.0, 0.3);
    let n = (radius * 2.0) as i32 + 4;
    for _ in 0..n {
        let d = rand_dir(&mut rng);
        let d = Vec3::new(d.x, d.y.abs() * 0.8 + 0.2, d.z);
        particle(commands, &a.ball, &a.fire, pos + d * 0.3, Particle {
            vel: d * rng.gen_range(2.0..5.0) * radius / 4.0,
            life: rng.gen_range(0.35..0.6),
            max: 0.6,
            gravity: -1.0,
            drag: 3.0,
            size: (radius * 0.18, radius * 0.32),
            pop: 0.1,
            spin: Vec3::ZERO,
            lands: false,
        });
    }
    for _ in 0..n + 4 {
        let d = rand_dir(&mut rng);
        let start = pos + Vec3::new(d.x, d.y.abs(), d.z) * radius * 0.3;
        particle(commands, &a.ball, &a.smoke, start, Particle {
            vel: Vec3::new(d.x * 1.5, rng.gen_range(1.0..2.8), d.z * 1.5),
            life: rng.gen_range(1.4..2.4),
            max: 2.4,
            gravity: -0.3,
            drag: 1.2,
            size: (radius * 0.15, radius * rng.gen_range(0.35..0.55)),
            pop: 0.15,
            spin: Vec3::ZERO,
            lands: false,
        });
    }
    for _ in 0..12 {
        let d = rand_dir(&mut rng);
        let v = Vec3::new(d.x * 6.0, rng.gen_range(5.0..11.0), d.z * 6.0);
        let s = rng.gen_range(0.07..0.16);
        particle(commands, &a.cube, &a.debris, pos, Particle {
            vel: v,
            life: rng.gen_range(1.2..2.0),
            max: 2.0,
            gravity: 18.0,
            drag: 0.2,
            size: (s, s),
            pop: 0.0,
            spin: rand_dir(&mut rng) * 10.0,
            lands: true,
        });
    }
    for _ in 0..14 {
        let d = rand_dir(&mut rng);
        particle(commands, &a.cube, &a.spark, pos, Particle {
            vel: Vec3::new(d.x, d.y.abs(), d.z) * rng.gen_range(8.0..15.0),
            life: rng.gen_range(0.25..0.5),
            max: 0.5,
            gravity: 12.0,
            drag: 1.0,
            size: (0.05, 0.02),
            pop: 0.0,
            spin: Vec3::ZERO,
            lands: false,
        });
    }
    // Scorch mark, gone after a while.
    burst(
        commands,
        &a.disc,
        a.scorch.clone(),
        Transform::from_translation(ground + Vec3::Y * 0.02)
            .with_scale(Vec3::new(radius * 0.55, 1.0, radius * 0.55)),
        7.0,
        Grow::Hold,
    );
}

pub fn play(
    mut commands: Commands,
    mut queue: ResMut<FxQueue>,
    mut lines: ResMut<Lines>,
    assets: Res<FxAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();
    let a = &*assets;
    for fx in queue.0.drain(..) {
        match fx {
            Fx::Tracer { a, b, .. } => {
                lines.0.push((
                    Vec3::from_array(a),
                    Vec3::from_array(b),
                    Color::srgb(1.0, 0.9, 0.5),
                    0.06,
                ));
            }
            Fx::Explosion { pos, radius, color } => {
                explosion(&mut commands, a, &mut materials, Vec3::from_array(pos), radius, color);
            }
            Fx::Ring { pos, radius, color } => {
                burst(
                    &mut commands,
                    &a.ring,
                    glow_material(&mut materials, color, 0.6),
                    Transform::from_translation(Vec3::from_array(pos) + Vec3::Y * 0.1)
                        .with_scale(Vec3::new(0.3, 2.0, 0.3)),
                    0.6,
                    Grow::Flat(radius),
                );
            }
            Fx::Heal { pos, radius } => {
                let p = Vec3::from_array(pos);
                burst(
                    &mut commands,
                    &a.ring,
                    a.heal.clone(),
                    Transform::from_translation(p + Vec3::Y * 0.12).with_scale(Vec3::new(0.3, 3.0, 0.3)),
                    0.7,
                    Grow::Flat(radius),
                );
                burst(
                    &mut commands,
                    &a.disc,
                    a.heal_soft.clone(),
                    Transform::from_translation(p + Vec3::Y * 0.05).with_scale(Vec3::new(0.3, 1.0, 0.3)),
                    0.8,
                    Grow::Flat(radius),
                );
                // Light column.
                commands.spawn((
                    InGameEntity,
                    Burst {
                        life: 0.9,
                        max: 0.9,
                        grow: Grow::Column,
                    },
                    Mesh3d(a.beam.clone()),
                    MeshMaterial3d(a.heal_soft.clone()),
                    Transform::from_translation(p + Vec3::Y * 30.0).with_scale(Vec3::new(0.9, 1.0, 0.9)),
                    NotShadowCaster,
                ));
                flash_light(&mut commands, p + Vec3::Y * 1.5, Color::srgb(0.4, 1.0, 0.5), 300_000.0, radius * 2.0, 0.8);
                for _ in 0..18 {
                    let ang = rng.gen_range(0.0..TAU);
                    let r = radius * rng.gen_range(0.0f32..1.0).sqrt();
                    let start = p + Vec3::new(ang.cos() * r, rng.gen_range(0.2..1.2), ang.sin() * r);
                    particle(&mut commands, &a.cross, &a.heal, start, Particle {
                        vel: Vec3::Y * rng.gen_range(1.2..2.6),
                        life: rng.gen_range(0.9..1.5),
                        max: 1.5,
                        gravity: 0.0,
                        drag: 0.5,
                        size: (0.0, rng.gen_range(0.8..1.3)),
                        pop: 0.2,
                        spin: Vec3::Y * 2.0,
                        lands: false,
                    });
                }
            }
            Fx::Nova { pos, radius } => {
                let p = Vec3::from_array(pos);
                // Frost sheet and an expanding cold ring.
                burst(
                    &mut commands,
                    &a.disc,
                    a.frost.clone(),
                    Transform::from_translation(p + Vec3::Y * 0.03).with_scale(Vec3::new(0.5, 1.0, 0.5)),
                    2.2,
                    Grow::Sheet(radius),
                );
                burst(
                    &mut commands,
                    &a.ring,
                    a.mist.clone(),
                    Transform::from_translation(p + Vec3::Y * 0.2).with_scale(Vec3::new(0.3, 4.0, 0.3)),
                    0.5,
                    Grow::Flat(radius * 1.1),
                );
                flash_light(&mut commands, p + Vec3::Y * 1.0, Color::srgb(0.5, 0.85, 1.0), 300_000.0, radius * 2.0, 0.6);
                // Spikes burst out of the ground in two rings.
                for ring in 0..2 {
                    let count = if ring == 0 { 9 } else { 16 };
                    let rr = radius * if ring == 0 { 0.4 } else { 0.85 };
                    for i in 0..count {
                        let ang = i as f32 / count as f32 * TAU + rng.gen_range(-0.15..0.15);
                        let base = p + Vec3::new(ang.cos() * rr, 0.0, ang.sin() * rr);
                        let out = Vec3::new(ang.cos(), 0.0, ang.sin());
                        let tilt = Quat::from_rotation_arc(Vec3::Y, (Vec3::Y * 2.0 + out * 0.8).normalize());
                        let h = rng.gen_range(0.9..1.8) * if ring == 0 { 0.8 } else { 1.0 };
                        let w = rng.gen_range(0.8..1.3);
                        let delay = ring as f32 * 0.08 + rng.gen_range(0.0..0.05);
                        commands.spawn((
                            InGameEntity,
                            Spike {
                                life: 1.6 + delay,
                                max: 1.6 + delay,
                                scale: Vec3::new(w, h, w),
                                base,
                            },
                            Mesh3d(a.spike.clone()),
                            MeshMaterial3d(a.ice.clone()),
                            Transform::from_translation(base)
                                .with_rotation(tilt * Quat::from_rotation_y(rng.gen_range(0.0..TAU)))
                                .with_scale(Vec3::ZERO),
                        ));
                    }
                }
                for _ in 0..16 {
                    let d = rand_dir(&mut rng);
                    let d = Vec3::new(d.x, 0.15, d.z).normalize();
                    particle(&mut commands, &a.ball, &a.mist, p + Vec3::Y * 0.6, Particle {
                        vel: d * rng.gen_range(5.0..9.0),
                        life: rng.gen_range(0.6..1.0),
                        max: 1.0,
                        gravity: 0.0,
                        drag: 3.0,
                        size: (0.3, 1.0),
                        pop: 0.1,
                        spin: Vec3::ZERO,
                        lands: false,
                    });
                }
            }
            Fx::Lightning { a, b } => {
                let (a, b) = (Vec3::from_array(a), Vec3::from_array(b));
                let mut prev = a;
                for i in 1..=6 {
                    let t = i as f32 / 6.0;
                    let mut p = a.lerp(b, t);
                    if i < 6 {
                        p += Vec3::new(
                            rng.gen_range(-0.4..0.4),
                            rng.gen_range(-0.4..0.4),
                            rng.gen_range(-0.4..0.4),
                        );
                    }
                    lines.0.push((prev, p, Color::srgb(0.8, 0.85, 1.0), 0.15));
                    prev = p;
                }
            }
            Fx::Beam { pos, radius, delay } => {
                let p = Vec3::from_array(pos);
                commands.spawn((
                    InGameEntity,
                    Pending {
                        delay,
                        pos: p,
                        radius,
                    },
                    Mesh3d(a.marker.clone()),
                    MeshMaterial3d(a.marker_mat.clone()),
                    Transform::from_translation(p + Vec3::Y * 0.08).with_scale(Vec3::splat(radius)),
                    NotShadowCaster,
                ));
            }
            Fx::Slash { pos, dir, radius } => {
                let (p, d) = (Vec3::from_array(pos), Vec3::from_array(dir));
                let a0 = d.z.atan2(d.x);
                for k in 0..4 {
                    let h = -0.5 + k as f32 * 0.35;
                    let r = radius * (0.55 + 0.15 * k as f32);
                    let mut prev = None;
                    for i in 0..=18 {
                        let ang = a0 - 1.1 + 2.2 * i as f32 / 18.0;
                        let q = p + Vec3::new(ang.cos() * r, h + 0.25 * (i as f32 / 18.0 - 0.5), ang.sin() * r);
                        if let Some(pr) = prev {
                            lines.0.push((pr, q, Color::srgb(1.0, 0.92 - 0.1 * k as f32, 0.6), 0.18 + k as f32 * 0.03));
                        }
                        prev = Some(q);
                    }
                }
                for _ in 0..20 {
                    let ang = a0 + rng.gen_range(-1.1..1.1);
                    let out = Vec3::new(ang.cos(), 0.0, ang.sin());
                    particle(&mut commands, &a.ball, &a.spark, p + out * radius * rng.gen_range(0.3..0.9), Particle {
                        vel: out * rng.gen_range(2.0..6.0) + Vec3::Y * rng.gen_range(0.0..2.0),
                        life: rng.gen_range(0.2..0.45),
                        max: 0.45,
                        gravity: 6.0,
                        drag: 2.0,
                        size: (0.05, 0.0),
                        pop: 0.0,
                        spin: Vec3::ZERO,
                        lands: false,
                    });
                }
                flash_light(&mut commands, p, Color::srgb(1.0, 0.85, 0.5), 80_000.0, radius * 1.5, 0.25);
            }
            Fx::Cone { pos, dir, range } => {
                let (p, d) = (Vec3::from_array(pos), Vec3::from_array(dir).normalize_or(Vec3::NEG_Z));
                for _ in 0..55 {
                    let spread = rand_dir(&mut rng) * rng.gen_range(0.0..0.35);
                    let v = (d + spread).normalize() * range * rng.gen_range(1.6..2.4);
                    particle(&mut commands, &a.ball, &a.fire, p + spread * 0.2, Particle {
                        vel: v,
                        life: rng.gen_range(0.35..0.55),
                        max: 0.55,
                        gravity: -2.0,
                        drag: 1.2,
                        size: (0.08, rng.gen_range(0.5..0.9)),
                        pop: 0.05,
                        spin: Vec3::ZERO,
                        lands: false,
                    });
                }
                for _ in 0..12 {
                    let spread = rand_dir(&mut rng) * 0.3;
                    particle(&mut commands, &a.ball, &a.smoke, p + d * range * 0.6, Particle {
                        vel: (d + spread) * 3.0 + Vec3::Y * 1.5,
                        life: rng.gen_range(0.6..1.1),
                        max: 1.1,
                        gravity: -1.0,
                        drag: 1.0,
                        size: (0.3, 1.0),
                        pop: 0.2,
                        spin: Vec3::ZERO,
                        lands: false,
                    });
                }
                flash_light(&mut commands, p + d * 2.0, Color::srgb(1.0, 0.5, 0.15), 250_000.0, range * 1.5, 0.4);
            }
            Fx::Zone { pos, radius, life, follow, kind } => {
                let p = Vec3::from_array(pos);
                let zone = commands
                    .spawn((
                        InGameEntity,
                        ZoneFx {
                            life,
                            max: life,
                            radius,
                            follow: (follow != 255).then_some(follow),
                            kind,
                            emit: 0.0,
                        },
                        Transform::from_translation(p),
                        Visibility::default(),
                    ))
                    .id();
                let (color, light) = match kind {
                    0 => ([1.0, 0.8, 0.35], Color::srgb(1.0, 0.8, 0.4)),
                    1 => ([0.45, 0.8, 1.0], Color::srgb(0.5, 0.8, 1.0)),
                    _ => ([1.0, 0.45, 0.1], Color::srgb(1.0, 0.5, 0.15)),
                };
                commands.entity(zone).with_children(|z| {
                    z.spawn((
                        Mesh3d(a.ring.clone()),
                        MeshMaterial3d(glow_material(&mut materials, color, 0.55)),
                        Transform::from_xyz(0.0, 0.08, 0.0).with_scale(Vec3::new(radius, 2.0, radius)),
                        NotShadowCaster,
                    ));
                    let floor = if kind == 1 { a.tesla_mat.clone() } else if kind == 0 { glow_material(&mut materials, color, 0.12) } else { a.scorch.clone() };
                    z.spawn((
                        Mesh3d(a.disc.clone()),
                        MeshMaterial3d(floor),
                        Transform::from_xyz(0.0, 0.03, 0.0).with_scale(Vec3::new(radius, 1.0, radius)),
                        NotShadowCaster,
                    ));
                    z.spawn((
                        PointLight {
                            intensity: 120_000.0,
                            color: light,
                            range: radius * 2.0,
                            ..default()
                        },
                        Transform::from_xyz(0.0, 1.5, 0.0),
                    ));
                    if kind == 0 {
                        for i in 0..6 {
                            z.spawn((
                                StormBlade(zone, i as f32 / 6.0 * TAU),
                                Mesh3d(a.blade.clone()),
                                MeshMaterial3d(a.blade_mat.clone()),
                                Transform::default(),
                                NotShadowCaster,
                            ));
                        }
                    }
                });
            }
            Fx::Ping { .. } => {}
            Fx::Dash { a: from, b: to, .. } => {
                let (from, to) = (Vec3::from_array(from), Vec3::from_array(to));
                for i in 0..5 {
                    let t = i as f32 / 5.0;
                    let pos = from.lerp(to, t) + Vec3::Y * 0.9;
                    particle(&mut commands, &a.ghost, &a.ghost_mat, pos, Particle {
                        vel: Vec3::ZERO,
                        life: 0.2 + t * 0.25,
                        max: 0.45,
                        gravity: 0.0,
                        drag: 0.0,
                        size: (1.0, 0.6),
                        pop: 0.0,
                        spin: Vec3::ZERO,
                        lands: false,
                    });
                }
                for _ in 0..10 {
                    let off = Vec3::new(rng.gen_range(-0.5..0.5), rng.gen_range(0.2..1.7), rng.gen_range(-0.5..0.5));
                    lines.0.push((from + off, to + off * 0.6, Color::srgba(0.6, 0.85, 1.0, 0.8), 0.18));
                }
            }
        }
    }
}

fn animate(
    mut commands: Commands,
    time: Res<Time>,
    mut bursts: Query<(Entity, &mut Burst, &mut Transform, Option<&mut PointLight>), Without<Pending>>,
    mut spikes: Query<(Entity, &mut Spike, &mut Transform), (Without<Burst>, Without<Pending>)>,
) {
    let dt = time.delta_secs();
    for (e, mut b, mut tf, light) in &mut bursts {
        b.life -= dt;
        if b.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let t = 1.0 - b.life / b.max;
        let fade = 1.0 - ((t - 0.75) / 0.25).clamp(0.0, 1.0);
        let flat = |tf: &mut Transform, r: f32| {
            let r = r.max(0.001);
            tf.scale = Vec3::new(r, tf.scale.y, r);
        };
        match b.grow {
            Grow::Light => {
                if let Some(mut light) = light {
                    light.intensity *= 0.85;
                }
            }
            Grow::Ball(r) => tf.scale = Vec3::splat(r * (0.3 + 0.7 * t.sqrt())),
            Grow::Flat(r) => flat(&mut tf, r * t.max(0.05)),
            Grow::Sheet(r) => flat(&mut tf, r * (t * 8.0).min(1.0) * fade),
            Grow::Hold => {
                let r = tf.scale.x;
                if t > 0.75 {
                    flat(&mut tf, r * (1.0 - dt * 4.0));
                }
            }
            Grow::Column => {
                let w = (0.9 * (1.0 - t)).max(0.001);
                tf.scale = Vec3::new(w, 1.0, w);
            }
            Grow::Column2(r) => {
                // Slams in at full width, then narrows away.
                let w = (r * (1.0 - t * t)).max(0.001);
                tf.scale = Vec3::new(w, 1.0, w);
            }
        }
    }
    for (e, mut s, mut tf) in &mut spikes {
        s.life -= dt;
        if s.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let age = s.max - s.life;
        let grow = (age / 0.12).clamp(0.0, 1.0);
        let sink = (s.life / 0.4).clamp(0.0, 1.0);
        let k = grow.min(sink);
        tf.scale = s.scale * k.max(0.001);
        tf.translation = s.base - Vec3::Y * (1.0 - sink) * 0.5;
    }
}

fn particles(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Particle, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (e, mut p, mut tf) in &mut q {
        p.life -= dt;
        if p.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let g = p.gravity;
        p.vel.y -= g * dt;
        let drag = (1.0 - p.drag * dt).max(0.0);
        p.vel *= drag;
        tf.translation += p.vel * dt;
        if p.lands && tf.translation.y < 0.05 {
            tf.translation.y = 0.05;
            p.vel = Vec3::ZERO;
            p.spin = Vec3::ZERO;
        }
        if p.spin != Vec3::ZERO {
            let s = p.spin * dt;
            tf.rotate(Quat::from_euler(EulerRot::XYZ, s.x, s.y, s.z));
        }
        let t = 1.0 - p.life / p.max;
        let size = p.size.0 + (p.size.1 - p.size.0) * t.min(1.0);
        let pop = if p.pop > 0.0 { (t / p.pop).min(1.0) } else { 1.0 };
        let fade = (p.life / (p.max * 0.3)).min(1.0);
        let s = (size * pop * fade).max(0.001);
        if p.size == (0.05, 0.02) {
            // Sparks stretch along their motion.
            let dir = p.vel.normalize_or_zero();
            if dir != Vec3::ZERO {
                tf.rotation = Quat::from_rotation_arc(Vec3::Z, dir);
            }
            tf.scale = Vec3::new(0.025, 0.025, 0.12 + p.vel.length() * 0.02) * fade;
        } else {
            tf.scale = Vec3::splat(s);
        }
    }
}

/// Orbital strike: the marker spins and tightens, a thin laser points down
/// from the sky, then the beam and blast hit.
fn markers(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<FxAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pending: Query<(Entity, &mut Pending, &mut Transform), Without<Burst>>,
    mut lines: ResMut<Lines>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let a = &*assets;
    for (e, mut p, mut tf) in &mut pending {
        p.delay -= dt;
        let pulse = 1.0 + 0.06 * (t * 14.0).sin();
        tf.rotation = Quat::from_rotation_y(t * 2.5);
        tf.scale = Vec3::new(p.radius * pulse, 1.0, p.radius * pulse);
        lines.0.push((p.pos + Vec3::Y * 0.1, p.pos + Vec3::Y * 60.0, Color::srgb(1.0, 0.25, 0.2), 0.0));
        if p.delay > 0.0 {
            continue;
        }
        commands.entity(e).despawn();
        // Outer glow and white-hot core.
        burst(
            &mut commands,
            &a.beam,
            a.beam_outer.clone(),
            Transform::from_translation(p.pos + Vec3::Y * 30.0),
            0.8,
            Grow::Column2(p.radius * 0.45),
        );
        burst(
            &mut commands,
            &a.beam,
            a.beam_core.clone(),
            Transform::from_translation(p.pos + Vec3::Y * 30.0),
            0.6,
            Grow::Column2(p.radius * 0.18),
        );
        explosion(&mut commands, a, &mut materials, p.pos + Vec3::Y * 0.5, p.radius, [1.0, 0.5, 0.15]);
    }
}

/// Moves lasting zones with their player, spins the blades and keeps
/// flames and sparks coming.
fn zone_fx(
    mut commands: Commands,
    time: Res<Time>,
    roster: Res<crate::Roster>,
    assets: Res<FxAssets>,
    mut lines: ResMut<Lines>,
    mut zones: Query<(Entity, &mut ZoneFx, &mut Transform), Without<StormBlade>>,
    mut blades: Query<(&StormBlade, &mut Transform), Without<ZoneFx>>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let a = &*assets;
    let mut rng = rand::thread_rng();
    let mut sizes = Vec::new();
    for (e, mut z, mut tf) in &mut zones {
        z.life -= dt;
        if z.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        if let Some(p) = z.follow.and_then(|id| roster.0.get(&id)) {
            tf.translation = p.feet();
        }
        let fade = (z.life / 0.4).min(1.0).min((z.max - z.life) / 0.25 + 0.2).min(1.0);
        tf.scale = Vec3::new(fade, 1.0, fade);
        sizes.push((e, z.radius));
        z.emit -= dt;
        if z.emit > 0.0 {
            continue;
        }
        let p = tf.translation;
        match z.kind {
            0 => {
                z.emit = 0.05;
                let ang = rng.gen_range(0.0..TAU);
                let r = z.radius * rng.gen_range(0.3..1.0);
                let q = p + Vec3::new(ang.cos() * r, rng.gen_range(0.4..1.6), ang.sin() * r);
                let tan = Vec3::new(-ang.sin(), 0.0, ang.cos());
                lines.0.push((q, q + tan * 1.2, Color::srgb(1.0, 0.9, 0.6), 0.1));
            }
            1 => {
                z.emit = 0.12;
                let top = p + Vec3::Y * 2.5;
                let ang = rng.gen_range(0.0..TAU);
                let r = z.radius * rng.gen_range(0.3..1.0);
                let end = p + Vec3::new(ang.cos() * r, 0.05, ang.sin() * r);
                let mut prev = top;
                for i in 1..=6 {
                    let f = i as f32 / 6.0;
                    let mut q = top.lerp(end, f);
                    if i < 6 {
                        q += Vec3::new(rng.gen_range(-0.3..0.3), rng.gen_range(-0.3..0.3), rng.gen_range(-0.3..0.3));
                    }
                    lines.0.push((prev, q, Color::srgb(0.7, 0.9, 1.0), 0.08));
                    prev = q;
                }
            }
            kind => {
                // Fire pool or a ring of flame around the player.
                z.emit = 0.025;
                for _ in 0..2 {
                    let ang = rng.gen_range(0.0..TAU);
                    let r = if kind == 3 { z.radius * rng.gen_range(0.85..1.0) } else { z.radius * rng.gen_range(0.0f32..1.0).sqrt() };
                    let q = p + Vec3::new(ang.cos() * r, 0.1, ang.sin() * r);
                    particle(&mut commands, &a.ball, &a.fire, q, Particle {
                        vel: Vec3::Y * rng.gen_range(1.5..3.5),
                        life: rng.gen_range(0.35..0.7),
                        max: 0.7,
                        gravity: -1.0,
                        drag: 0.5,
                        size: (0.3, 0.05),
                        pop: 0.1,
                        spin: Vec3::ZERO,
                        lands: false,
                    });
                }
            }
        }
    }
    for (b, mut tf) in &mut blades {
        let Some((_, r)) = sizes.iter().find(|(e, _)| *e == b.0) else { continue };
        let ang = b.1 + t * 7.0;
        let r = r * 0.55;
        tf.translation = Vec3::new(ang.cos() * r, 1.0 + 0.25 * (t * 5.0 + b.1).sin(), ang.sin() * r);
        tf.rotation = Quat::from_rotation_y(-ang) * Quat::from_rotation_z(0.3);
    }
}

fn draw_lines(time: Res<Time>, mut lines: ResMut<Lines>, mut gizmos: Gizmos) {
    let dt = time.delta_secs();
    for (a, b, color, life) in lines.0.iter_mut() {
        gizmos.line(*a, *b, *color);
        *life -= dt;
    }
    lines.0.retain(|l| l.3 > 0.0);
}

fn clear(mut lines: ResMut<Lines>, mut queue: ResMut<FxQueue>, mut out: ResMut<FxOutbox>) {
    lines.0.clear();
    queue.0.clear();
    out.0.clear();
}

/// Host helper: show an effect here and send it to everyone else.
pub fn emit(queue: &mut FxQueue, out: &mut FxOutbox, fx: Fx) {
    queue.0.push(fx.clone());
    out.0.push(fx);
}
