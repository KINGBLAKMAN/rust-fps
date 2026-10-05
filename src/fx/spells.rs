//! Looks for the abilities: one-shot spell effects (shield bashes, shockwaves,
//! scythe sweeps, chains, toxic blasts, stone waves, sniper hits), things
//! falling from the sky (the Payload bomb, Arrow Storm arrows), the grapple
//! hook, warning circles, and the trails thrown gadgets leave behind.

use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::{
    burst, explosion, flash_light, glow_material, particle, rand_dir, FxAssets, Grow, Lines,
    Particle, Spike,
};
use crate::data::Ability;
use crate::kit::{c, Kit};
use crate::markers::{Marker, Markers, Shape};
use crate::sim::powers::{falling as fall_kind, look};
use crate::{InGameEntity, NetKind, Replicated};

#[derive(Resource)]
pub struct SpellAssets {
    crescent: Handle<Mesh>,
    bomb: Handle<Mesh>,
    arrow: Handle<Mesh>,
    hook: Handle<Mesh>,
    rock: Handle<Mesh>,
    link: Handle<Mesh>,
    mark: Handle<Mesh>,
    pub scythe: Handle<Mesh>,
    solid: Handle<StandardMaterial>,
    ember: Handle<StandardMaterial>,
    gold: Handle<StandardMaterial>,
    holy: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    uv: Handle<StandardMaterial>,
    medic: Handle<StandardMaterial>,
    wraith: Handle<StandardMaterial>,
    glass: Handle<StandardMaterial>,
    stone: Handle<StandardMaterial>,
    /// Spectral teal (Revenant), solid and as a faint mist.
    pub teal: Handle<StandardMaterial>,
    pub spirit: Handle<StandardMaterial>,
    /// Chemist's glowing green and the yellow-green gas.
    pub toxic: Handle<StandardMaterial>,
    pub gas: Handle<StandardMaterial>,
    pub dust: Handle<StandardMaterial>,
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let plain = |materials: &mut Assets<StandardMaterial>, color: Color| {
        materials.add(StandardMaterial {
            base_color: color,
            alpha_mode: if color.alpha() < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
            perceptual_roughness: 1.0,
            ..default()
        })
    };
    commands.insert_resource(SpellAssets {
        crescent: meshes.add(crescent_kit().build_or_empty()),
        bomb: meshes.add(bomb_kit().build_or_empty()),
        arrow: meshes.add(arrow_kit().build_or_empty()),
        hook: meshes.add(hook_kit().build_or_empty()),
        rock: meshes.add(rock_kit().build_or_empty()),
        link: meshes.add(link_kit().build_or_empty()),
        mark: meshes.add(mark_kit().build_or_empty()),
        scythe: meshes.add(scythe_kit().build_or_empty()),
        solid: materials.add(crate::kit::vertex_material(0.6, 0.3)),
        ember: glow_material(&mut materials, [1.0, 0.45, 0.1], 0.9),
        gold: glow_material(&mut materials, [1.0, 0.82, 0.35], 0.9),
        holy: glow_material(&mut materials, [1.0, 0.96, 0.75], 0.45),
        red: glow_material(&mut materials, [1.0, 0.15, 0.1], 0.9),
        uv: glow_material(&mut materials, [0.65, 0.45, 1.0], 0.75),
        medic: glow_material(&mut materials, [0.4, 1.0, 0.8], 0.8),
        wraith: glow_material(&mut materials, [0.55, 0.65, 0.85], 0.22),
        glass: materials.add(StandardMaterial {
            base_color: Color::srgba(0.8, 0.95, 1.0, 0.7),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        stone: plain(&mut materials, Color::srgb(0.5, 0.48, 0.44)),
        teal: glow_material(&mut materials, [0.4, 1.0, 0.8], 0.85),
        spirit: glow_material(&mut materials, [0.35, 1.0, 0.75], 0.18),
        toxic: glow_material(&mut materials, [0.55, 1.0, 0.15], 0.9),
        gas: materials.add(StandardMaterial {
            base_color: Color::srgba(0.62, 0.8, 0.2, 0.32),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        dust: plain(&mut materials, Color::srgba(0.5, 0.45, 0.38, 0.5)),
    });
}

/// A crescent cut of light, flat, bulging forward along -Z.
fn crescent_kit() -> Kit {
    let mut g = Kit::new();
    let r = 1.0;
    let n = 20;
    let span = 1.25;
    for layer in 0..2 {
        let (thick, color) = if layer == 0 {
            (0.26, c(1.0, 0.85, 0.6))
        } else {
            (0.11, c(1.0, 1.0, 1.0))
        };
        for i in 0..n {
            let a0 = -span + 2.0 * span * i as f32 / n as f32;
            let a1 = -span + 2.0 * span * (i + 1) as f32 / n as f32;
            let am = (a0 + a1) / 2.0;
            let taper = 1.0 - (am / span).powi(2);
            let p = |a: f32| Vec3::new(a.sin() * r, 0.0, -a.cos() * r + r * 0.75);
            let (pa, pb) = (p(a0), p(a1));
            let mid = (pa + pb) / 2.0;
            let len = pa.distance(pb) + 0.02;
            let w = (thick * taper).max(0.015);
            // Thicker toward the inside of the curve, like a real cut.
            let inward = -Vec3::new(am.sin(), 0.0, -am.cos());
            g.cuboid_rot(
                mid + inward * w * 0.3 + Vec3::new(0.0, 0.004 * layer as f32, 0.0),
                Vec3::new(len, 0.03 + 0.02 * layer as f32, w),
                Quat::from_rotation_y(-am),
                color,
            );
        }
    }
    g
}

/// The Payload: a big finned bomb pointing down (+Y up is its tail).
fn bomb_kit() -> Kit {
    let mut k = Kit::new();
    let olive = c(0.28, 0.32, 0.2);
    let dark = c(0.18, 0.2, 0.14);
    k.capsule_between(Vec3::new(0.0, -0.8, 0.0), Vec3::new(0.0, 0.7, 0.0), 0.45, olive);
    k.cyl(Vec3::new(0.0, -0.35, 0.0), 0.455, 0.14, Quat::IDENTITY, c(0.95, 0.75, 0.15));
    k.cyl(Vec3::new(0.0, 0.25, 0.0), 0.455, 0.08, Quat::IDENTITY, c(0.8, 0.15, 0.1));
    k.sphere(Vec3::new(0.0, -1.22, 0.0), 0.08, c(0.6, 0.6, 0.6));
    k.frustum(Vec3::new(0.0, 1.35, 0.0), 0.14, 0.4, 0.7, Quat::IDENTITY, dark);
    for i in 0..4 {
        let a = i as f32 * TAU / 4.0;
        k.cuboid_rot(
            Vec3::new(a.cos() * 0.42, 1.55, a.sin() * 0.42),
            Vec3::new(0.5, 0.75, 0.04),
            Quat::from_rotation_y(-a),
            olive,
        );
    }
    k.torus(Vec3::new(0.0, 1.85, 0.0), 0.035, 0.55, Quat::IDENTITY, dark);
    k
}

/// An arrow pointing down (tip at -Y, fletching up).
fn arrow_kit() -> Kit {
    let mut k = Kit::new();
    k.cyl(Vec3::ZERO, 0.022, 1.1, Quat::IDENTITY, c(0.55, 0.4, 0.25));
    k.cone(Vec3::new(0.0, -0.6, 0.0), 0.05, 0.16, Quat::from_rotation_x(PI), c(0.75, 0.75, 0.8));
    for i in 0..3 {
        let a = i as f32 * TAU / 3.0;
        k.cuboid_rot(
            Vec3::new(a.cos() * 0.04, 0.44, a.sin() * 0.04),
            Vec3::new(0.07, 0.2, 0.008),
            Quat::from_rotation_y(-a),
            if i == 0 { c(0.85, 0.2, 0.15) } else { c(0.92, 0.9, 0.85) },
        );
    }
    k
}

/// The grapple hook: three prongs pointing along -Z.
fn hook_kit() -> Kit {
    let mut k = Kit::new();
    let steel = c(0.6, 0.62, 0.66);
    k.cyl_z(Vec3::new(0.0, 0.0, 0.05), 0.03, 0.35, steel);
    k.cone(Vec3::new(0.0, 0.0, -0.18), 0.045, 0.12, Quat::from_rotation_x(-FRAC_PI_2), steel);
    for i in 0..3 {
        let a = i as f32 * TAU / 3.0;
        let out = Vec3::new(a.cos(), a.sin(), 0.0);
        k.cuboid_rot(
            Vec3::new(0.0, 0.0, -0.05) + out * 0.08,
            Vec3::new(0.025, 0.025, 0.2),
            Quat::from_axis_angle(Vec3::new(-out.y, out.x, 0.0), -0.7),
            steel,
        );
    }
    k.torus(Vec3::new(0.0, 0.0, 0.25), 0.012, 0.04, Quat::from_rotation_x(FRAC_PI_2), steel);
    k
}

/// A jagged rock spike, base at the origin.
fn rock_kit() -> Kit {
    let mut k = Kit::new();
    k.cone(Vec3::new(0.0, 0.5, 0.0), 0.25, 1.0, Quat::IDENTITY, c(0.42, 0.37, 0.31));
    k.cone(Vec3::new(0.12, 0.3, 0.05), 0.15, 0.6, Quat::from_rotation_z(-0.4), c(0.36, 0.31, 0.27));
    k.cone(Vec3::new(-0.1, 0.25, -0.07), 0.13, 0.5, Quat::from_rotation_x(0.45), c(0.5, 0.45, 0.38));
    k
}

/// One link of a spectral chain, long along Z.
fn link_kit() -> Kit {
    let mut k = Kit::new();
    k.torus(Vec3::ZERO, 0.02, 0.07, Quat::IDENTITY, c(0.8, 1.0, 0.95));
    k.scaled(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.5))
}

/// Hunter's Mark: a diamond pointing down.
fn mark_kit() -> Kit {
    let mut k = Kit::new();
    let red = c(1.0, 0.3, 0.25);
    k.cone(Vec3::new(0.0, 0.12, 0.0), 0.14, 0.24, Quat::IDENTITY, red);
    k.cone(Vec3::new(0.0, -0.2, 0.0), 0.14, 0.4, Quat::from_rotation_x(PI), red);
    k
}

/// A spectral scythe lying flat: a short handle along Z and a curved blade
/// sweeping out to +X from its front end.
fn scythe_kit() -> Kit {
    let mut k = Kit::new();
    let shaft = c(0.35, 0.6, 0.55);
    let blade = c(0.8, 1.0, 0.95);
    k.cyl_z(Vec3::new(0.0, 0.0, 0.1), 0.03, 1.1, shaft);
    let top = Vec3::new(0.0, 0.0, -0.45);
    let n = 12;
    let p = |t: f32| top + Vec3::new((t * 1.3).sin() * 0.85, 0.0, -(1.0 - (t * 1.3).cos()) * 0.55);
    for i in 0..n {
        let (t0, t1) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
        let (a, b) = (p(t0), p(t1));
        let d = b - a;
        let w = 0.16 * (1.0 - t0) + 0.02;
        let ang = d.z.atan2(d.x);
        // The blade's back edge sits toward the handle side.
        k.cuboid_rot(
            (a + b) / 2.0 + Vec3::new(-d.z, 0.0, d.x).normalize_or_zero() * w * 0.4,
            Vec3::new(d.length() + 0.02, 0.025, w),
            Quat::from_rotation_y(-ang),
            blade,
        );
    }
    k.cuboid(top, Vec3::new(0.1, 0.06, 0.1), shaft);
    k
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
    /// Arrows stay stuck in the ground for a moment.
    stuck: bool,
}

/// A warning circle on the ground that fills in until something lands.
#[derive(Component)]
pub struct Warn {
    pos: Vec3,
    radius: f32,
    life: f32,
    max: f32,
    color: Color,
    /// A red laser line points down from the sky.
    beam: bool,
}

/// A mark that spins, changes size and drifts up, then goes.
#[derive(Component)]
pub struct Twirl {
    life: f32,
    max: f32,
    spin: f32,
    size: (f32, f32),
    rise: f32,
}

/// The grapple hook flying out on its rope.
#[derive(Component)]
pub struct Hook {
    from: Vec3,
    dir: Vec3,
    len: f32,
    age: f32,
}

const HOOK_SPEED: f32 = 90.0;

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

/// A plain moving bit: no spin, doesn't land.
#[allow(clippy::too_many_arguments)]
fn bit(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    mat: &Handle<StandardMaterial>,
    pos: Vec3,
    vel: Vec3,
    life: f32,
    gravity: f32,
    drag: f32,
    size: (f32, f32),
) {
    particle(
        commands,
        mesh,
        mat,
        pos,
        Particle {
            vel,
            life,
            max: life,
            gravity,
            drag,
            size,
            pop: if size.0 < size.1 { 0.15 } else { 0.0 },
            spin: Vec3::ZERO,
            lands: false,
        },
    );
}

/// Chunks that fly up, tumble and land.
fn chunks(commands: &mut Commands, a: &FxAssets, mat: &Handle<StandardMaterial>, pos: Vec3, n: usize, speed: f32, size: f32) {
    let mut rng = rand::thread_rng();
    for _ in 0..n {
        let d = rand_dir(&mut rng);
        let sz = size * rng.gen_range(0.6..1.4);
        particle(
            commands,
            &a.cube,
            mat,
            pos,
            Particle {
                vel: Vec3::new(d.x, d.y.abs() + 0.6, d.z) * speed * rng.gen_range(0.5..1.0),
                life: rng.gen_range(1.0..1.8),
                max: 1.8,
                gravity: 16.0,
                drag: 0.3,
                size: (sz, sz),
                pop: 0.0,
                spin: rand_dir(&mut rng) * 9.0,
                lands: true,
            },
        );
    }
}

/// Rock spikes bursting out of the ground at `base`.
fn rock(commands: &mut Commands, s: &SpellAssets, base: Vec3, out: Vec3, height: f32, delay: f32, life: f32) {
    let mut rng = rand::thread_rng();
    let tilt = Quat::from_rotation_arc(Vec3::Y, (Vec3::Y * 2.0 + out * 0.7).normalize());
    let w = rng.gen_range(0.8..1.3) * (0.5 + height * 0.4);
    commands.spawn((
        InGameEntity,
        Spike {
            life: life + delay,
            max: life + delay,
            scale: Vec3::new(w, height, w),
            base,
            delay,
        },
        Mesh3d(s.rock.clone()),
        MeshMaterial3d(s.solid.clone()),
        Transform::from_translation(base)
            .with_rotation(tilt * Quat::from_rotation_y(rng.gen_range(0.0..TAU)))
            .with_scale(Vec3::ZERO),
    ));
}

/// Jagged cracks running out across the ground.
fn cracks(lines: &mut Lines, pos: Vec3, n: usize, reach: f32, color: Color, life: f32) {
    let mut rng = rand::thread_rng();
    for i in 0..n {
        let ang = i as f32 / n as f32 * TAU + rng.gen_range(-0.3..0.3);
        let mut prev = pos.with_y(0.04);
        let steps = 4;
        for k in 1..=steps {
            let a = ang + rng.gen_range(-0.35..0.35);
            let q = pos.with_y(0.04) + Vec3::new(a.cos(), 0.0, a.sin()) * reach * k as f32 / steps as f32;
            lines.0.push((prev, q, color, life));
            prev = q;
        }
    }
}

/// A shield or fist hitting the ground: shockwave, cracks, dust, flying
/// rock and a ring of spikes.
#[allow(clippy::too_many_arguments)]
fn slam(
    commands: &mut Commands,
    a: &FxAssets,
    s: &SpellAssets,
    materials: &mut Assets<StandardMaterial>,
    lines: &mut Lines,
    pos: Vec3,
    radius: f32,
    color: [f32; 3],
) {
    let mut rng = rand::thread_rng();
    ring(commands, a, materials, pos, radius, color, 0.4);
    ring(commands, a, materials, pos + Vec3::Y * 0.4, radius * 0.8, [1.0, 0.9, 0.7], 0.3);
    cracks(lines, pos, 10, radius * 0.8, Color::srgb(1.0, 0.7, 0.3), 0.6);
    for _ in 0..24 {
        let ang = rng.gen_range(0.0..TAU);
        let out = Vec3::new(ang.cos(), 0.0, ang.sin());
        bit(
            commands,
            &a.ball,
            &s.dust,
            pos + out * rng.gen_range(0.5..1.5) + Vec3::Y * 0.2,
            out * radius * rng.gen_range(1.2..2.0) + Vec3::Y * rng.gen_range(0.3..1.5),
            rng.gen_range(0.7..1.2),
            -0.3,
            2.0,
            (0.3, rng.gen_range(0.9..1.5)),
        );
    }
    chunks(commands, a, &a.debris, pos + Vec3::Y * 0.2, 16, 8.0, 0.14);
    let n = (radius * 1.6) as usize + 4;
    for i in 0..n {
        let ang = i as f32 / n as f32 * TAU + rng.gen_range(-0.2..0.2);
        let out = Vec3::new(ang.cos(), 0.0, ang.sin());
        let r = radius * rng.gen_range(0.45..0.75);
        rock(commands, s, pos.with_y(0.0) + out * r, out, rng.gen_range(0.6..1.2), r / 30.0, 1.1);
    }
    flash_light(commands, pos + Vec3::Y, Color::srgb(color[0], color[1], color[2]), 250_000.0, radius * 2.5, 0.35);
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
        // --- Bulwark ---------------------------------------------------------
        A::ShieldCharge => {
            ring(commands, a, materials, pos, 2.0, color, 0.35);
            let end = pos + flat * size;
            // Gold streaks at shield height and dust kicked up all the way.
            for _ in 0..10 {
                let off = side * rng.gen_range(-0.7..0.7) + Vec3::Y * rng.gen_range(0.4..1.8);
                lines.0.push((pos + off, end + off - flat * 0.5, Color::srgba(1.0, 0.85, 0.4, 0.8), 0.25));
            }
            let n = (size * 1.5) as usize + 3;
            for i in 0..n {
                let at = pos.lerp(end, i as f32 / n as f32) + side * rng.gen_range(-0.6..0.6);
                bit(
                    commands,
                    &a.ball,
                    &s.dust,
                    at + Vec3::Y * 0.15,
                    Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(0.5..1.5), rng.gen_range(-1.0..1.0)) - flat,
                    rng.gen_range(0.6..1.1),
                    -0.2,
                    1.5,
                    (0.2, 0.7),
                );
            }
            // The bash at the end.
            let hit = end + Vec3::Y * 1.1 + flat * 0.4;
            cut(commands, s, materials, hit, flat, 0.0, 1.4, [1.0, 0.9, 0.6], 0.25);
            cut(commands, s, materials, hit - Vec3::Y * 0.4, flat, 0.25, 1.1, color, 0.3);
            sparks(commands, a, &a.spark, hit + flat * 0.4, 14, 7.0);
            flash_light(commands, hit, col, 150_000.0, 8.0, 0.25);
        }
        A::GroundPound => slam(commands, a, s, materials, lines, pos, size, color),
        A::Fortress => {
            // A pillar of gold light and rings rolling out.
            burst(
                commands,
                &a.beam,
                glow_material(materials, color, 0.5),
                Transform::from_translation(pos + Vec3::Y * 30.0),
                0.9,
                Grow::Column2(1.6),
            );
            burst(
                commands,
                &a.beam,
                a.beam_core.clone(),
                Transform::from_translation(pos + Vec3::Y * 30.0),
                0.6,
                Grow::Column2(0.6),
            );
            for i in 0..3 {
                burst(
                    commands,
                    &a.ring,
                    s.gold.clone(),
                    Transform::from_translation(pos + Vec3::Y * (0.1 + 0.8 * i as f32))
                        .with_scale(Vec3::new(0.3, 3.0, 0.3)),
                    0.5 + 0.12 * i as f32,
                    Grow::Flat(size * (1.0 - 0.15 * i as f32)),
                );
            }
            for _ in 0..30 {
                let ang = rng.gen_range(0.0..TAU);
                let r = rng.gen_range(0.4..size * 0.6);
                bit(
                    commands,
                    &a.cube,
                    &s.gold,
                    pos + Vec3::new(ang.cos() * r, 0.1, ang.sin() * r),
                    Vec3::Y * rng.gen_range(3.0..8.0),
                    rng.gen_range(0.5..1.0),
                    0.0,
                    1.0,
                    (0.05, 0.02),
                );
            }
            flash_light(commands, pos + Vec3::Y * 2.0, col, 600_000.0, 16.0, 0.6);
        }
        A::RallyCry => {
            // The war cry rolls out in waves, embers rise around everyone in
            // range, and the shout carries forward.
            for (i, life) in [0.45f32, 0.65, 0.85].into_iter().enumerate() {
                burst(
                    commands,
                    &a.ring,
                    glow_material(materials, color, 0.65 - 0.15 * i as f32),
                    Transform::from_translation(pos + Vec3::Y * (0.15 + 0.6 * i as f32))
                        .with_scale(Vec3::new(0.3, 3.0, 0.3)),
                    life,
                    Grow::Flat(size),
                );
            }
            for i in 0..4 {
                let k = i as f32;
                cut(commands, s, materials, pos + Vec3::Y * 1.6 + flat * (1.0 + k * 1.3), flat, 0.0, 0.6 + 0.45 * k, color, 0.22 + 0.06 * k);
            }
            for _ in 0..36 {
                let ang = rng.gen_range(0.0..TAU);
                let r = size * rng.gen_range(0.0f32..1.0).sqrt();
                bit(
                    commands,
                    &a.cube,
                    &s.ember,
                    pos + Vec3::new(ang.cos() * r, 0.1, ang.sin() * r),
                    Vec3::Y * rng.gen_range(2.0..5.0),
                    rng.gen_range(0.6..1.2),
                    0.0,
                    0.8,
                    (0.05, 0.02),
                );
            }
            flash_light(commands, pos + Vec3::Y * 1.5, col, 400_000.0, size * 2.0, 0.5);
        }
        A::Earthshaker => {
            if size <= 0.0 {
                slam(commands, a, s, materials, lines, pos, 4.0, color);
            } else {
                // One shockwave: a ring of rock bursting out of the ground.
                ring(commands, a, materials, pos, size, color, 0.35);
                ring(commands, a, materials, pos + Vec3::Y * 0.5, size, [1.0, 0.85, 0.6], 0.3);
                let n = (size * 2.2) as usize;
                for i in 0..n {
                    let ang = i as f32 / n as f32 * TAU + rng.gen_range(-0.1..0.1);
                    let out = Vec3::new(ang.cos(), 0.0, ang.sin());
                    let r = size * rng.gen_range(0.9..1.05);
                    rock(commands, s, pos.with_y(0.0) + out * r, out, rng.gen_range(0.8..1.6), rng.gen_range(0.0..0.08), 1.0);
                    if i % 2 == 0 {
                        bit(
                            commands,
                            &a.ball,
                            &s.dust,
                            pos + out * r + Vec3::Y * 0.3,
                            out * rng.gen_range(1.0..3.0) + Vec3::Y * rng.gen_range(0.5..1.5),
                            rng.gen_range(0.8..1.3),
                            -0.2,
                            1.5,
                            (0.4, 1.4),
                        );
                    }
                }
                chunks(commands, a, &a.debris, pos + Vec3::Y * 0.2, 6, 5.0, 0.12);
                flash_light(commands, pos + Vec3::Y, col, 150_000.0, size * 2.0, 0.3);
            }
        }

        // --- Medic -----------------------------------------------------------
        A::NeurotoxinDart => {
            // The poison arcs over to the next zombie.
            let to = pos + dir;
            let len = dir.length();
            let h = (len * 0.25).clamp(0.4, 2.0);
            let n = ((len / 0.25) as usize).clamp(6, 40);
            let mut prev = pos;
            for i in 1..=n {
                let t = i as f32 / n as f32;
                let q = pos.lerp(to, t) + Vec3::Y * 4.0 * t * (1.0 - t) * h;
                lines.0.push((prev, q, Color::srgb(0.7, 1.0, 0.25), 0.3));
                bit(
                    commands,
                    &a.ball,
                    &s.toxic,
                    q,
                    rand_dir(&mut rng) * 0.3 + Vec3::Y * 0.3,
                    0.25 + t * 0.3,
                    0.0,
                    1.0,
                    (0.09, 0.02),
                );
                prev = q;
            }
            puff(commands, a, &s.gas, to, 4, 0.5);
            sparks(commands, a, &s.toxic, to, 6, 4.0);
        }
        A::Resurrection => {
            let (w, life) = if size <= 0.0 { (2.2, 1.4) } else { (0.9, 1.0) };
            // A column of holy light, rings and drifting motes.
            burst(
                commands,
                &a.beam,
                s.holy.clone(),
                Transform::from_translation(pos + Vec3::Y * 30.0),
                life,
                Grow::Column2(w),
            );
            burst(
                commands,
                &a.beam,
                a.beam_core.clone(),
                Transform::from_translation(pos + Vec3::Y * 30.0),
                life * 0.6,
                Grow::Column2(w * 0.3),
            );
            ring(commands, a, materials, pos, w * 3.0, [1.0, 0.95, 0.7], 0.6);
            let n = if size <= 0.0 { 30 } else { 14 };
            for _ in 0..n {
                let ang = rng.gen_range(0.0..TAU);
                let r = w * rng.gen_range(0.2..1.2);
                bit(
                    commands,
                    &a.cube,
                    &s.gold,
                    pos + Vec3::new(ang.cos() * r, rng.gen_range(0.1..1.0), ang.sin() * r),
                    Vec3::Y * rng.gen_range(1.5..4.0),
                    rng.gen_range(0.8..1.4),
                    0.0,
                    0.6,
                    (0.05, 0.02),
                );
            }
            if size > 0.0 {
                // Back on their feet: crosses rise around them.
                for _ in 0..8 {
                    let ang = rng.gen_range(0.0..TAU);
                    let at = pos + Vec3::new(ang.cos() * 0.7, rng.gen_range(0.3..1.4), ang.sin() * 0.7);
                    particle(
                        commands,
                        &a.cross,
                        &a.heal,
                        at,
                        Particle {
                            vel: Vec3::Y * rng.gen_range(1.2..2.4),
                            life: rng.gen_range(0.9..1.4),
                            max: 1.4,
                            gravity: 0.0,
                            drag: 0.5,
                            size: (0.0, 1.0),
                            pop: 0.2,
                            spin: Vec3::Y * 2.0,
                            lands: false,
                        },
                    );
                }
            }
            flash_light(commands, pos + Vec3::Y * 2.0, col, if size <= 0.0 { 700_000.0 } else { 250_000.0 }, 14.0, life * 0.6);
        }
        A::MedDrone => {
            // A healing beam from the drone to a teammate's chest.
            let to = pos + dir;
            let len = dir.length().max(0.01);
            burst(
                commands,
                &a.cube,
                s.medic.clone(),
                Transform::from_translation((pos + to) / 2.0)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, dir / len))
                    .with_scale(Vec3::new(0.05, len, 0.05)),
                0.3,
                Grow::Bolt(0.05),
            );
            lines.0.push((pos, to, Color::srgba(0.5, 1.0, 0.85, 0.6), 0.2));
            for _ in 0..2 {
                particle(
                    commands,
                    &a.cross,
                    &a.heal,
                    to + rand_dir(&mut rng) * 0.3,
                    Particle {
                        vel: Vec3::Y * 1.2,
                        life: 0.7,
                        max: 0.7,
                        gravity: 0.0,
                        drag: 0.5,
                        size: (0.0, 0.6),
                        pop: 0.25,
                        spin: Vec3::Y * 2.0,
                        lands: false,
                    },
                );
            }
            sparks(commands, a, &s.medic, pos, 3, 2.0);
        }
        A::Sterilize => {
            // A wall of ultraviolet light rolling outward.
            for (i, h) in [0.1f32, 0.7, 1.4, 2.1].into_iter().enumerate() {
                burst(
                    commands,
                    &a.ring,
                    s.uv.clone(),
                    Transform::from_translation(pos + Vec3::Y * h)
                        .with_scale(Vec3::new(0.3, 4.0 - i as f32 * 0.8, 0.3)),
                    0.55,
                    Grow::Flat(size),
                );
            }
            burst(
                commands,
                &a.disc,
                glow_material(materials, color, 0.12),
                Transform::from_translation(pos + Vec3::Y * 0.05).with_scale(Vec3::new(0.3, 1.0, 0.3)),
                0.6,
                Grow::Flat(size),
            );
            for _ in 0..60 {
                let ang = rng.gen_range(0.0..TAU);
                let out = Vec3::new(ang.cos(), 0.0, ang.sin());
                bit(
                    commands,
                    &a.cube,
                    &s.uv,
                    pos + out * 0.5 + Vec3::Y * rng.gen_range(0.2..2.2),
                    out * rng.gen_range(26.0..32.0),
                    rng.gen_range(0.4..0.55),
                    0.0,
                    0.0,
                    (0.05, 0.02),
                );
            }
            flash_light(commands, pos + Vec3::Y * 1.5, col, 900_000.0, size * 2.0, 0.6);
        }

