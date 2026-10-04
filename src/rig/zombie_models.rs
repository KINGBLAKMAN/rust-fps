//! The three kinds of zombie, built on the same skeleton as the players:
//! walkers (in three outfits), spitters and brutes.

use super::*;

/// Colours for a zombie body.
struct ZPal {
    skin: Color,
    shirt: Color,
    pants: Color,
    shoe: Color,
}

const BONE: Color = Color::srgb(0.86, 0.82, 0.7);
const BLOOD: Color = Color::srgb(0.32, 0.03, 0.03);
const GORE: Color = Color::srgb(0.45, 0.08, 0.08);

fn darker(c: Color, k: f32) -> Color {
    let l = c.to_srgba();
    Color::srgb(l.red * k, l.green * k, l.blue * k)
}

/// Skin-coloured limbs with torn sleeves and trouser legs; `bulk` scales
/// the muscles, `thin` the bones showing through.
fn zombie_body(p: &mut Parts, pal: &ZPal, bulk: f32, bare_feet: [bool; 2]) {
    let dark_skin = darker(pal.skin, 0.75);
    p.k(Bone::Pelvis)
        .blob(v(0.0, -0.02, 0.0), v(0.18, 0.12, 0.13) * bulk, pal.pants);
    let s = Bone::Spine;
    p.k(s)
        .blob(v(0.0, 0.1, 0.0), v(0.16, 0.16, 0.12) * bulk, pal.shirt);
    p.k(s)
        .blob(v(0.0, 0.33, 0.0), v(0.23, 0.2, 0.14) * bulk, pal.shirt);
    p.k(s).cyl(
        v(0.0, 0.52, 0.0),
        0.06 * bulk,
        0.1,
        Quat::IDENTITY,
        pal.skin,
    );
    // Ragged hem.
    for i in 0..7 {
        let a = i as f32 / 7.0 * std::f32::consts::TAU;
        let len = 0.05 + 0.04 * ((i * 7 % 5) as f32 / 5.0);
        p.k(s).cuboid_rot(
            v(
                a.sin() * 0.15 * bulk,
                -0.02 - len / 2.0,
                a.cos() * 0.11 * bulk,
            ),
            v(0.06, len, 0.015),
            Quat::from_rotation_y(a),
            pal.shirt,
        );
    }
    for side in 0..2 {
        let x = sx(side);
        let u = Bone::UpperArm(side);
        // Torn sleeve stub, then bare arm.
        p.k(u)
            .blob(v(0.0, -0.03, 0.0), v(0.085, 0.09, 0.085) * bulk, pal.shirt);
        p.k(u).capsule_between(
            v(0.0, -0.05, 0.0),
            v(0.0, -0.27, 0.0),
            0.05 * bulk,
            pal.skin,
        );
        p.k(u)
            .blob(v(x * 0.02, -0.15, -0.03), v(0.03, 0.05, 0.02), dark_skin);
        let f = Bone::Forearm(side);
        p.k(f).sphere(v(0.0, 0.0, 0.0), 0.047 * bulk, pal.skin);
        p.k(f).capsule_between(
            v(0.0, -0.02, 0.0),
            v(0.0, -0.23, 0.0),
            0.043 * bulk,
            pal.skin,
        );
        // A wound on the forearm.
        p.k(f)
            .blob(v(0.0, -0.12, -0.035 * bulk), v(0.025, 0.04, 0.012), GORE);
        let h = Bone::Hand(side);
        p.k(h)
            .cuboid(v(0.0, -0.05, 0.0), v(0.08, 0.1, 0.03), pal.skin);
        p.k(h).capsule_between(
            v(-x * 0.042, -0.03, -0.014),
            v(-x * 0.05, -0.08, -0.04),
            0.013,
            pal.skin,
        );
        let t = Bone::Thigh(side);
        p.k(t).capsule_between(
            v(0.0, -0.02, 0.0),
            v(0.0, -0.42, 0.0),
            0.075 * bulk,
            pal.pants,
        );
        p.k(t)
            .blob(v(x * 0.03, -0.25, -0.06), v(0.03, 0.05, 0.02), BLOOD);
        let n = Bone::Shin(side);
        p.k(n).sphere(v(0.0, 0.0, 0.0), 0.064 * bulk, pal.pants);
        p.k(n).capsule_between(
            v(0.0, -0.02, 0.0),
            v(0.0, -0.3, 0.0),
            0.06 * bulk,
            pal.pants,
        );
        // Shredded trouser ends.
        for i in 0..5 {
            let a = i as f32 / 5.0 * std::f32::consts::TAU + side as f32;
            p.k(n).cuboid_rot(
                v(
                    a.sin() * 0.055,
                    -0.32 - (i % 2) as f32 * 0.03,
                    a.cos() * 0.055,
                ),
                v(0.04, 0.06, 0.012),
                Quat::from_rotation_y(a),
                pal.pants,
            );
        }
        p.k(n).capsule_between(
            v(0.0, -0.3, 0.0),
            v(0.0, -0.42, 0.0),
            0.045 * bulk,
            pal.skin,
        );
        if bare_feet[side] {
            p.k(n)
                .blob(v(0.0, -0.475, -0.06), v(0.055, 0.035, 0.12), pal.skin);
            for i in 0..4 {
                p.k(n)
                    .sphere(v((i as f32 - 1.5) * 0.022, -0.49, -0.17), 0.014, dark_skin);
            }
        } else {
            p.k(n)
                .cuboid(v(0.0, -0.47, -0.05), v(0.12, 0.08, 0.25), pal.shoe);
            p.k(n).cuboid(
                v(0.0, -0.505, -0.05),
                v(0.13, 0.02, 0.27),
                darker(pal.shoe, 0.5),
            );
        }
    }
}

