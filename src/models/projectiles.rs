//! Models for ability projectiles and gadgets: the neurotoxin dart, sticky
//! bomb, healing canister, acid flask, bear trap, claymore, the med drone
//! and the Revenant's spectral warriors. Each is a solid part and a glowing
//! part, pointing along -Z.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

use crate::kit::{c, Kit};
use crate::sim::powers::look;

const V: fn(f32, f32, f32) -> Vec3 = Vec3::new;

/// Where the sticky bomb's blinking light sits.
pub const STICKY_LED: Vec3 = Vec3::new(0.0, 0.085, 0.02);
/// Where the claymore's sensor light sits.
pub const CLAYMORE_LED: Vec3 = Vec3::new(0.0, 0.16, -0.005);

/// (solid, glowing) parts and the light colour for a projectile look.
pub fn missile_kit(look: u8) -> (Kit, Kit, Color) {
    let (mut k, mut g) = (Kit::new(), Kit::new());
    let light = match look {
        look::DART => {
            dart(&mut k, &mut g);
            Color::srgb(0.4, 1.0, 0.3)
        }
        look::STICKY => {
            sticky(&mut k, &mut g);
            Color::srgb(1.0, 0.15, 0.1)
        }
        look::MEDKIT => {
            medkit(&mut k, &mut g);
            Color::srgb(0.35, 1.0, 0.6)
        }
        look::FLASK => {
            flask(&mut k, &mut g);
            Color::srgb(0.5, 1.0, 0.2)
        }
        look::TRAP => {
            trap(&mut k);
            Color::srgb(0.8, 0.8, 0.8)
        }
        _ => {
            claymore(&mut k, &mut g);
            Color::srgb(1.0, 0.15, 0.1)
        }
    };
    (k, g, light)
}

/// A slim syringe dart: needle, a barrel of green toxin, plunger and fletching.
fn dart(k: &mut Kit, g: &mut Kit) {
    let steel = c(0.7, 0.72, 0.76);
    let cap = c(0.85, 0.87, 0.88);
    let toxin = c(0.4, 1.0, 0.25);
    // Needle.
    k.cyl_z(V(0.0, 0.0, -0.09), 0.003, 0.07, steel);
    k.cone(V(0.0, 0.0, -0.13), 0.003, 0.012, Quat::from_rotation_x(-FRAC_PI_2), steel);
    k.frustum(V(0.0, 0.0, -0.05), 0.004, 0.012, 0.02, Quat::from_rotation_x(-FRAC_PI_2), cap);
    // Barrel: glowing toxin between two caps, with graduation rings.
    g.cyl_z(V(0.0, 0.0, 0.0), 0.011, 0.08, toxin);
    k.cyl_z(V(0.0, 0.0, -0.04), 0.0125, 0.008, cap);
    k.cyl_z(V(0.0, 0.0, 0.04), 0.0125, 0.008, cap);
    for i in 0..3 {
        k.torus(
            V(0.0, 0.0, -0.02 + i as f32 * 0.02),
            0.0015,
            0.0115,
            Quat::from_rotation_x(FRAC_PI_2),
            cap,
        );
    }
    // Plunger and flange.
    k.cyl_z(V(0.0, 0.0, 0.06), 0.004, 0.035, steel);
    k.cuboid(V(0.0, 0.0, 0.046), V(0.034, 0.005, 0.004), cap);
    // Fletching: three tufts round the tail.
    let tuft = c(0.2, 0.45, 0.25);
    for i in 0..3 {
        let a = i as f32 * TAU / 3.0 + FRAC_PI_2;
        k.cuboid_rot(
            V(a.cos() * 0.013, a.sin() * 0.013, 0.085),
            V(0.022, 0.002, 0.04),
            Quat::from_rotation_z(a),
            tuft,
        );
    }
    g.sphere(V(0.0, 0.0, -0.126), 0.004, toxin);
}

