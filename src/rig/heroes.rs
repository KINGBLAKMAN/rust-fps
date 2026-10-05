//! The playable characters' outfits, built on the shared body.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use super::{base_body, sx, v, Bone, Palette, Parts};
use crate::data::Character;
use crate::kit::{c, Kit};

/// Bone structure for a bare face, so a small head doesn't read as a ball:
/// a brow ridge, cheekbones, a jaw line and chin. `eye_y` is the eye line,
/// `front` how far forward the face is, `w` its half width.
fn face_shape(p: &mut Parts, eye_y: f32, front: f32, w: f32, skin: Color) {
    let n = Bone::Neck;
    let shade = {
        let l = skin.to_srgba();
        c(l.red * 0.92, l.green * 0.9, l.blue * 0.9)
    };
    p.k(n).blob(
        v(0.0, eye_y + 0.022, front + 0.012),
        v(w * 0.85, 0.014, 0.022),
        skin,
    );
    for x in [-1.0f32, 1.0] {
        p.k(n).blob(
            v(x * w * 0.55, eye_y - 0.03, front + 0.018),
            v(0.028, 0.018, 0.022),
            skin,
        );
        // Jaw line from below the ear to the chin.
        p.k(n).beam(
            v(x * w * 0.85, eye_y - 0.04, front + 0.06),
            v(x * w * 0.3, eye_y - 0.1, front + 0.022),
            Vec2::new(0.022, 0.026),
            shade,
        );
    }
    p.k(n).blob(
        v(0.0, eye_y - 0.105, front + 0.022),
        v(0.026, 0.02, 0.02),
        skin,
    );
    // Nose bridge.
    p.k(n).wedge(
        v(0.0, eye_y - 0.03, front - 0.004),
        v(0.022, 0.045, 0.018),
        Quat::from_rotation_x(-FRAC_PI_2) * Quat::from_rotation_z(PI),
        skin,
    );
}

/// Eyes with whites, irises and brows. `z` is the middle of the eyeballs.
fn eyes(p: &mut Parts, y: f32, z: f32, gap: f32, iris: Color, brow: Color) {
    let n = Bone::Neck;
    for x in [-gap, gap] {
        p.k(n).sphere(v(x, y, z), 0.012, c(0.95, 0.95, 0.97));
        p.k(n).sphere(v(x, y, z - 0.007), 0.007, iris);
        p.k(n).cuboid_rot(
            v(x, y + 0.024, z - 0.012),
            v(0.034, 0.008, 0.01),
            Quat::from_rotation_z(-x.signum() * 0.12),
            brow,
        );
    }
}

/// A short cylinder facing along `dir` (lenses, filters, gauges, caps).
fn disc(k: &mut Kit, at: Vec3, dir: Vec3, r: f32, depth: f32, col: Color) {
    k.cyl(
        at,
        r,
        depth,
        Quat::from_rotation_arc(Vec3::Y, dir.normalize()),
        col,
    );
}

/// A hose or cable through the given points.
fn hose(k: &mut Kit, pts: &[Vec3], r: f32, col: Color) {
    for w in pts.windows(2) {
        k.capsule_between(w[0], w[1], r, col);
    }
}

/// A small hash in 0..1, for ragged edges that don't repeat.
fn jitter(i: usize) -> f32 {
    ((i * 37 + 11) % 13) as f32 / 13.0
}

pub(super) fn bulwark(p: &mut Parts) {
    let steel = Character::Bulwark.suit_color();
    let gold = Character::Bulwark.trim_color();
    let plate = c(0.44, 0.47, 0.52);
    let dark = c(0.13, 0.14, 0.16);
    let slit = c(0.02, 0.02, 0.03);
    base_body(
        p,
        &Palette {
            skin: c(0.7, 0.52, 0.4),
            suit: c(0.2, 0.22, 0.25),
            suit_dark: c(0.16, 0.17, 0.19),
            glove: dark,
            boot: c(0.1, 0.1, 0.11),
        },
        1.12,
    );
    let s = Bone::Spine;
    // Heavy cuirass: a rounded chest with a flat front plate and a ridge.
    p.k(s).blob(v(0.0, 0.34, 0.0), v(0.29, 0.23, 0.19), steel);
    p.k(s).cuboid(v(0.0, 0.36, -0.165), v(0.38, 0.26, 0.06), plate);
    p.k(s).cuboid(v(0.0, 0.36, -0.198), v(0.03, 0.24, 0.02), steel);
    for y in [0.495, 0.225] {
        p.k(s).cuboid(v(0.0, y, -0.168), v(0.4, 0.022, 0.07), gold);
    }
    // Gold chevron and rivets.
    for x in [-1.0f32, 1.0] {
        p.k(s).cuboid_rot(
            v(x * 0.045, 0.37, -0.2),
            v(0.11, 0.024, 0.014),
            Quat::from_rotation_z(-x * 0.6),
            gold,
        );
        for y in [0.46, 0.26] {
            p.k(s).sphere(v(x * 0.16, y, -0.197), 0.012, gold);
        }
    }
    // Segmented belly plates.
    for i in 0..3 {
        let f = i as f32;
        p.k(s).cuboid_rot(
            v(0.0, 0.17 - f * 0.055, -0.135 + f * 0.006),
            v(0.34 - f * 0.025, 0.05, 0.05),
            Quat::from_rotation_x(-0.1),
            if i == 1 { plate } else { steel },
        );
    }
    // Back plate with two ridges, a high gorget and a belt.
    p.k(s).cuboid(v(0.0, 0.34, 0.15), v(0.36, 0.32, 0.06), steel);
    for x in [-0.09, 0.09] {
        p.k(s).cuboid(v(x, 0.34, 0.183), v(0.03, 0.28, 0.02), plate);
    }
    p.k(s).cuboid(v(0.0, 0.18, 0.18), v(0.36, 0.022, 0.03), gold);
    p.k(s).frustum(
        v(0.0, 0.52, 0.0),
        0.125,
        0.17,
        0.1,
        Quat::IDENTITY,
        steel,
    );
    p.k(s).torus(v(0.0, 0.48, 0.0), 0.012, 0.172, Quat::IDENTITY, gold);
    let pv = Bone::Pelvis;
    p.k(pv).cyl(v(0.0, 0.05, 0.0), 0.21, 0.07, Quat::IDENTITY, dark);
    p.k(pv).cuboid(v(0.0, 0.05, -0.215), v(0.08, 0.06, 0.02), gold);
    // Hip plates (tassets) front and back.
    for x in [-1.0f32, 1.0] {
        p.k(pv).cuboid_rot(
            v(x * 0.1, -0.11, -0.16),
            v(0.15, 0.17, 0.03),
            Quat::from_rotation_x(0.15) * Quat::from_rotation_z(x * 0.1),
            steel,
        );
        p.k(pv).cuboid_rot(
            v(x * 0.1, -0.19, -0.175),
            v(0.15, 0.02, 0.035),
            Quat::from_rotation_x(0.15) * Quat::from_rotation_z(x * 0.1),
            gold,
        );
        p.k(pv).cuboid_rot(
            v(x * 0.19, -0.09, 0.0),
            v(0.03, 0.15, 0.17),
            Quat::from_rotation_z(x * 0.2),
            plate,
        );
    }
    p.k(pv).cuboid_rot(
        v(0.0, -0.1, 0.16),
        v(0.26, 0.16, 0.03),
        Quat::from_rotation_x(-0.15),
        steel,
    );
    // Full helmet: dome, square face plate, dark visor slit, breathing vents.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.19, 0.01), v(0.165, 0.17, 0.175), steel);
    p.k(n).cuboid(v(0.0, 0.335, 0.02), v(0.04, 0.05, 0.28), gold);
    p.k(n).cuboid(v(0.0, 0.15, -0.135), v(0.27, 0.2, 0.08), plate);
    for x in [-1.0f32, 1.0] {
        p.k(n).cuboid_rot(
            v(x * 0.095, 0.07, -0.125),
            v(0.1, 0.08, 0.08),
            Quat::from_rotation_z(x * 0.4),
            plate,
        );
        // Ear plates with a gold ring.
        disc(p.k(n), v(x * 0.17, 0.17, 0.0), Vec3::X, 0.06, 0.04, plate);
        p.k(n).torus(
            v(x * 0.19, 0.17, 0.0),
            0.008,
            0.045,
            Quat::from_rotation_z(FRAC_PI_2),
            gold,
        );
    }
    p.k(n).cuboid(v(0.0, 0.19, -0.178), v(0.23, 0.035, 0.014), slit);
    p.k(n).cuboid(v(0.0, 0.226, -0.178), v(0.27, 0.018, 0.016), gold);
    for i in 0..3 {
        let f = i as f32;
        p.k(n).cuboid(
            v(0.0, 0.135 - f * 0.025, -0.177),
            v(0.1 - f * 0.02, 0.01, 0.01),
            slit,
        );
    }
    p.k(n).cuboid_rot(
        v(0.0, 0.07, 0.13),
        v(0.26, 0.1, 0.05),
        Quat::from_rotation_x(-0.3),
        steel,
    );
    for side in 0..2 {
        let x = sx(side);
        // Big layered pauldrons with gold rims.
        let u = Bone::UpperArm(side);
        p.k(u).blob(v(x * 0.04, 0.03, 0.0), v(0.15, 0.1, 0.15), steel);
        p.k(u).torus(v(x * 0.04, 0.0, 0.0), 0.012, 0.145, Quat::IDENTITY, gold);
        for j in 0..2 {
            let f = j as f32;
            p.k(u).cuboid_rot(
                v(x * (0.11 + f * 0.01), -0.07 - f * 0.05, 0.0),
                v(0.03, 0.06, 0.2 - f * 0.03),
                Quat::from_rotation_z(x * 0.3),
                if j == 0 { plate } else { steel },
            );
        }
        p.k(u).cuboid_rot(
            v(x * 0.125, -0.125, 0.0),
            v(0.032, 0.015, 0.17),
            Quat::from_rotation_z(x * 0.3),
            gold,
        );
        p.k(u).cyl(v(0.0, -0.18, 0.0), 0.068, 0.12, Quat::IDENTITY, steel);
        p.k(u).sphere(v(0.0, -0.3, 0.02), 0.06, plate);
        // Vambraces and armoured gauntlets.
        let f = Bone::Forearm(side);
        p.k(f).frustum(
            v(0.0, -0.13, 0.0),
            0.058,
            0.07,
            0.17,
            Quat::IDENTITY,
            steel,
        );
        p.k(f).torus(v(0.0, -0.05, 0.0), 0.01, 0.07, Quat::IDENTITY, gold);
        p.k(f).frustum(
            v(0.0, -0.235, 0.0),
            0.06,
            0.05,
            0.05,
            Quat::IDENTITY,
            plate,
        );
        let h = Bone::Hand(side);
        p.k(h).cuboid(v(0.0, -0.06, 0.022), v(0.08, 0.06, 0.014), plate);
        p.k(h).cuboid(v(0.0, -0.035, 0.03), v(0.06, 0.012, 0.01), gold);
        if side == 0 {
            // Riot shield strapped to the left forearm: steel, a gold
            // frame, a viewport window near the top and a chevron.
            let sh = v(x * 0.11, -0.12, 0.0);
            let out = v(x, 0.0, 0.0);
            p.k(f).cuboid(sh, v(0.03, 0.66, 0.42), steel);
            p.k(f).cuboid(sh + out * 0.018, v(0.01, 0.6, 0.36), plate);
            for y in [0.32, -0.32] {
                p.k(f)
                    .cuboid(sh + out * 0.02 + v(0.0, y, 0.0), v(0.022, 0.03, 0.43), gold);
            }
            for z in [-0.2, 0.2] {
                p.k(f)
                    .cuboid(sh + out * 0.02 + v(0.0, 0.0, z), v(0.022, 0.67, 0.03), gold);
            }
            let win = sh + v(0.0, 0.19, 0.0);
            p.k(f).cuboid(win + out * 0.024, v(0.012, 0.11, 0.27), gold);
            p.k(f)
                .cuboid(win + out * 0.03, v(0.008, 0.08, 0.23), c(0.14, 0.2, 0.24));
            p.k(f).cuboid_rot(
                win + out * 0.035 + v(0.0, 0.0, -0.05),
                v(0.003, 0.07, 0.016),
                Quat::from_rotation_x(0.5),
                c(0.6, 0.7, 0.75),
            );
            p.k(f).cuboid(
                sh + out * 0.026 + v(0.0, -0.1, 0.0),
                v(0.012, 0.36, 0.04),
                steel,
            );
            for z in [-1.0f32, 1.0] {
                p.k(f).cuboid_rot(
                    sh + out * 0.033 + v(0.0, -0.02, z * 0.05),
                    v(0.01, 0.026, 0.14),
                    Quat::from_rotation_x(z * 0.6),
                    gold,
                );
                for y in [-1.0f32, 1.0] {
                    p.k(f)
                        .sphere(sh + out * 0.032 + v(0.0, y * 0.29, z * 0.17), 0.013, gold);
                }
            }
            for y in [-0.04, -0.2] {
                p.k(f).cuboid(v(x * 0.08, y, 0.0), v(0.03, 0.04, 0.1), dark);
            }
        }
        // Armoured thighs, big knee pads, greaves and heavy boots.
        let t = Bone::Thigh(side);
        p.k(t).cuboid(v(0.0, -0.18, -0.085), v(0.16, 0.26, 0.05), steel);
        p.k(t).cuboid(v(0.0, -0.05, -0.09), v(0.17, 0.02, 0.055), gold);
        p.k(t).cuboid(v(x * 0.09, -0.2, 0.0), v(0.03, 0.22, 0.14), plate);
        let n = Bone::Shin(side);
        p.k(n).blob(v(0.0, 0.0, -0.07), v(0.09, 0.095, 0.065), plate);
        disc(p.k(n), v(0.0, 0.0, -0.13), Vec3::Z, 0.035, 0.012, gold);
        p.k(n).cuboid(v(0.0, -0.22, -0.065), v(0.12, 0.26, 0.05), steel);
        p.k(n).cuboid(v(0.0, -0.22, -0.092), v(0.025, 0.22, 0.01), plate);
        p.k(n).cuboid(v(0.0, -0.43, -0.04), v(0.15, 0.16, 0.25), dark);
        p.k(n).cuboid(v(0.0, -0.46, -0.17), v(0.145, 0.09, 0.065), plate);
        p.k(n).cuboid(v(0.0, -0.508, -0.05), v(0.16, 0.025, 0.31), c(0.06, 0.06, 0.06));
    }
}