/// A rotting head: sunken eyes that glow, a hanging jaw and broken teeth.
fn zombie_head(p: &mut Parts, skin: Color, eyes: Color, jaw: f32, scale: f32) {
    let n = Bone::Neck;
    let dark = darker(skin, 0.6);
    let o = v(0.0, 0.16, 0.0);
    let k = |x: f32, y: f32, z: f32| o + v(x, y, z) * scale;
    p.k(n)
        .blob(k(0.0, 0.01, 0.0), v(0.115, 0.135, 0.125) * scale, skin);
    // Brow ridge, cheekbones and hollow cheeks.
    p.k(n)
        .cuboid(k(0.0, 0.05, -0.1), v(0.17, 0.03, 0.04) * scale, dark);
    for x in [-1.0, 1.0] {
        p.k(n).blob(
            k(x * 0.07, -0.01, -0.085),
            v(0.035, 0.025, 0.03) * scale,
            skin,
        );
        p.k(n)
            .blob(k(x * 0.08, -0.04, -0.06), v(0.03, 0.04, 0.03) * scale, dark);
        // Deep eye sockets with a glow at the back.
        p.k(n)
            .sphere(k(x * 0.045, 0.02, -0.1), 0.03 * scale, c(0.06, 0.04, 0.04));
        p.g(n)
            .sphere(k(x * 0.045, 0.02, -0.118), 0.013 * scale, eyes);
        // Ears, one torn.
        p.k(n).blob(
            k(x * 0.115, 0.0, 0.0),
            v(0.015, 0.04 - (x + 1.0) * 0.008, 0.025) * scale,
            skin,
        );
    }
    // Nose cavity.
    p.k(n).blob(
        k(0.0, -0.02, -0.12),
        v(0.018, 0.022, 0.012) * scale,
        c(0.08, 0.04, 0.04),
    );
    // Mouth: dark inside, upper teeth, hanging jaw with lower teeth.
    p.k(n).blob(
        k(0.0, -0.07, -0.08),
        v(0.055, 0.025 + jaw * 0.02, 0.04) * scale,
        c(0.2, 0.03, 0.03),
    );
    for i in 0..6 {
        let x = (i as f32 - 2.5) * 0.016;
        let len = if i % 3 == 1 { 0.012 } else { 0.02 };
        p.k(n).cuboid(
            k(x, -0.057, -0.112),
            v(0.011, len, 0.008) * scale,
            c(0.85, 0.8, 0.55),
        );
        p.k(n).cuboid(
            k(x * 0.9, -0.09 - jaw * 0.035, -0.105),
            v(0.011, 0.016, 0.008) * scale,
            c(0.8, 0.75, 0.5),
        );
    }
    p.k(n).blob(
        k(0.0, -0.105 - jaw * 0.035, -0.07),
        v(0.07, 0.025, 0.065) * scale,
        skin,
    );
    // Bite wound on the neck and a patch of bare skull.
    p.k(n)
        .blob(k(0.06, -0.12, -0.03), v(0.03, 0.02, 0.025) * scale, GORE);
    p.k(n)
        .blob(k(-0.06, 0.09, 0.02), v(0.045, 0.035, 0.05) * scale, BONE);
}

