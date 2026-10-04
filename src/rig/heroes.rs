//! The playable characters' outfits, built on the shared body.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use super::{base_body, sx, v, Bone, Palette, Parts};
use crate::kit::c;

pub(super) fn striker(p: &mut Parts) {
    let blue = c(0.15, 0.35, 0.85);
    let navy = c(0.1, 0.14, 0.26);
    let orange = c(0.95, 0.55, 0.1);
    let dark = c(0.09, 0.09, 0.1);
    let metal = c(0.32, 0.34, 0.38);
    base_body(
        p,
        &Palette {
            skin: c(0.82, 0.62, 0.48),
            suit: navy,
            suit_dark: c(0.12, 0.15, 0.22),
            glove: dark,
            boot: c(0.14, 0.12, 0.1),
        },
        1.0,
    );
    let s = Bone::Spine;
    // Plate carrier with mag pouches.
    p.k(s).cuboid(v(0.0, 0.33, 0.0), v(0.44, 0.34, 0.3), blue);
    p.k(s).cuboid(
        v(0.0, 0.33, -0.155),
        v(0.36, 0.26, 0.02),
        c(0.12, 0.28, 0.7),
    );
    for i in 0..3 {
        let x = -0.12 + i as f32 * 0.12;
        p.k(s).cuboid(v(x, 0.24, -0.175), v(0.1, 0.13, 0.05), navy);
        p.k(s).cuboid(v(x, 0.3, -0.19), v(0.1, 0.035, 0.03), orange);
    }
    p.k(s)
        .cuboid(v(0.0, 0.43, -0.16), v(0.3, 0.04, 0.02), orange);
    // Shoulder straps, collar, radio and backpack.
    for x in [-0.15, 0.15] {
        p.k(s).cuboid(v(x, 0.5, 0.0), v(0.08, 0.04, 0.32), blue);
    }
    p.k(s)
        .torus(v(0.0, 0.5, 0.0), 0.03, 0.1, Quat::IDENTITY, navy);
    p.k(s)
        .cuboid(v(-0.17, 0.45, -0.17), v(0.05, 0.1, 0.04), dark);
    p.k(s)
        .cyl(v(-0.17, 0.56, -0.17), 0.006, 0.14, Quat::IDENTITY, dark);
    p.k(s)
        .cuboid(v(0.0, 0.3, 0.2), v(0.32, 0.3, 0.12), c(0.16, 0.2, 0.3));
    p.k(s)
        .cuboid(v(0.0, 0.42, 0.265), v(0.26, 0.05, 0.03), orange);
    p.k(s).cuboid(v(0.0, 0.07, 0.0), v(0.36, 0.06, 0.26), dark);
    p.g(s).cuboid(
        v(0.12, 0.38, -0.168),
        v(0.04, 0.015, 0.01),
        c(1.0, 0.6, 0.2),
    );
    // Helmet with a wraparound visor, ear protectors and an antenna.
    let n = Bone::Neck;
    p.k(n)
        .blob(v(0.0, 0.16, 0.0), v(0.12, 0.14, 0.13), c(0.82, 0.62, 0.48));
    p.k(n).blob(v(0.0, 0.2, 0.01), v(0.155, 0.135, 0.16), blue);
    p.k(n).cuboid(v(0.0, 0.27, 0.0), v(0.05, 0.05, 0.3), orange);
    p.k(n)
        .cuboid(v(0.0, 0.07, -0.11), v(0.17, 0.07, 0.06), dark);
    for x in [-1.0, 1.0] {
        p.k(n).cyl(
            v(x * 0.15, 0.15, 0.0),
            0.055,
            0.04,
            Quat::from_rotation_z(FRAC_PI_2),
            dark,
        );
        p.k(n).cyl(
            v(x * 0.172, 0.15, 0.0),
            0.03,
            0.01,
            Quat::from_rotation_z(FRAC_PI_2),
            orange,
        );
    }
    p.k(n).cyl(
        v(-0.15, 0.3, 0.06),
        0.005,
        0.22,
        Quat::from_rotation_x(-0.2),
        dark,
    );
    p.k(n)
        .cuboid(v(0.0, 0.17, -0.14), v(0.27, 0.085, 0.03), dark);
    p.g(n).cuboid(
        v(0.0, 0.17, -0.153),
        v(0.25, 0.06, 0.012),
        c(1.0, 0.55, 0.12),
    );
    for side in 0..2 {
        let x = sx(side);
        // Armoured shoulder pads with a stripe.
        let u = Bone::UpperArm(side);
        p.k(u).blob(v(x * 0.02, 0.0, 0.0), v(0.1, 0.07, 0.11), blue);
        p.k(u)
            .cuboid(v(x * 0.06, -0.02, 0.0), v(0.03, 0.03, 0.18), orange);
        p.k(u).cyl(
            v(0.0, -0.18, 0.0),
            0.063,
            0.05,
            Quat::IDENTITY,
            c(0.12, 0.28, 0.7),
        );
        // Wrist guard and elbow pads.
        let f = Bone::Forearm(side);
        p.k(f).blob(v(0.0, 0.0, 0.01), v(0.06, 0.06, 0.07), dark);
        p.k(f)
            .cuboid(v(0.0, -0.16, -0.03), v(0.08, 0.09, 0.05), metal);
        // Knee pads with an orange cap.
        let n = Bone::Shin(side);
        p.k(n).blob(v(0.0, 0.0, -0.05), v(0.08, 0.08, 0.06), dark);
        p.k(n)
            .blob(v(0.0, 0.0, -0.085), v(0.05, 0.05, 0.03), orange);
        // Thigh pocket and holster (right).
        let t = Bone::Thigh(side);
        p.k(t).cuboid(
            v(x * 0.075, -0.24, 0.0),
            v(0.04, 0.12, 0.12),
            c(0.12, 0.16, 0.3),
        );
        if side == 1 {
            p.k(t)
                .cuboid(v(0.095, -0.16, -0.01), v(0.05, 0.16, 0.08), dark);
            p.k(t)
                .cuboid(v(0.095, -0.06, -0.02), v(0.05, 0.06, 0.04), metal);
        }
    }
}