        // --- Revenant --------------------------------------------------------
        A::ScytheSweep => {
            // Crescents all the way round, on a circle centred on the player.
            for k in 0..8 {
                let ang = k as f32 / 4.0 * TAU / 2.0 + if k >= 4 { 0.4 } else { 0.0 };
                let d = Vec3::new(ang.cos(), 0.0, ang.sin());
                let r = size * if k >= 4 { 0.6 } else { 0.75 };
                let tint = if k % 2 == 0 { color } else { [0.85, 1.0, 0.95] };
                cut(commands, s, materials, pos + d * r * 0.75 + Vec3::Y * (k as f32 * 0.03 - 0.1), d, 0.0, r, tint, 0.28 + 0.03 * (k % 4) as f32);
            }
            ring(commands, a, materials, pos - Vec3::Y * 0.9, size, color, 0.3);
            for _ in 0..30 {
                let ang = rng.gen_range(0.0..TAU);
                let out = Vec3::new(ang.cos(), 0.0, ang.sin());
                let tan = Vec3::new(-ang.sin(), 0.0, ang.cos());
                bit(
                    commands,
                    &a.cube,
                    &s.teal,
                    pos + out * size * rng.gen_range(0.5..1.0) + Vec3::Y * rng.gen_range(-0.4..0.4),
                    tan * rng.gen_range(4.0..8.0) + out * 1.5,
                    rng.gen_range(0.2..0.45),
                    0.0,
                    2.0,
                    (0.05, 0.02),
                );
            }
            flash_light(commands, pos, col, 150_000.0, size * 2.0, 0.25);
        }
        A::SoulChains => {
            // Spectral chain links out to the zombie (or the end of the reach).
            let len = dir.length();
            let fwd = dir / len.max(0.01);
            let n = ((len / 0.17) as usize).clamp(3, 100);
            let hit = size > 0.0;
            let base = Quat::from_rotation_arc(Vec3::Z, fwd);
            for i in 0..n {
                let t = (i as f32 + 0.5) / n as f32;
                let twist = if i % 2 == 0 { 0.0 } else { FRAC_PI_2 };
                commands.spawn((
                    InGameEntity,
                    Mesh3d(s.link.clone()),
                    MeshMaterial3d(s.teal.clone()),
                    Transform::from_translation(pos + dir * t)
                        .with_rotation(base * Quat::from_rotation_z(twist))
                        .with_scale(Vec3::ONE),
                    NotShadowCaster,
                    Particle {
                        vel: Vec3::ZERO,
                        life: if hit { 0.55 + 0.25 * t } else { 0.3 },
                        max: if hit { 0.8 } else { 0.3 },
                        gravity: 0.0,
                        drag: 0.0,
                        size: (1.0, 0.8),
                        pop: 0.0,
                        spin: Vec3::ZERO,
                        lands: false,
                    },
                ));
            }
            lines.0.push((pos, pos + dir, Color::srgba(0.4, 1.0, 0.8, 0.5), 0.3));
            let to = pos + dir;
            if hit {
                puff(commands, a, &s.spirit, to, 6, 0.6);
                sparks(commands, a, &s.teal, to, 10, 5.0);
                flash_light(commands, to, col, 60_000.0, 6.0, 0.25);
            } else {
                puff(commands, a, &s.spirit, to, 3, 0.4);
            }
        }
        A::WraithStep => {
            // A trail of drifting mist where you passed.
            let end = pos + flat * size;
            let n = (size * 2.5) as usize + 4;
            for i in 0..n {
                let t = i as f32 / n as f32;
                bit(
                    commands,
                    &a.ball,
                    &s.wraith,
                    pos.lerp(end, t) + Vec3::Y * rng.gen_range(0.3..1.7) + side * rng.gen_range(-0.4..0.4),
                    Vec3::new(rng.gen_range(-0.3..0.3), rng.gen_range(0.2..0.6), rng.gen_range(-0.3..0.3)),
                    rng.gen_range(0.8..1.4),
                    0.0,
                    0.5,
                    (0.25, 0.9),
                );
            }
            for _ in 0..6 {
                let off = side * rng.gen_range(-0.4..0.4) + Vec3::Y * rng.gen_range(0.4..1.6);
                lines.0.push((pos + off, end + off, Color::srgba(0.7, 0.8, 1.0, 0.4), 0.35));
            }
            puff(commands, a, &s.wraith, pos + Vec3::Y, 8, 0.8);
        }
        A::ArmyOfTheDead => {
            if size <= 0.0 {
                // The call: a spectral column and mist across the ground.
                burst(
                    commands,
                    &a.beam,
                    s.spirit.clone(),
                    Transform::from_translation(pos + Vec3::Y * 30.0),
                    0.9,
                    Grow::Column2(1.2),
                );
                ring(commands, a, materials, pos, 4.0, color, 0.6);
                cracks(lines, pos, 8, 3.0, Color::srgb(0.4, 1.0, 0.8), 1.0);
                for _ in 0..14 {
                    let ang = rng.gen_range(0.0..TAU);
                    let r = rng.gen_range(0.5..3.0);
                    bit(
                        commands,
                        &a.ball,
                        &s.spirit,
                        pos + Vec3::new(ang.cos() * r, 0.2, ang.sin() * r),
                        Vec3::Y * rng.gen_range(0.3..1.0),
                        rng.gen_range(1.0..1.6),
                        0.0,
                        0.5,
                        (0.4, 1.2),
                    );
                }
                flash_light(commands, pos + Vec3::Y, col, 300_000.0, 12.0, 0.6);
            } else {
                // A warrior rising out of (or sinking back into) the ground.
                burst(
                    commands,
                    &a.beam,
                    s.spirit.clone(),
                    Transform::from_translation(pos + Vec3::Y * 30.0),
                    0.7,
                    Grow::Column2(0.55),
                );
                ring(commands, a, materials, pos, 1.3, color, 0.5);
                for _ in 0..12 {
                    let ang = rng.gen_range(0.0..TAU);
                    let r = rng.gen_range(0.1..0.6);
                    bit(
                        commands,
                        &a.cube,
                        &s.teal,
                        pos + Vec3::new(ang.cos() * r, rng.gen_range(0.0..0.5), ang.sin() * r),
                        Vec3::Y * rng.gen_range(2.0..4.0),
                        rng.gen_range(0.5..0.9),
                        0.0,
                        1.0,
                        (0.05, 0.02),
                    );
                }
                puff(commands, a, &s.spirit, pos + Vec3::Y * 0.5, 6, 0.7);
            }
        }