/// Exposed ribs on one side of the chest.
fn ribs(p: &mut Parts, side: f32, bulk: f32) {
    let s = Bone::Spine;
    p.k(s).blob(
        v(side * 0.13 * bulk, 0.3, -0.07 * bulk),
        v(0.06, 0.1, 0.05),
        c(0.18, 0.03, 0.03),
    );
    for i in 0..4 {
        let y = 0.24 + i as f32 * 0.045;
        p.k(s).beam(
            v(side * 0.09 * bulk, y, -0.11 * bulk),
            v(side * 0.17 * bulk, y - 0.015, -0.05 * bulk),
            Vec2::new(0.012, 0.014),
            BONE,
        );
    }
}

/// Blood splashes on clothes.
fn stains(p: &mut Parts, seed: u32) {
    let s = Bone::Spine;
    for i in 0..5u32 {
        let h = |k: u32| (((seed * 31 + i * 17 + k * 7) % 97) as f32) / 97.0;
        let x = (h(1) - 0.5) * 0.3;
        let y = 0.1 + h(2) * 0.35;
        let front = if h(3) > 0.4 { -1.0 } else { 1.0 };
        p.k(s).blob(
            v(x, y, front * 0.135),
            v(0.03 + h(4) * 0.04, 0.03 + h(5) * 0.05, 0.012),
            BLOOD,
        );
    }
}