pub(super) fn warden(p: &mut Parts) {
    let white = c(0.9, 0.91, 0.93);
    let green = c(0.2, 0.65, 0.35);
    let deep = c(0.1, 0.3, 0.18);
    let grey = c(0.35, 0.37, 0.4);
    base_body(
        p,
        &Palette {
            skin: c(0.55, 0.38, 0.28),
            suit: deep,
            suit_dark: c(0.12, 0.2, 0.15),
            glove: c(0.85, 0.86, 0.88),
            boot: c(0.25, 0.27, 0.3),
        },
        1.08,
    );
    let s = Bone::Spine;
    // Heavy chest armour with a glowing cross.
    p.k(s).blob(v(0.0, 0.34, -0.02), v(0.26, 0.22, 0.17), white);
    p.k(s).cuboid(v(0.0, 0.33, -0.16), v(0.2, 0.2, 0.04), green);
    p.g(s).cuboid(
        v(0.0, 0.33, -0.183),
        v(0.12, 0.035, 0.01),
        c(0.4, 1.0, 0.55),
    );
    p.g(s).cuboid(
        v(0.0, 0.33, -0.183),
        v(0.035, 0.12, 0.01),
        c(0.4, 1.0, 0.55),
    );
    p.k(s)
        .cuboid(v(0.0, 0.13, -0.02), v(0.34, 0.08, 0.26), white);
    p.k(s).cuboid(v(0.0, 0.05, 0.0), v(0.38, 0.05, 0.28), grey);
    for x in [-0.12, 0.0, 0.12] {
        p.k(s).cuboid(v(x, 0.05, -0.15), v(0.08, 0.08, 0.05), green);
    }
    // Backpack: medical canister and a frost coil.
    p.k(s).cuboid(v(0.0, 0.32, 0.2), v(0.34, 0.36, 0.14), grey);
    p.k(s)
        .cyl(v(0.0, 0.32, 0.28), 0.075, 0.3, Quat::IDENTITY, white);
    p.g(s).cyl(
        v(0.0, 0.32, 0.28),
        0.06,
        0.24,
        Quat::IDENTITY,
        c(0.35, 1.0, 0.55),
    );
    for y in [0.21, 0.32, 0.43] {
        p.k(s)
            .torus(v(0.0, y, 0.28), 0.012, 0.08, Quat::IDENTITY, grey);
    }
    for x in [-0.13, 0.13] {
        p.k(s)
            .cyl(v(x, 0.32, 0.27), 0.035, 0.3, Quat::IDENTITY, grey);
        p.g(s).torus(
            v(x, 0.4, 0.27),
            0.01,
            0.045,
            Quat::IDENTITY,
            c(0.55, 0.85, 1.0),
        );
        p.g(s).torus(
            v(x, 0.26, 0.27),
            0.01,
            0.045,
            Quat::IDENTITY,
            c(0.55, 0.85, 1.0),
        );
    }
    // Tabard hanging from the belt.
    let pv = Bone::Pelvis;
    p.k(pv)
        .cuboid(v(0.0, -0.17, -0.12), v(0.2, 0.3, 0.02), white);
    p.k(pv)
        .cuboid(v(0.0, -0.17, -0.132), v(0.05, 0.3, 0.01), green);
    p.k(pv)
        .cuboid(v(0.0, -0.17, 0.12), v(0.22, 0.26, 0.02), white);
    // Domed helmet, dark face plate, round green eyes, rebreather.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.19, 0.0), v(0.16, 0.16, 0.165), white);
    p.k(n).cuboid(v(0.0, 0.32, 0.0), v(0.04, 0.04, 0.3), green);
    p.k(n)
        .blob(v(0.0, 0.15, -0.09), v(0.13, 0.11, 0.08), c(0.1, 0.12, 0.12));
    for x in [-0.055, 0.055] {
        p.k(n).cyl(
            v(x, 0.18, -0.16),
            0.038,
            0.03,
            Quat::from_rotation_x(FRAC_PI_2),
            grey,
        );
        p.g(n).cyl(
            v(x, 0.18, -0.172),
            0.03,
            0.01,
            Quat::from_rotation_x(FRAC_PI_2),
            c(0.4, 1.0, 0.55),
        );
    }
    p.k(n).cyl(
        v(0.0, 0.08, -0.16),
        0.04,
        0.05,
        Quat::from_rotation_x(FRAC_PI_2),
        grey,
    );
    for x in [-1.0, 1.0] {
        p.k(n).cyl(
            v(x * 0.08, 0.06, -0.12),
            0.03,
            0.06,
            Quat::from_rotation_z(FRAC_PI_2),
            grey,
        );
        p.k(n)
            .cuboid(v(x * 0.16, 0.18, 0.0), v(0.03, 0.12, 0.12), green);
    }
    for side in 0..2 {
        let x = sx(side);
        // Big pauldrons.
        let u = Bone::UpperArm(side);
        p.k(u)
            .blob(v(x * 0.03, 0.01, 0.0), v(0.14, 0.11, 0.14), white);
        p.k(u)
            .torus(v(x * 0.03, -0.05, 0.0), 0.018, 0.12, Quat::IDENTITY, green);
        p.k(u)
            .cuboid(v(x * 0.15, 0.0, 0.0), v(0.02, 0.06, 0.12), green);
        // Bracers.
        let f = Bone::Forearm(side);
        p.k(f)
            .frustum(v(0.0, -0.13, 0.0), 0.06, 0.07, 0.16, Quat::IDENTITY, white);
        p.k(f)
            .torus(v(0.0, -0.06, 0.0), 0.01, 0.066, Quat::IDENTITY, green);
        // Armoured thighs and knees.
        let t = Bone::Thigh(side);
        p.k(t)
            .cuboid(v(0.0, -0.18, -0.06), v(0.15, 0.24, 0.05), white);
        let n = Bone::Shin(side);
        p.k(n).blob(v(0.0, 0.0, -0.06), v(0.085, 0.09, 0.06), white);
        p.k(n)
            .cuboid(v(0.0, -0.2, -0.065), v(0.11, 0.24, 0.04), white);
        p.k(n)
            .cuboid(v(0.0, -0.2, -0.088), v(0.03, 0.2, 0.01), green);
    }
}

