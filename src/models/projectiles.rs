//! Models for ability projectiles and gadgets: Iaido's crescent cut, kunai,
//! rockets, fireballs, the cryo orb, bomblets, smoke and grav grenades,
//! proximity mines and the combat drone. Each is a solid part and a glowing
//! part, pointing along -Z.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, TAU};

use crate::kit::{c, Kit};
use crate::sim::powers::look;

const V: fn(f32, f32, f32) -> Vec3 = Vec3::new;

/// (solid, glowing) parts and the light colour for a projectile look.
pub fn missile_kit(look: u8) -> (Kit, Kit, Color) {
    let (mut k, mut g) = (Kit::new(), Kit::new());
    let light = match look {
        0..=2 => {
            crescent(&mut g, 1.0 + 0.45 * look as f32);
            Color::srgb(1.0, 0.8, 0.35)
        }
        look::KUNAI => {
            let mut kunai = Kit::new();
            kunai_kit(&mut kunai);
            k.append(kunai, Transform::from_scale(Vec3::splat(0.7)));
            g.cuboid(V(0.0, 0.0, -0.25), V(0.01, 0.01, 0.35), c(1.0, 0.35, 0.35));
            Color::srgb(1.0, 0.3, 0.3)
        }
        look::ROCKET => {
            let body = c(0.35, 0.37, 0.4);
            let red = c(0.8, 0.15, 0.1);
            k.cyl_z(V(0.0, 0.0, 0.0), 0.05, 0.42, body);
            k.cone(V(0.0, 0.0, -0.27), 0.05, 0.12, Quat::from_rotation_x(-FRAC_PI_2), red);
            k.cyl_z(V(0.0, 0.0, -0.12), 0.052, 0.04, red);
            for i in 0..4 {
                let a = i as f32 * TAU / 4.0;
                k.cuboid_rot(
                    V(a.cos() * 0.07, a.sin() * 0.07, 0.17),
                    V(0.07, 0.008, 0.1),
                    Quat::from_rotation_z(a),
                    body,
                );
            }
            g.cyl_z(V(0.0, 0.0, 0.23), 0.035, 0.04, c(1.0, 0.85, 0.5));
            g.cone(V(0.0, 0.0, 0.33), 0.04, 0.18, Quat::from_rotation_x(FRAC_PI_2), c(1.0, 0.55, 0.15));
            Color::srgb(1.0, 0.55, 0.2)
        }
        look::FIREBALL => {
            g.sphere(Vec3::ZERO, 0.28, c(1.0, 0.95, 0.6));
            for i in 0..14 {
                let a = i as f32 * 2.4;
                let y = (i as f32 / 13.0) * 2.0 - 1.0;
                let r = (1.0 - y * y).sqrt();
                let d = V(a.cos() * r, y, a.sin() * r);
                let back = d.z.max(0.0);
                g.blob(
                    d * 0.28 + V(0.0, 0.0, back * 0.25),
                    Vec3::splat(0.18 + 0.12 * back),
                    if i % 2 == 0 { c(1.0, 0.5, 0.1) } else { c(1.0, 0.3, 0.05) },
                );
            }
            Color::srgb(1.0, 0.5, 0.15)
        }
        look::CRYO => {
            g.sphere(Vec3::ZERO, 0.22, c(0.85, 0.97, 1.0));
            let ice = c(0.6, 0.85, 1.0);
            for i in 0..12 {
                let a = i as f32 * 2.4;
                let y = (i as f32 / 11.0) * 2.0 - 1.0;
                let r = (1.0 - y * y).sqrt();
                let d = V(a.cos() * r, y, a.sin() * r);
                k.cone(d * 0.28, 0.07, 0.32, Quat::from_rotation_arc(Vec3::Y, d), ice);
            }
            g.torus(Vec3::ZERO, 0.015, 0.42, Quat::from_rotation_x(0.6), c(0.5, 0.85, 1.0));
            g.torus(Vec3::ZERO, 0.015, 0.42, Quat::from_rotation_z(0.9), c(0.5, 0.85, 1.0));
            Color::srgb(0.5, 0.85, 1.0)
        }
        look::BOMBLET => {
            k.sphere(Vec3::ZERO, 0.08, c(0.25, 0.27, 0.22));
            g.torus(Vec3::ZERO, 0.012, 0.08, Quat::IDENTITY, c(1.0, 0.3, 0.1));
            Color::srgb(1.0, 0.4, 0.15)
        }
        look::SMOKE => {
            k.cyl(Vec3::ZERO, 0.045, 0.16, Quat::IDENTITY, c(0.3, 0.32, 0.3));
            k.cyl(V(0.0, 0.09, 0.0), 0.02, 0.03, Quat::IDENTITY, c(0.6, 0.6, 0.6));
            g.cyl(V(0.0, 0.03, 0.0), 0.047, 0.025, Quat::IDENTITY, c(0.7, 0.6, 0.9));
            Color::srgb(0.6, 0.55, 0.75)
        }
        _ => {
            // Grav grenade: a dark core in spinning violet rings.
            k.sphere(Vec3::ZERO, 0.07, c(0.08, 0.06, 0.12));
            g.torus(Vec3::ZERO, 0.01, 0.1, Quat::from_rotation_x(FRAC_PI_2), c(0.75, 0.4, 1.0));
            g.torus(Vec3::ZERO, 0.01, 0.1, Quat::from_rotation_z(FRAC_PI_2), c(0.75, 0.4, 1.0));
            g.sphere(Vec3::ZERO, 0.035, c(0.95, 0.85, 1.0));
            Color::srgb(0.75, 0.4, 1.0)
        }
    };
    (k, g, light)
}