/// A sticky bomb: a lump of taped-up charge with a timer, a red light and goo.
fn sticky(k: &mut Kit, g: &mut Kit) {
    let putty = c(0.62, 0.58, 0.46);
    let tape = c(0.1, 0.1, 0.11);
    let goo = c(0.55, 0.72, 0.15);
    k.blob(V(0.0, 0.0, 0.0), V(0.09, 0.06, 0.08), putty);
    k.blob(V(0.04, -0.01, -0.03), V(0.05, 0.045, 0.05), putty);
    k.blob(V(-0.045, 0.005, 0.03), V(0.05, 0.045, 0.05), putty);
    for x in [-0.035, 0.035] {
        k.cuboid(V(x, 0.0, 0.0), V(0.02, 0.125, 0.165), tape);
    }
    // Timer box with a wire and the light.
    k.cuboid(V(0.0, 0.065, 0.0), V(0.055, 0.02, 0.07), c(0.18, 0.19, 0.2));
    g.cuboid(V(0.0, 0.076, -0.012), V(0.035, 0.003, 0.025), c(1.0, 0.25, 0.1));
    k.cyl(STICKY_LED - V(0.0, 0.006, 0.0), 0.011, 0.008, Quat::IDENTITY, tape);
    g.sphere(STICKY_LED, 0.008, c(1.0, 0.1, 0.05));
    k.capsule_between(V(0.027, 0.07, 0.02), V(0.06, 0.04, 0.05), 0.003, c(0.9, 0.15, 0.1));
    k.capsule_between(V(-0.027, 0.07, 0.02), V(-0.06, 0.03, 0.04), 0.003, c(0.2, 0.4, 0.9));
    // Goo underneath and dripping off the sides.
    k.blob(V(0.0, -0.05, 0.0), V(0.085, 0.02, 0.075), goo);
    for (x, z, len) in [(0.07, 0.02, 0.05), (-0.06, -0.04, 0.035), (0.01, 0.07, 0.045)] {
        k.capsule_tapered(V(x, -0.04, z), V(x * 1.1, -0.04 - len, z * 1.1), 0.012, 0.006, goo);
    }
}

/// A healing canister: white body, green crosses and a glowing band.
fn medkit(k: &mut Kit, g: &mut Kit) {
    let white = c(0.92, 0.93, 0.92);
    let steel = c(0.55, 0.57, 0.6);
    let green = c(0.3, 1.0, 0.55);
    k.cyl(V(0.0, 0.0, 0.0), 0.05, 0.15, Quat::IDENTITY, white);
    for y in [-0.08, 0.08] {
        k.cyl(V(0.0, y, 0.0), 0.053, 0.02, Quat::IDENTITY, steel);
    }
    k.cyl(V(0.0, 0.1, 0.0), 0.016, 0.025, Quat::IDENTITY, steel);
    k.cuboid(V(0.0, 0.11, 0.0), V(0.05, 0.008, 0.012), c(0.2, 0.6, 0.45));
    g.torus(V(0.0, -0.045, 0.0), 0.006, 0.051, Quat::IDENTITY, green);
    // A cross on each of four sides.
    for i in 0..4 {
        let a = i as f32 * FRAC_PI_2;
        let n = V(a.sin(), 0.0, a.cos());
        let rot = Quat::from_rotation_y(a);
        g.cuboid_rot(n * 0.051 + V(0.0, 0.02, 0.0), V(0.014, 0.045, 0.004), rot, green);
        g.cuboid_rot(n * 0.051 + V(0.0, 0.02, 0.0), V(0.045, 0.014, 0.004), rot, green);
    }
}