pub(super) fn ronin(p: &mut Parts) {
    let red = c(0.6, 0.08, 0.08);
    let black = c(0.07, 0.06, 0.07);
    let gold = c(0.9, 0.7, 0.25);
    let iron = c(0.2, 0.2, 0.22);
    let cloth = c(0.18, 0.17, 0.2);
    base_body(
        p,
        &Palette {
            skin: c(0.86, 0.68, 0.55),
            suit: c(0.14, 0.12, 0.16),
            suit_dark: cloth,
            glove: black,
            boot: black,
        },
        1.0,
    );
    let s = Bone::Spine;
    // Lacquered chest plate (do) laced in rows.
    p.k(s)
        .frustum(v(0.0, 0.3, 0.0), 0.24, 0.19, 0.38, Quat::IDENTITY, red);
    for i in 0..5 {
        let y = 0.14 + i as f32 * 0.07;
        p.k(s).torus(
            v(0.0, y, 0.0),
            0.008,
            0.19 + i as f32 * 0.011,
            Quat::IDENTITY,
            black,
        );
    }
    p.k(s).cyl(
        v(0.0, 0.36, -0.235),
        0.045,
        0.01,
        Quat::from_rotation_x(FRAC_PI_2),
        gold,
    );
    p.k(s).torus(
        v(0.0, 0.36, -0.235),
        0.006,
        0.055,
        Quat::from_rotation_x(FRAC_PI_2),
        gold,
    );
    // Sash.
    p.k(s).cyl(
        v(0.0, 0.06, 0.0),
        0.2,
        0.08,
        Quat::IDENTITY,
        c(0.85, 0.82, 0.75),
    );
    p.k(s).blob(
        v(0.13, 0.05, -0.16),
        v(0.05, 0.04, 0.03),
        c(0.85, 0.82, 0.75),
    );
    // Katana across the back.
    let tilt = Quat::from_rotation_z(0.7);
    p.k(s).cyl(v(0.0, 0.3, 0.17), 0.025, 0.8, tilt, black);
    p.k(s).cyl(
        tilt * v(0.0, 0.4, 0.0) + v(0.0, 0.3, 0.17),
        0.04,
        0.012,
        tilt,
        gold,
    );
    p.k(s).cyl(
        tilt * v(0.0, 0.52, 0.0) + v(0.0, 0.3, 0.17),
        0.018,
        0.22,
        tilt,
        c(0.85, 0.82, 0.75),
    );
    for i in 0..4 {
        let at = tilt * v(0.0, 0.44 + i as f32 * 0.05, 0.0) + v(0.0, 0.3, 0.17);
        p.k(s).torus(at, 0.005, 0.02, tilt, black);
    }
    p.k(s).cyl(
        tilt * v(0.0, -0.4, 0.0) + v(0.0, 0.3, 0.17),
        0.028,
        0.03,
        tilt,
        gold,
    );
    // Skirt plates (kusazuri).
    let pv = Bone::Pelvis;
    for i in 0..5 {
        let a = -1.1 + i as f32 * 0.55;
        let d = v(a.sin(), 0.0, -a.cos());
        let rot = Quat::from_rotation_y(-a) * Quat::from_rotation_x(0.18);
        for j in 0..3 {
            let col = if j % 2 == 0 { red } else { black };
            p.k(pv).cuboid_rot(
                d * (0.17 + j as f32 * 0.012) + v(0.0, -0.08 - j as f32 * 0.08, 0.0),
                v(0.14, 0.075, 0.02),
                rot,
                col,
            );
        }
    }
    for i in 0..2 {
        let a = PI - 0.4 + i as f32 * 0.8;
        let d = v(a.sin(), 0.0, -a.cos());
        p.k(pv).cuboid_rot(
            d * 0.17 + v(0.0, -0.15, 0.0),
            v(0.15, 0.22, 0.02),
            Quat::from_rotation_y(-a) * Quat::from_rotation_x(0.15),
            red,
        );
    }
    // Kabuto helmet, neck guard, crest and an iron mask with red eyes.
    let n = Bone::Neck;
    p.k(n)
        .blob(v(0.0, 0.16, 0.0), v(0.12, 0.14, 0.13), c(0.86, 0.68, 0.55));
    p.k(n).blob(v(0.0, 0.22, 0.0), v(0.15, 0.12, 0.155), black);
    for i in 0..8 {
        let a = i as f32 * PI / 4.0;
        p.k(n)
            .sphere(v(a.sin() * 0.135, 0.27, a.cos() * 0.14), 0.012, gold);
    }
    for i in 0..3 {
        let y = 0.17 - i as f32 * 0.05;
        let r = 0.17 + i as f32 * 0.03;
        p.k(n).frustum(
            v(0.0, y, 0.035),
            r - 0.02,
            r,
            0.05,
            Quat::from_rotation_x(-0.25),
            if i % 2 == 0 { red } else { black },
        );
    }
    // Golden crescent crest.
    for side in [-1.0, 1.0] {
        for i in 0..4 {
            let a = 0.35 + i as f32 * 0.25;
            let at = v(side * a.sin() * 0.16, 0.25 + a.cos() * 0.17, -0.16);
            p.k(n).cuboid_rot(
                at,
                v(0.03, 0.06, 0.012),
                Quat::from_rotation_z(-side * a),
                gold,
            );
        }
    }
    p.k(n).cyl(
        v(0.0, 0.25, -0.15),
        0.025,
        0.02,
        Quat::from_rotation_x(FRAC_PI_2),
        gold,
    );
    // Mask (menpo).
    p.k(n).blob(v(0.0, 0.11, -0.1), v(0.11, 0.08, 0.06), iron);
    p.k(n)
        .cuboid(v(0.0, 0.13, -0.155), v(0.03, 0.05, 0.03), iron);
    for x in [-1.0, 1.0] {
        p.k(n).cuboid_rot(
            v(x * 0.04, 0.095, -0.155),
            v(0.05, 0.012, 0.012),
            Quat::from_rotation_z(x * 0.4),
            c(0.85, 0.85, 0.85),
        );
        p.g(n).cuboid_rot(
            v(x * 0.05, 0.18, -0.13),
            v(0.05, 0.014, 0.01),
            Quat::from_rotation_z(-x * 0.2),
            c(1.0, 0.15, 0.1),
        );
    }
    for side in 0..2 {
        let x = sx(side);
        // Layered shoulder plates (sode).
        let u = Bone::UpperArm(side);
        for j in 0..4 {
            let col = if j % 2 == 0 { red } else { black };
            p.k(u).cuboid_rot(
                v(x * (0.09 + j as f32 * 0.008), 0.0 - j as f32 * 0.055, 0.0),
                v(0.02, 0.07, 0.19),
                Quat::from_rotation_z(x * 0.25),
                col,
            );
        }
        p.k(u).cuboid_rot(
            v(x * 0.09, 0.03, 0.0),
            v(0.025, 0.02, 0.2),
            Quat::from_rotation_z(x * 0.25),
            gold,
        );
        // Armoured sleeves (kote).
        let f = Bone::Forearm(side);
        for j in 0..3 {
            p.k(f).cuboid(
                v(0.0, -0.06 - j as f32 * 0.06, -0.045),
                v(0.08, 0.05, 0.02),
                if j == 1 { gold } else { iron },
            );
        }
        // Wide trousers and shin guards (suneate).
        let t = Bone::Thigh(side);
        p.k(t)
            .frustum(v(0.0, -0.22, 0.0), 0.08, 0.11, 0.4, Quat::IDENTITY, cloth);
        let n = Bone::Shin(side);
        p.k(n)
            .frustum(v(0.0, -0.05, 0.0), 0.11, 0.075, 0.12, Quat::IDENTITY, cloth);
        p.k(n).cuboid(v(0.0, -0.2, -0.06), v(0.11, 0.26, 0.03), red);
        for j in 0..3 {
            p.k(n).cuboid(
                v(0.0, -0.12 - j as f32 * 0.08, -0.078),
                v(0.1, 0.012, 0.01),
                gold,
            );
        }
    }
}