pub(super) fn medic(p: &mut Parts) {
    let white = Character::Medic.suit_color();
    let teal = Character::Medic.trim_color();
    let shade = c(0.76, 0.78, 0.79);
    let grey = c(0.5, 0.53, 0.56);
    let dark = c(0.12, 0.13, 0.14);
    let skin = c(0.62, 0.45, 0.34);
    let hair = c(0.12, 0.09, 0.07);
    let fluid = c(0.35, 1.0, 0.6);
    base_body(
        p,
        &Palette {
            skin,
            suit: c(0.17, 0.3, 0.3),
            suit_dark: c(0.16, 0.22, 0.24),
            glove: c(0.86, 0.88, 0.89),
            boot: c(0.2, 0.22, 0.24),
        },
        1.0,
    );
    let s = Bone::Spine;
    // White armoured vest with teal piping and a green cross.
    p.k(s).blob(v(0.0, 0.33, 0.0), v(0.25, 0.21, 0.16), white);
    p.k(s).cuboid(v(0.0, 0.34, -0.14), v(0.3, 0.24, 0.04), white);
    p.k(s).cuboid(v(0.0, 0.465, -0.15), v(0.3, 0.015, 0.042), teal);
    for x in [-1.0f32, 1.0] {
        p.k(s).cuboid(v(x * 0.15, 0.34, -0.15), v(0.015, 0.24, 0.042), teal);
        // Chest pouches.
        p.k(s).cuboid(v(x * 0.08, 0.255, -0.172), v(0.08, 0.075, 0.04), shade);
        p.k(s).cuboid(v(x * 0.08, 0.29, -0.19), v(0.082, 0.02, 0.012), teal);
    }
    p.k(s).cuboid(v(-0.07, 0.39, -0.163), v(0.07, 0.022, 0.012), teal);
    p.k(s).cuboid(v(-0.07, 0.39, -0.163), v(0.022, 0.07, 0.012), teal);
    // Coat body below the vest, zipped, with a standing collar.
    p.k(s).blob(v(0.0, 0.13, 0.0), v(0.17, 0.15, 0.13), white);
    p.k(s).cuboid(v(0.0, 0.14, -0.128), v(0.012, 0.2, 0.01), grey);
    p.k(s).frustum(
        v(0.0, 0.52, 0.01),
        0.1,
        0.12,
        0.07,
        Quat::IDENTITY,
        white,
    );
    p.k(s).torus(v(0.0, 0.555, 0.01), 0.008, 0.1, Quat::IDENTITY, teal);
    // Medical backpack: two canisters of glowing fluid in metal cages.
    p.k(s).cuboid(v(0.0, 0.32, 0.2), v(0.32, 0.38, 0.13), shade);
    p.k(s).cuboid(v(0.0, 0.52, 0.2), v(0.3, 0.04, 0.12), grey);
    for x in [-0.1f32, 0.1] {
        p.g(s).cyl(v(x, 0.32, 0.29), 0.042, 0.3, Quat::IDENTITY, fluid);
        for y in [0.15, 0.49] {
            p.k(s).cyl(v(x, y, 0.29), 0.05, 0.04, Quat::IDENTITY, grey);
        }
        for y in [0.25, 0.39] {
            p.k(s).torus(v(x, y, 0.29), 0.007, 0.047, Quat::IDENTITY, grey);
        }
        for a in 0..3 {
            let a = a as f32 * 2.094 + 0.5;
            p.k(s).cyl(
                v(x + a.sin() * 0.047, 0.32, 0.29 + a.cos() * 0.047),
                0.005,
                0.32,
                Quat::IDENTITY,
                grey,
            );
        }
    }
    // Pouch at the bottom of the pack with a green cross on the back.
    p.k(s).cuboid(v(0.0, 0.08, 0.22), v(0.26, 0.1, 0.1), white);
    p.k(s).cuboid(v(0.0, 0.08, 0.272), v(0.07, 0.022, 0.01), teal);
    p.k(s).cuboid(v(0.0, 0.08, 0.272), v(0.022, 0.07, 0.01), teal);
    // Antenna with a green tip, and shoulder straps.
    p.k(s).cyl(v(-0.14, 0.54, 0.22), 0.015, 0.05, Quat::IDENTITY, dark);
    p.k(s).cyl(v(-0.14, 0.7, 0.22), 0.005, 0.3, Quat::IDENTITY, dark);
    p.g(s).sphere(v(-0.14, 0.86, 0.22), 0.012, fluid);
    for x in [-0.11, 0.11] {
        p.k(s).beam(
            v(x, 0.47, -0.13),
            v(x * 1.1, 0.53, 0.13),
            Vec2::new(0.04, 0.015),
            grey,
        );
    }
    // Belt with pouches, and long coat tails split at the front.
    let pv = Bone::Pelvis;
    p.k(pv).cyl(v(0.0, 0.05, 0.0), 0.19, 0.06, Quat::IDENTITY, grey);
    p.k(pv).cuboid(v(0.0, 0.05, -0.19), v(0.06, 0.05, 0.02), shade);
    for x in [-1.0f32, 1.0] {
        p.k(pv).cuboid(v(x * 0.19, 0.0, -0.06), v(0.05, 0.1, 0.1), white);
        p.k(pv).cuboid(v(x * 0.19, 0.045, -0.06), v(0.055, 0.02, 0.105), teal);
        p.k(pv).cuboid_rot(
            v(x * 0.1, -0.24, -0.13),
            v(0.15, 0.42, 0.02),
            Quat::from_rotation_x(0.12),
            white,
        );
        p.k(pv).cuboid_rot(
            v(x * 0.1, -0.44, -0.155),
            v(0.15, 0.025, 0.025),
            Quat::from_rotation_x(0.12),
            teal,
        );
        p.k(pv).cuboid_rot(
            v(x * 0.185, -0.24, 0.02),
            v(0.02, 0.42, 0.2),
            Quat::from_rotation_z(x * 0.12),
            white,
        );
    }
    p.k(pv).cuboid_rot(
        v(0.0, -0.24, 0.14),
        v(0.32, 0.42, 0.02),
        Quat::from_rotation_x(-0.12),
        white,
    );
    p.k(pv).cuboid_rot(
        v(0.0, -0.44, 0.165),
        v(0.32, 0.025, 0.025),
        Quat::from_rotation_x(-0.12),
        teal,
    );
    // Face, short hair, goggles pushed up and a headset.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.16, 0.0), v(0.12, 0.14, 0.13), skin);
    face_shape(p, 0.18, -0.125, 0.1, skin);
    eyes(p, 0.18, -0.115, 0.045, c(0.3, 0.2, 0.1), hair);
    p.k(n).blob(v(0.0, 0.245, 0.02), v(0.126, 0.08, 0.13), hair);
    p.k(n).blob(v(0.0, 0.18, 0.06), v(0.122, 0.11, 0.09), hair);
    p.k(n).torus(
        v(0.0, 0.25, 0.0),
        0.012,
        0.13,
        Quat::from_rotation_x(0.15),
        dark,
    );
    let up = v(0.0, 0.5, -0.87);
    for x in [-0.05f32, 0.05] {
        disc(p.k(n), v(x, 0.265, -0.105), up, 0.04, 0.035, grey);
        disc(p.g(n), v(x, 0.275, -0.122), up, 0.031, 0.01, c(0.45, 0.95, 0.85));
    }
    disc(p.k(n), v(0.125, 0.16, 0.0), Vec3::X, 0.035, 0.03, dark);
    p.k(n).beam(
        v(0.13, 0.13, -0.02),
        v(0.05, 0.08, -0.13),
        Vec2::new(0.01, 0.01),
        dark,
    );
    p.g(n).sphere(v(0.05, 0.08, -0.13), 0.008, fluid);
    for side in 0..2 {
        let x = sx(side);
        // Rounded shoulder pads over white coat sleeves.
        let u = Bone::UpperArm(side);
        p.k(u).blob(v(x * 0.02, 0.0, 0.0), v(0.1, 0.075, 0.11), white);
        p.k(u).cuboid(v(x * 0.08, -0.01, 0.0), v(0.02, 0.04, 0.12), teal);
        p.k(u).capsule_tapered(
            v(0.0, -0.05, 0.0),
            v(0.0, -0.28, 0.0),
            0.066,
            0.055,
            white,
        );
        if side == 0 {
            // Teal armband with a white cross.
            p.k(u).cyl(v(0.0, -0.16, 0.0), 0.068, 0.06, Quat::IDENTITY, teal);
            p.k(u).cuboid(v(-0.07, -0.16, 0.0), v(0.008, 0.045, 0.014), white);
            p.k(u).cuboid(v(-0.07, -0.16, 0.0), v(0.008, 0.014, 0.045), white);
        }
        let f = Bone::Forearm(side);
        p.k(f).capsule_tapered(
            v(0.0, -0.01, 0.0),
            v(0.0, -0.16, 0.0),
            0.061,
            0.052,
            white,
        );
        p.k(f).torus(v(0.0, -0.17, 0.0), 0.012, 0.052, Quat::IDENTITY, teal);
        p.k(f).frustum(
            v(0.0, -0.215, 0.0),
            0.05,
            0.046,
            0.08,
            Quat::IDENTITY,
            c(0.86, 0.88, 0.89),
        );
        if side == 1 {
            // Wrist scanner with a green screen.
            p.k(f).cuboid(v(0.0, -0.12, -0.06), v(0.05, 0.07, 0.02), dark);
            p.g(f).cuboid(v(0.0, -0.12, -0.071), v(0.038, 0.05, 0.005), fluid);
            // Medkit pouch strapped to the right thigh.
            let t = Bone::Thigh(side);
            p.k(t).torus(v(0.0, -0.25, 0.0), 0.01, 0.085, Quat::IDENTITY, dark);
            p.k(t).cuboid(v(0.095, -0.25, 0.0), v(0.05, 0.12, 0.13), white);
            p.k(t).cuboid(v(0.122, -0.25, 0.0), v(0.006, 0.06, 0.018), teal);
            p.k(t).cuboid(v(0.122, -0.25, 0.0), v(0.006, 0.018, 0.06), teal);
        }
        // Knee pads and boots with ankle straps.
        let n = Bone::Shin(side);
        p.k(n).blob(v(0.0, 0.0, -0.055), v(0.075, 0.075, 0.055), grey);
        disc(p.k(n), v(0.0, 0.0, -0.105), Vec3::Z, 0.03, 0.01, teal);
        p.k(n).frustum(
            v(0.0, -0.33, 0.0),
            0.06,
            0.07,
            0.12,
            Quat::IDENTITY,
            c(0.2, 0.22, 0.24),
        );
        p.k(n).torus(v(0.0, -0.3, 0.0), 0.008, 0.066, Quat::IDENTITY, shade);
    }
}