/// A round glass flask of bubbling green acid with a cork.
fn flask(k: &mut Kit, g: &mut Kit) {
    let acid = c(0.45, 1.0, 0.15);
    let glass = c(0.7, 0.85, 0.8);
    g.sphere(V(0.0, -0.01, 0.0), 0.07, acid);
    // Bubbles breaking the surface.
    for (i, (x, y, z, r)) in [
        (0.03, 0.045, -0.02, 0.014),
        (-0.04, 0.035, 0.025, 0.011),
        (0.0, 0.055, 0.03, 0.009),
        (0.05, 0.0, 0.04, 0.01),
        (-0.02, -0.03, -0.06, 0.012),
    ]
    .into_iter()
    .enumerate()
    {
        let tint = if i % 2 == 0 { c(0.8, 1.0, 0.5) } else { c(0.6, 1.0, 0.3) };
        g.sphere(V(x, y, z), r, tint);
    }
    // Glass rim round the middle, the neck, lip and cork.
    k.torus(V(0.0, -0.01, 0.0), 0.004, 0.07, Quat::IDENTITY, glass);
    k.frustum(V(0.0, 0.07, 0.0), 0.018, 0.03, 0.03, Quat::IDENTITY, glass);
    k.cyl(V(0.0, 0.1, 0.0), 0.018, 0.04, Quat::IDENTITY, glass);
    k.torus(V(0.0, 0.12, 0.0), 0.005, 0.02, Quat::IDENTITY, glass);
    k.frustum(V(0.0, 0.135, 0.0), 0.02, 0.016, 0.03, Quat::IDENTITY, c(0.6, 0.42, 0.24));
    k.cuboid(V(0.0, 0.0, -0.068), V(0.05, 0.035, 0.006), c(0.9, 0.85, 0.65));
}

/// An open steel bear trap lying flat: two jagged jaws round a trigger pan,
/// springs out to the sides and a chain. Rests with its base 0.1 m below
/// the origin (thrown things land at 0.1 m).
fn trap(k: &mut Kit) {
    let steel = c(0.38, 0.39, 0.41);
    let dark = c(0.2, 0.2, 0.22);
    let rust = c(0.45, 0.3, 0.2);
    let y = -0.09;
    let r = 0.16;
    // Base bar and trigger pan.
    k.cuboid(V(0.0, y, 0.0), V(0.62, 0.012, 0.04), dark);
    k.cyl(V(0.0, y + 0.01, 0.0), 0.06, 0.008, Quat::IDENTITY, rust);
    k.cyl(V(0.0, y + 0.016, 0.0), 0.012, 0.006, Quat::IDENTITY, steel);
    // Jaws: a ring of bars, hinged at the sides, teeth pointing up and in.
    let n = 20;
    for i in 0..n {
        let a = (i as f32 + 0.5) * TAU / n as f32;
        let p = V(a.cos() * r, y + 0.006, a.sin() * r);
        k.cuboid_rot(p, V(0.012, 0.014, r * TAU / n as f32 + 0.004), Quat::from_rotation_y(-a), steel);
        // Skip the hinge points.
        if (a.cos().abs()) > 0.93 {
            continue;
        }
        let tall = if i % 2 == 0 { 0.045 } else { 0.03 };
        let tip = Quat::from_rotation_arc(Vec3::Y, V(-a.cos() * 0.35, 1.0, -a.sin() * 0.35).normalize());
        k.cone(p + V(0.0, 0.007 + tall / 2.0, 0.0) - V(a.cos(), 0.0, a.sin()) * 0.006, 0.009, tall, tip, steel);
    }
    // Hinges and leaf springs.
    for s in [-1.0, 1.0] {
        k.cyl(V(s * r, y + 0.01, 0.0), 0.014, 0.05, Quat::from_rotation_x(FRAC_PI_2), dark);
        k.cuboid(V(s * 0.25, y + 0.012, 0.0), V(0.16, 0.008, 0.035), steel);
        k.torus(V(s * 0.3, y + 0.012, 0.0), 0.006, 0.026, Quat::from_rotation_z(FRAC_PI_2), dark);
    }
    // Chain trailing back.
    for i in 0..4 {
        let z = 0.06 + i as f32 * 0.035;
        let rot = if i % 2 == 0 {
            Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_z(FRAC_PI_2)
        } else {
            Quat::from_rotation_z(FRAC_PI_2)
        };
        k.torus(V(0.0, y + 0.005, z), 0.004, 0.018, rot, dark);
    }
    k.cyl(V(0.0, y + 0.02, 0.21), 0.012, 0.05, Quat::IDENTITY, rust);
}