        // --- Demolisher ------------------------------------------------------
        A::BlastJump => {
            ring(commands, a, materials, pos, size, color, 0.35);
            // Smoke and sparks blasting up under you.
            for _ in 0..14 {
                bit(
                    commands,
                    &a.ball,
                    &a.smoke,
                    pos + Vec3::new(rng.gen_range(-0.5..0.5), 0.2, rng.gen_range(-0.5..0.5)),
                    Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(3.0..7.0), rng.gen_range(-1.0..1.0)),
                    rng.gen_range(0.8..1.4),
                    -0.3,
                    1.5,
                    (0.3, 1.1),
                );
            }
            for _ in 0..16 {
                let d = rand_dir(&mut rng);
                bit(
                    commands,
                    &a.cube,
                    &s.ember,
                    pos + Vec3::Y * 0.3,
                    Vec3::new(d.x * 3.0, rng.gen_range(6.0..12.0), d.z * 3.0),
                    rng.gen_range(0.3..0.6),
                    10.0,
                    1.0,
                    (0.05, 0.02),
                );
            }
        }
        A::Payload => {
            if size > 0.0 {
                // Warning circle and a red smoke flare where it will land.
                commands.spawn((
                    InGameEntity,
                    Warn {
                        pos,
                        radius: size,
                        life: 1.8,
                        max: 1.8,
                        color: crate::markers::DANGER,
                        beam: true,
                    },
                ));
                for i in 0..14 {
                    bit(
                        commands,
                        &a.ball,
                        &s.red,
                        pos + Vec3::Y * 0.2,
                        Vec3::new(rng.gen_range(-0.3..0.3), rng.gen_range(1.5..3.0), rng.gen_range(-0.3..0.3)),
                        1.0 + i as f32 * 0.07,
                        -0.2,
                        0.6,
                        (0.15, 0.9),
                    );
                }
            } else {
                // The bomb hits: a huge fireball, a rising stem and a
                // mushroom cloud.
                let r = 11.0;
                explosion(commands, a, materials, pos + Vec3::Y * 0.5, r, [1.0, 0.5, 0.15]);
                burst(
                    commands,
                    &a.ball,
                    glow_material(materials, [1.0, 0.75, 0.35], 0.85),
                    Transform::from_translation(pos + Vec3::Y * 2.0).with_scale(Vec3::splat(1.0)),
                    0.7,
                    Grow::Ball(6.0),
                );
                burst(
                    commands,
                    &a.ring,
                    s.dust.clone(),
                    Transform::from_translation(pos + Vec3::Y * 0.3).with_scale(Vec3::new(0.3, 8.0, 0.3)),
                    0.9,
                    Grow::Flat(r * 2.0),
                );
                for _ in 0..24 {
                    let h = rng.gen_range(0.0..7.0);
                    let off = Vec3::new(rng.gen_range(-1.0..1.0), 0.0, rng.gen_range(-1.0..1.0));
                    bit(
                        commands,
                        &a.ball,
                        if h < 3.0 { &a.fire } else { &a.smoke },
                        pos + off + Vec3::Y * h,
                        Vec3::Y * rng.gen_range(4.0..8.0),
                        rng.gen_range(1.2..2.2),
                        0.0,
                        0.8,
                        (1.0, rng.gen_range(1.8..2.6)),
                    );
                }
                for i in 0..28 {
                    let ang = i as f32 / 28.0 * TAU + rng.gen_range(-0.1..0.1);
                    let out = Vec3::new(ang.cos(), 0.0, ang.sin());
                    let fire = i % 4 == 0;
                    bit(
                        commands,
                        &a.ball,
                        if fire { &a.fire } else { &a.smoke },
                        pos + out * rng.gen_range(1.5..3.5) + Vec3::Y * rng.gen_range(8.0..11.0),
                        out * rng.gen_range(1.5..3.0) + Vec3::Y * rng.gen_range(1.0..2.5),
                        if fire { 1.2 } else { rng.gen_range(2.5..3.5) },
                        0.0,
                        0.5,
                        (1.5, rng.gen_range(3.0..4.2)),
                    );
                }
                chunks(commands, a, &a.debris, pos + Vec3::Y * 0.5, 20, 16.0, 0.22);
                flash_light(commands, pos + Vec3::Y * 4.0, col, 3_000_000.0, 60.0, 0.9);
            }
        }
        A::Claymore => {
            // A cone of shrapnel blasting out of the mine.
            let d = dir.normalize_or(Vec3::NEG_Z);
            let sd = Vec3::new(-d.z, 0.0, d.x).normalize_or(Vec3::X);
            for _ in 0..50 {
                let spread = sd * rng.gen_range(-0.75..0.75) + Vec3::Y * rng.gen_range(-0.05..0.3);
                let v = (d + spread).normalize() * rng.gen_range(25.0..45.0);
                bit(commands, &a.cube, &a.spark, pos, v, rng.gen_range(0.2..0.32), 4.0, 0.5, (0.05, 0.02));
            }
            for _ in 0..12 {
                let spread = sd * rng.gen_range(-0.75..0.75) + Vec3::Y * rng.gen_range(0.0..0.2);
                lines.0.push((pos, pos + (d + spread).normalize() * size * rng.gen_range(0.6..1.0), Color::srgb(1.0, 0.9, 0.6), 0.12));
            }
            burst(
                commands,
                &a.ball,
                glow_material(materials, [1.0, 0.65, 0.25], 0.8),
                Transform::from_translation(pos + d * 0.4).with_scale(Vec3::splat(0.2)),
                0.25,
                Grow::Ball(1.0),
            );
            puff(commands, a, &a.smoke, pos + d * 0.5, 8, 0.7);
            for _ in 0..10 {
                let spread = sd * rng.gen_range(-0.7..0.7);
                bit(
                    commands,
                    &a.ball,
                    &s.dust,
                    pos.with_y(0.15) + (d + spread) * rng.gen_range(1.0..size * 0.8),
                    Vec3::Y * rng.gen_range(0.5..1.5),
                    rng.gen_range(0.6..1.0),
                    -0.2,
                    1.0,
                    (0.2, 0.7),
                );
            }
            flash_light(commands, pos + d, col, 250_000.0, 12.0, 0.2);
        }
        A::ChainReaction => {
            // Sparks rush in and the charge flashes off.
            for _ in 0..30 {
                let ang = rng.gen_range(0.0..TAU);
                let out = Vec3::new(ang.cos(), 0.0, ang.sin());
                bit(
                    commands,
                    &a.cube,
                    &s.ember,
                    pos + out * 2.6 + Vec3::Y * rng.gen_range(0.3..1.8),
                    -out * rng.gen_range(5.0..6.0),
                    0.45,
                    0.0,
                    0.0,
                    (0.05, 0.02),
                );
            }
            ring(commands, a, materials, pos, 3.5, color, 0.5);
            burst(
                commands,
                &a.ball,
                glow_material(materials, color, 0.6),
                Transform::from_translation(pos + Vec3::Y).with_scale(Vec3::splat(0.2)),
                0.35,
                Grow::Ball(1.2),
            );
            for _ in 0..6 {
                let mut prev = pos + Vec3::Y;
                let d = rand_dir(&mut rng);
                for k in 1..4 {
                    let q = pos + Vec3::Y + d * 0.5 * k as f32 + rand_dir(&mut rng) * 0.2;
                    lines.0.push((prev, q, Color::srgb(1.0, 0.7, 0.2), 0.15));
                    prev = q;
                }
            }
            flash_light(commands, pos + Vec3::Y, col, 300_000.0, 10.0, 0.4);
        }

        // --- Chemist ---------------------------------------------------------
        A::AcidFlask => {
            // Glass shatters and acid splashes out.
            chunks(commands, a, &s.glass, pos + Vec3::Y * 0.3, 14, 5.0, 0.06);
            for _ in 0..24 {
                let ang = rng.gen_range(0.0..TAU);
                let out = Vec3::new(ang.cos(), 0.0, ang.sin());
                particle(
                    commands,
                    &a.ball,
                    &s.toxic,
                    pos + Vec3::Y * 0.2,
                    Particle {
                        vel: out * rng.gen_range(2.0..5.0) * size / 3.0 + Vec3::Y * rng.gen_range(3.0..6.0),
                        life: rng.gen_range(0.6..1.0),
                        max: 1.0,
                        gravity: 14.0,
                        drag: 0.3,
                        size: (0.12, 0.05),
                        pop: 0.0,
                        spin: Vec3::ZERO,
                        lands: true,
                    },
                );
            }
            ring(commands, a, materials, pos, size, color, 0.4);
            puff(commands, a, &s.gas, pos + Vec3::Y * 0.4, 6, 0.8);
            flash_light(commands, pos + Vec3::Y, col, 120_000.0, size * 2.5, 0.3);
        }
        A::ToxicCloud => {
            // A spray of gas and droplets.
            let d = dir.normalize_or(Vec3::NEG_Z);
            for _ in 0..45 {
                let spread = rand_dir(&mut rng) * rng.gen_range(0.0..0.3);
                bit(
                    commands,
                    &a.ball,
                    &s.gas,
                    pos + d * 0.4,
                    (d + spread).normalize() * size * rng.gen_range(1.0..1.8),
                    rng.gen_range(0.6..1.0),
                    -0.3,
                    1.8,
                    (0.1, rng.gen_range(0.8..1.5)),
                );
            }
            for _ in 0..14 {
                let spread = rand_dir(&mut rng) * 0.2;
                bit(commands, &a.ball, &s.toxic, pos + d * 0.3, (d + spread) * rng.gen_range(6.0..12.0), 0.6, 8.0, 0.5, (0.06, 0.03));
            }
            flash_light(commands, pos + d, col, 60_000.0, 6.0, 0.25);
        }
        A::Catalyst => {
            if size <= 0.0 {
                // The snap of the fingers.
                sparks(commands, a, &s.toxic, pos, 12, 5.0);
                burst(
                    commands,
                    &a.ball,
                    glow_material(materials, color, 0.7),
                    Transform::from_translation(pos).with_scale(Vec3::splat(0.05)),
                    0.2,
                    Grow::Ball(0.35),
                );
                flash_light(commands, pos, col, 60_000.0, 5.0, 0.2);
            } else {
                // A pool or cloud goes up in a toxic blast.
                burst(
                    commands,
                    &a.ball,
                    glow_material(materials, color, 0.6),
                    Transform::from_translation(pos + Vec3::Y * 0.6).with_scale(Vec3::splat(0.3)),
                    0.4,
                    Grow::Ball(size * 0.7),
                );
                ring(commands, a, materials, pos, size * 1.2, color, 0.4);
                for _ in 0..14 {
                    let d = rand_dir(&mut rng);
                    bit(
                        commands,
                        &a.ball,
                        &s.gas,
                        pos + Vec3::new(d.x, d.y.abs(), d.z) * size * 0.4,
                        Vec3::new(d.x * 2.0, rng.gen_range(1.0..3.0), d.z * 2.0),
                        rng.gen_range(1.0..1.8),
                        -0.3,
                        1.2,
                        (size * 0.2, size * rng.gen_range(0.4..0.6)),
                    );
                }
                for _ in 0..18 {
                    let d = rand_dir(&mut rng);
                    particle(
                        commands,
                        &a.ball,
                        &s.toxic,
                        pos + Vec3::Y * 0.5,
                        Particle {
                            vel: Vec3::new(d.x * 5.0, rng.gen_range(4.0..9.0), d.z * 5.0),
                            life: rng.gen_range(0.7..1.2),
                            max: 1.2,
                            gravity: 14.0,
                            drag: 0.3,
                            size: (0.12, 0.05),
                            pop: 0.0,
                            spin: Vec3::ZERO,
                            lands: true,
                        },
                    );
                }
                sparks(commands, a, &s.toxic, pos + Vec3::Y * 0.5, 16, 9.0);
                flash_light(commands, pos + Vec3::Y, col, 500_000.0, size * 3.0, 0.35);
            }
        }
        A::Petrify => {
            if size > 0.0 {
                // A wave of stone spikes rolls out across the cone.
                let ground = pos.with_y(0.0);
                let half = 0.75;
                let rows = (size / 2.0) as usize;
                for k in 1..=rows {
                    let dist = k as f32 * 2.0;
                    let count = (dist * half * 2.0 / 1.5) as usize + 1;
                    for j in 0..count {
                        let ang = -half + (j as f32 + 0.5) / count as f32 * 2.0 * half + rng.gen_range(-0.08..0.08);
                        let d = Quat::from_rotation_y(ang) * flat;
                        rock(commands, s, ground + d * dist, d, rng.gen_range(0.4..0.9), dist / 24.0, 1.2);
                    }
                }
                for _ in 0..40 {
                    let ang = rng.gen_range(-half..half);
                    let d = Quat::from_rotation_y(ang) * flat;
                    bit(
                        commands,
                        &a.ball,
                        &s.dust,
                        ground + d * 0.8 + Vec3::Y * rng.gen_range(0.2..1.2),
                        d * rng.gen_range(14.0..24.0),
                        rng.gen_range(0.7..1.1),
                        -0.2,
                        1.2,
                        (0.2, 0.9),
                    );
                }
                flash_light(commands, pos + flat * 2.0, col, 200_000.0, 14.0, 0.4);
            } else {
                // A statue shatters into shards.
                chunks(commands, a, &s.stone, pos, 18, 7.0, 0.14);
                puff(commands, a, &s.dust, pos, 6, 0.8);
                sparks(commands, a, &a.spark, pos, 6, 5.0);
                flash_light(commands, pos, col, 60_000.0, 6.0, 0.2);
            }
        }

        // --- Ranger ----------------------------------------------------------
        A::Grapple => {
            let d = dir.normalize_or(Vec3::NEG_Z);
            commands.spawn((
                InGameEntity,
                Hook {
                    from: pos,
                    dir: d,
                    len: size.max(1.0),
                    age: 0.0,
                },
                Mesh3d(s.hook.clone()),
                MeshMaterial3d(s.solid.clone()),
                Transform::from_translation(pos).looking_to(d, Vec3::Y).with_scale(Vec3::splat(1.6)),
                NotShadowCaster,
            ));
            puff(commands, a, &a.smoke, pos + d * 0.3, 3, 0.25);
        }
        A::HuntersMark => {
            if size <= 0.0 {
                cut(commands, s, materials, pos + dir * 1.2, dir, FRAC_PI_2, 0.6, color, 0.2);
                flash_light(commands, pos, col, 50_000.0, 5.0, 0.2);
            } else {
                commands.spawn((
                    InGameEntity,
                    Twirl {
                        life: 1.3,
                        max: 1.3,
                        spin: 5.0,
                        size: (1.6, 1.0),
                        rise: 0.3,
                    },
                    Mesh3d(s.mark.clone()),
                    MeshMaterial3d(s.red.clone()),
                    Transform::from_translation(pos).with_scale(Vec3::ZERO),
                    NotShadowCaster,
                ));
                ring(commands, a, materials, pos - Vec3::Y * 0.4, 0.8, color, 0.4);
            }
        }
        A::Deadeye => {
            if size <= 0.0 {
                // Lock-on: a red reticle closes in on the target.
                commands.spawn((
                    InGameEntity,
                    Twirl {
                        life: 1.0,
                        max: 1.0,
                        spin: 3.0,
                        size: (1.4, 0.55),
                        rise: 0.0,
                    },
                    Mesh3d(a.marker.clone()),
                    MeshMaterial3d(s.red.clone()),
                    Transform::from_translation(pos).with_scale(Vec3::ZERO),
                    NotShadowCaster,
                ));
            } else {
                // The sniper round lands.
                let d = dir.normalize_or(Vec3::NEG_Z);
                let from = pos - d * 30.0;
                burst(
                    commands,
                    &a.cube,
                    a.tracer.clone(),
                    Transform::from_translation((from + pos) / 2.0)
                        .with_rotation(Quat::from_rotation_arc(Vec3::Y, d))
                        .with_scale(Vec3::new(0.05, 30.0, 0.05)),
                    0.18,
                    Grow::Bolt(0.05),
                );
                burst(
                    commands,
                    &a.ball,
                    glow_material(materials, color, 0.7),
                    Transform::from_translation(pos).with_scale(Vec3::splat(0.1)),
                    0.2,
                    Grow::Ball(0.6),
                );
                sparks(commands, a, &s.red, pos, 10, 7.0);
                sparks(commands, a, &a.spark, pos, 6, 9.0);
                flash_light(commands, pos, col, 80_000.0, 6.0, 0.2);
            }
        }
        A::BearTrap => {
            // The jaws snap shut.
            let at = pos + Vec3::Y * 0.25;
            for k in [-1.0f32, 1.0] {
                cut(commands, s, materials, at + side * k * 0.2, side * -k, FRAC_PI_2, 0.5, [0.85, 0.85, 0.9], 0.18);
            }
            sparks(commands, a, &a.spark, at, 14, 5.0);
            puff(commands, a, &s.dust, pos + Vec3::Y * 0.1, 4, 0.5);
            flash_light(commands, at, col, 30_000.0, 4.0, 0.15);
        }
        A::ArrowStorm => {
            // The target area stays marked while the arrows come down.
            let life = 3.6;
            commands.spawn((
                InGameEntity,
                Warn {
                    pos,
                    radius: size,
                    life,
                    max: life,
                    color: col,
                    beam: false,
                },
            ));
            burst(
                commands,
                &a.ring,
                glow_material(materials, color, 0.5),
                Transform::from_translation(pos + Vec3::Y * 0.08).with_scale(Vec3::new(size, 2.0, size)),
                life,
                Grow::Hold,
            );
        }

        // Thrown and placed things, and Reaper and Plague Bloom, show
        // through their projectiles and zones.
        A::HealingGrenade | A::StickyBomb | A::Reaper | A::PlagueBloom => {}
    }
}