pub(super) fn walker(p: &mut Parts, variant: u8) {
    let skins = [c(0.45, 0.55, 0.4), c(0.62, 0.63, 0.55), c(0.52, 0.47, 0.42)];
    let skin = skins[variant as usize % 3];
    let eyes = c(1.0, 0.75, 0.2);
    match variant % 3 {
        0 => {
            // Construction worker: hi-vis vest, hard hat, jeans, work boots.
            zombie_body(
                p,
                &ZPal {
                    skin,
                    shirt: c(0.35, 0.33, 0.3),
                    pants: c(0.2, 0.28, 0.45),
                    shoe: c(0.35, 0.22, 0.12),
                },
                1.05,
                [false, false],
            );
            let s = Bone::Spine;
            let orange = c(0.95, 0.45, 0.08);
            for x in [-1.0, 1.0] {
                p.k(s)
                    .cuboid(v(x * 0.12, 0.3, -0.13), v(0.12, 0.36, 0.03), orange);
                p.k(s).cuboid(
                    v(x * 0.12, 0.25, -0.147),
                    v(0.12, 0.03, 0.01),
                    c(0.85, 0.85, 0.8),
                );
                p.k(s)
                    .cuboid(v(x * 0.12, 0.35, 0.13), v(0.12, 0.36, 0.03), orange);
            }
            p.k(s)
                .cuboid(v(0.0, 0.25, 0.147), v(0.3, 0.03, 0.01), c(0.85, 0.85, 0.8));
            p.k(s)
                .cuboid(v(0.0, 0.42, 0.147), v(0.3, 0.03, 0.01), c(0.85, 0.85, 0.8));
            // Tool belt.
            p.k(s)
                .cuboid(v(0.0, 0.03, 0.0), v(0.36, 0.05, 0.27), c(0.3, 0.2, 0.1));
            p.k(s)
                .cuboid(v(0.15, -0.02, -0.1), v(0.06, 0.1, 0.05), c(0.3, 0.2, 0.1));
            zombie_head(p, skin, eyes, 0.6, 1.0);
            // Dented hard hat, tipped back.
            let n = Bone::Neck;
            let hat = Quat::from_rotation_x(0.25) * Quat::from_rotation_z(0.15);
            p.k(n).blob_rot(
                v(0.0, 0.28, 0.02),
                v(0.14, 0.08, 0.16),
                hat,
                c(0.95, 0.8, 0.1),
            );
            p.k(n).cuboid_rot(
                v(0.0, 0.25, -0.08),
                v(0.2, 0.015, 0.1),
                hat,
                c(0.9, 0.75, 0.1),
            );
            ribs(p, 1.0, 1.05);
            stains(p, 3);
        }
        1 => {
            // Office worker: bloody white shirt, loose tie, one shoe.
            zombie_body(
                p,
                &ZPal {
                    skin,
                    shirt: c(0.85, 0.85, 0.82),
                    pants: c(0.3, 0.3, 0.33),
                    shoe: c(0.08, 0.07, 0.07),
                },
                0.95,
                [true, false],
            );
            let s = Bone::Spine;
            p.k(s).beam(
                v(0.0, 0.48, -0.14),
                v(0.03, 0.2, -0.15),
                Vec2::new(0.05, 0.012),
                c(0.6, 0.08, 0.1),
            );
            p.k(s)
                .cuboid(v(0.0, 0.47, -0.13), v(0.05, 0.04, 0.03), c(0.6, 0.08, 0.1));
            for i in 0..4 {
                p.k(s).sphere(
                    v(0.0, 0.42 - i as f32 * 0.08, -0.138),
                    0.008,
                    c(0.9, 0.9, 0.9),
                );
            }
            p.k(s)
                .cuboid(v(0.0, 0.03, 0.0), v(0.35, 0.04, 0.26), c(0.12, 0.08, 0.06));
            zombie_head(p, skin, eyes, 1.0, 0.98);
            // Broken glasses and combed-over hair.
            let n = Bone::Neck;
            for x in [-1.0, 1.0] {
                p.k(n).torus(
                    v(x * 0.045, 0.18, -0.13),
                    0.004,
                    0.025,
                    Quat::from_rotation_x(FRAC_PI_2),
                    c(0.1, 0.1, 0.1),
                );
            }
            p.k(n)
                .blob(v(0.02, 0.28, 0.0), v(0.11, 0.035, 0.11), c(0.25, 0.2, 0.15));
            ribs(p, -1.0, 0.95);
            stains(p, 7);
            stains(p, 11);
        }
        _ => {
            // Hoodie, cargo trousers and trainers.
            zombie_body(
                p,
                &ZPal {
                    skin,
                    shirt: c(0.3, 0.38, 0.3),
                    pants: c(0.45, 0.42, 0.3),
                    shoe: c(0.85, 0.85, 0.85),
                },
                1.0,
                [false, false],
            );
            let s = Bone::Spine;
            // Hood bunched at the back, pocket, drawstrings.
            p.k(s).torus(
                v(0.0, 0.52, 0.04),
                0.045,
                0.12,
                Quat::from_rotation_x(0.3),
                c(0.28, 0.35, 0.28),
            );
            p.k(s)
                .cuboid(v(0.0, 0.12, -0.13), v(0.22, 0.1, 0.03), c(0.26, 0.33, 0.26));
            for x in [-0.04, 0.04] {
                p.k(s).cyl(
                    v(x, 0.4, -0.14),
                    0.005,
                    0.14,
                    Quat::IDENTITY,
                    c(0.9, 0.9, 0.9),
                );
            }
            for side in 0..2 {
                p.k(Bone::Thigh(side)).cuboid(
                    v(sx(side) * 0.075, -0.22, 0.0),
                    v(0.04, 0.12, 0.12),
                    c(0.4, 0.37, 0.26),
                );
                p.k(Bone::Shin(side)).cuboid(
                    v(0.0, -0.505, -0.05),
                    v(0.13, 0.025, 0.27),
                    c(0.8, 0.2, 0.2),
                );
            }
            zombie_head(p, skin, eyes, 0.3, 1.0);
            let n = Bone::Neck;
            for (x, z) in [(-0.05, 0.02), (0.04, -0.03), (0.0, 0.06), (0.07, 0.05)] {
                p.k(n)
                    .blob(v(x, 0.27, z), v(0.05, 0.03, 0.05), c(0.12, 0.1, 0.08));
            }
            stains(p, 13);
        }
    }
}