pub(super) fn revenant(p: &mut Parts) {
    let black = Character::Revenant.suit_color();
    let spirit = Character::Revenant.trim_color();
    let cloth = c(0.07, 0.07, 0.09);
    let rag = c(0.1, 0.1, 0.13);
    let bone = c(0.8, 0.78, 0.7);
    let bone_dark = c(0.55, 0.52, 0.46);
    let iron = c(0.26, 0.27, 0.3);
    let void = c(0.015, 0.015, 0.02);
    let wrap = c(0.08, 0.07, 0.08);
    base_body(
        p,
        &Palette {
            skin: cloth,
            suit: black,
            suit_dark: cloth,
            glove: wrap,
            boot: c(0.06, 0.06, 0.07),
        },
        0.98,
    );
    let s = Bone::Spine;
    // Wrapped robe crossed over the chest, bone ribs strapped on top.
    p.k(s).blob(v(0.0, 0.33, 0.0), v(0.24, 0.21, 0.15), black);
    for x in [-1.0f32, 1.0] {
        p.k(s).beam(
            v(x * 0.15, 0.47, -0.11),
            v(-x * 0.07, 0.12, -0.13),
            Vec2::new(0.07, 0.02),
            rag,
        );
    }
    for i in 0..3 {
        let y = 0.37 - i as f32 * 0.06;
        for x in [-1.0f32, 1.0] {
            p.k(s).cuboid_rot(
                v(x * 0.065, y, -0.162),
                v(0.09, 0.016, 0.016),
                Quat::from_rotation_z(-x * 0.25),
                bone,
            );
        }
    }
    p.k(s).cuboid(v(0.0, 0.31, -0.165), v(0.024, 0.2, 0.016), bone);
    // Hood mantle over the shoulders and a clasp with a soul gem.
    p.k(s).frustum(
        v(0.0, 0.46, 0.01),
        0.13,
        0.25,
        0.14,
        Quat::IDENTITY,
        cloth,
    );
    p.k(s).torus(
        v(0.0, 0.44, -0.2),
        0.008,
        0.03,
        Quat::from_rotation_x(FRAC_PI_2),
        iron,
    );
    p.g(s).sphere(v(0.0, 0.44, -0.205), 0.02, spirit);
    // Long cloak from the shoulders.
    p.k(s).cuboid_rot(
        v(0.0, 0.22, 0.18),
        v(0.46, 0.5, 0.02),
        Quat::from_rotation_x(-0.08),
        cloth,
    );
    for x in [-1.0f32, 1.0] {
        p.k(s).cuboid_rot(
            v(x * 0.22, 0.22, 0.06),
            v(0.02, 0.5, 0.24),
            Quat::from_rotation_z(x * 0.05),
            cloth,
        );
    }
    // Scythe slung across the back: the blade arcs over the head, with a
    // glowing edge on the inside of the curve.
    let tilt = Quat::from_rotation_z(-0.5);
    let back = v(0.0, 0.3, 0.25);
    let wood = c(0.16, 0.12, 0.1);
    p.k(s).cyl(back, 0.017, 1.3, tilt, wood);
    for y in [-0.3, 0.0, 0.3] {
        p.k(s)
            .torus(back + tilt * v(0.0, y, 0.0), 0.006, 0.02, tilt, iron);
    }
    p.k(s)
        .cyl(back + tilt * v(0.0, -0.66, 0.0), 0.022, 0.04, tilt, iron);
    let top = back + tilt * v(0.0, 0.62, 0.0);
    p.k(s).cyl(top, 0.03, 0.07, tilt, iron);
    let mut at = top;
    let mut a = 2.64f32;
    for i in 0..7 {
        let len = 0.085;
        let w = 0.09 - i as f32 * 0.011;
        let rot = Quat::from_rotation_z(a);
        let mid = at + rot * v(len * 0.5, 0.0, 0.0);
        p.k(s).cuboid_rot(
            mid + rot * v(0.0, w * 0.5, 0.0),
            v(len * 1.08, w, 0.01),
            rot,
            c(0.3, 0.32, 0.35),
        );
        p.k(s)
            .cuboid_rot(mid, v(len * 1.08, 0.016, 0.02), rot, iron);
        p.g(s).cuboid_rot(
            mid + rot * v(0.0, w, 0.0),
            v(len * 1.08, 0.012, 0.014),
            rot,
            spirit,
        );
        at += rot * v(len, 0.0, 0.0);
        a += 0.2;
    }
    // Cloak below the waist down to the ankles, ragged at the hem.
    let pv = Bone::Pelvis;
    p.k(pv).cuboid_rot(
        v(0.0, -0.38, 0.2),
        v(0.42, 0.74, 0.02),
        Quat::from_rotation_x(-0.12),
        cloth,
    );
    for i in 0..9 {
        let l = 0.07 + jitter(i) * 0.13;
        let x = -0.19 + i as f32 * 0.0475;
        p.k(pv).cuboid_rot(
            v(x, -0.75 - l * 0.5, 0.245 + l * 0.06),
            v(0.04, l, 0.015),
            Quat::from_rotation_x(-0.12) * Quat::from_rotation_z((jitter(i + 3) - 0.5) * 0.3),
            if i % 2 == 0 { cloth } else { rag },
        );
    }
    for x in [-1.0f32, 1.0] {
        p.k(pv).cuboid_rot(
            v(x * 0.2, -0.36, 0.06),
            v(0.02, 0.7, 0.26),
            Quat::from_rotation_z(x * 0.1),
            cloth,
        );
        for j in 0..4 {
            let l = 0.06 + jitter(j * 2 + side_i(x)) * 0.12;
            p.k(pv).cuboid_rot(
                v(x * (0.235 + l * 0.05), -0.71 - l * 0.5, -0.04 + j as f32 * 0.06),
                v(0.015, l, 0.045),
                Quat::from_rotation_z(x * 0.1),
                rag,
            );
        }
        // Open robe at the front, tattered at the knee.
        p.k(pv).cuboid_rot(
            v(x * 0.09, -0.2, -0.13),
            v(0.14, 0.36, 0.02),
            Quat::from_rotation_x(0.1),
            rag,
        );
        for j in 0..3 {
            let l = 0.05 + jitter(j + side_i(x) * 5) * 0.1;
            p.k(pv).cuboid_rot(
                v(x * (0.05 + j as f32 * 0.04), -0.38 - l * 0.5, -0.15 - l * 0.05),
                v(0.035, l, 0.015),
                Quat::from_rotation_x(0.1),
                rag,
            );
        }
    }
    // Sash with a skull buckle, and chains slung round the waist.
    p.k(pv).cyl(v(0.0, 0.05, 0.0), 0.19, 0.06, Quat::IDENTITY, rag);
    p.k(pv).blob(v(0.0, 0.05, -0.195), v(0.035, 0.04, 0.025), bone);
    for x in [-1.0f32, 1.0] {
        p.k(pv).sphere(v(x * 0.013, 0.058, -0.214), 0.009, void);
    }
    for i in 0..=20 {
        let t = i as f32 / 20.0;
        let a = -1.3 + 2.6 * t;
        let radial = v(a.sin(), 0.0, -a.cos());
        let at = radial * 0.205 + v(0.0, 0.02 - 0.1 * (1.0 - (2.0 * t - 1.0).powi(2)), 0.0);
        let normal = if i % 2 == 0 { radial } else { Vec3::Y };
        p.k(pv).torus(
            at,
            0.006,
            0.017,
            Quat::from_rotation_arc(Vec3::Y, normal),
            iron,
        );
    }
    for j in 0..6 {
        let normal = if j % 2 == 0 { Vec3::X } else { Vec3::Z };
        p.k(pv).torus(
            v(0.205, -0.03 - j as f32 * 0.03, -0.05),
            0.006,
            0.017,
            Quat::from_rotation_arc(Vec3::Y, normal),
            iron,
        );
    }
    p.k(pv).cuboid(v(0.205, -0.22, -0.05), v(0.035, 0.04, 0.02), iron);
    // Deep hood framing a skull mask with glowing eyes.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.2, 0.05), v(0.17, 0.19, 0.17), cloth);
    p.k(n).cone(
        v(0.0, 0.2, 0.23),
        0.06,
        0.15,
        Quat::from_rotation_x(2.09),
        cloth,
    );
    p.k(n).blob(v(0.0, 0.3, -0.1), v(0.14, 0.05, 0.09), cloth);
    for x in [-1.0f32, 1.0] {
        p.k(n).blob(v(x * 0.115, 0.16, -0.09), v(0.05, 0.15, 0.08), cloth);
    }
    p.k(n).torus(
        v(0.0, 0.18, -0.155),
        0.022,
        0.115,
        Quat::from_rotation_x(FRAC_PI_2 - 0.15),
        rag,
    );
    p.k(n).blob(v(0.0, 0.17, -0.08), v(0.115, 0.13, 0.06), void);
    p.k(n).blob(v(0.0, 0.18, -0.095), v(0.08, 0.09, 0.06), bone);
    p.k(n).blob(v(0.0, 0.11, -0.1), v(0.06, 0.04, 0.045), bone);
    for x in [-1.0f32, 1.0] {
        p.k(n).sphere(v(x * 0.035, 0.185, -0.137), 0.02, void);
        p.g(n).sphere(v(x * 0.035, 0.185, -0.15), 0.011, spirit);
        p.k(n).cuboid_rot(
            v(x * 0.055, 0.145, -0.135),
            v(0.04, 0.012, 0.012),
            Quat::from_rotation_z(x * 0.3),
            bone_dark,
        );
    }
    p.k(n).wedge(
        v(0.0, 0.15, -0.152),
        v(0.024, 0.03, 0.01),
        Quat::IDENTITY,
        void,
    );
    p.k(n).cuboid(v(0.0, 0.115, -0.14), v(0.075, 0.024, 0.008), void);
    for i in 0..5 {
        p.k(n).cuboid(
            v(-0.03 + i as f32 * 0.015, 0.115, -0.145),
            v(0.011, 0.02, 0.008),
            bone,
        );
    }
    p.k(n).cuboid_rot(
        v(0.02, 0.24, -0.148),
        v(0.005, 0.05, 0.006),
        Quat::from_rotation_z(0.4),
        void,
    );
    for side in 0..2 {
        let x = sx(side);
        // Bone pauldrons with a spike, and ragged flared sleeves.
        let u = Bone::UpperArm(side);
        p.k(u).blob(v(x * 0.02, 0.01, 0.0), v(0.1, 0.07, 0.11), cloth);
        for j in 0..3 {
            let f = j as f32;
            p.k(u).cuboid_rot(
                v(x * (0.085 - f * 0.005), 0.03 - f * 0.04, 0.0),
                v(0.02, 0.025, 0.16 - f * 0.03),
                Quat::from_rotation_z(x * 0.5),
                bone,
            );
        }
        p.k(u).cone(
            v(x * 0.1, 0.08, 0.0),
            0.02,
            0.08,
            Quat::from_rotation_z(-x * 0.6),
            bone,
        );
        p.k(u).frustum(
            v(0.0, -0.2, 0.0),
            0.062,
            0.085,
            0.16,
            Quat::IDENTITY,
            rag,
        );
        for j in 0..4 {
            let a = j as f32 * FRAC_PI_2 + 0.4;
            let l = 0.05 + jitter(j + side * 4) * 0.07;
            p.k(u).cuboid_rot(
                v(a.sin() * 0.08, -0.28 - l * 0.5, a.cos() * 0.08),
                v(0.03, l, 0.012),
                Quat::from_rotation_y(a),
                rag,
            );
        }
        // Bony gauntlets: a dark bracer with vertebrae along it, knuckle
        // bones on the back of the hand.
        let f = Bone::Forearm(side);
        p.k(f).frustum(
            v(0.0, -0.14, 0.0),
            0.05,
            0.062,
            0.16,
            Quat::IDENTITY,
            wrap,
        );
        for j in 0..4 {
            let y = -0.07 - j as f32 * 0.045;
            p.k(f).blob(v(0.0, y, -0.054), v(0.03, 0.018, 0.016), bone);
            p.k(f).blob(v(x * 0.054, y, 0.0), v(0.014, 0.016, 0.022), bone_dark);
        }
        let h = Bone::Hand(side);
        for i in 0..4 {
            let fx = x * (0.03 - i as f32 * 0.02);
            p.k(h).beam(
                v(fx * 0.4, -0.012, 0.02),
                v(fx, -0.09, 0.02),
                Vec2::new(0.009, 0.007),
                bone,
            );
            p.k(h).sphere(v(fx, -0.093, 0.022), 0.01, bone);
        }
        // Wrapped shins with bone knee caps.
        let n = Bone::Shin(side);
        p.k(n).blob(v(0.0, 0.0, -0.055), v(0.05, 0.05, 0.035), bone);
        for j in 0..4 {
            p.k(n).torus(
                v(0.0, -0.08 - j as f32 * 0.07, 0.0),
                0.01,
                0.06 - j as f32 * 0.004,
                Quat::from_rotation_x(if j % 2 == 0 { 0.15 } else { -0.15 }),
                rag,
            );
        }
    }
}

