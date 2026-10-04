//! Looks for the abilities: one-shot spell effects (crescent cuts, fire
//! dragons, ice walls, thunderclaps, the Bifrost beam), things falling from
//! the sky (bombs, shells, meteors, spears), jets, and the trails ability
//! projectiles leave behind.

use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::{
    bolt, burst, flash_light, glow_material, particle, rand_dir, FxAssets, Grow, Lines, Particle,
    Spike,
};
use crate::data::Ability;
use crate::kit::{c, Kit};
use crate::sim::powers::{falling as fall_kind, look};
use crate::{InGameEntity, NetKind, Replicated};

#[derive(Resource)]
pub struct SpellAssets {
    crescent: Handle<Mesh>,
    meteor: (Handle<Mesh>, Handle<Mesh>),
    bomb: Handle<Mesh>,
    shell: Handle<Mesh>,
    jet: (Handle<Mesh>, Handle<Mesh>),
    solid: Handle<StandardMaterial>,
    glow: Handle<StandardMaterial>,
    ember: Handle<StandardMaterial>,
    pub void: Handle<StandardMaterial>,
    pub snow: Handle<StandardMaterial>,
    /// Blizzard's pale storm cloud.
    pub frost_cloud: Handle<StandardMaterial>,
    pub violet: Handle<StandardMaterial>,
    pub cyan: Handle<StandardMaterial>,
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let (_, crescent, _) = crate::models::projectiles::missile_kit(look::CRESCENT);
    commands.insert_resource(SpellAssets {
        crescent: meshes.add(crescent.build_or_empty()),
        meteor: {
            let (k, g) = meteor_kit();
            (meshes.add(k.build_or_empty()), meshes.add(g.build_or_empty()))
        },
        bomb: meshes.add(bomb_kit().build_or_empty()),
        shell: meshes.add(shell_kit().build_or_empty()),
        jet: {
            let (k, g) = jet_kit();
            (meshes.add(k.build_or_empty()), meshes.add(g.build_or_empty()))
        },
        solid: materials.add(crate::kit::vertex_material(0.6, 0.3)),
        glow: materials.add(crate::kit::glow_material(3.0)),
        ember: glow_material(&mut materials, [1.0, 0.45, 0.1], 0.9),
        void: materials.add(StandardMaterial {
            base_color: Color::BLACK,
            unlit: true,
            ..default()
        }),
        snow: materials.add(StandardMaterial {
            base_color: Color::srgba(0.95, 0.97, 1.0, 0.85),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        frost_cloud: materials.add(StandardMaterial {
            base_color: Color::srgba(0.72, 0.8, 0.9, 0.6),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 1.0,
            ..default()
        }),
        violet: glow_material(&mut materials, [0.75, 0.4, 1.0], 0.9),
        cyan: glow_material(&mut materials, [0.4, 0.95, 1.0], 0.9),
    });
}

/// A lumpy rock with glowing cracks.
fn meteor_kit() -> (Kit, Kit) {
    let (mut k, mut g) = (Kit::new(), Kit::new());
    let rock = c(0.22, 0.16, 0.13);
    k.blob(Vec3::ZERO, Vec3::new(0.7, 0.6, 0.65), rock);
    k.blob(Vec3::new(0.3, 0.2, 0.1), Vec3::new(0.4, 0.35, 0.4), rock);
    k.blob(Vec3::new(-0.25, -0.2, 0.2), Vec3::new(0.4, 0.3, 0.35), c(0.18, 0.13, 0.1));
    for i in 0..6 {
        let a = i as f32 * 1.1;
        g.cuboid_rot(
            Vec3::new(a.cos() * 0.55, (i as f32 * 0.7).sin() * 0.4, a.sin() * 0.55),
            Vec3::new(0.06, 0.4, 0.06),
            Quat::from_rotation_y(a) * Quat::from_rotation_z(0.6),
            c(1.0, 0.45, 0.08),
        );
    }
    (k, g)
}

/// An aircraft bomb pointing down (+Y up is its tail).
fn bomb_kit() -> Kit {
    let mut k = Kit::new();
    let green = c(0.25, 0.3, 0.2);
    k.capsule_between(Vec3::new(0.0, -0.35, 0.0), Vec3::new(0.0, 0.35, 0.0), 0.16, green);
    k.cyl(Vec3::new(0.0, -0.1, 0.0), 0.165, 0.05, Quat::IDENTITY, c(0.8, 0.7, 0.2));
    for i in 0..4 {
        let a = i as f32 * TAU / 4.0;
        k.cuboid_rot(
            Vec3::new(a.cos() * 0.15, 0.5, a.sin() * 0.15),
            Vec3::new(0.2, 0.25, 0.015),
            Quat::from_rotation_y(-a),
            green,
        );
    }
    k
}

/// A mortar shell pointing down.
fn shell_kit() -> Kit {
    let mut k = Kit::new();
    k.capsule_between(Vec3::new(0.0, -0.18, 0.0), Vec3::new(0.0, 0.15, 0.0), 0.07, c(0.3, 0.32, 0.28));
    k.cyl(Vec3::new(0.0, 0.25, 0.0), 0.03, 0.15, Quat::IDENTITY, c(0.3, 0.32, 0.28));
    for i in 0..4 {
        let a = i as f32 * TAU / 4.0;
        k.cuboid_rot(
            Vec3::new(a.cos() * 0.06, 0.28, a.sin() * 0.06),
            Vec3::new(0.1, 0.1, 0.01),
            Quat::from_rotation_y(-a),
            c(0.3, 0.32, 0.28),
        );
    }
    k
}

/// A fighter jet seen from below, nose along -Z, with glowing engines.
fn jet_kit() -> (Kit, Kit) {
    let (mut k, mut g) = (Kit::new(), Kit::new());
    let grey = c(0.42, 0.45, 0.5);
    let dark = c(0.2, 0.22, 0.25);
    k.capsule_between(Vec3::new(0.0, 0.0, -3.0), Vec3::new(0.0, 0.0, 3.0), 0.55, grey);
    k.cone(Vec3::new(0.0, 0.0, -3.9), 0.5, 1.4, Quat::from_rotation_x(-FRAC_PI_2), grey);
    k.blob(Vec3::new(0.0, 0.45, -1.8), Vec3::new(0.35, 0.3, 0.9), c(0.15, 0.2, 0.3));
    // Swept wings and tail.
    for side in [-1.0f32, 1.0] {
        k.wedge(
            Vec3::new(side * 2.4, 0.0, 0.6),
            Vec3::new(4.0, 0.12, 2.6),
            Quat::from_rotation_y(side * 0.35),
            grey,
        );
        k.wedge(
            Vec3::new(side * 1.1, 0.0, 3.0),
            Vec3::new(1.8, 0.1, 1.2),
            Quat::from_rotation_y(side * 0.3),
            dark,
        );
        k.cuboid_rot(
            Vec3::new(side * 0.5, 0.9, 2.9),
            Vec3::new(0.1, 1.4, 1.2),
            Quat::from_rotation_z(side * 0.3),
            dark,
        );
        g.cyl_z(Vec3::new(side * 0.32, -0.05, 3.5), 0.28, 0.2, c(1.0, 0.55, 0.2));
    }
    (k, g)
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Something dropping out of the sky.
#[derive(Component)]
pub struct Faller {
    kind: u8,
    from: Vec3,
    to: Vec3,
    time: f32,
    age: f32,
}

/// Flies in a straight line, then goes (jets).
#[derive(Component)]
pub struct Mover {
    vel: Vec3,
    life: f32,
}

/// The Bifrost beam sweeping along the ground.
#[derive(Component)]
pub struct Sweep {
    from: Vec3,
    dir: Vec3,
    len: f32,
    delay: f32,
    age: f32,
    time: f32,
}

/// Glowing ground cracks before the Magma Geyser goes up.
#[derive(Component)]
pub struct Rumble {
    pos: Vec3,
    life: f32,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A crescent cut of light along `dir`, rolled by `roll`, that flashes out
/// and thins away.
#[allow(clippy::too_many_arguments)]
pub(super) fn cut(
    commands: &mut Commands,
    s: &SpellAssets,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    dir: Vec3,
    roll: f32,
    scale: f32,
    color: [f32; 3],
    life: f32,
) {
    let rot = Transform::IDENTITY
        .looking_to(dir.normalize_or(Vec3::NEG_Z), Vec3::Y)
        .rotation
        * Quat::from_rotation_z(roll);
    burst(
        commands,
        &s.crescent,
        glow_material(materials, color, 0.9),
        Transform::from_translation(pos).with_rotation(rot),
        life,
        Grow::Cut(scale),
    );
}

fn ring(
    commands: &mut Commands,
    a: &FxAssets,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    radius: f32,
    color: [f32; 3],
    life: f32,
) {
    burst(
        commands,
        &a.ring,
        glow_material(materials, color, 0.7),
        Transform::from_translation(pos + Vec3::Y * 0.1).with_scale(Vec3::new(0.3, 3.0, 0.3)),
        life,
        Grow::Flat(radius),
    );
}

fn sparks(commands: &mut Commands, a: &FxAssets, mat: &Handle<StandardMaterial>, pos: Vec3, n: usize, speed: f32) {
    let mut rng = rand::thread_rng();
    for _ in 0..n {
        let d = rand_dir(&mut rng);
        particle(
            commands,
            &a.cube,
            mat,
            pos,
            Particle {
                vel: d * rng.gen_range(0.4..1.0) * speed,
                life: rng.gen_range(0.2..0.5),
                max: 0.5,
                gravity: 6.0,
                drag: 2.0,
                size: (0.05, 0.02),
                pop: 0.0,
                spin: Vec3::ZERO,
                lands: false,
            },
        );
    }
}

fn puff(commands: &mut Commands, a: &FxAssets, mat: &Handle<StandardMaterial>, pos: Vec3, n: usize, size: f32) {
    let mut rng = rand::thread_rng();
    for _ in 0..n {
        let d = rand_dir(&mut rng);
        particle(
            commands,
            &a.ball,
            mat,
            pos + d * size * 0.3,
            Particle {
                vel: Vec3::new(d.x, d.y.abs() * 0.6, d.z) * rng.gen_range(1.0..3.0),
                life: rng.gen_range(0.6..1.2),
                max: 1.2,
                gravity: -0.4,
                drag: 1.5,
                size: (size * 0.3, size),
                pop: 0.15,
                spin: Vec3::ZERO,
                lands: false,
            },
        );
    }
}

fn rgb3(ability: Ability) -> [f32; 3] {
    ability.def().color
}

// ---------------------------------------------------------------------------
// Spells
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn spell(
    commands: &mut Commands,
    a: &FxAssets,
    s: &SpellAssets,
    materials: &mut Assets<StandardMaterial>,
    lines: &mut Lines,
    ability: Ability,
    pos: Vec3,
    dir: Vec3,
    size: f32,
) {
    let mut rng = rand::thread_rng();
    let color = rgb3(ability);
    let col = ability.color();
    let flat = dir.with_y(0.0).normalize_or(Vec3::NEG_Z);
    let side = Vec3::new(-flat.z, 0.0, flat.x);
    use Ability as A;
    match ability {
        A::Dash => {
            ring(commands, a, materials, pos, 2.5, color, 0.35);
            for _ in 0..8 {
                let off = side * rng.gen_range(-0.6..0.6) + Vec3::Y * rng.gen_range(0.2..1.6);
                lines.0.push((pos + off, pos + off - flat * 3.0, col.with_alpha(0.7), 0.2));
            }
        }
        A::Overdrive | A::CombatStim => {
            for i in 0..3 {
                burst(
                    commands,
                    &a.ring,
                    glow_material(materials, color, 0.7),
                    Transform::from_translation(pos + Vec3::Y * (0.3 + 0.6 * i as f32))
                        .with_scale(Vec3::new(0.3, 2.0, 0.3)),
                    0.45 + 0.1 * i as f32,
                    Grow::Flat(1.6 + 0.8 * i as f32),
                );
            }
            let mat = glow_material(materials, color, 0.9);
            for _ in 0..24 {
                let ang = rng.gen_range(0.0..TAU);
                let r = rng.gen_range(0.3..1.0);
                particle(
                    commands,
                    &a.cube,
                    &mat,
                    pos + Vec3::new(ang.cos() * r, 0.1, ang.sin() * r),
                    Particle {
                        vel: Vec3::Y * rng.gen_range(3.0..7.0),
                        life: rng.gen_range(0.4..0.8),
                        max: 0.8,
                        gravity: 0.0,
                        drag: 1.0,
                        size: (0.05, 0.02),
                        pop: 0.0,
                        spin: Vec3::ZERO,
                        lands: false,
                    },
                );
            }
            flash_light(commands, pos + Vec3::Y, col, 200_000.0, 8.0, 0.4);
        }
        A::RocketBarrage | A::Fireball => {
            flash_light(commands, pos, col, 150_000.0, 6.0, 0.15);
            puff(commands, a, &a.smoke, pos + dir * 0.4, 6, 0.4);
            sparks(commands, a, &a.spark, pos + dir * 0.3, 10, 6.0);
        }
        A::Airstrike => {
            // Two jets scream over, low, along the bombing line.
            for k in 0..2 {
                let start = pos - flat * 70.0 + side * (k as f32 * 6.0 - 3.0) + Vec3::Y * (20.0 + k as f32 * 2.0);
                let vel = flat * 70.0;
                commands
                    .spawn((
                        InGameEntity,
                        Mover { vel, life: 2.4 },
                        Transform::from_translation(start - flat * k as f32 * 8.0)
                            .looking_to(flat, Vec3::Y),
                        Visibility::default(),
                    ))
                    .with_children(|j| {
                        j.spawn((Mesh3d(s.jet.0.clone()), MeshMaterial3d(s.solid.clone())));
                        j.spawn((Mesh3d(s.jet.1.clone()), MeshMaterial3d(s.glow.clone())));
                        j.spawn((
                            PointLight {
                                intensity: 300_000.0,
                                color: Color::srgb(1.0, 0.6, 0.25),
                                range: 20.0,
                                ..default()
                            },
                            Transform::from_xyz(0.0, 0.0, 4.0),
                        ));
                    });
            }
            // A red smoke flare marks the line.
            let flare = glow_material(materials, [1.0, 0.2, 0.15], 0.5);
            for i in 0..14 {
                particle(
                    commands,
                    &a.ball,
                    &flare,
                    pos + Vec3::Y * 0.2,
                    Particle {
                        vel: Vec3::new(rng.gen_range(-0.3..0.3), rng.gen_range(1.5..3.0), rng.gen_range(-0.3..0.3)),
                        life: 1.0 + i as f32 * 0.1,
                        max: 2.4,
                        gravity: -0.2,
                        drag: 0.6,
                        size: (0.2, 1.1),
                        pop: 0.1,
                        spin: Vec3::ZERO,
                        lands: false,
                    },
                );
            }
            let end = pos + flat * size;
            lines.0.push((pos + Vec3::Y * 0.1, end + Vec3::Y * 0.1, Color::srgb(1.0, 0.3, 0.2), 1.2));
        }
        A::GlacierSpike => {
            // Spikes burst up one after another along the line, leaning out.
            let n = (size / 0.8) as usize;
            for i in 0..n {
                let d = (i as f32 + 0.5) * size / n as f32;
                let delay = d / 40.0;
                for k in 0..2 {
                    let lean = if k == 0 { 1.0 } else { -1.0 };
                    let base = pos + flat * d + side * lean * rng.gen_range(0.2..0.9);
                    let tilt = Quat::from_rotation_arc(Vec3::Y, (Vec3::Y * 2.0 + side * lean * 0.6 + flat * 0.5).normalize());
                    let h = rng.gen_range(1.4..2.6);
                    let w = rng.gen_range(1.0..1.6);
                    commands.spawn((
                        InGameEntity,
                        Spike {
                            life: 2.2,
                            max: 2.2,
                            scale: Vec3::new(w, h, w),
                            base,
                            delay,
                        },
                        Mesh3d(a.spike.clone()),
                        MeshMaterial3d(a.ice.clone()),
                        Transform::from_translation(base)
                            .with_rotation(tilt * Quat::from_rotation_y(rng.gen_range(0.0..TAU)))
                            .with_scale(Vec3::ZERO),
                    ));
                }
            }
            burst(
                commands,
                &a.disc,
                a.frost.clone(),
                Transform::from_translation(pos + flat * size / 2.0 + Vec3::Y * 0.03)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Z, flat))
                    .with_scale(Vec3::new(1.5, 1.0, size / 2.0)),
                2.4,
                Grow::Hold,
            );
            for i in 0..10 {
                particle(
                    commands,
                    &a.ball,
                    &a.mist,
                    pos + flat * (size * i as f32 / 10.0) + Vec3::Y * 0.4,
                    Particle {
                        vel: Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(0.5..1.5), rng.gen_range(-1.0..1.0)),
                        life: 1.0,
                        max: 1.0,
                        gravity: 0.0,
                        drag: 1.0,
                        size: (0.3, 1.2),
                        pop: 0.2,
                        spin: Vec3::ZERO,
                        lands: false,
                    },
                );
            }
            flash_light(commands, pos + flat * 3.0 + Vec3::Y, col, 200_000.0, 12.0, 0.5);
        }
        A::Iaido => {
            // The draw: a bright crescent leaves the blade, with a wake of
            // sparks, bigger the longer it was held.
            let k = size;
            cut(commands, s, materials, pos + dir * 0.6, dir, 0.0, 0.9 + 0.8 * k, [1.0, 0.92, 0.6], 0.22);
            cut(commands, s, materials, pos + dir * 0.4, dir, 0.08, 1.2 + 1.0 * k, color, 0.3);
            sparks(commands, a, &a.spark, pos + dir * 1.0, 12 + (16.0 * k) as usize, 8.0);
            flash_light(commands, pos + dir, col, 120_000.0 + 200_000.0 * k, 10.0, 0.2);
        }
        A::ShadowStep => {
            let smoke = glow_material(materials, [0.35, 0.25, 0.6], 0.35);
            puff(commands, a, &smoke, pos + Vec3::Y, 10, 0.8);
            puff(commands, a, &smoke, pos + flat * size + Vec3::Y, 10, 0.8);
            for i in 0..3 {
                let at = pos + flat * (size * (0.3 + 0.3 * i as f32)) + Vec3::Y * 1.1;
                cut(commands, s, materials, at, side * if i % 2 == 0 { 1.0 } else { -1.0 }, 0.6 * if i % 2 == 0 { 1.0 } else { -1.0 }, 1.4, [0.65, 0.5, 1.0], 0.3);
            }
        }
        A::ThousandCuts => {
            if size <= 0.0 {
                // Gone in a puff of crimson and black.
                let smoke = glow_material(materials, [0.4, 0.05, 0.08], 0.4);
                puff(commands, a, &smoke, pos + Vec3::Y, 16, 1.0);
                ring(commands, a, materials, pos, 15.0, [1.0, 0.15, 0.2], 0.6);
                flash_light(commands, pos + Vec3::Y, col, 200_000.0, 12.0, 0.3);
            } else {
                // Two cuts cross on the target.
                let d = Vec3::from(dir);
                cut(commands, s, materials, pos, d, 0.9, 0.9, [1.0, 0.2, 0.25], 0.25);
                cut(commands, s, materials, pos, -d, -0.9, 0.9, [1.0, 0.9, 0.9], 0.2);
                sparks(commands, a, &a.spark, pos, 8, 6.0);
                lines.0.push((pos - d * 1.5 + Vec3::Y * 0.6, pos + d * 1.5 - Vec3::Y * 0.6, Color::srgb(1.0, 0.85, 0.85), 0.12));
            }
        }
        A::RisingDragon => {
            // A fiery dragon spirals up out of the cut.
            cut(commands, s, materials, pos + flat * 1.2 + Vec3::Y * 1.0, Vec3::Y, FRAC_PI_2, 1.5, color, 0.35);
            for i in 0..60 {
                let t = i as f32 / 60.0;
                let ang = t * TAU * 2.0;
                let r = 0.9 * (1.0 - t * 0.5);
                let at = pos + flat * 1.0 + Vec3::new(ang.cos() * r, 0.2 + t * 0.3, ang.sin() * r);
                particle(
                    commands,
                    &a.ball,
                    if i % 3 == 0 { &a.spark } else { &a.fire },
                    at,
                    Particle {
                        vel: Vec3::new(-ang.sin(), 0.0, ang.cos()) * 2.0 + Vec3::Y * (5.0 + 6.0 * t),
                        life: 0.5 + t * 0.4,
                        max: 0.9,
                        gravity: 0.0,
                        drag: 1.2,
                        size: (0.35 * (1.0 - t) + 0.08, 0.02),
                        pop: 0.05,
                        spin: Vec3::ZERO,
                        lands: false,
                    },
                );
            }
            flash_light(commands, pos + Vec3::Y * 2.0, col, 300_000.0, 10.0, 0.4);
        }
        A::SupplyDrop => {
            ring(commands, a, materials, pos, size, [1.0, 0.8, 0.2], 0.6);
            ring(commands, a, materials, pos + Vec3::Y * 0.6, size * 0.75, [0.3, 0.9, 1.0], 0.6);
            let mat = glow_material(materials, [1.0, 0.85, 0.3], 0.9);
            for _ in 0..14 {
                let ang = rng.gen_range(0.0..TAU);
                let r = size * rng.gen_range(0.1f32..0.9).sqrt();
                particle(
                    commands,
                    &a.cube,
                    &mat,
                    pos + Vec3::new(ang.cos() * r, 0.2, ang.sin() * r),
                    Particle {
                        vel: Vec3::Y * rng.gen_range(1.5..3.0),
                        life: 1.0,
                        max: 1.0,
                        gravity: 0.0,
                        drag: 0.5,
                        size: (0.0, 0.18),
                        pop: 0.3,
                        spin: Vec3::new(1.0, 2.0, 0.0),
                        lands: false,
                    },
                );
            }
        }
        A::MortarBattery | A::MeteorShower | A::SpearRain => {
            // The target area stays marked while the barrage lasts.
            let life = match ability {
                A::MortarBattery => 6.0,
                A::MeteorShower => 5.5,
                _ => 1.8,
            };
            burst(
                commands,
                &a.ring,
                glow_material(materials, color, 0.5),
                Transform::from_translation(pos + Vec3::Y * 0.08).with_scale(Vec3::new(size, 2.0, size)),
                life,
                Grow::Hold,
            );
            burst(
                commands,
                &a.disc,
                glow_material(materials, color, 0.08),
                Transform::from_translation(pos + Vec3::Y * 0.04).with_scale(Vec3::new(size, 1.0, size)),
                life,
                Grow::Hold,
            );
            if ability != A::MortarBattery {
                // Clouds gather overhead.
                let cloud = if ability == A::MeteorShower { s.ember.clone() } else { a.smoke.clone() };
                for _ in 0..10 {
                    let ang = rng.gen_range(0.0..TAU);
                    let r = size * rng.gen_range(0.0..0.8);
                    particle(
                        commands,
                        &a.ball,
                        &cloud,
                        pos + Vec3::new(ang.cos() * r, rng.gen_range(18.0..22.0), ang.sin() * r),
                        Particle {
                            vel: Vec3::ZERO,
                            life: life,
                            max: life,
                            gravity: 0.0,
                            drag: 0.0,
                            size: (size * 0.3, size * 0.45),
                            pop: 0.2,
                            spin: Vec3::ZERO,
                            lands: false,
                        },
                    );
                }
            }
        }
        A::FlameDash => {
            for i in 0..20 {
                let at = pos + flat * (size * i as f32 / 20.0) + Vec3::Y * 0.3;
                particle(
                    commands,
                    &a.ball,
                    &a.fire,
                    at + side * rng.gen_range(-0.5..0.5),
                    Particle {
                        vel: Vec3::Y * rng.gen_range(2.0..4.0) - flat * 1.5,
                        life: rng.gen_range(0.4..0.8),
                        max: 0.8,
                        gravity: -1.0,
                        drag: 0.8,
                        size: (0.45, 0.05),
                        pop: 0.1,
                        spin: Vec3::ZERO,
                        lands: false,
                    },
                );
            }
            ring(commands, a, materials, pos, 3.0, color, 0.4);
            flash_light(commands, pos + Vec3::Y, col, 200_000.0, 8.0, 0.4);
        }
        A::MagmaGeyser => {
            if size > 0.0 {
                // The ground glows and splits first.
                commands.spawn((
                    InGameEntity,
                    Rumble { pos, life: 0.75 },
                    Transform::from_translation(pos),
                ));
                burst(
                    commands,
                    &a.disc,
                    glow_material(materials, [1.0, 0.3, 0.05], 0.5),
                    Transform::from_translation(pos + Vec3::Y * 0.04).with_scale(Vec3::new(0.2, 1.0, 0.2)),
                    0.8,
                    Grow::Flat(size),
                );
            } else {
                // Eruption: a pillar of magma, rocks and fire.
                let r = 3.6;
                burst(
                    commands,
                    &a.beam,
                    glow_material(materials, [1.0, 0.45, 0.08], 0.85),
                    Transform::from_translation(pos + Vec3::Y * 30.0),
                    0.9,
                    Grow::Column2(r * 0.55),
                );
                burst(
                    commands,
                    &a.beam,
                    a.beam_core.clone(),
                    Transform::from_translation(pos + Vec3::Y * 30.0),
                    0.6,
                    Grow::Column2(r * 0.22),
                );
                for _ in 0..50 {
                    let d = rand_dir(&mut rng);
                    particle(
                        commands,
                        &a.ball,
                        &a.fire,
                        pos + Vec3::new(d.x, 0.0, d.z) * rng.gen_range(0.0..r * 0.5),
                        Particle {
                            vel: Vec3::new(d.x * 3.0, rng.gen_range(10.0..20.0), d.z * 3.0),
                            life: rng.gen_range(0.6..1.2),
                            max: 1.2,
                            gravity: 14.0,
                            drag: 0.3,
                            size: (0.5, 0.1),
                            pop: 0.05,
                            spin: Vec3::ZERO,
                            lands: false,
                        },
                    );
                }
                for _ in 0..16 {
                    let d = rand_dir(&mut rng);
                    let sz = rng.gen_range(0.12..0.3);
                    particle(
                        commands,
                        &a.cube,
                        &s.ember,
                        pos + Vec3::Y * 0.5,
                        Particle {
                            vel: Vec3::new(d.x * 6.0, rng.gen_range(8.0..15.0), d.z * 6.0),
                            life: rng.gen_range(1.2..2.0),
                            max: 2.0,
                            gravity: 18.0,
                            drag: 0.2,
                            size: (sz, sz),
                            pop: 0.0,
                            spin: rand_dir(&mut rng) * 8.0,
                            lands: true,
                        },
                    );
                }
                flash_light(commands, pos + Vec3::Y * 2.0, col, 800_000.0, 18.0, 0.6);
            }
        }
        A::ThunderClap => {
            ring(commands, a, materials, pos, size, [0.6, 0.85, 1.0], 0.4);
            ring(commands, a, materials, pos + Vec3::Y * 0.8, size * 0.7, [0.9, 0.95, 1.0], 0.3);
            for i in 0..10 {
                let ang = i as f32 / 10.0 * TAU + rng.gen_range(-0.2..0.2);
                let out = Vec3::new(ang.cos(), 0.0, ang.sin());
                bolt(commands, a, pos + Vec3::Y * 0.3, pos + out * size + Vec3::Y * 0.1, 0.04, &mut rng);
            }
            burst(
                commands,
                &a.ball,
                glow_material(materials, [0.7, 0.9, 1.0], 0.4),
                Transform::from_translation(pos + Vec3::Y).with_scale(Vec3::splat(0.3)),
                0.3,
                Grow::Ball(size * 0.6),
            );
            flash_light(commands, pos + Vec3::Y, col, 500_000.0, size * 3.0, 0.3);
        }
        A::Bifrost => {
            commands.spawn((
                InGameEntity,
                Sweep {
                    from: pos,
                    dir: flat,
                    len: size,
                    delay: 0.5,
                    age: 0.0,
                    time: 2.2,
                },
                Mesh3d(a.beam.clone()),
                MeshMaterial3d(glow_material(materials, [0.9, 0.7, 1.0], 0.6)),
                Transform::from_translation(pos + Vec3::Y * 30.0).with_scale(Vec3::new(0.001, 1.0, 0.001)),
                NotShadowCaster,
            ));
            // A rainbow arc across the sky above the line.
            let colors = [[1.0, 0.2, 0.2], [1.0, 0.6, 0.1], [1.0, 1.0, 0.2], [0.3, 1.0, 0.3], [0.3, 0.6, 1.0], [0.7, 0.3, 1.0]];
            for (k, c3) in colors.iter().enumerate() {
                let r = 14.0 - k as f32 * 0.35;
                let mut prev = None;
                for i in 0..=24 {
                    let t = i as f32 / 24.0 * PI;
                    let q = pos + flat * (size / 2.0) - flat * t.cos() * (size / 2.0 + 2.0) + Vec3::Y * (t.sin() * r + 1.0);
                    if let Some(p) = prev {
                        lines.0.push((p, q, Color::srgb(c3[0], c3[1], c3[2]), 2.8));
                    }
                    prev = Some(q);
                }
            }
        }
        _ => {}
    }
}

/// Something drops out of the sky onto `to` over `time` seconds. The host
/// shows the impact when it lands.
pub fn falling(commands: &mut Commands, s: &SpellAssets, a: &FxAssets, kind: u8, from: Vec3, to: Vec3, time: f32) {
    let dir = (to - from).normalize_or(Vec3::NEG_Y);
    // Models point down along -Y with their tail up.
    let rot = Quat::from_rotation_arc(Vec3::NEG_Y, dir);
    let mut e = commands.spawn((
        InGameEntity,
        Faller {
            kind,
            from,
            to,
            time: time.max(0.05),
            age: 0.0,
        },
        Transform::from_translation(from).with_rotation(rot),
        Visibility::default(),
        NotShadowCaster,
    ));
    e.with_children(|p| match kind {
        fall_kind::BOMB => {
            p.spawn((Mesh3d(s.bomb.clone()), MeshMaterial3d(s.solid.clone())));
        }
        fall_kind::SHELL => {
            p.spawn((Mesh3d(s.shell.clone()), MeshMaterial3d(s.solid.clone())));
        }
        fall_kind::METEOR => {
            p.spawn((Mesh3d(s.meteor.0.clone()), MeshMaterial3d(s.solid.clone())));
            p.spawn((Mesh3d(s.meteor.1.clone()), MeshMaterial3d(s.glow.clone())));
            p.spawn((
                PointLight {
                    intensity: 400_000.0,
                    color: Color::srgb(1.0, 0.45, 0.1),
                    range: 18.0,
                    ..default()
                },
                Transform::default(),
            ));
        }
        _ => {
            p.spawn((
                Mesh3d(a.spear.clone()),
                MeshMaterial3d(a.spear_mat.clone()),
                Transform::from_rotation(Quat::from_rotation_x(PI)).with_scale(Vec3::splat(1.4)),
            ));
            p.spawn((
                PointLight {
                    intensity: 60_000.0,
                    color: Color::srgb(0.6, 0.85, 1.0),
                    range: 8.0,
                    ..default()
                },
                Transform::default(),
            ));
        }
    });
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Falling things move down their line, trailing smoke, fire or sparks.
pub fn fall(
    mut commands: Commands,
    time: Res<Time>,
    a: Res<FxAssets>,
    s: Res<SpellAssets>,
    mut q: Query<(Entity, &mut Faller, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (e, mut f, mut tf) in &mut q {
        f.age += dt;
        let k = (f.age / f.time).min(1.0);
        // Speeds up as it comes down.
        let pos = f.from.lerp(f.to, k * k * 0.6 + k * 0.4);
        tf.translation = pos;
        if f.kind == fall_kind::METEOR {
            tf.rotate_local_x(dt * 3.0);
        }
        if k >= 1.0 {
            commands.entity(e).despawn();
            continue;
        }
        let (mat, size, life) = match f.kind {
            fall_kind::METEOR => (&a.fire, 0.9, 0.5),
            fall_kind::SPEAR => (&a.spark, 0.12, 0.2),
            fall_kind::BOMB => (&a.smoke, 0.25, 0.6),
            _ => (&s.ember, 0.12, 0.25),
        };
        particle(
            &mut commands,
            &a.ball,
            mat,
            pos + rand_dir(&mut rng) * 0.1,
            Particle {
                vel: rand_dir(&mut rng) * 0.5,
                life,
                max: life,
                gravity: 0.0,
                drag: 1.0,
                size: (size, size * 0.3),
                pop: 0.0,
                spin: Vec3::ZERO,
                lands: false,
            },
        );
        if f.kind == fall_kind::METEOR {
            particle(
                &mut commands,
                &a.ball,
                &a.smoke,
                pos,
                Particle {
                    vel: Vec3::Y * 1.0,
                    life: 1.2,
                    max: 1.2,
                    gravity: 0.0,
                    drag: 0.5,
                    size: (0.6, 1.4),
                    pop: 0.1,
                    spin: Vec3::ZERO,
                    lands: false,
                },
            );
        }
    }
}

/// Jets fly straight on, leaving vapour trails.
pub fn movers(
    mut commands: Commands,
    time: Res<Time>,
    mut lines: ResMut<Lines>,
    mut q: Query<(Entity, &mut Mover, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (e, mut m, mut tf) in &mut q {
        m.life -= dt;
        if m.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let before = tf.translation;
        tf.translation += m.vel * dt;
        for side in [-4.5f32, 4.5] {
            let off = tf.rotation * Vec3::new(side, 0.0, 1.0);
            lines.0.push((before + off, tf.translation + off, Color::srgba(1.0, 1.0, 1.0, 0.5), 0.8));
        }
    }
}

/// The Bifrost beam: waits, slams down, then sweeps along its line,
/// shifting through the colours of the rainbow.
pub fn sweeps(
    mut commands: Commands,
    time: Res<Time>,
    a: Res<FxAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(Entity, &mut Sweep, &mut Transform, &MeshMaterial3d<StandardMaterial>)>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (e, mut sw, mut tf, mat) in &mut q {
        sw.age += dt;
        let t = sw.age - sw.delay;
        if t > sw.time + 0.3 {
            commands.entity(e).despawn();
            continue;
        }
        let k = (t / sw.time).clamp(0.0, 1.0);
        let at = sw.from + sw.dir * sw.len * k;
        let width = if t < 0.0 {
            0.12
        } else if t > sw.time {
            1.6 * (1.0 - (t - sw.time) / 0.3)
        } else {
            1.6 + 0.2 * (sw.age * 30.0).sin()
        };
        tf.translation = at + Vec3::Y * 30.0;
        tf.scale = Vec3::new(width.max(0.001), 1.0, width.max(0.001));
        let hue = (sw.age * 300.0) % 360.0;
        if let Some(m) = materials.get_mut(&mat.0) {
            let col = Color::hsl(hue, 1.0, 0.65);
            m.base_color = col.with_alpha(0.75);
            m.emissive = LinearRgba::from(col) * 6.0;
        }
        if t >= 0.0 && t <= sw.time {
            let col = Color::hsl(hue, 1.0, 0.6);
            let gl = glow_material(&mut materials, super::rgb(col), 0.9);
            for _ in 0..3 {
                let d = rand_dir(&mut rng);
                particle(
                    &mut commands,
                    &a.cube,
                    &gl,
                    at + Vec3::Y * 0.2,
                    Particle {
                        vel: Vec3::new(d.x * 6.0, d.y.abs() * 8.0, d.z * 6.0),
                        life: 0.5,
                        max: 0.5,
                        gravity: 10.0,
                        drag: 1.0,
                        size: (0.05, 0.02),
                        pop: 0.0,
                        spin: Vec3::ZERO,
                        lands: false,
                    },
                );
            }
            // A scorched, glittering path behind it.
            burst(
                &mut commands,
                &a.disc,
                a.scorch.clone(),
                Transform::from_translation(at.with_y(0.02)).with_scale(Vec3::new(1.4, 1.0, 1.4)),
                4.0,
                Grow::Hold,
            );
        }
    }
}

/// Magma Geyser's warning: glowing cracks spread and the ground shakes up
/// grit.
pub fn rumbles(
    mut commands: Commands,
    time: Res<Time>,
    a: Res<FxAssets>,
    s: Res<SpellAssets>,
    mut lines: ResMut<Lines>,
    mut q: Query<(Entity, &mut Rumble)>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (e, mut r) in &mut q {
        r.life -= dt;
        if r.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let ang = rng.gen_range(0.0..TAU);
        let mut prev = r.pos + Vec3::Y * 0.05;
        for i in 1..5 {
            let q = r.pos
                + Vec3::new((ang + rng.gen_range(-0.3..0.3)).cos(), 0.0, (ang + rng.gen_range(-0.3..0.3)).sin())
                    * (i as f32 * 0.9)
                + Vec3::Y * 0.05;
            lines.0.push((prev, q, Color::srgb(1.0, 0.45, 0.05), 0.4));
            prev = q;
        }
        particle(
            &mut commands,
            &a.cube,
            &s.ember,
            r.pos + Vec3::new(rng.gen_range(-2.0..2.0), 0.1, rng.gen_range(-2.0..2.0)),
            Particle {
                vel: Vec3::Y * rng.gen_range(2.0..4.0),
                life: 0.5,
                max: 0.5,
                gravity: 12.0,
                drag: 0.5,
                size: (0.08, 0.08),
                pop: 0.0,
                spin: Vec3::ONE * 6.0,
                lands: false,
            },
        );
    }
}

/// Trails behind ability projectiles.
pub fn trails(
    mut commands: Commands,
    a: Res<FxAssets>,
    s: Res<SpellAssets>,
    mut lines: ResMut<Lines>,
    mut last: Local<HashMap<Entity, Vec3>>,
    q: Query<(Entity, &Replicated, &GlobalTransform)>,
) {
    let mut rng = rand::thread_rng();
    let mut seen = Vec::new();
    for (e, r, gt) in &q {
        let NetKind::Missile(l) = r.kind else { continue };
        let pos = gt.translation();
        seen.push(e);
        let prev = last.insert(e, pos).unwrap_or(pos);
        let step = pos - prev;
        match l {
            0..=2 => {
                // Gold streaks off the crescent's tips and a shimmer behind.
                let size = 1.0 + 0.45 * l as f32;
                let side = Vec3::new(-step.z, 0.0, step.x).normalize_or_zero();
                for k in [-1.0f32, 1.0] {
                    let tip = pos + side * k * 0.9 * size;
                    lines.0.push((tip - step, tip, Color::srgb(1.0, 0.85, 0.45), 0.15));
                }
                particle(
                    &mut commands,
                    &a.cube,
                    &a.spark,
                    pos + side * rng.gen_range(-1.0..1.0) * size,
                    Particle {
                        vel: Vec3::Y * rng.gen_range(-0.5..1.5),
                        life: 0.3,
                        max: 0.3,
                        gravity: 0.0,
                        drag: 1.0,
                        size: (0.05, 0.02),
                        pop: 0.0,
                        spin: Vec3::ZERO,
                        lands: false,
                    },
                );
            }
            look::KUNAI => lines.0.push((prev, pos, Color::srgba(1.0, 0.35, 0.35, 0.7), 0.1)),
            look::ROCKET => {
                particle(
                    &mut commands,
                    &a.ball,
                    &a.smoke,
                    pos,
                    Particle {
                        vel: Vec3::Y * 0.3,
                        life: 0.7,
                        max: 0.7,
                        gravity: 0.0,
                        drag: 1.0,
                        size: (0.08, 0.35),
                        pop: 0.1,
                        spin: Vec3::ZERO,
                        lands: false,
                    },
                );
                lines.0.push((prev, pos, Color::srgb(1.0, 0.6, 0.2), 0.08));
            }
            look::FIREBALL => {
                for _ in 0..2 {
                    particle(
                        &mut commands,
                        &a.ball,
                        &a.fire,
                        pos + rand_dir(&mut rng) * 0.25,
                        Particle {
                            vel: rand_dir(&mut rng) * 0.8 + Vec3::Y,
                            life: 0.4,
                            max: 0.4,
                            gravity: -1.0,
                            drag: 1.0,
                            size: (0.35, 0.05),
                            pop: 0.0,
                            spin: Vec3::ZERO,
                            lands: false,
                        },
                    );
                }
            }
            look::CRYO => {
                particle(
                    &mut commands,
                    &a.ball,
                    &a.mist,
                    pos + rand_dir(&mut rng) * 0.6,
                    Particle {
                        vel: rand_dir(&mut rng) * 0.5,
                        life: 0.9,
                        max: 0.9,
                        gravity: 0.5,
                        drag: 1.0,
                        size: (0.2, 0.7),
                        pop: 0.2,
                        spin: Vec3::ZERO,
                        lands: false,
                    },
                );
                if rng.gen_bool(0.3) {
                    particle(
                        &mut commands,
                        &a.ball,
                        &s.snow,
                        pos + rand_dir(&mut rng) * 1.5,
                        Particle {
                            vel: Vec3::NEG_Y * 0.5,
                            life: 1.0,
                            max: 1.0,
                            gravity: 1.0,
                            drag: 0.5,
                            size: (0.05, 0.03),
                            pop: 0.0,
                            spin: Vec3::ZERO,
                            lands: false,
                        },
                    );
                }
            }
            look::GRAV => {
                let d = rand_dir(&mut rng);
                particle(
                    &mut commands,
                    &a.cube,
                    &s.void,
                    pos + d * 0.4,
                    Particle {
                        vel: -d * 1.0,
                        life: 0.4,
                        max: 0.4,
                        gravity: 0.0,
                        drag: 0.0,
                        size: (0.04, 0.0),
                        pop: 0.0,
                        spin: Vec3::ZERO,
                        lands: false,
                    },
                );
            }
            look::SMOKE => {
                particle(
                    &mut commands,
                    &a.ball,
                    &a.smoke,
                    pos,
                    Particle {
                        vel: Vec3::Y * 0.4,
                        life: 0.6,
                        max: 0.6,
                        gravity: 0.0,
                        drag: 1.0,
                        size: (0.05, 0.25),
                        pop: 0.1,
                        spin: Vec3::ZERO,
                        lands: false,
                    },
                );
            }
            _ => {}
        }
    }
    last.retain(|e, _| seen.contains(e));
}