pub(super) fn spitter(p: &mut Parts) {
    let skin = c(0.55, 0.48, 0.58);
    let acid = c(0.5, 1.0, 0.25);
    zombie_body(
        p,
        &ZPal {
            skin,
            shirt: c(0.55, 0.68, 0.72),
            pants: c(0.5, 0.62, 0.66),
            shoe: skin,
        },
        0.82,
        [true, true],
    );
    let s = Bone::Spine;
    // Hospital gown ties at the back, open to show the spine.
    for y in [0.15, 0.3, 0.45] {
        p.k(s)
            .cuboid(v(0.0, y, 0.135), v(0.08, 0.012, 0.02), c(0.9, 0.9, 0.9));
    }
    for i in 0..6 {
        p.k(s).sphere(
            v(0.0, 0.08 + i as f32 * 0.07, 0.11),
            0.022,
            darker(skin, 0.8),
        );
    }
    // Glowing pustules on the back and shoulders.
    for (x, y, r) in [
        (-0.1, 0.4, 0.04),
        (0.08, 0.32, 0.05),
        (-0.04, 0.22, 0.03),
        (0.12, 0.45, 0.03),
        (-0.13, 0.15, 0.035),
    ] {
        p.k(s).sphere(v(x, y, 0.12), r * 1.15, darker(skin, 0.7));
        p.g(s).sphere(v(x, y, 0.13), r, acid);
    }
    // Swollen, glowing throat sac.
    p.g(s).blob(v(0.0, 0.53, -0.07), v(0.07, 0.06, 0.06), acid);
    p.k(s).blob(
        v(0.0, 0.53, -0.06),
        v(0.08, 0.07, 0.065),
        darker(skin, 0.85),
    );
    zombie_head(p, skin, acid, 1.6, 0.95);
    let n = Bone::Neck;
    // Veins on the bald head and drool.
    for (a, b) in [
        (v(-0.06, 0.24, -0.08), v(-0.02, 0.29, 0.02)),
        (v(0.05, 0.22, -0.09), v(0.08, 0.27, 0.03)),
        (v(0.0, 0.29, -0.04), v(0.0, 0.27, 0.08)),
    ] {
        p.k(n).beam(a, b, Vec2::splat(0.008), c(0.3, 0.15, 0.35));
    }
    p.g(n).beam(
        v(0.01, 0.06, -0.12),
        v(0.015, -0.06, -0.13),
        Vec2::splat(0.008),
        acid,
    );
    p.g(n).sphere(v(0.015, -0.065, -0.13), 0.012, acid);
    for side in 0..2 {
        // Long bony fingers: knuckles.
        p.k(Bone::Hand(side))
            .sphere(v(0.0, -0.1, 0.0), 0.02, darker(skin, 0.8));
        p.g(Bone::Forearm(side))
            .sphere(v(0.0, -0.08, 0.04), 0.018, acid);
    }
}