/// 0 for the left side, 1 for the right, from -1 or +1.
fn side_i(x: f32) -> usize {
    usize::from(x > 0.0)
}

pub(super) fn demolisher(p: &mut Parts) {
    let olive = Character::Demolisher.suit_color();
    let orange = Character::Demolisher.trim_color();
    let pad = c(0.37, 0.43, 0.28);
    let dark = c(0.12, 0.12, 0.11);
    let rubber = c(0.08, 0.08, 0.08);
    let metal = c(0.45, 0.46, 0.44);
    let tan = c(0.72, 0.66, 0.5);
    let red = c(0.7, 0.14, 0.1);
    let leather = c(0.55, 0.42, 0.25);
    base_body(
        p,
        &Palette {
            skin: c(0.6, 0.45, 0.35),
            suit: olive,
            suit_dark: c(0.24, 0.29, 0.18),
            glove: leather,
            boot: c(0.14, 0.12, 0.1),
        },
        1.15,
    );
    let s = Bone::Spine;
    // Bomb suit: a thick chest quilted in rows of pads.
    p.k(s).blob(v(0.0, 0.32, 0.0), v(0.28, 0.24, 0.19), olive);
    for i in 0..5 {
        let y = 0.16 + i as f32 * 0.065;
        let k = (1.0 - ((y - 0.32) / 0.24).powi(2)).max(0.0).sqrt();
        for j in -1..=1 {
            p.k(s).blob(
                v(j as f32 * 0.085 * k, y, -(0.19 * k - 0.015)),
                v(0.045, 0.03, 0.035),
                pad,
            );
        }
    }
    // Hazard panel on the left chest.
    for i in 0..4 {
        p.k(s).cuboid_rot(
            v(-0.13 + i as f32 * 0.022, 0.43, -0.16),
            v(0.016, 0.06, 0.012),
            Quat::from_rotation_z(0.5),
            if i % 2 == 0 { orange } else { dark },
        );
    }
    // High padded collar, raised at the back of the head.
    p.k(s).frustum(
        v(0.0, 0.53, 0.02),
        0.15,
        0.19,
        0.14,
        Quat::IDENTITY,
        pad,
    );
    p.k(s).cuboid_rot(
        v(0.0, 0.62, 0.13),
        v(0.3, 0.18, 0.06),
        Quat::from_rotation_x(-0.2),
        pad,
    );
    // Bandolier of charges from the right shoulder to the left hip.
    let path = [
        v(0.18, 0.5, 0.12),
        v(0.19, 0.5, -0.12),
        v(0.06, 0.34, -0.2),
        v(-0.08, 0.2, -0.19),
        v(-0.19, 0.06, -0.15),
    ];
    for w in path.windows(2) {
        p.k(s).beam(w[0], w[1], Vec2::new(0.06, 0.02), dark);
    }
    for (i, w) in path[1..].windows(2).enumerate() {
        let d = (w[1] - w[0]).normalize();
        let rot = Quat::from_rotation_arc(Vec3::Y, d);
        for t in [0.3, 0.75] {
            let at = w[0].lerp(w[1], t) + v(0.0, 0.0, -0.022);
            p.k(s).cuboid_rot(at, v(0.05, 0.065, 0.035), rot, tan);
            p.k(s)
                .cuboid_rot(at + rot * v(0.0, 0.0, -0.018), v(0.052, 0.012, 0.004), rot, red);
            p.g(s).sphere(
                at + rot * v(0.016, 0.02, -0.019),
                0.006,
                if i % 2 == 0 { orange } else { c(0.4, 1.0, 0.3) },
            );
        }
    }
    // Backpack frame with a fuel tank: hazard band, valve and gauge.
    p.k(s).cuboid(v(0.0, 0.3, 0.2), v(0.3, 0.4, 0.08), dark);
    let tc = v(0.0, 0.3, 0.34);
    let tank = c(0.28, 0.32, 0.2);
    p.k(s).cyl(tc, 0.11, 0.38, Quat::IDENTITY, tank);
    p.k(s).sphere(tc + v(0.0, 0.19, 0.0), 0.11, tank);
    p.k(s).sphere(tc - v(0.0, 0.19, 0.0), 0.11, tank);
    for i in 0..14 {
        let a = i as f32 / 14.0 * 2.0 * PI;
        p.k(s).cuboid_rot(
            tc + v(a.sin() * 0.108, 0.04, a.cos() * 0.108),
            v(0.05, 0.07, 0.008),
            Quat::from_rotation_y(a) * Quat::from_rotation_z(0.6),
            if i % 2 == 0 { orange } else { dark },
        );
    }
    p.k(s)
        .torus(tc + v(0.0, 0.11, 0.0), 0.008, 0.112, Quat::IDENTITY, orange);
    p.k(s)
        .cyl(tc + v(0.0, 0.31, 0.0), 0.02, 0.06, Quat::IDENTITY, metal);
    p.k(s)
        .torus(tc + v(0.0, 0.34, 0.0), 0.007, 0.035, Quat::IDENTITY, red);
    disc(p.k(s), tc + v(0.07, 0.14, 0.08), v(0.6, 0.0, 0.8), 0.03, 0.02, metal);
    disc(p.k(s), tc + v(0.077, 0.14, 0.088), v(0.6, 0.0, 0.8), 0.024, 0.006, c(0.9, 0.9, 0.85));
    hose(
        p.k(s),
        &[
            tc + v(0.06, -0.24, 0.0),
            v(0.18, 0.02, 0.24),
            v(0.24, 0.02, 0.05),
        ],
        0.018,
        rubber,
    );
    // Groin plate and a belt with a dynamite bundle and a detonator.
    let pv = Bone::Pelvis;
    p.k(pv).cuboid_rot(
        v(0.0, -0.16, -0.16),
        v(0.24, 0.3, 0.05),
        Quat::from_rotation_x(0.1),
        olive,
    );
    for i in 0..3 {
        p.k(pv).cuboid_rot(
            v(0.0, -0.06 - i as f32 * 0.09, -0.19 - i as f32 * 0.009),
            v(0.2, 0.05, 0.02),
            Quat::from_rotation_x(0.1),
            pad,
        );
    }
    p.k(pv).cyl(v(0.0, 0.05, 0.0), 0.22, 0.07, Quat::IDENTITY, dark);
    p.k(pv).cuboid(v(0.0, 0.05, -0.225), v(0.07, 0.05, 0.02), metal);
    let dyn_at = v(-0.22, -0.03, -0.06);
    for (dx, dz) in [(-0.02, -0.02), (0.02, -0.02), (0.0, 0.015), (-0.035, 0.015), (0.035, 0.015)] {
        p.k(pv)
            .cyl(dyn_at + v(dx, 0.0, dz), 0.018, 0.17, Quat::IDENTITY, red);
    }
    for y in [-0.05, 0.05] {
        p.k(pv)
            .cyl(dyn_at + v(0.0, y, 0.0), 0.06, 0.02, Quat::IDENTITY, dark);
    }
    hose(
        p.k(pv),
        &[
            dyn_at + v(0.0, 0.085, -0.02),
            dyn_at + v(0.01, 0.13, -0.03),
            dyn_at + v(0.04, 0.15, -0.02),
        ],
        0.004,
        tan,
    );
    p.g(pv).sphere(dyn_at + v(0.04, 0.15, -0.02), 0.009, orange);
    let det = v(0.22, -0.03, -0.05);
    p.k(pv).cuboid(det, v(0.06, 0.09, 0.08), c(0.5, 0.42, 0.2));
    p.k(pv).cuboid(det + v(0.0, 0.0, -0.041), v(0.04, 0.05, 0.004), orange);
    p.k(pv)
        .cyl(det + v(0.0, 0.08, 0.0), 0.007, 0.08, Quat::IDENTITY, metal);
    p.k(pv)
        .cuboid(det + v(0.0, 0.12, 0.0), v(0.012, 0.016, 0.08), dark);
    p.k(pv).torus(
        det + v(0.0, -0.06, 0.03),
        0.004,
        0.025,
        Quat::from_rotation_z(FRAC_PI_2),
        red,
    );
    p.k(pv).cuboid(v(0.0, -0.01, 0.2), v(0.18, 0.1, 0.06), olive);
    // Padded hood, domed blast helmet and a gas mask with round filters.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.17, 0.01), v(0.16, 0.17, 0.165), pad);
    p.k(n).blob(v(0.0, 0.24, 0.01), v(0.168, 0.12, 0.175), olive);
    p.k(n).torus(v(0.0, 0.2, 0.01), 0.014, 0.162, Quat::IDENTITY, dark);
    p.k(n).cuboid(v(0.0, 0.345, 0.01), v(0.05, 0.03, 0.3), orange);
    p.k(n).cuboid_rot(
        v(0.0, 0.25, -0.16),
        v(0.24, 0.02, 0.06),
        Quat::from_rotation_x(-0.25),
        olive,
    );
    p.k(n).blob(v(0.0, 0.14, -0.09), v(0.13, 0.12, 0.085), rubber);
    for x in [-1.0f32, 1.0] {
        disc(p.k(n), v(x * 0.05, 0.18, -0.16), Vec3::Z, 0.042, 0.03, metal);
        disc(p.k(n), v(x * 0.05, 0.18, -0.172), Vec3::Z, 0.034, 0.01, c(0.16, 0.2, 0.18));
        p.k(n)
            .sphere(v(x * 0.05 - 0.012, 0.192, -0.177), 0.007, c(0.85, 0.9, 0.85));
        let d = (Quat::from_rotation_y(-x * 0.7) * v(0.0, -0.3, -0.95)).normalize();
        let f = v(x * 0.085, 0.08, -0.15);
        disc(p.k(n), f, d, 0.045, 0.06, dark);
        disc(p.k(n), f + d * 0.033, d, 0.047, 0.012, orange);
        disc(p.k(n), f + d * 0.04, d, 0.032, 0.006, rubber);
    }
    disc(p.k(n), v(0.0, 0.08, -0.17), v(0.0, -0.3, -0.95), 0.03, 0.04, metal);
    disc(p.k(n), v(0.0, 0.074, -0.19), v(0.0, -0.3, -0.95), 0.034, 0.01, orange);
    for side in 0..2 {
        let x = sx(side);
        // Thick shoulder pads and quilted sleeves with an orange stripe.
        let u = Bone::UpperArm(side);
        p.k(u).blob(v(x * 0.03, 0.01, 0.0), v(0.13, 0.1, 0.13), pad);
        p.k(u).torus(v(x * 0.03, -0.03, 0.0), 0.02, 0.11, Quat::IDENTITY, olive);
        for j in 0..3 {
            p.k(u).torus(
                v(0.0, -0.12 - j as f32 * 0.065, 0.0),
                0.022,
                0.066,
                Quat::IDENTITY,
                pad,
            );
        }
        p.k(u).cyl(v(0.0, -0.085, 0.0), 0.076, 0.03, Quat::IDENTITY, orange);
        let f = Bone::Forearm(side);
        for j in 0..3 {
            p.k(f).torus(
                v(0.0, -0.04 - j as f32 * 0.06, 0.0),
                0.02,
                0.06,
                Quat::IDENTITY,
                pad,
            );
        }
        p.k(f).frustum(
            v(0.0, -0.225, 0.0),
            0.065,
            0.058,
            0.06,
            Quat::IDENTITY,
            leather,
        );
        if side == 0 {
            // Wrist timer.
            p.k(f).cuboid(v(0.0, -0.13, -0.065), v(0.05, 0.035, 0.02), dark);
            p.g(f).cuboid(v(0.0, -0.13, -0.076), v(0.04, 0.02, 0.005), orange);
        }
        // Quilted legs, padded shin plates and heavy boots.
        let t = Bone::Thigh(side);
        for j in 0..4 {
            p.k(t).torus(
                v(0.0, -0.08 - j as f32 * 0.09, 0.0),
                0.022,
                0.095 - j as f32 * 0.007,
                Quat::IDENTITY,
                pad,
            );
        }
        p.k(t).cuboid(v(x * 0.1, -0.22, 0.0), v(0.04, 0.12, 0.12), olive);
        let n = Bone::Shin(side);
        p.k(n).blob(v(0.0, 0.0, -0.07), v(0.085, 0.09, 0.06), pad);
        p.k(n).cuboid(v(0.0, -0.21, -0.07), v(0.13, 0.27, 0.06), pad);
        for y in [-0.15, -0.27] {
            p.k(n).cuboid(v(0.0, y, -0.101), v(0.13, 0.025, 0.01), orange);
        }
        p.k(n).cuboid(v(0.0, -0.44, -0.04), v(0.15, 0.13, 0.26), c(0.14, 0.12, 0.1));
        p.k(n).cuboid(v(0.0, -0.505, -0.05), v(0.16, 0.03, 0.3), rubber);
    }
}