pub(super) fn tinker(p: &mut Parts) {
    let yellow = c(0.95, 0.75, 0.12);
    let overall = c(0.28, 0.36, 0.45);
    let shirt = c(0.75, 0.38, 0.15);
    let leather = c(0.38, 0.24, 0.13);
    let steel = c(0.6, 0.62, 0.66);
    let dark = c(0.12, 0.12, 0.13);
    let skin = c(0.9, 0.7, 0.55);
    base_body(
        p,
        &Palette {
            skin,
            suit: shirt,
            suit_dark: overall,
            glove: c(0.55, 0.42, 0.25),
            boot: c(0.32, 0.2, 0.1),
        },
        1.03,
    );
    let s = Bone::Spine;
    // Overall bib, braces with buckles and a chest pocket with tools.
    p.k(s)
        .cuboid(v(0.0, 0.22, -0.11), v(0.3, 0.32, 0.06), overall);
    p.k(s)
        .cuboid(v(0.0, 0.1, 0.0), v(0.33, 0.16, 0.25), overall);
    for x in [-0.1, 0.1] {
        p.k(s).beam(
            v(x, 0.36, -0.14),
            v(x * 1.3, 0.5, 0.0),
            Vec2::new(0.04, 0.015),
            overall,
        );
        p.k(s).beam(
            v(x * 1.3, 0.5, 0.0),
            v(x, 0.3, 0.16),
            Vec2::new(0.04, 0.015),
            overall,
        );
        p.k(s)
            .cuboid(v(x, 0.35, -0.145), v(0.05, 0.04, 0.015), steel);
    }
    p.k(s).cuboid(
        v(0.05, 0.22, -0.145),
        v(0.1, 0.1, 0.015),
        c(0.24, 0.31, 0.4),
    );
    p.k(s).cyl(
        v(0.03, 0.29, -0.148),
        0.008,
        0.1,
        Quat::IDENTITY,
        c(0.9, 0.2, 0.2),
    );
    p.k(s).cyl(
        v(0.07, 0.29, -0.148),
        0.008,
        0.09,
        Quat::IDENTITY,
        c(0.2, 0.3, 0.9),
    );
    // Backpack frame: battery cell, coiled cable, antenna.
    p.k(s)
        .cuboid(v(0.0, 0.3, 0.22), v(0.36, 0.42, 0.16), c(0.4, 0.4, 0.38));
    p.k(s)
        .cuboid(v(0.0, 0.3, 0.305), v(0.3, 0.36, 0.01), yellow);
    p.k(s)
        .cyl(v(0.1, 0.3, 0.33), 0.05, 0.3, Quat::IDENTITY, dark);
    p.g(s).cyl(
        v(0.1, 0.3, 0.33),
        0.035,
        0.26,
        Quat::IDENTITY,
        c(0.3, 0.9, 1.0),
    );
    p.k(s).torus(
        v(-0.08, 0.22, 0.32),
        0.015,
        0.07,
        Quat::from_rotation_x(FRAC_PI_2),
        dark,
    );
    p.k(s)
        .cyl(v(-0.14, 0.7, 0.26), 0.006, 0.42, Quat::IDENTITY, dark);
    p.g(s).sphere(v(-0.14, 0.92, 0.26), 0.02, c(1.0, 0.2, 0.15));
    for x in [-0.14, 0.14] {
        p.k(s).cuboid(v(x, 0.53, 0.18), v(0.04, 0.1, 0.04), steel);
    }
    // Tool belt: wrench, hammer and pouches.
    let pv = Bone::Pelvis;
    p.k(pv)
        .cyl(v(0.0, 0.06, 0.0), 0.2, 0.07, Quat::IDENTITY, leather);
    p.k(pv)
        .cuboid(v(0.0, 0.06, -0.2), v(0.07, 0.06, 0.02), steel);
    p.k(pv)
        .cuboid(v(-0.17, -0.03, -0.08), v(0.08, 0.13, 0.1), leather);
    p.k(pv)
        .cuboid(v(0.18, -0.02, 0.06), v(0.07, 0.12, 0.12), leather);
    p.k(pv)
        .cuboid(v(-0.2, -0.08, 0.06), v(0.015, 0.22, 0.03), steel);
    p.k(pv).torus(
        v(-0.2, 0.04, 0.06),
        0.012,
        0.03,
        Quat::from_rotation_z(FRAC_PI_2),
        steel,
    );
    p.k(pv)
        .cyl(v(0.21, -0.08, -0.06), 0.012, 0.2, Quat::IDENTITY, leather);
    p.k(pv)
        .cuboid(v(0.21, 0.03, -0.06), v(0.03, 0.03, 0.1), steel);
    // Hard hat with a headlamp, goggles with glowing lenses, beard.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.16, 0.0), v(0.12, 0.14, 0.13), skin);
    p.k(n).blob(
        v(0.0, 0.07, -0.05),
        v(0.11, 0.08, 0.09),
        c(0.42, 0.26, 0.14),
    );
    p.k(n).blob(
        v(0.0, 0.135, -0.12),
        v(0.03, 0.025, 0.03),
        c(0.85, 0.62, 0.48),
    );
    p.k(n).blob(v(0.0, 0.25, 0.0), v(0.145, 0.1, 0.155), yellow);
    p.k(n)
        .cyl(v(0.0, 0.25, 0.0), 0.19, 0.015, Quat::IDENTITY, yellow);
    p.k(n)
        .cuboid(v(0.0, 0.33, 0.0), v(0.04, 0.03, 0.26), c(0.85, 0.65, 0.08));
    p.k(n).cyl(
        v(0.0, 0.29, -0.15),
        0.03,
        0.04,
        Quat::from_rotation_x(FRAC_PI_2),
        dark,
    );
    p.g(n).cyl(
        v(0.0, 0.29, -0.172),
        0.024,
        0.01,
        Quat::from_rotation_x(FRAC_PI_2),
        c(1.0, 1.0, 0.85),
    );
    p.k(n)
        .torus(v(0.0, 0.18, 0.0), 0.012, 0.135, Quat::IDENTITY, dark);
    for x in [-0.05, 0.05] {
        p.k(n).cyl(
            v(x, 0.18, -0.13),
            0.038,
            0.04,
            Quat::from_rotation_x(FRAC_PI_2),
            steel,
        );
        p.g(n).cyl(
            v(x, 0.18, -0.152),
            0.03,
            0.01,
            Quat::from_rotation_x(FRAC_PI_2),
            c(0.3, 0.9, 1.0),
        );
    }
    for side in 0..2 {
        let x = sx(side);
        // Rolled sleeves; the left forearm has a hydraulic brace.
        let u = Bone::UpperArm(side);
        p.k(u)
            .torus(v(0.0, -0.24, 0.0), 0.02, 0.06, Quat::IDENTITY, shirt);
        let f = Bone::Forearm(side);
        p.k(f)
            .capsule_between(v(0.0, -0.02, 0.0), v(0.0, -0.21, 0.0), 0.047, skin);
        p.k(f).frustum(
            v(0.0, -0.21, 0.0),
            0.065,
            0.05,
            0.08,
            Quat::IDENTITY,
            c(0.55, 0.42, 0.25),
        );
        if side == 0 {
            p.k(f)
                .cuboid(v(0.0, -0.1, 0.0), v(0.11, 0.025, 0.11), steel);
            p.k(f)
                .cuboid(v(0.0, -0.18, 0.0), v(0.11, 0.025, 0.11), steel);
            for z in [-0.05, 0.05] {
                p.k(f).cyl(
                    v(x * 0.06, -0.14, z),
                    0.012,
                    0.11,
                    Quat::IDENTITY,
                    c(0.85, 0.75, 0.2),
                );
            }
            p.g(f)
                .cuboid(v(-0.056, -0.14, 0.0), v(0.01, 0.03, 0.04), c(0.3, 0.9, 1.0));
        }
        // Overall legs with knee pads and steel-capped boots.
        let n = Bone::Shin(side);
        p.k(n).blob(v(0.0, 0.0, -0.05), v(0.08, 0.08, 0.06), dark);
        p.k(n)
            .cuboid(v(0.0, -0.475, -0.15), v(0.125, 0.07, 0.06), steel);
        let t = Bone::Thigh(side);
        p.k(t).cuboid(
            v(x * 0.07, -0.2, 0.02),
            v(0.03, 0.12, 0.1),
            c(0.24, 0.31, 0.4),
        );
    }
}