/// Something drops out of the sky onto `to` over `time` seconds. The host
/// shows the impact when it lands.
pub fn falling(commands: &mut Commands, s: &SpellAssets, kind: u8, from: Vec3, to: Vec3, time: f32) {
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
            stuck: false,
        },
        Transform::from_translation(from).with_rotation(rot),
        Visibility::default(),
        NotShadowCaster,
    ));
    e.with_children(|p| match kind {
        fall_kind::BOMB => {
            p.spawn((Mesh3d(s.bomb.clone()), MeshMaterial3d(s.solid.clone())));
            p.spawn((
                PointLight {
                    intensity: 80_000.0,
                    color: Color::srgb(1.0, 0.3, 0.15),
                    range: 10.0,
                    ..default()
                },
                Transform::from_xyz(0.0, 2.0, 0.0),
            ));
        }
        _ => {
            p.spawn((
                Mesh3d(s.arrow.clone()),
                MeshMaterial3d(s.solid.clone()),
                Transform::from_scale(Vec3::splat(1.3)),
            ));
        }
    });
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Falling things move down their line; the bomb trails smoke, arrows
/// streak and then stick in the ground for a moment.
pub fn fall(
    mut commands: Commands,
    time: Res<Time>,
    a: Res<FxAssets>,
    s: Res<SpellAssets>,
    mut lines: ResMut<Lines>,
    mut q: Query<(Entity, &mut Faller, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (e, mut f, mut tf) in &mut q {
        f.age += dt;
        let k = (f.age / f.time).min(1.0);
        let dir = (f.to - f.from).normalize_or(Vec3::NEG_Y);
        if f.kind == fall_kind::ARROW {
            if k >= 1.0 {
                if !f.stuck {
                    f.stuck = true;
                    tf.translation = f.to - dir * 0.25;
                    puff(&mut commands, &a, &s.dust, f.to + Vec3::Y * 0.1, 2, 0.35);
                }
                if f.age > f.time + 1.4 {
                    commands.entity(e).despawn();
                }
                continue;
            }
            let pos = f.from.lerp(f.to, k);
            tf.translation = pos;
            lines.0.push((pos - dir * 2.0, pos - dir * 0.6, Color::srgba(1.0, 0.95, 0.85, 0.45), 0.05));
            continue;
        }
        // The bomb speeds up as it comes down, turning slowly.
        let pos = f.from.lerp(f.to, k * k * 0.6 + k * 0.4);
        tf.translation = pos;
        tf.rotate_local_y(dt * 2.0);
        if k >= 1.0 {
            commands.entity(e).despawn();
            continue;
        }
        particle(
            &mut commands,
            &a.ball,
            &a.smoke,
            pos - dir * 2.0 + rand_dir(&mut rng) * 0.2,
            Particle {
                vel: rand_dir(&mut rng) * 0.5,
                life: 0.8,
                max: 0.8,
                gravity: 0.0,
                drag: 1.0,
                size: (0.4, 0.9),
                pop: 0.1,
                spin: Vec3::ZERO,
                lands: false,
            },
        );
        for side in [-1.0f32, 1.0] {
            let off = Vec3::new(side * 0.5, 0.0, 0.0);
            lines.0.push((pos - dir * 6.0 + off, pos - dir * 1.8 + off, Color::srgba(1.0, 1.0, 1.0, 0.35), 0.05));
        }
    }
}

/// Warning circles fill in on the ground until something lands.
pub fn warns(
    mut commands: Commands,
    time: Res<Time>,
    mut lines: ResMut<Lines>,
    mut ground: ResMut<Markers>,
    mut q: Query<(Entity, &mut Warn)>,
) {
    let dt = time.delta_secs();
    for (e, mut w) in &mut q {
        w.life -= dt;
        if w.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        ground.push(Marker::new(w.pos, Shape::Circle { radius: w.radius }, w.color).fill(1.0 - w.life / w.max));
        if w.beam {
            lines.0.push((w.pos + Vec3::Y * 0.1, w.pos + Vec3::Y * 60.0, Color::srgb(1.0, 0.25, 0.2), 0.0));
        }
    }
}

/// Marks and reticles spin, resize and drift, popping in and out.
pub fn twirls(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Twirl, &mut Transform)>) {
    let dt = time.delta_secs();
    for (e, mut t, mut tf) in &mut q {
        t.life -= dt;
        if t.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let k = 1.0 - t.life / t.max;
        let pop = (k / 0.15).min(1.0) * (t.life / 0.15).min(1.0);
        let size = t.size.0 + (t.size.1 - t.size.0) * (k * 2.0).min(1.0);
        tf.scale = Vec3::splat((size * pop).max(0.001));
        tf.rotate_y(t.spin * dt);
        tf.translation.y += t.rise * dt;
    }
}