/// A curved olive claymore on scissor legs, its front ("FRONT TOWARD
/// ENEMY") facing -Z, with a red sensor light. Stands on the origin.
fn claymore(k: &mut Kit, g: &mut Kit) {
    let olive = c(0.32, 0.36, 0.2);
    let dark = c(0.2, 0.22, 0.14);
    let letters = c(0.85, 0.82, 0.6);
    let rad = 0.3;
    let y = 0.11;
    let segs = 6;
    let span = 0.42;
    for i in 0..segs {
        let a = -span + (i as f32 + 0.5) * 2.0 * span / segs as f32;
        let p = V(a.sin() * rad, y, rad * (1.0 - a.cos()));
        let rot = Quat::from_rotation_y(-a);
        k.cuboid_rot(p, V(2.0 * span * rad / segs as f32 + 0.004, 0.09, 0.035), rot, olive);
        // Raised rim along the top and bottom.
        for dy in [-0.046, 0.046] {
            k.cuboid_rot(p + V(0.0, dy, 0.0), V(2.0 * span * rad / segs as f32 + 0.004, 0.006, 0.04), rot, dark);
        }
        // "FRONT TOWARD ENEMY" as two rows of little letter blocks.
        let front = p + V(-a.sin(), 0.0, -a.cos()) * 0.0185;
        for (row, w) in [(0.016, 0.012), (-0.008, 0.01)] {
            for j in 0..2 {
                let off = V(a.cos(), 0.0, a.sin()) * ((j as f32 - 0.5) * 0.016);
                k.cuboid_rot(front + off + V(0.0, row, 0.0), V(w, 0.008, 0.002), rot, letters);
            }
        }
    }
    // Fuse wells on top and the sensor.
    for x in [-0.05, 0.05] {
        k.cyl(V(x, y + 0.055, 0.0), 0.009, 0.02, Quat::IDENTITY, dark);
    }
    k.cuboid(CLAYMORE_LED - V(0.0, 0.006, -0.004), V(0.03, 0.012, 0.022), dark);
    g.sphere(CLAYMORE_LED, 0.007, c(1.0, 0.1, 0.05));
    // Scissor legs at each end.
    for s in [-1.0, 1.0] {
        let x = s * 0.09;
        let z = rad * (1.0 - (0.09f32 / rad).asin().cos()) + 0.01;
        for dz in [-0.04, 0.04] {
            k.cyl_between(V(x, y - 0.045, z), V(x + s * 0.02, 0.0, z + dz), 0.004, dark);
        }
    }
    // Wire trailing back to the detonator.
    k.capsule_between(V(0.05, y + 0.06, 0.0), V(0.07, 0.01, 0.14), 0.003, c(0.15, 0.15, 0.12));
}