pub(super) fn blaze(p: &mut Parts) {
    let char_ = c(0.13, 0.12, 0.12);
    let suit = c(0.22, 0.2, 0.19);
    let orange = c(1.0, 0.45, 0.08);
    let tank = c(0.7, 0.12, 0.08);
    let metal = c(0.45, 0.43, 0.42);
    let glove = c(0.62, 0.42, 0.2);
    base_body(
        p,
        &Palette {
            skin: char_,
            suit,
            suit_dark: char_,
            glove,
            boot: c(0.1, 0.1, 0.1),
        },
        1.06,
    );
    let s = Bone::Spine;
    // Heat suit with hazard stripes, harness and a scorched chest plate.
    p.k(s).blob(v(0.0, 0.33, 0.0), v(0.25, 0.21, 0.155), suit);
    p.k(s)
        .cuboid(v(0.0, 0.34, -0.14), v(0.26, 0.2, 0.04), metal);
    p.k(s)
        .cuboid(v(0.0, 0.34, -0.162), v(0.2, 0.05, 0.01), c(0.25, 0.22, 0.2));
    for y in [0.12, 0.2] {
        p.k(s)
            .torus(v(0.0, y, 0.0), 0.012, 0.17, Quat::IDENTITY, orange);
    }
    for x in [-0.12, 0.12] {
        p.k(s).beam(
            v(x, 0.05, -0.15),
            v(x, 0.52, 0.0),
            Vec2::new(0.04, 0.02),
            c(0.12, 0.1, 0.08),
        );
        p.k(s).beam(
            v(x, 0.52, 0.0),
            v(x, 0.12, 0.17),
            Vec2::new(0.04, 0.02),
            c(0.12, 0.1, 0.08),
        );
    }
    // Twin fuel tanks with pipes, a gauge and a pilot light.
    for x in [-0.1, 0.1] {
        p.k(s)
            .cyl(v(x, 0.3, 0.22), 0.085, 0.46, Quat::IDENTITY, tank);
        p.k(s).sphere(v(x, 0.53, 0.22), 0.085, tank);
        p.k(s).sphere(v(x, 0.07, 0.22), 0.085, tank);
        p.k(s)
            .cyl(v(x, 0.62, 0.22), 0.025, 0.06, Quat::IDENTITY, metal);
        p.k(s).torus(
            v(x, 0.4, 0.22),
            0.01,
            0.088,
            Quat::IDENTITY,
            c(0.95, 0.85, 0.2),
        );
    }
    p.k(s).cyl(
        v(0.0, 0.62, 0.22),
        0.015,
        0.22,
        Quat::from_rotation_z(FRAC_PI_2),
        metal,
    );
    p.k(s).cyl(
        v(0.0, 0.42, 0.33),
        0.04,
        0.02,
        Quat::from_rotation_x(FRAC_PI_2),
        metal,
    );
    p.k(s).cyl(
        v(0.0, 0.42, 0.342),
        0.032,
        0.005,
        Quat::from_rotation_x(FRAC_PI_2),
        c(0.95, 0.95, 0.9),
    );
    p.k(s).capsule_between(
        v(0.1, 0.02, 0.2),
        v(0.25, 0.02, 0.0),
        0.02,
        c(0.15, 0.15, 0.15),
    );
    p.g(s).sphere(v(0.0, 0.02, 0.3), 0.03, c(1.0, 0.55, 0.1));
    // Belt with spare canisters.
    let pv = Bone::Pelvis;
    p.k(pv).cyl(
        v(0.0, 0.06, 0.0),
        0.2,
        0.06,
        Quat::IDENTITY,
        c(0.12, 0.1, 0.08),
    );
    for a in [-0.6, 0.6, 2.6] {
        let d = v((a as f32).sin(), 0.0, -(a as f32).cos());
        p.k(pv).cyl(
            d * 0.21 + v(0.0, 0.0, 0.0),
            0.035,
            0.12,
            Quat::IDENTITY,
            tank,
        );
    }
    // Hood and gas mask with big amber lenses and three filters.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.18, 0.01), v(0.16, 0.17, 0.165), suit);
    p.k(n).torus(
        v(0.0, 0.17, -0.06),
        0.02,
        0.12,
        Quat::from_rotation_x(FRAC_PI_2 - 0.2),
        char_,
    );
    p.k(n).blob(
        v(0.0, 0.15, -0.09),
        v(0.12, 0.12, 0.08),
        c(0.08, 0.08, 0.08),
    );
    for x in [-0.055, 0.055] {
        p.k(n).cyl(
            v(x, 0.19, -0.155),
            0.045,
            0.03,
            Quat::from_rotation_x(FRAC_PI_2),
            metal,
        );
        p.g(n).cyl(
            v(x, 0.19, -0.168),
            0.036,
            0.01,
            Quat::from_rotation_x(FRAC_PI_2),
            c(1.0, 0.6, 0.12),
        );
    }
    p.k(n).cyl(
        v(0.0, 0.09, -0.19),
        0.045,
        0.08,
        Quat::from_rotation_x(FRAC_PI_2 - 0.4),
        metal,
    );
    p.k(n).cyl(
        v(0.0, 0.08, -0.23),
        0.05,
        0.02,
        Quat::from_rotation_x(FRAC_PI_2 - 0.4),
        orange,
    );
    for x in [-1.0, 1.0] {
        p.k(n).cyl(
            v(x * 0.1, 0.08, -0.13),
            0.035,
            0.06,
            Quat::from_rotation_y(x * 0.8) * Quat::from_rotation_x(FRAC_PI_2),
            metal,
        );
    }
    p.k(n)
        .cuboid(v(0.0, 0.3, -0.05), v(0.2, 0.025, 0.2), orange);
    for side in 0..2 {
        let x = sx(side);
        // Heat shields on the shoulders, long gauntlets.
        let u = Bone::UpperArm(side);
        p.k(u)
            .blob(v(x * 0.02, 0.0, 0.0), v(0.1, 0.075, 0.11), metal);
        p.k(u)
            .torus(v(0.0, -0.15, 0.0), 0.012, 0.064, Quat::IDENTITY, orange);
        let f = Bone::Forearm(side);
        p.k(f)
            .frustum(v(0.0, -0.15, 0.0), 0.058, 0.075, 0.2, Quat::IDENTITY, glove);
        p.k(f).torus(
            v(0.0, -0.06, 0.0),
            0.012,
            0.073,
            Quat::IDENTITY,
            c(0.4, 0.26, 0.1),
        );
        // Hazard stripes and heavy boots.
        let t = Bone::Thigh(side);
        p.k(t)
            .torus(v(0.0, -0.25, 0.0), 0.012, 0.085, Quat::IDENTITY, orange);
        let n = Bone::Shin(side);
        p.k(n)
            .torus(v(0.0, -0.2, 0.0), 0.012, 0.07, Quat::IDENTITY, orange);
        p.k(n)
            .cuboid(v(0.0, -0.46, -0.05), v(0.135, 0.11, 0.27), c(0.1, 0.1, 0.1));
        p.k(n)
            .cuboid(v(0.0, -0.46, -0.17), v(0.13, 0.07, 0.04), metal);
    }
}