/// The grapple hook flies out on its rope, holds, then goes.
pub fn hooks(
    mut commands: Commands,
    time: Res<Time>,
    a: Res<FxAssets>,
    mut lines: ResMut<Lines>,
    mut q: Query<(Entity, &mut Hook, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (e, mut h, mut tf) in &mut q {
        let before = h.age * HOOK_SPEED;
        h.age += dt;
        let d = (h.age * HOOK_SPEED).min(h.len);
        if h.age > h.len / HOOK_SPEED + 0.45 {
            commands.entity(e).despawn();
            continue;
        }
        tf.translation = h.from + h.dir * d;
        let rope = Color::srgb(0.75, 0.65, 0.45);
        // Slack while it flies, taut once it bites.
        let sag = if d < h.len { 0.15 * d.min(4.0) } else { 0.0 };
        let mid = h.from.lerp(tf.translation, 0.5) - Vec3::Y * sag;
        lines.0.push((h.from, mid, rope, 0.0));
        lines.0.push((mid, tf.translation, rope, 0.0));
        if before < h.len && d >= h.len {
            sparks(&mut commands, &a, &a.spark, tf.translation, 8, 4.0);
        }
    }
}

/// Trails behind ability projectiles and gadgets.
pub fn trails(
    mut commands: Commands,
    time: Res<Time>,
    a: Res<FxAssets>,
    s: Res<SpellAssets>,
    mut lines: ResMut<Lines>,
    mut last: Local<HashMap<Entity, Vec3>>,
    q: Query<(Entity, &Replicated, &GlobalTransform)>,
) {
    let mut rng = rand::thread_rng();
    let (t, dt) = (time.elapsed_secs(), time.delta_secs());
    let mut seen = Vec::new();
    for (e, r, gt) in &q {
        let NetKind::Missile(l) = r.kind else { continue };
        let pos = gt.translation();
        seen.push(e);
        let prev = last.insert(e, pos).unwrap_or(pos);
        match l {
            look::DART => lines.0.push((prev, pos, Color::srgba(0.7, 1.0, 0.3, 0.5), 0.12)),
            look::STICKY => {
                // A blinking red light.
                let phase = t * 3.0 + e.index() as f32 * 0.37;
                if phase.fract() < dt * 3.0 {
                    bit(&mut commands, &a.ball, &s.red, pos + Vec3::Y * 0.12, Vec3::ZERO, 0.15, 0.0, 0.0, (0.08, 0.06));
                }
            }
            look::MEDKIT => {
                if rng.gen_bool((dt * 12.0).min(1.0) as f64) {
                    bit(
                        &mut commands,
                        &a.cube,
                        &a.heal,
                        pos + rand_dir(&mut rng) * 0.15,
                        Vec3::Y * 0.6,
                        0.4,
                        0.0,
                        1.0,
                        (0.04, 0.0),
                    );
                }
            }
            look::FLASK => {
                if rng.gen_bool((dt * 10.0).min(1.0) as f64) {
                    bit(&mut commands, &a.ball, &s.toxic, pos, Vec3::ZERO, 0.5, 9.0, 0.0, (0.04, 0.03));
                }
            }
            look::CLAYMORE => {
                // A faint red tripwire laser.
                let fwd = gt.forward().with_y(0.0).normalize_or(Vec3::NEG_Z);
                let at = pos + Vec3::Y * 0.3;
                lines.0.push((at, at + fwd * 2.5, Color::srgba(1.0, 0.1, 0.1, 0.35), 0.0));
            }
            _ => {}
        }
    }
    last.retain(|e, _| seen.contains(e));
}