pub(super) fn chemist(p: &mut Parts) {
    let yellow = Character::Chemist.suit_color();
    let toxic = Character::Chemist.trim_color();
    let fold = c(0.72, 0.58, 0.12);
    let rubber = c(0.1, 0.1, 0.1);
    let tape = c(0.3, 0.3, 0.3);
    let tank = c(0.78, 0.8, 0.78);
    let metal = c(0.45, 0.46, 0.48);
    let skin = c(0.8, 0.6, 0.47);
    let glove = c(0.72, 0.58, 0.1);
    let goo = c(0.45, 1.0, 0.25);
    let glass = c(0.75, 0.88, 0.8);
    base_body(
        p,
        &Palette {
            skin,
            suit: yellow,
            suit_dark: c(0.8, 0.65, 0.13),
            glove,
            boot: rubber,
        },
        1.06,
    );
    let s = Bone::Spine;
    // Baggy hazmat suit: loose folds, a zip flap and taped side seams.
    p.k(s).blob(v(0.0, 0.32, 0.0), v(0.27, 0.23, 0.18), yellow);
    p.k(s).blob(v(0.0, 0.12, 0.0), v(0.2, 0.15, 0.15), yellow);
    for i in 0..3 {
        p.k(s).torus(
            v(0.0, 0.05 + i as f32 * 0.05, 0.0),
            0.012,
            0.19 - i as f32 * 0.005,
            Quat::from_rotation_z(if i % 2 == 0 { 0.06 } else { -0.06 }),
            fold,
        );
    }
    p.k(s).cuboid(v(0.0, 0.27, -0.177), v(0.05, 0.4, 0.02), fold);
    for x in [-1.0f32, 1.0] {
        p.k(s).cuboid(v(x * 0.265, 0.3, 0.0), v(0.012, 0.3, 0.06), tape);
    }
    // Toxic badge: a green disc with a dark trefoil.
    let badge = v(-0.12, 0.38, -0.168);
    disc(p.k(s), badge, Vec3::Z, 0.04, 0.012, toxic);
    for i in 0..3 {
        let a = i as f32 * 2.0 * PI / 3.0 + FRAC_PI_2;
        p.k(s).sphere(
            badge + v(a.cos() * 0.017, a.sin() * 0.017, -0.006),
            0.011,
            rubber,
        );
    }
    p.k(s).torus(v(0.0, 0.5, 0.0), 0.025, 0.13, Quat::IDENTITY, rubber);
    // Two air tanks on the back with hoses over the shoulders.
    p.k(s).cuboid(v(0.0, 0.3, 0.18), v(0.3, 0.34, 0.04), metal);
    for x in [-0.1f32, 0.1] {
        let xs = x.signum();
        p.k(s).cyl(v(x, 0.3, 0.26), 0.075, 0.42, Quat::IDENTITY, tank);
        p.k(s).sphere(v(x, 0.51, 0.26), 0.075, tank);
        p.k(s).sphere(v(x, 0.09, 0.26), 0.075, tank);
        p.k(s).torus(v(x, 0.36, 0.26), 0.01, 0.077, Quat::IDENTITY, toxic);
        p.k(s).torus(v(x, 0.2, 0.26), 0.008, 0.077, Quat::IDENTITY, rubber);
        p.k(s).cyl(v(x, 0.6, 0.26), 0.02, 0.06, Quat::IDENTITY, metal);
        p.k(s).torus(v(x, 0.63, 0.26), 0.006, 0.026, Quat::IDENTITY, metal);
        hose(
            p.k(s),
            &[
                v(x, 0.6, 0.27),
                v(xs * 0.15, 0.66, 0.14),
                v(xs * 0.17, 0.58, -0.02),
                v(xs * 0.1, 0.5, -0.13),
            ],
            0.016,
            rubber,
        );
    }
    for y in [0.17, 0.43] {
        p.k(s).cuboid(v(0.0, y, 0.26), v(0.36, 0.03, 0.16), rubber);
    }
    // Belt of glass vials of glowing liquid.
    let pv = Bone::Pelvis;
    p.k(pv).cyl(v(0.0, 0.05, 0.0), 0.2, 0.06, Quat::IDENTITY, rubber);
    p.k(pv).cuboid(v(0.0, 0.05, -0.2), v(0.06, 0.045, 0.02), metal);
    for a in [-1.25f32, -0.8, -0.35, 0.35, 0.8, 1.25] {
        let at = v(a.sin(), 0.0, -a.cos()) * 0.215;
        p.g(pv)
            .cyl(at + v(0.0, 0.03, 0.0), 0.016, 0.06, Quat::IDENTITY, goo);
        p.k(pv)
            .cyl(at + v(0.0, 0.075, 0.0), 0.016, 0.03, Quat::IDENTITY, glass);
        p.k(pv).cyl(
            at + v(0.0, 0.097, 0.0),
            0.011,
            0.016,
            Quat::IDENTITY,
            c(0.45, 0.3, 0.18),
        );
        p.k(pv)
            .torus(at + v(0.0, 0.05, 0.0), 0.004, 0.019, Quat::IDENTITY, rubber);
    }
    // Hood with a big round visor; the face shows behind the glass.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.19, 0.02), v(0.19, 0.2, 0.19), yellow);
    p.k(n).blob(v(0.0, 0.06, 0.0), v(0.17, 0.08, 0.17), yellow);
    p.k(n).cyl(v(0.0, 0.38, 0.06), 0.03, 0.03, Quat::IDENTITY, rubber);
    let vis = v(0.0, 0.215, -0.17);
    disc(p.k(n), vis, Vec3::Z, 0.1, 0.012, c(0.1, 0.12, 0.1));
    p.k(n)
        .blob(vis + v(0.0, 0.0, -0.002), v(0.075, 0.09, 0.03), skin);
    eyes(p, 0.23, -0.195, 0.032, c(0.25, 0.35, 0.2), c(0.3, 0.2, 0.12));
    p.k(n)
        .blob(vis + v(0.0, -0.025, -0.03), v(0.014, 0.02, 0.012), skin);
    p.k(n).torus(
        vis + v(0.0, 0.0, -0.016),
        0.018,
        0.105,
        Quat::from_rotation_x(FRAC_PI_2),
        rubber,
    );
    for (dx, dy, l) in [(-0.045, 0.04, 0.06), (-0.02, 0.055, 0.035)] {
        p.k(n).cuboid_rot(
            vis + v(dx, dy, -0.036),
            v(0.006, l, 0.003),
            Quat::from_rotation_z(-0.6),
            c(0.88, 0.96, 0.92),
        );
    }
    // Respirator under the visor with twin filters, hosed to the collar.
    p.k(n).blob(v(0.0, 0.075, -0.15), v(0.08, 0.05, 0.05), rubber);
    for x in [-1.0f32, 1.0] {
        let d = (Quat::from_rotation_y(-x * 0.6) * v(0.0, -0.35, -0.94)).normalize();
        let f = v(x * 0.065, 0.06, -0.18);
        disc(p.k(n), f, d, 0.04, 0.07, c(0.22, 0.24, 0.22));
        disc(p.k(n), f + d * 0.02, d, 0.042, 0.014, toxic);
        disc(p.k(n), f + d * 0.04, d, 0.034, 0.008, rubber);
        hose(
            p.k(n),
            &[f - d * 0.03, v(x * 0.1, 0.02, -0.12), v(x * 0.1, -0.03, -0.09)],
            0.013,
            rubber,
        );
    }
    for side in 0..2 {
        let x = sx(side);
        // Baggy sleeves with taped black cuffs over the gloves.
        let u = Bone::UpperArm(side);
        p.k(u).blob(v(x * 0.015, -0.02, 0.0), v(0.085, 0.085, 0.085), yellow);
        p.k(u).capsule_tapered(
            v(0.0, -0.06, 0.0),
            v(0.0, -0.27, 0.0),
            0.072,
            0.062,
            yellow,
        );
        p.k(u).torus(
            v(0.0, -0.2, 0.0),
            0.01,
            0.068,
            Quat::from_rotation_z(x * 0.15),
            fold,
        );
        let f = Bone::Forearm(side);
        p.k(f).capsule_tapered(
            v(0.0, -0.02, 0.0),
            v(0.0, -0.16, 0.0),
            0.066,
            0.058,
            yellow,
        );
        p.k(f).frustum(
            v(0.0, -0.19, 0.0),
            0.058,
            0.055,
            0.05,
            Quat::IDENTITY,
            rubber,
        );
        p.k(f).torus(v(0.0, -0.172, 0.0), 0.01, 0.058, Quat::IDENTITY, tape);
        p.k(f).frustum(
            v(0.0, -0.228, 0.0),
            0.05,
            0.045,
            0.04,
            Quat::IDENTITY,
            glove,
        );
        // Baggy legs, knee folds and tall rubber boots.
        let t = Bone::Thigh(side);
        p.k(t).capsule_tapered(
            v(0.0, -0.03, 0.0),
            v(0.0, -0.4, 0.0),
            0.098,
            0.075,
            yellow,
        );
        p.k(t)
            .torus(v(0.0, -0.32, 0.0), 0.012, 0.08, Quat::from_rotation_x(0.2), fold);
        let n = Bone::Shin(side);
        p.k(n).blob(v(0.0, 0.0, -0.03), v(0.08, 0.08, 0.075), fold);
        p.k(n).frustum(
            v(0.0, -0.28, 0.0),
            0.06,
            0.068,
            0.26,
            Quat::IDENTITY,
            rubber,
        );
        p.k(n).torus(v(0.0, -0.15, 0.0), 0.014, 0.07, Quat::IDENTITY, rubber);
        p.k(n).torus(v(0.0, -0.17, 0.0), 0.008, 0.068, Quat::IDENTITY, tape);
        if side == 1 {
            // Sprayer wand holstered on the right thigh.
            p.k(t).torus(v(0.0, -0.12, 0.0), 0.008, 0.1, Quat::IDENTITY, rubber);
            p.k(t).torus(v(0.0, -0.3, 0.0), 0.008, 0.085, Quat::IDENTITY, rubber);
            p.k(t).cuboid(v(0.11, -0.22, 0.0), v(0.04, 0.2, 0.07), rubber);
            p.k(t).cyl_between(
                v(0.12, -0.05, -0.01),
                v(0.12, -0.4, -0.01),
                0.012,
                metal,
            );
            p.k(t).frustum(
                v(0.12, -0.42, -0.01),
                0.012,
                0.02,
                0.04,
                Quat::IDENTITY,
                metal,
            );
            p.g(t).sphere(v(0.12, -0.445, -0.01), 0.01, goo);
            p.k(t).cuboid_rot(
                v(0.12, -0.03, -0.045),
                v(0.03, 0.08, 0.035),
                Quat::from_rotation_x(0.4),
                rubber,
            );
            p.g(t)
                .cyl(v(0.145, -0.1, -0.01), 0.022, 0.07, Quat::IDENTITY, goo);
            for y in [-0.06, -0.14] {
                p.k(t)
                    .cyl(v(0.145, y, -0.01), 0.026, 0.012, Quat::IDENTITY, metal);
            }
        }
    }
}