/// A curved blade of light, bulging forward, thick in the middle and
/// tapering to points at the tips.
fn crescent(g: &mut Kit, scale: f32) {
    let r = 1.0 * scale;
    let n = 20;
    let span = 1.25;
    for layer in 0..2 {
        let (thick, color) = if layer == 0 {
            (0.26, c(1.0, 0.72, 0.25))
        } else {
            (0.11, c(1.0, 0.98, 0.9))
        };
        for i in 0..n {
            let a0 = -span + 2.0 * span * i as f32 / n as f32;
            let a1 = -span + 2.0 * span * (i + 1) as f32 / n as f32;
            let am = (a0 + a1) / 2.0;
            let taper = 1.0 - (am / span).powi(2);
            let p = |a: f32| V(a.sin() * r, 0.0, -a.cos() * r + r * 0.75);
            let (pa, pb) = (p(a0), p(a1));
            let mid = (pa + pb) / 2.0;
            let len = pa.distance(pb) + 0.02;
            let w = (thick * scale * taper).max(0.015);
            // Thicker toward the inside of the curve, like a real cut.
            let inward = -V(am.sin(), 0.0, -am.cos());
            g.cuboid_rot(
                mid + inward * w * 0.3 + V(0.0, 0.004 * layer as f32, 0.0),
                V(len, 0.03 + 0.02 * layer as f32, w),
                Quat::from_rotation_y(-am),
                color,
            );
        }
    }
}

/// A kunai: leaf blade, cord-wrapped handle and ring, pointing along -Z.
fn kunai_kit(k: &mut Kit) {
    let steel = c(0.55, 0.57, 0.62);
    k.blob(V(0.0, 0.0, -0.32), V(0.11, 0.018, 0.3), steel);
    k.blob(V(0.0, 0.0, -0.32), V(0.125, 0.008, 0.31), c(0.95, 0.95, 1.0));
    k.cyl_z(V(0.0, 0.0, 0.06), 0.025, 0.2, c(0.1, 0.08, 0.08));
    for i in 0..4 {
        k.torus(
            V(0.0, 0.0, -0.01 + i as f32 * 0.04),
            0.008,
            0.026,
            Quat::from_rotation_x(FRAC_PI_2),
            c(0.5, 0.08, 0.1),
        );
    }
    k.torus(V(0.0, 0.0, 0.2), 0.012, 0.045, Quat::IDENTITY, steel);
}