pub(super) fn brute(p: &mut Parts) {
    let skin = c(0.5, 0.4, 0.36);
    let orange = c(0.9, 0.42, 0.1);
    let metal = c(0.35, 0.35, 0.38);
    zombie_body(
        p,
        &ZPal {
            skin,
            shirt: orange,
            pants: orange,
            shoe: c(0.15, 0.12, 0.1),
        },
        1.35,
        [false, false],
    );
    let s = Bone::Spine;
    // Torn-open jumpsuit showing a huge chest.
    p.k(s)
        .blob(v(-0.07, 0.36, -0.12), v(0.11, 0.09, 0.07), skin);
    p.k(s).blob(v(0.07, 0.36, -0.12), v(0.11, 0.09, 0.07), skin);
    p.k(s)
        .blob(v(0.0, 0.2, -0.12), v(0.12, 0.08, 0.06), darker(skin, 0.9));
    p.k(s)
        .cuboid(v(0.0, 0.25, -0.18), v(0.05, 0.12, 0.01), darker(skin, 0.7));
    // Prison number on the back.
    p.k(s)
        .cuboid(v(0.0, 0.36, 0.19), v(0.2, 0.08, 0.01), c(0.95, 0.95, 0.9));
    for i in 0..4 {
        p.k(s).cuboid(
            v(-0.06 + i as f32 * 0.04, 0.36, 0.197),
            v(0.02, 0.05, 0.005),
            c(0.1, 0.1, 0.1),
        );
    }
    // Bone spikes bursting out of the back and shoulders.
    for (pos, rot, len) in [
        (v(0.0, 0.45, 0.17), -0.6, 0.18),
        (v(-0.1, 0.35, 0.18), -0.9, 0.14),
        (v(0.1, 0.33, 0.18), -0.8, 0.16),
        (v(0.0, 0.25, 0.17), -1.0, 0.12),
    ] {
        p.k(s).blob(pos, v(0.05, 0.03, 0.04), GORE);
        p.k(s).cone(
            pos + Quat::from_rotation_x(rot) * v(0.0, len / 2.0, 0.0),
            0.035,
            len,
            Quat::from_rotation_x(rot),
            BONE,
        );
    }
    p.k(s)
        .cuboid(v(0.0, 0.03, 0.0), v(0.5, 0.06, 0.36), c(0.2, 0.15, 0.1));
    zombie_head(p, skin, c(1.0, 0.15, 0.05), 0.4, 0.92);
    let n = Bone::Neck;
    // Iron muzzle strapped over the jaw.
    p.k(n)
        .cuboid(v(0.0, 0.08, -0.11), v(0.15, 0.08, 0.04), metal);
    for i in 0..4 {
        p.k(n).cuboid(
            v(-0.045 + i as f32 * 0.03, 0.08, -0.133),
            v(0.008, 0.07, 0.01),
            c(0.2, 0.2, 0.22),
        );
    }
    for x in [-1.0, 1.0] {
        p.k(n).beam(
            v(x * 0.075, 0.09, -0.1),
            v(x * 0.11, 0.12, 0.06),
            Vec2::new(0.02, 0.008),
            c(0.2, 0.15, 0.1),
        );
    }
    for side in 0..2 {
        let x = sx(side);
        // Massive shoulders with spikes, and forearms like hams.
        let u = Bone::UpperArm(side);
        p.k(u)
            .blob(v(x * 0.03, 0.02, 0.0), v(0.14, 0.12, 0.13), skin);
        p.k(u).cone(
            v(x * 0.08, 0.12, 0.02),
            0.03,
            0.12,
            Quat::from_rotation_z(-x * 0.5),
            BONE,
        );
        let f = Bone::Forearm(side);
        p.k(f).blob(v(0.0, -0.1, 0.0), v(0.085, 0.13, 0.085), skin);
        // Shackle with a broken chain.
        p.k(f)
            .torus(v(0.0, -0.2, 0.0), 0.018, 0.075, Quat::IDENTITY, metal);
        for i in 0..3 {
            let rot = if i % 2 == 0 {
                Quat::from_rotation_x(FRAC_PI_2)
            } else {
                Quat::from_rotation_z(FRAC_PI_2)
            };
            p.k(f).torus(
                v(x * 0.02, -0.25 - i as f32 * 0.045, 0.06),
                0.008,
                0.025,
                rot,
                metal,
            );
        }
        p.k(Bone::Hand(side))
            .cuboid(v(0.0, -0.05, 0.0), v(0.11, 0.12, 0.05), skin);
        p.k(Bone::Thigh(side))
            .blob(v(0.0, -0.18, 0.0), v(0.1, 0.16, 0.1), orange);
    }
    stains(p, 21);
}