pub(super) fn ranger(p: &mut Parts) {
    let green = Character::Ranger.suit_color();
    let tan = Character::Ranger.trim_color();
    let cloak = c(0.2, 0.27, 0.17);
    let leather = c(0.36, 0.23, 0.12);
    let dark_leather = c(0.22, 0.14, 0.08);
    let wood = c(0.42, 0.28, 0.15);
    let metal = c(0.5, 0.5, 0.5);
    let skin = c(0.85, 0.66, 0.52);
    let hair = c(0.35, 0.22, 0.12);
    base_body(
        p,
        &Palette {
            skin,
            suit: green,
            suit_dark: c(0.3, 0.26, 0.19),
            glove: c(0.42, 0.28, 0.15),
            boot: c(0.3, 0.19, 0.1),
        },
        0.96,
    );
    let s = Bone::Spine;
    // Leather jerkin laced up the front.
    p.k(s).blob(v(0.0, 0.32, 0.0), v(0.24, 0.21, 0.15), leather);
    p.k(s).blob(v(0.0, 0.13, 0.0), v(0.16, 0.14, 0.12), leather);
    p.k(s).cuboid(v(0.0, 0.27, -0.146), v(0.014, 0.34, 0.01), dark_leather);
    for i in 0..5 {
        for r in [0.6f32, -0.6] {
            p.k(s).cuboid_rot(
                v(0.0, 0.14 + i as f32 * 0.06, -0.153),
                v(0.04, 0.005, 0.005),
                Quat::from_rotation_z(r),
                tan,
            );
        }
    }
    // Quiver strap over the left shoulder, round to the right hip.
    let front = [
        v(-0.15, 0.5, -0.06),
        v(-0.07, 0.4, -0.16),
        v(0.07, 0.22, -0.16),
        v(0.16, 0.06, -0.13),
    ];
    let behind = [
        v(-0.15, 0.5, -0.06),
        v(-0.12, 0.46, 0.16),
        v(0.06, 0.24, 0.17),
        v(0.16, 0.06, 0.13),
    ];
    for pts in [front, behind] {
        for w in pts.windows(2) {
            p.k(s).beam(w[0], w[1], Vec2::new(0.035, 0.012), dark_leather);
        }
    }
    p.k(s).cuboid_rot(
        v(0.0, 0.31, -0.168),
        v(0.035, 0.03, 0.01),
        Quat::from_rotation_z(0.85),
        metal,
    );
    // Short hooded mantle rounded over the shoulders, with a tan edge,
    // pointed at the back.
    p.k(s).blob(v(0.0, 0.4, 0.02), v(0.255, 0.07, 0.2), tan);
    p.k(s).blob(v(0.0, 0.44, 0.02), v(0.25, 0.1, 0.195), cloak);
    p.k(s).cuboid_rot(
        v(0.0, 0.27, 0.19),
        v(0.36, 0.3, 0.02),
        Quat::from_rotation_x(-0.1),
        cloak,
    );
    p.k(s).cuboid_rot(
        v(0.0, 0.13, 0.205),
        v(0.18, 0.18, 0.02),
        Quat::from_rotation_x(-0.1) * Quat::from_rotation_z(PI / 4.0),
        cloak,
    );
    // Scarf tail down the chest.
    p.k(s).beam(
        v(0.05, 0.52, -0.12),
        v(0.09, 0.36, -0.17),
        Vec2::new(0.045, 0.012),
        tan,
    );
    // Quiver over the right shoulder, full of fletched arrows.
    let qt = Quat::from_rotation_z(-0.4);
    let qc = v(0.06, 0.3, 0.25);
    p.k(s).cyl(qc, 0.055, 0.45, qt, leather);
    p.k(s)
        .torus(qc + qt * v(0.0, 0.225, 0.0), 0.009, 0.056, qt, dark_leather);
    p.k(s)
        .cyl(qc + qt * v(0.0, -0.225, 0.0), 0.058, 0.03, qt, dark_leather);
    for y in [-0.1, 0.1] {
        p.k(s).torus(qc + qt * v(0.0, y, 0.0), 0.006, 0.057, qt, tan);
    }
    let feathers = [tan, c(0.9, 0.88, 0.82), c(0.6, 0.22, 0.15)];
    for i in 0..6 {
        let a = i as f32 * 1.05;
        let off = v(a.cos() * 0.03, 0.0, a.sin() * 0.03);
        let tip = qc + qt * (off + v(0.0, 0.38 + jitter(i) * 0.03, 0.0));
        p.k(s).cyl_between(
            qc + qt * (off + v(0.0, 0.18, 0.0)),
            tip,
            0.005,
            wood,
        );
        for r in [0.0, FRAC_PI_2] {
            p.k(s).cuboid_rot(
                tip - qt * v(0.0, 0.035, 0.0),
                v(0.004, 0.06, 0.024),
                qt * Quat::from_rotation_y(r + a),
                feathers[i % 3],
            );
        }
    }
    // Longbow across the back the other way, with its string.
    let bt = Quat::from_rotation_z(0.6);
    let bc = v(0.0, 0.3, 0.32);
    let stave = |y: f32| bc + bt * v(0.08 * (1.0 - (y / 0.62).powi(2)), y, 0.0);
    for i in 0..10 {
        let y0 = -0.62 + i as f32 * 0.124;
        let thick = 0.016 - (y0 + 0.062).abs() * 0.012;
        p.k(s)
            .capsule_between(stave(y0), stave(y0 + 0.124), thick, wood);
    }
    p.k(s)
        .cyl(stave(0.0), 0.02, 0.1, bt, dark_leather);
    p.k(s)
        .cyl_between(stave(-0.62), stave(0.62), 0.003, c(0.85, 0.82, 0.7));
    // Belt with a pouch and a hunting knife; short tunic tails.
    let pv = Bone::Pelvis;
    p.k(pv).cyl(v(0.0, 0.05, 0.0), 0.19, 0.055, Quat::IDENTITY, dark_leather);
    p.k(pv).cuboid(v(0.0, 0.05, -0.19), v(0.05, 0.045, 0.015), metal);
    p.k(pv).cuboid(v(0.19, 0.0, -0.04), v(0.05, 0.09, 0.09), leather);
    p.k(pv).cuboid(v(0.19, 0.04, -0.04), v(0.055, 0.025, 0.095), dark_leather);
    let kr = Quat::from_rotation_z(-0.3);
    p.k(pv)
        .cuboid_rot(v(-0.2, -0.06, -0.06), v(0.03, 0.18, 0.045), kr, dark_leather);
    p.k(pv)
        .cyl(v(-0.175, 0.06, -0.06), 0.012, 0.07, kr, wood);
    p.k(pv).sphere(v(-0.165, 0.1, -0.06), 0.014, metal);
    for (z, r) in [(-0.135, 0.12f32), (0.135, -0.12)] {
        p.k(pv).cuboid_rot(
            v(0.0, -0.13, z),
            v(0.26, 0.22, 0.02),
            Quat::from_rotation_x(r),
            green,
        );
    }
    // Face under a deep hood, with a scarf over the lower face.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.16, 0.0), v(0.12, 0.14, 0.13), skin);
    face_shape(p, 0.18, -0.125, 0.1, skin);
    eyes(p, 0.18, -0.115, 0.045, c(0.25, 0.4, 0.2), hair);
    p.k(n).blob(v(0.0, 0.25, -0.08), v(0.09, 0.03, 0.05), hair);
    p.k(n).blob(v(0.0, 0.2, 0.05), v(0.155, 0.165, 0.15), cloak);
    p.k(n).blob_rot(
        v(0.0, 0.29, -0.07),
        v(0.14, 0.045, 0.09),
        Quat::from_rotation_x(0.3),
        cloak,
    );
    for x in [-1.0f32, 1.0] {
        p.k(n).blob(v(x * 0.1, 0.15, -0.05), v(0.04, 0.12, 0.08), cloak);
    }
    p.k(n).cone(
        v(0.0, 0.22, 0.21),
        0.06,
        0.16,
        Quat::from_rotation_x(2.2),
        cloak,
    );
    p.k(n).torus(
        v(0.0, 0.19, -0.11),
        0.012,
        0.11,
        Quat::from_rotation_x(FRAC_PI_2 - 0.15),
        c(0.15, 0.2, 0.12),
    );
    // Scarf wrapped round the neck and up over the chin.
    p.k(n).blob(v(0.0, 0.06, -0.02), v(0.12, 0.1, 0.12), tan);
    for y in [0.04, 0.08] {
        p.k(n).cuboid_rot(
            v(0.0, y, -0.135),
            v(0.14, 0.008, 0.01),
            Quat::from_rotation_z(0.08),
            c(0.66, 0.44, 0.24),
        );
    }
    for side in 0..2 {
        let x = sx(side);
        let u = Bone::UpperArm(side);
        if side == 0 {
            // Layered leather guard on the bow shoulder.
            for j in 0..3 {
                p.k(u).cuboid_rot(
                    v(x * 0.075, -0.02 - j as f32 * 0.045, 0.0),
                    v(0.02, 0.06, 0.13 - j as f32 * 0.02),
                    Quat::from_rotation_z(x * 0.3),
                    if j % 2 == 0 { leather } else { dark_leather },
                );
            }
        }
        // Laced leather bracers.
        let f = Bone::Forearm(side);
        p.k(f).frustum(
            v(0.0, -0.14, 0.0),
            0.052,
            0.062,
            0.16,
            Quat::IDENTITY,
            leather,
        );
        for j in 0..3 {
            p.k(f).torus(
                v(0.0, -0.08 - j as f32 * 0.05, 0.0),
                0.006,
                0.06 - j as f32 * 0.003,
                Quat::IDENTITY,
                dark_leather,
            );
        }
        if side == 1 {
            // Wrist grapple launcher: housing, barrel, three-pronged hook
            // and a spool of rope.
            let gun = c(0.25, 0.26, 0.27);
            p.k(f).cuboid(v(0.075, -0.13, 0.0), v(0.05, 0.15, 0.07), gun);
            p.k(f).cyl_between(
                v(0.085, -0.06, -0.02),
                v(0.085, -0.235, -0.02),
                0.018,
                metal,
            );
            p.k(f).cone(
                v(0.085, -0.25, -0.02),
                0.014,
                0.03,
                Quat::from_rotation_x(PI),
                metal,
            );
            for k in 0..3 {
                let a = k as f32 * 2.094;
                let d = v(a.cos(), 0.0, a.sin());
                p.k(f).beam(
                    v(0.085, -0.24, -0.02),
                    v(0.085, -0.22, -0.02) + d * 0.03,
                    Vec2::new(0.006, 0.006),
                    metal,
                );
            }
            disc(p.k(f), v(0.1, -0.1, 0.03), Vec3::X, 0.03, 0.03, gun);
            for dx in [-0.008, 0.008] {
                p.k(f).torus(
                    v(0.1 + dx, -0.1, 0.03),
                    0.007,
                    0.026,
                    Quat::from_rotation_z(FRAC_PI_2),
                    tan,
                );
            }
            p.g(f).sphere(v(0.1, -0.16, -0.03), 0.007, c(0.4, 1.0, 0.4));
        } else {
            // Archer's bracer plate on the inside of the bow arm.
            p.k(f).cuboid(v(0.05, -0.14, 0.0), v(0.015, 0.13, 0.07), tan);
        }
        // Tall boots with folded cuffs and buckles.
        let n = Bone::Shin(side);
        p.k(n).frustum(
            v(0.0, -0.22, 0.0),
            0.058,
            0.066,
            0.36,
            Quat::IDENTITY,
            c(0.3, 0.19, 0.1),
        );
        p.k(n).frustum(
            v(0.0, -0.04, 0.0),
            0.078,
            0.07,
            0.07,
            Quat::IDENTITY,
            dark_leather,
        );
        for y in [-0.18, -0.3] {
            p.k(n).torus(v(0.0, y, 0.0), 0.006, 0.064, Quat::IDENTITY, dark_leather);
            p.k(n).cuboid(v(x * 0.064, y, 0.0), v(0.008, 0.02, 0.02), metal);
        }
    }
}