/// Tinker's proximity mine: a squat disc on three legs with a red eye.
pub fn mine_kit() -> (Kit, Kit) {
    let (mut k, mut g) = (Kit::new(), Kit::new());
    let dark = c(0.2, 0.22, 0.2);
    let yellow = c(0.95, 0.75, 0.12);
    k.cyl(V(0.0, 0.06, 0.0), 0.2, 0.08, Quat::IDENTITY, dark);
    k.cyl(V(0.0, 0.105, 0.0), 0.16, 0.02, Quat::IDENTITY, yellow);
    for i in 0..3 {
        let a = i as f32 * TAU / 3.0;
        k.cyl_between(V(a.cos() * 0.15, 0.05, a.sin() * 0.15), V(a.cos() * 0.26, 0.0, a.sin() * 0.26), 0.012, dark);
    }
    for i in 0..8 {
        let a = i as f32 * TAU / 8.0;
        k.cuboid_rot(V(a.cos() * 0.2, 0.06, a.sin() * 0.2), V(0.03, 0.05, 0.02), Quat::from_rotation_y(-a), c(0.1, 0.1, 0.1));
    }
    g.sphere(V(0.0, 0.13, 0.0), 0.04, c(1.0, 0.15, 0.1));
    (k, g)
}

/// Tinker's combat drone body: a pod with four arms, a gun underneath and
/// a sensor eye. The rotors spin separately (`rotor_kit`).
pub fn drone_kit() -> (Kit, Kit) {
    let (mut k, mut g) = (Kit::new(), Kit::new());
    let yellow = c(0.95, 0.75, 0.12);
    let dark = c(0.16, 0.16, 0.18);
    let steel = c(0.55, 0.57, 0.6);
    k.blob(V(0.0, 0.0, 0.0), V(0.2, 0.09, 0.26), yellow);
    k.cuboid(V(0.0, -0.05, 0.0), V(0.24, 0.05, 0.3), dark);
    for i in 0..4 {
        let a = i as f32 * TAU / 4.0 + TAU / 8.0;
        let tip = V(a.cos() * 0.36, 0.03, a.sin() * 0.36);
        k.cyl_between(V(0.0, 0.0, 0.0), tip, 0.022, dark);
        k.cyl(tip, 0.045, 0.06, Quat::IDENTITY, steel);
    }
    // Gun under the nose.
    k.cuboid(V(0.0, -0.11, -0.08), V(0.07, 0.06, 0.16), dark);
    k.cyl_z(V(0.0, -0.11, -0.22), 0.016, 0.14, steel);
    g.sphere(V(0.0, 0.0, -0.25), 0.045, c(0.3, 1.0, 0.85));
    g.cuboid(V(0.0, 0.065, 0.0), V(0.16, 0.012, 0.02), c(0.3, 1.0, 0.85));
    (k, g)
}

/// One spinning rotor (blurred into a faint disc with two blades).
pub fn rotor_kit() -> Kit {
    let mut k = Kit::new();
    k.cuboid(V(0.0, 0.0, 0.0), V(0.26, 0.008, 0.03), c(0.12, 0.12, 0.13));
    k.cuboid(V(0.0, 0.0, 0.0), V(0.03, 0.008, 0.26), c(0.12, 0.12, 0.13));
    k
}

/// Where the drone's four rotors sit.
pub fn rotor_spots() -> [Vec3; 4] {
    std::array::from_fn(|i| {
        let a = i as f32 * TAU / 4.0 + TAU / 8.0;
        V(a.cos() * 0.36, 0.07, a.sin() * 0.36)
    })
}