/// The Medic's med drone: a white pod with teal-green trim, a green cross
/// on top and a heal emitter underneath. The rotors spin separately
/// (`rotor_kit`).
pub fn drone_kit() -> (Kit, Kit) {
    let (mut k, mut g) = (Kit::new(), Kit::new());
    let white = c(0.92, 0.93, 0.92);
    let teal = c(0.15, 0.6, 0.5);
    let dark = c(0.16, 0.17, 0.18);
    let steel = c(0.55, 0.57, 0.6);
    let glow = c(0.35, 1.0, 0.7);
    k.blob(V(0.0, 0.0, 0.0), V(0.2, 0.09, 0.26), white);
    k.cuboid(V(0.0, -0.045, 0.0), V(0.22, 0.04, 0.28), teal);
    for i in 0..4 {
        let a = i as f32 * TAU / 4.0 + TAU / 8.0;
        let tip = V(a.cos() * 0.36, 0.03, a.sin() * 0.36);
        k.cyl_between(V(0.0, 0.0, 0.0), tip, 0.022, white);
        k.cyl(tip, 0.045, 0.06, Quat::IDENTITY, teal);
        k.cyl(tip + V(0.0, 0.035, 0.0), 0.012, 0.02, Quat::IDENTITY, steel);
    }
    // Green cross on top.
    g.cuboid(V(0.0, 0.088, 0.0), V(0.04, 0.008, 0.13), glow);
    g.cuboid(V(0.0, 0.088, 0.0), V(0.13, 0.008, 0.04), glow);
    // Heal emitter underneath: a lens in a ring.
    k.cyl(V(0.0, -0.075, 0.0), 0.07, 0.025, Quat::IDENTITY, dark);
    g.torus(V(0.0, -0.09, 0.0), 0.008, 0.06, Quat::IDENTITY, glow);
    g.sphere(V(0.0, -0.09, 0.0), 0.035, c(0.7, 1.0, 0.85));
    // Sensor eye and trim lights.
    g.sphere(V(0.0, 0.0, -0.25), 0.035, glow);
    for s in [-1.0, 1.0] {
        g.cuboid(V(s * 0.19, -0.02, 0.0), V(0.012, 0.012, 0.14), glow);
    }
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

/// The Revenant's spectral warrior: a hooded, armoured ghost-knight with a
/// blade, about 1.8 m tall, feet at the origin, facing -Z. No legs: the
/// robe fades into glowing wisps trailing behind.
pub fn wraith_kit() -> (Kit, Kit) {
    let (mut k, mut g) = (Kit::new(), Kit::new());
    let armour = c(0.1, 0.11, 0.13);
    let plate = c(0.16, 0.18, 0.2);
    let cloak = c(0.05, 0.06, 0.07);
    let void = c(0.01, 0.01, 0.015);
    let teal = c(0.3, 1.0, 0.8);
    let pale = c(0.6, 1.0, 0.9);

    // Robe from the waist down, tattered strips, then wisps.
    k.frustum(V(0.0, 0.78, 0.02), 0.2, 0.3, 0.5, Quat::IDENTITY, cloak);
    for i in 0..9 {
        let a = i as f32 / 9.0 * TAU;
        let top = V(a.sin() * 0.26, 0.56, a.cos() * 0.26 + 0.02);
        let len = 0.18 + 0.08 * ((i * 5) % 3) as f32;
        let bot = top + V(a.sin() * 0.05, -len, a.cos() * 0.05 + 0.08);
        k.beam(top, bot, Vec2::new(0.09, 0.02), cloak);
        g.capsule_tapered(bot, bot + V(0.0, -0.08, 0.06), 0.02, 0.006, teal);
    }
    for (i, x) in [-0.14f32, -0.05, 0.05, 0.14].into_iter().enumerate() {
        let mut p = V(x, 0.55, 0.0);
        let mut r = 0.07;
        for j in 0..4 {
            let sway = if (i + j) % 2 == 0 { 0.04 } else { -0.04 };
            let next = p + V(sway, -0.12, 0.09 + 0.03 * j as f32);
            g.capsule_tapered(p, next, r, r * 0.7, if j == 0 { pale } else { teal });
            p = next;
            r *= 0.7;
        }
    }
    g.torus(V(0.0, 0.54, 0.02), 0.012, 0.3, Quat::IDENTITY, teal);

    // Torso: armoured chest over a dark tunic.
    k.blob(V(0.0, 1.08, 0.0), V(0.19, 0.16, 0.13), armour);
    k.blob(V(0.0, 1.3, -0.01), V(0.23, 0.2, 0.15), plate);
    k.cuboid(V(0.0, 1.03, 0.0), V(0.4, 0.05, 0.26), armour);
    g.cuboid(V(0.0, 1.03, -0.13), V(0.36, 0.012, 0.012), teal);
    g.cuboid(V(0.0, 1.07, -0.135), V(0.05, 0.05, 0.01), pale);
    // A glowing rune on the breastplate.
    g.cuboid(V(0.0, 1.31, -0.155), V(0.014, 0.14, 0.01), teal);
    g.cuboid(V(0.0, 1.34, -0.155), V(0.1, 0.012, 0.01), teal);
    g.cuboid_rot(V(0.0, 1.26, -0.15), V(0.06, 0.06, 0.008), Quat::from_rotation_z(PI / 4.0), teal);
    // Cloak down the back.
    k.beam(V(0.0, 1.5, 0.12), V(0.0, 0.62, 0.3), Vec2::new(0.46, 0.03), cloak);

    // Pauldrons with glowing edges.
    for s in [-1.0, 1.0] {
        let sh = V(s * 0.26, 1.46, 0.0);
        k.blob(sh, V(0.13, 0.08, 0.13), plate);
        k.blob(sh + V(s * 0.02, -0.05, 0.0), V(0.12, 0.05, 0.12), armour);
        g.torus(sh + V(s * 0.015, -0.07, 0.0), 0.008, 0.115, Quat::from_rotation_z(s * 0.35), teal);
        k.cone(sh + V(s * 0.04, 0.09, 0.0), 0.03, 0.1, Quat::from_rotation_z(-s * 0.5), armour);
    }

    // Arms: the left hangs down, the right holds the blade out front.
    let l_sh = V(-0.27, 1.4, 0.0);
    let l_el = V(-0.32, 1.12, 0.03);
    let l_ha = V(-0.3, 0.88, -0.03);
    k.capsule_tapered(l_sh, l_el, 0.065, 0.055, armour);
    k.capsule_tapered(l_el, l_ha, 0.055, 0.05, plate);
    k.sphere(l_ha, 0.055, armour);
    g.torus(l_el + (l_ha - l_el) * 0.7, 0.008, 0.056, Quat::from_rotation_arc(Vec3::Y, (l_ha - l_el).normalize()), teal);
    let r_sh = V(0.27, 1.4, 0.0);
    let r_el = V(0.34, 1.15, -0.12);
    let r_ha = V(0.28, 1.05, -0.34);
    k.capsule_tapered(r_sh, r_el, 0.065, 0.055, armour);
    k.capsule_tapered(r_el, r_ha, 0.055, 0.05, plate);
    k.sphere(r_ha, 0.055, armour);
    g.torus(r_el + (r_ha - r_el) * 0.7, 0.008, 0.056, Quat::from_rotation_arc(Vec3::Y, (r_ha - r_el).normalize()), teal);

    // The blade: held point up and forward, with a glowing edge.
    let up = V(0.04, 0.85, -0.5).normalize();
    let hilt = r_ha;
    k.cyl_between(hilt - up * 0.14, hilt + up * 0.05, 0.016, armour);
    k.sphere(hilt - up * 0.15, 0.025, plate);
    let side = up.cross(Vec3::Z).normalize();
    g.beam(hilt + up * 0.06 - side * 0.12, hilt + up * 0.06 + side * 0.12, Vec2::new(0.025, 0.025), teal);
    let tip = hilt + up * 1.0;
    k.beam(hilt + up * 0.07, tip - up * 0.06, Vec2::new(0.07, 0.014), plate);
    g.beam(hilt + up * 0.08 + side * 0.04, tip - up * 0.04, Vec2::new(0.01, 0.016), pale);
    g.beam(hilt + up * 0.08 - side * 0.04, tip - up * 0.04, Vec2::new(0.01, 0.016), teal);
    g.cone(tip - up * 0.03, 0.035, 0.08, Quat::from_rotation_arc(Vec3::Y, up), pale);

    // Neck, head and hood, eyes burning in the dark of the hood.
    g.torus(V(0.0, 1.5, 0.0), 0.012, 0.09, Quat::IDENTITY, teal);
    k.cyl(V(0.0, 1.52, 0.0), 0.07, 0.06, Quat::IDENTITY, armour);
    k.blob(V(0.0, 1.66, 0.04), V(0.16, 0.18, 0.17), cloak);
    k.cone(V(0.0, 1.84, 0.1), 0.08, 0.14, Quat::from_rotation_x(0.6), cloak);
    k.blob(V(0.0, 1.63, -0.07), V(0.1, 0.12, 0.06), void);
    k.cuboid(V(0.0, 1.57, -0.115), V(0.11, 0.05, 0.02), plate);
    for s in [-1.0, 1.0] {
        g.blob(V(s * 0.04, 1.65, -0.125), V(0.022, 0.012, 0.01), pale);
    }
    g.torus(V(0.0, 1.64, -0.1), 0.007, 0.13, Quat::from_rotation_x(FRAC_PI_2 - 0.25), teal);
    (k, g)
}