pub(super) fn valkyrie(p: &mut Parts) {
    let violet = c(0.26, 0.2, 0.42);
    let deep = c(0.16, 0.12, 0.26);
    let silver = c(0.78, 0.8, 0.86);
    let steel = c(0.5, 0.52, 0.58);
    let gold = c(0.92, 0.74, 0.32);
    let hair = c(0.93, 0.88, 0.74);
    let skin = c(0.93, 0.78, 0.68);
    let rune = c(0.45, 0.9, 1.0);
    base_body(
        p,
        &Palette {
            skin,
            suit: violet,
            suit_dark: deep,
            glove: silver,
            boot: c(0.24, 0.22, 0.32),
        },
        0.9,
    );
    // A woman's build: narrower waist, fuller hips.
    let pv = Bone::Pelvis;
    p.k(pv).blob(v(0.0, -0.03, 0.0), v(0.19, 0.12, 0.14), deep);
    let s = Bone::Spine;
    p.k(s).blob(v(0.0, 0.12, 0.0), v(0.14, 0.13, 0.11), violet);
    // Fitted silver cuirass shaped over the chest, with a ridge and runes.
    p.k(s).blob(v(0.0, 0.33, -0.01), v(0.2, 0.17, 0.13), silver);
    for x in [-0.06, 0.06] {
        p.k(s)
            .blob(v(x, 0.355, -0.075), v(0.085, 0.07, 0.065), silver);
    }
    p.k(s)
        .blob(v(0.0, 0.35, -0.07), v(0.07, 0.075, 0.06), silver);
    p.k(s)
        .cuboid(v(0.0, 0.33, -0.12), v(0.012, 0.18, 0.02), steel);
    p.g(s)
        .cuboid(v(0.0, 0.25, -0.128), v(0.06, 0.008, 0.008), rune);
    p.g(s)
        .cuboid(v(0.0, 0.21, -0.12), v(0.04, 0.008, 0.008), rune);
    // Laced bodice over the stomach and a gold belt.
    p.k(s)
        .blob(v(0.0, 0.15, -0.005), v(0.135, 0.1, 0.105), deep);
    for i in 0..4 {
        let y = 0.09 + i as f32 * 0.035;
        p.k(s).cuboid_rot(
            v(0.0, y, -0.105),
            v(0.05, 0.006, 0.006),
            Quat::from_rotation_z(0.5),
            silver,
        );
        p.k(s).cuboid_rot(
            v(0.0, y, -0.105),
            v(0.05, 0.006, 0.006),
            Quat::from_rotation_z(-0.5),
            silver,
        );
    }
    p.k(pv)
        .cyl(v(0.0, 0.06, 0.0), 0.19, 0.05, Quat::IDENTITY, gold);
    p.k(pv).cyl(
        v(0.0, 0.06, -0.19),
        0.035,
        0.012,
        Quat::from_rotation_x(FRAC_PI_2),
        silver,
    );
    // Gorget and a short cape.
    p.k(s)
        .torus(v(0.0, 0.5, 0.0), 0.025, 0.095, Quat::IDENTITY, silver);
    p.k(s).cuboid_rot(
        v(0.0, 0.3, 0.17),
        v(0.36, 0.42, 0.02),
        Quat::from_rotation_x(-0.12),
        deep,
    );
    p.k(s).cuboid_rot(
        v(0.0, 0.08, 0.2),
        v(0.34, 0.06, 0.02),
        Quat::from_rotation_x(-0.12),
        gold,
    );
    // A short spear slung across the back.
    let tilt = Quat::from_rotation_z(-0.6);
    let back = v(0.0, 0.3, 0.2);
    p.k(s).cyl(back, 0.015, 0.95, tilt, steel);
    p.k(s).blob(
        back + tilt * v(0.0, 0.55, 0.0),
        v(0.03, 0.09, 0.008),
        silver,
    );
    p.g(s)
        .cyl(back + tilt * v(0.0, 0.3, 0.0), 0.017, 0.12, tilt, rune);
    // Battle skirt: cloth with armoured panels front, sides and back.
    for i in 0..6 {
        let a = i as f32 * PI / 3.0 + PI / 6.0;
        let d = v(a.sin(), 0.0, -a.cos());
        let rot = Quat::from_rotation_y(-a) * Quat::from_rotation_x(0.2);
        p.k(pv).cuboid_rot(
            d * 0.18 + v(0.0, -0.13, 0.0),
            v(0.17, 0.24, 0.015),
            rot,
            if i % 2 == 0 { violet } else { deep },
        );
        p.k(pv).cuboid_rot(
            d * 0.19 + v(0.0, -0.06, 0.0),
            v(0.12, 0.08, 0.01),
            rot,
            silver,
        );
    }
    // Face, braided hair and a winged helm.
    let n = Bone::Neck;
    p.k(n)
        .blob(v(0.0, 0.16, -0.005), v(0.098, 0.122, 0.108), skin);
    for x in [-0.036, 0.036] {
        p.k(n)
            .sphere(v(x, 0.168, -0.098), 0.011, c(0.95, 0.95, 0.97));
        p.k(n).sphere(v(x, 0.168, -0.105), 0.0065, c(0.2, 0.4, 0.6));
        p.k(n)
            .cuboid(v(x, 0.181, -0.1), v(0.028, 0.005, 0.008), c(0.12, 0.1, 0.1));
        p.k(n).cuboid_rot(
            v(x, 0.198, -0.098),
            v(0.03, 0.005, 0.008),
            Quat::from_rotation_z(-x * 4.0),
            c(0.75, 0.68, 0.52),
        );
    }
    p.k(n).cuboid(
        v(0.0, 0.14, -0.108),
        v(0.012, 0.025, 0.01),
        c(0.9, 0.74, 0.64),
    );
    p.k(n).blob(
        v(0.0, 0.105, -0.1),
        v(0.018, 0.006, 0.006),
        c(0.78, 0.42, 0.44),
    );
    // Hair framing the face under the helm.
    for x in [-1.0f32, 1.0] {
        p.k(n)
            .blob(v(x * 0.088, 0.12, -0.035), v(0.025, 0.08, 0.04), hair);
    }
    p.k(n)
        .blob(v(0.0, 0.222, -0.085), v(0.085, 0.022, 0.03), hair);
    p.k(n).blob(v(0.0, 0.17, 0.035), v(0.12, 0.13, 0.11), hair);
    for i in 0..7 {
        let y = 0.12 - i as f32 * 0.075;
        let z = 0.12 + (i as f32 * 0.02).min(0.06);
        p.k(n).sphere(v(0.0, y, z), 0.038 - i as f32 * 0.003, hair);
    }
    p.k(n)
        .cyl(v(0.0, -0.42, 0.18), 0.02, 0.03, Quat::IDENTITY, gold);
    p.k(n).blob(v(0.0, 0.25, 0.0), v(0.13, 0.08, 0.135), silver);
    p.k(n)
        .cuboid(v(0.0, 0.25, -0.12), v(0.025, 0.1, 0.03), steel);
    p.k(n)
        .torus(v(0.0, 0.2, 0.0), 0.012, 0.125, Quat::IDENTITY, gold);
    for x in [-1.0f32, 1.0] {
        p.k(n).cuboid_rot(
            v(x * 0.11, 0.13, -0.05),
            v(0.02, 0.1, 0.07),
            Quat::from_rotation_z(x * 0.15),
            silver,
        );
        // Wings sweeping up and back.
        for j in 0..4 {
            let j = j as f32;
            p.k(n).cuboid_rot(
                v(x * (0.14 + j * 0.01), 0.28 + j * 0.035, 0.03 + j * 0.035),
                v(0.012, 0.035, 0.13 - j * 0.02),
                Quat::from_rotation_x(-0.7 - j * 0.1) * Quat::from_rotation_z(-x * 0.35),
                if j as i32 % 2 == 0 {
                    silver
                } else {
                    c(0.92, 0.94, 0.98)
                },
            );
        }
    }
    p.g(n)
        .cuboid(v(0.0, 0.29, -0.13), v(0.012, 0.035, 0.01), rune);
    for side in 0..2 {
        let x = sx(side);
        // Feathered pauldrons.
        let u = Bone::UpperArm(side);
        for j in 0..3 {
            let j = j as f32;
            p.k(u).cuboid_rot(
                v(x * (0.075 + j * 0.006), 0.02 - j * 0.045, 0.0),
                v(0.018, 0.065, 0.15 - j * 0.02),
                Quat::from_rotation_z(x * 0.3),
                if j as i32 % 2 == 0 { silver } else { steel },
            );
        }
        // Silver bracers with a rune.
        let f = Bone::Forearm(side);
        p.k(f)
            .frustum(v(0.0, -0.13, 0.0), 0.05, 0.06, 0.16, Quat::IDENTITY, silver);
        p.g(f)
            .cuboid(v(0.0, -0.13, -0.058), v(0.012, 0.08, 0.006), rune);
        // Tall greaves and knee cops.
        let t = Bone::Thigh(side);
        p.k(t)
            .cuboid(v(0.0, -0.3, -0.05), v(0.1, 0.18, 0.03), steel);
        let n = Bone::Shin(side);
        p.k(n).blob(v(0.0, 0.0, -0.05), v(0.07, 0.07, 0.05), silver);
        p.k(n).frustum(
            v(0.0, -0.22, -0.005),
            0.062,
            0.07,
            0.3,
            Quat::IDENTITY,
            silver,
        );
        p.k(n)
            .cuboid(v(0.0, -0.22, -0.068), v(0.02, 0.24, 0.01), gold);
    }
}
