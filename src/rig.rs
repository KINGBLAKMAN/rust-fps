//! The five player characters: fully modelled, jointed figures with
//! shoulders, elbows, wrists, fingers, hips and knees.
//!
//! Each character is built from the modelling kit, one mesh per body part
//! (plus a glowing mesh for lenses, visors and lights). Poses are worked out
//! every frame: walking, crouching, holding the gun (both hands reach for it
//! with two-bone IK) and the emotes (dance, wave, flip off, point, backflip).
//!
//! The model's origin is at its feet, it faces -Z and its right side is +X.

use bevy::prelude::*;
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::data::Character;
use crate::humanoid::GunMount;
use crate::kit::{c, glow_material, vertex_material, Kit};
use crate::Phase;

pub struct RigPlugin;

impl Plugin for RigPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, build_models)
            .add_systems(Update, animate.in_set(Phase::Present))
            ;
    }
}

/// Emotes, in menu order. 0 means none.
pub const EMOTES: [(&str, f32); 5] = [
    ("Dance", 4.0),
    ("Wave", 2.2),
    ("Flip Off", 2.0),
    ("Point", 2.0),
    ("Backflip", 1.3),
];

pub fn emote_length(emote: u8) -> f32 {
    EMOTES.get((emote as usize).wrapping_sub(1)).map_or(0.0, |e| e.1)
}

// Skeleton measurements (metres).
const HIP_Y: f32 = 0.98;
const SPINE_UP: f32 = 0.08;
const NECK_UP: f32 = 0.56;
const SHOULDER: Vec3 = Vec3::new(0.27, 0.47, 0.0);
const HIP_X: f32 = 0.11;
const UPPER_ARM: f32 = 0.31;
const FOREARM: f32 = 0.27;
/// From the wrist to the middle of the palm.
const PALM: f32 = 0.065;
const THIGH: f32 = 0.45;
const KNUCKLES: f32 = 0.1;
/// Guns look right a little bigger than life on these figures.
const GUN_SCALE: f32 = 1.15;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Bone {
    Pelvis,
    Spine,
    Neck,
    UpperArm(usize),
    Forearm(usize),
    Hand(usize),
    Thigh(usize),
    Shin(usize),
    /// Fingers of each hand, index to little.
    Finger(usize, usize),
}

/// -1 for the left side (index 0), +1 for the right (index 1).
fn sx(side: usize) -> f32 {
    if side == 0 {
        -1.0
    } else {
        1.0
    }
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct RigAssets {
    models: HashMap<Character, Vec<(Bone, Handle<Mesh>, bool)>>,
    finger: HashMap<Character, Handle<Mesh>>,
    mat: Handle<StandardMaterial>,
    glow: Handle<StandardMaterial>,
}

/// Kits for each body part as it is being modelled.
#[derive(Default)]
struct Parts {
    solid: HashMap<Bone, Kit>,
    glow: HashMap<Bone, Kit>,
}

impl Parts {
    fn k(&mut self, b: Bone) -> &mut Kit {
        self.solid.entry(b).or_default()
    }
    fn g(&mut self, b: Bone) -> &mut Kit {
        self.glow.entry(b).or_default()
    }
}

/// Colours shared by the basic body.
struct Palette {
    skin: Color,
    suit: Color,
    suit_dark: Color,
    glove: Color,
    boot: Color,
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// The body every character shares: limbs, hands, torso, head shape.
fn base_body(p: &mut Parts, pal: &Palette, bulk: f32) {
    // Pelvis.
    p.k(Bone::Pelvis).blob(v(0.0, -0.02, 0.0), v(0.18, 0.12, 0.13) * bulk, pal.suit_dark);
    // Torso: abdomen and chest.
    let s = Bone::Spine;
    p.k(s).blob(v(0.0, 0.1, 0.0), v(0.16, 0.16, 0.12) * bulk, pal.suit);
    p.k(s).blob(v(0.0, 0.33, 0.0), v(0.23, 0.2, 0.14) * bulk, pal.suit);
    p.k(s).cyl(v(0.0, 0.52, 0.0), 0.06, 0.1, Quat::IDENTITY, pal.skin);
    for side in 0..2 {
        let x = sx(side);
        // Upper arm with a round shoulder.
        let u = Bone::UpperArm(side);
        p.k(u).sphere(v(0.0, -0.02, 0.0), 0.08 * bulk, pal.suit);
        p.k(u).capsule_between(v(0.0, -0.04, 0.0), v(0.0, -0.27, 0.0), 0.058 * bulk, pal.suit);
        // Forearm with a cuff.
        let f = Bone::Forearm(side);
        p.k(f).sphere(v(0.0, 0.0, 0.0), 0.052 * bulk, pal.suit);
        p.k(f).capsule_between(v(0.0, -0.02, 0.0), v(0.0, -0.22, 0.0), 0.05 * bulk, pal.suit);
        p.k(f).cyl(v(0.0, -0.235, 0.0), 0.047, 0.05, Quat::IDENTITY, pal.glove);
        // Hand: palm, back and thumb. Fingers are separate joints.
        let h = Bone::Hand(side);
        p.k(h).cuboid(v(0.0, -0.05, 0.0), v(0.085, 0.1, 0.035), pal.glove);
        p.k(h).capsule_between(v(-x * 0.045, -0.03, -0.015), v(-x * 0.05, -0.08, -0.04), 0.014, pal.glove);
        // Thigh.
        let t = Bone::Thigh(side);
        p.k(t).capsule_between(v(0.0, -0.02, 0.0), v(0.0, -0.42, 0.0), 0.078 * bulk, pal.suit_dark);
        // Shin and boot.
        let n = Bone::Shin(side);
        p.k(n).sphere(v(0.0, 0.0, 0.0), 0.066 * bulk, pal.suit_dark);
        p.k(n).capsule_between(v(0.0, -0.02, 0.0), v(0.0, -0.38, 0.0), 0.062 * bulk, pal.suit_dark);
        p.k(n).cyl(v(0.0, -0.4, 0.0), 0.068, 0.12, Quat::IDENTITY, pal.boot);
        p.k(n).cuboid(v(0.0, -0.47, -0.05), v(0.12, 0.08, 0.25), pal.boot);
        p.k(n).cuboid(v(0.0, -0.505, -0.05), v(0.13, 0.02, 0.27), c(0.08, 0.08, 0.08));
    }
}

fn striker(p: &mut Parts) {
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
    p.k(s).cuboid(v(0.0, 0.33, -0.155), v(0.36, 0.26, 0.02), c(0.12, 0.28, 0.7));
    for i in 0..3 {
        let x = -0.12 + i as f32 * 0.12;
        p.k(s).cuboid(v(x, 0.24, -0.175), v(0.1, 0.13, 0.05), navy);
        p.k(s).cuboid(v(x, 0.3, -0.19), v(0.1, 0.035, 0.03), orange);
    }
    p.k(s).cuboid(v(0.0, 0.43, -0.16), v(0.3, 0.04, 0.02), orange);
    // Shoulder straps, collar, radio and backpack.
    for x in [-0.15, 0.15] {
        p.k(s).cuboid(v(x, 0.5, 0.0), v(0.08, 0.04, 0.32), blue);
    }
    p.k(s).torus(v(0.0, 0.5, 0.0), 0.03, 0.1, Quat::IDENTITY, navy);
    p.k(s).cuboid(v(-0.17, 0.45, -0.17), v(0.05, 0.1, 0.04), dark);
    p.k(s).cyl(v(-0.17, 0.56, -0.17), 0.006, 0.14, Quat::IDENTITY, dark);
    p.k(s).cuboid(v(0.0, 0.3, 0.2), v(0.32, 0.3, 0.12), c(0.16, 0.2, 0.3));
    p.k(s).cuboid(v(0.0, 0.42, 0.265), v(0.26, 0.05, 0.03), orange);
    p.k(s).cuboid(v(0.0, 0.07, 0.0), v(0.36, 0.06, 0.26), dark);
    p.g(s).cuboid(v(0.12, 0.38, -0.168), v(0.04, 0.015, 0.01), c(1.0, 0.6, 0.2));
    // Helmet with a wraparound visor, ear protectors and an antenna.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.16, 0.0), v(0.12, 0.14, 0.13), c(0.82, 0.62, 0.48));
    p.k(n).blob(v(0.0, 0.2, 0.01), v(0.155, 0.135, 0.16), blue);
    p.k(n).cuboid(v(0.0, 0.27, 0.0), v(0.05, 0.05, 0.3), orange);
    p.k(n).cuboid(v(0.0, 0.07, -0.11), v(0.17, 0.07, 0.06), dark);
    for x in [-1.0, 1.0] {
        p.k(n).cyl(v(x * 0.15, 0.15, 0.0), 0.055, 0.04, Quat::from_rotation_z(FRAC_PI_2), dark);
        p.k(n).cyl(v(x * 0.172, 0.15, 0.0), 0.03, 0.01, Quat::from_rotation_z(FRAC_PI_2), orange);
    }
    p.k(n).cyl(v(-0.15, 0.3, 0.06), 0.005, 0.22, Quat::from_rotation_x(-0.2), dark);
    p.k(n).cuboid(v(0.0, 0.17, -0.14), v(0.27, 0.085, 0.03), dark);
    p.g(n).cuboid(v(0.0, 0.17, -0.153), v(0.25, 0.06, 0.012), c(1.0, 0.55, 0.12));
    for side in 0..2 {
        let x = sx(side);
        // Armoured shoulder pads with a stripe.
        let u = Bone::UpperArm(side);
        p.k(u).blob(v(x * 0.02, 0.0, 0.0), v(0.1, 0.07, 0.11), blue);
        p.k(u).cuboid(v(x * 0.06, -0.02, 0.0), v(0.03, 0.03, 0.18), orange);
        p.k(u).cyl(v(0.0, -0.18, 0.0), 0.063, 0.05, Quat::IDENTITY, c(0.12, 0.28, 0.7));
        // Wrist guard and elbow pads.
        let f = Bone::Forearm(side);
        p.k(f).blob(v(0.0, 0.0, 0.01), v(0.06, 0.06, 0.07), dark);
        p.k(f).cuboid(v(0.0, -0.16, -0.03), v(0.08, 0.09, 0.05), metal);
        // Knee pads with an orange cap.
        let n = Bone::Shin(side);
        p.k(n).blob(v(0.0, 0.0, -0.05), v(0.08, 0.08, 0.06), dark);
        p.k(n).blob(v(0.0, 0.0, -0.085), v(0.05, 0.05, 0.03), orange);
        // Thigh pocket and holster (right).
        let t = Bone::Thigh(side);
        p.k(t).cuboid(v(x * 0.075, -0.24, 0.0), v(0.04, 0.12, 0.12), c(0.12, 0.16, 0.3));
        if side == 1 {
            p.k(t).cuboid(v(0.095, -0.16, -0.01), v(0.05, 0.16, 0.08), dark);
            p.k(t).cuboid(v(0.095, -0.06, -0.02), v(0.05, 0.06, 0.04), metal);
        }
    }
}

fn warden(p: &mut Parts) {
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
    p.g(s).cuboid(v(0.0, 0.33, -0.183), v(0.12, 0.035, 0.01), c(0.4, 1.0, 0.55));
    p.g(s).cuboid(v(0.0, 0.33, -0.183), v(0.035, 0.12, 0.01), c(0.4, 1.0, 0.55));
    p.k(s).cuboid(v(0.0, 0.13, -0.02), v(0.34, 0.08, 0.26), white);
    p.k(s).cuboid(v(0.0, 0.05, 0.0), v(0.38, 0.05, 0.28), grey);
    for x in [-0.12, 0.0, 0.12] {
        p.k(s).cuboid(v(x, 0.05, -0.15), v(0.08, 0.08, 0.05), green);
    }
    // Backpack: medical canister and a frost coil.
    p.k(s).cuboid(v(0.0, 0.32, 0.2), v(0.34, 0.36, 0.14), grey);
    p.k(s).cyl(v(0.0, 0.32, 0.28), 0.075, 0.3, Quat::IDENTITY, white);
    p.g(s).cyl(v(0.0, 0.32, 0.28), 0.06, 0.24, Quat::IDENTITY, c(0.35, 1.0, 0.55));
    for y in [0.21, 0.32, 0.43] {
        p.k(s).torus(v(0.0, y, 0.28), 0.012, 0.08, Quat::IDENTITY, grey);
    }
    for x in [-0.13, 0.13] {
        p.k(s).cyl(v(x, 0.32, 0.27), 0.035, 0.3, Quat::IDENTITY, grey);
        p.g(s).torus(v(x, 0.4, 0.27), 0.01, 0.045, Quat::IDENTITY, c(0.55, 0.85, 1.0));
        p.g(s).torus(v(x, 0.26, 0.27), 0.01, 0.045, Quat::IDENTITY, c(0.55, 0.85, 1.0));
    }
    // Tabard hanging from the belt.
    let pv = Bone::Pelvis;
    p.k(pv).cuboid(v(0.0, -0.17, -0.12), v(0.2, 0.3, 0.02), white);
    p.k(pv).cuboid(v(0.0, -0.17, -0.132), v(0.05, 0.3, 0.01), green);
    p.k(pv).cuboid(v(0.0, -0.17, 0.12), v(0.22, 0.26, 0.02), white);
    // Domed helmet, dark face plate, round green eyes, rebreather.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.19, 0.0), v(0.16, 0.16, 0.165), white);
    p.k(n).cuboid(v(0.0, 0.32, 0.0), v(0.04, 0.04, 0.3), green);
    p.k(n).blob(v(0.0, 0.15, -0.09), v(0.13, 0.11, 0.08), c(0.1, 0.12, 0.12));
    for x in [-0.055, 0.055] {
        p.k(n).cyl(v(x, 0.18, -0.16), 0.038, 0.03, Quat::from_rotation_x(FRAC_PI_2), grey);
        p.g(n).cyl(v(x, 0.18, -0.172), 0.03, 0.01, Quat::from_rotation_x(FRAC_PI_2), c(0.4, 1.0, 0.55));
    }
    p.k(n).cyl(v(0.0, 0.08, -0.16), 0.04, 0.05, Quat::from_rotation_x(FRAC_PI_2), grey);
    for x in [-1.0, 1.0] {
        p.k(n).cyl(v(x * 0.08, 0.06, -0.12), 0.03, 0.06, Quat::from_rotation_z(FRAC_PI_2), grey);
        p.k(n).cuboid(v(x * 0.16, 0.18, 0.0), v(0.03, 0.12, 0.12), green);
    }
    for side in 0..2 {
        let x = sx(side);
        // Big pauldrons.
        let u = Bone::UpperArm(side);
        p.k(u).blob(v(x * 0.03, 0.01, 0.0), v(0.14, 0.11, 0.14), white);
        p.k(u).torus(v(x * 0.03, -0.05, 0.0), 0.018, 0.12, Quat::IDENTITY, green);
        p.k(u).cuboid(v(x * 0.15, 0.0, 0.0), v(0.02, 0.06, 0.12), green);
        // Bracers.
        let f = Bone::Forearm(side);
        p.k(f).frustum(v(0.0, -0.13, 0.0), 0.06, 0.07, 0.16, Quat::IDENTITY, white);
        p.k(f).torus(v(0.0, -0.06, 0.0), 0.01, 0.066, Quat::IDENTITY, green);
        // Armoured thighs and knees.
        let t = Bone::Thigh(side);
        p.k(t).cuboid(v(0.0, -0.18, -0.06), v(0.15, 0.24, 0.05), white);
        let n = Bone::Shin(side);
        p.k(n).blob(v(0.0, 0.0, -0.06), v(0.085, 0.09, 0.06), white);
        p.k(n).cuboid(v(0.0, -0.2, -0.065), v(0.11, 0.24, 0.04), white);
        p.k(n).cuboid(v(0.0, -0.2, -0.088), v(0.03, 0.2, 0.01), green);
    }
}

fn ronin(p: &mut Parts) {
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
    p.k(s).frustum(v(0.0, 0.3, 0.0), 0.24, 0.19, 0.38, Quat::IDENTITY, red);
    for i in 0..5 {
        let y = 0.14 + i as f32 * 0.07;
        p.k(s).torus(v(0.0, y, 0.0), 0.008, 0.19 + i as f32 * 0.011, Quat::IDENTITY, black);
    }
    p.k(s).cyl(v(0.0, 0.36, -0.235), 0.045, 0.01, Quat::from_rotation_x(FRAC_PI_2), gold);
    p.k(s).torus(v(0.0, 0.36, -0.235), 0.006, 0.055, Quat::from_rotation_x(FRAC_PI_2), gold);
    // Sash.
    p.k(s).cyl(v(0.0, 0.06, 0.0), 0.2, 0.08, Quat::IDENTITY, c(0.85, 0.82, 0.75));
    p.k(s).blob(v(0.13, 0.05, -0.16), v(0.05, 0.04, 0.03), c(0.85, 0.82, 0.75));
    // Katana across the back.
    let tilt = Quat::from_rotation_z(0.7);
    p.k(s).cyl(v(0.0, 0.3, 0.17), 0.025, 0.8, tilt, black);
    p.k(s).cyl(tilt * v(0.0, 0.4, 0.0) + v(0.0, 0.3, 0.17), 0.04, 0.012, tilt, gold);
    p.k(s).cyl(tilt * v(0.0, 0.52, 0.0) + v(0.0, 0.3, 0.17), 0.018, 0.22, tilt, c(0.85, 0.82, 0.75));
    for i in 0..4 {
        let at = tilt * v(0.0, 0.44 + i as f32 * 0.05, 0.0) + v(0.0, 0.3, 0.17);
        p.k(s).torus(at, 0.005, 0.02, tilt, black);
    }
    p.k(s).cyl(tilt * v(0.0, -0.4, 0.0) + v(0.0, 0.3, 0.17), 0.028, 0.03, tilt, gold);
    // Skirt plates (kusazuri).
    let pv = Bone::Pelvis;
    for i in 0..5 {
        let a = -1.1 + i as f32 * 0.55;
        let d = v(a.sin(), 0.0, -a.cos());
        let rot = Quat::from_rotation_y(-a) * Quat::from_rotation_x(0.18);
        for j in 0..3 {
            let col = if j % 2 == 0 { red } else { black };
            p.k(pv).cuboid_rot(d * (0.17 + j as f32 * 0.012) + v(0.0, -0.08 - j as f32 * 0.08, 0.0), v(0.14, 0.075, 0.02), rot, col);
        }
    }
    for i in 0..2 {
        let a = PI - 0.4 + i as f32 * 0.8;
        let d = v(a.sin(), 0.0, -a.cos());
        p.k(pv).cuboid_rot(d * 0.17 + v(0.0, -0.15, 0.0), v(0.15, 0.22, 0.02), Quat::from_rotation_y(-a) * Quat::from_rotation_x(0.15), red);
    }
    // Kabuto helmet, neck guard, crest and an iron mask with red eyes.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.16, 0.0), v(0.12, 0.14, 0.13), c(0.86, 0.68, 0.55));
    p.k(n).blob(v(0.0, 0.22, 0.0), v(0.15, 0.12, 0.155), black);
    for i in 0..8 {
        let a = i as f32 * PI / 4.0;
        p.k(n).sphere(v(a.sin() * 0.135, 0.27, a.cos() * 0.14), 0.012, gold);
    }
    for i in 0..3 {
        let y = 0.17 - i as f32 * 0.05;
        let r = 0.17 + i as f32 * 0.03;
        p.k(n).frustum(v(0.0, y, 0.035), r - 0.02, r, 0.05, Quat::from_rotation_x(-0.25), if i % 2 == 0 { red } else { black });
    }
    // Golden crescent crest.
    for side in [-1.0, 1.0] {
        for i in 0..4 {
            let a = 0.35 + i as f32 * 0.25;
            let at = v(side * a.sin() * 0.16, 0.25 + a.cos() * 0.17, -0.16);
            p.k(n).cuboid_rot(at, v(0.03, 0.06, 0.012), Quat::from_rotation_z(-side * a), gold);
        }
    }
    p.k(n).cyl(v(0.0, 0.25, -0.15), 0.025, 0.02, Quat::from_rotation_x(FRAC_PI_2), gold);
    // Mask (menpo).
    p.k(n).blob(v(0.0, 0.11, -0.1), v(0.11, 0.08, 0.06), iron);
    p.k(n).cuboid(v(0.0, 0.13, -0.155), v(0.03, 0.05, 0.03), iron);
    for x in [-1.0, 1.0] {
        p.k(n).cuboid_rot(v(x * 0.04, 0.095, -0.155), v(0.05, 0.012, 0.012), Quat::from_rotation_z(x * 0.4), c(0.85, 0.85, 0.85));
        p.g(n).cuboid_rot(v(x * 0.05, 0.18, -0.13), v(0.05, 0.014, 0.01), Quat::from_rotation_z(-x * 0.2), c(1.0, 0.15, 0.1));
    }
    for side in 0..2 {
        let x = sx(side);
        // Layered shoulder plates (sode).
        let u = Bone::UpperArm(side);
        for j in 0..4 {
            let col = if j % 2 == 0 { red } else { black };
            p.k(u).cuboid_rot(v(x * (0.09 + j as f32 * 0.008), 0.0 - j as f32 * 0.055, 0.0), v(0.02, 0.07, 0.19), Quat::from_rotation_z(x * 0.25), col);
        }
        p.k(u).cuboid_rot(v(x * 0.09, 0.03, 0.0), v(0.025, 0.02, 0.2), Quat::from_rotation_z(x * 0.25), gold);
        // Armoured sleeves (kote).
        let f = Bone::Forearm(side);
        for j in 0..3 {
            p.k(f).cuboid(v(0.0, -0.06 - j as f32 * 0.06, -0.045), v(0.08, 0.05, 0.02), if j == 1 { gold } else { iron });
        }
        // Wide trousers and shin guards (suneate).
        let t = Bone::Thigh(side);
        p.k(t).frustum(v(0.0, -0.22, 0.0), 0.08, 0.11, 0.4, Quat::IDENTITY, cloth);
        let n = Bone::Shin(side);
        p.k(n).frustum(v(0.0, -0.05, 0.0), 0.11, 0.075, 0.12, Quat::IDENTITY, cloth);
        p.k(n).cuboid(v(0.0, -0.2, -0.06), v(0.11, 0.26, 0.03), red);
        for j in 0..3 {
            p.k(n).cuboid(v(0.0, -0.12 - j as f32 * 0.08, -0.078), v(0.1, 0.012, 0.01), gold);
        }
    }
}

fn tinker(p: &mut Parts) {
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
    p.k(s).cuboid(v(0.0, 0.22, -0.11), v(0.3, 0.32, 0.06), overall);
    p.k(s).cuboid(v(0.0, 0.1, 0.0), v(0.33, 0.16, 0.25), overall);
    for x in [-0.1, 0.1] {
        p.k(s).beam(v(x, 0.36, -0.14), v(x * 1.3, 0.5, 0.0), Vec2::new(0.04, 0.015), overall);
        p.k(s).beam(v(x * 1.3, 0.5, 0.0), v(x, 0.3, 0.16), Vec2::new(0.04, 0.015), overall);
        p.k(s).cuboid(v(x, 0.35, -0.145), v(0.05, 0.04, 0.015), steel);
    }
    p.k(s).cuboid(v(0.05, 0.22, -0.145), v(0.1, 0.1, 0.015), c(0.24, 0.31, 0.4));
    p.k(s).cyl(v(0.03, 0.29, -0.148), 0.008, 0.1, Quat::IDENTITY, c(0.9, 0.2, 0.2));
    p.k(s).cyl(v(0.07, 0.29, -0.148), 0.008, 0.09, Quat::IDENTITY, c(0.2, 0.3, 0.9));
    // Backpack frame: battery cell, coiled cable, antenna.
    p.k(s).cuboid(v(0.0, 0.3, 0.22), v(0.36, 0.42, 0.16), c(0.4, 0.4, 0.38));
    p.k(s).cuboid(v(0.0, 0.3, 0.305), v(0.3, 0.36, 0.01), yellow);
    p.k(s).cyl(v(0.1, 0.3, 0.33), 0.05, 0.3, Quat::IDENTITY, dark);
    p.g(s).cyl(v(0.1, 0.3, 0.33), 0.035, 0.26, Quat::IDENTITY, c(0.3, 0.9, 1.0));
    p.k(s).torus(v(-0.08, 0.22, 0.32), 0.015, 0.07, Quat::from_rotation_x(FRAC_PI_2), dark);
    p.k(s).cyl(v(-0.14, 0.7, 0.26), 0.006, 0.42, Quat::IDENTITY, dark);
    p.g(s).sphere(v(-0.14, 0.92, 0.26), 0.02, c(1.0, 0.2, 0.15));
    for x in [-0.14, 0.14] {
        p.k(s).cuboid(v(x, 0.53, 0.18), v(0.04, 0.1, 0.04), steel);
    }
    // Tool belt: wrench, hammer and pouches.
    let pv = Bone::Pelvis;
    p.k(pv).cyl(v(0.0, 0.06, 0.0), 0.2, 0.07, Quat::IDENTITY, leather);
    p.k(pv).cuboid(v(0.0, 0.06, -0.2), v(0.07, 0.06, 0.02), steel);
    p.k(pv).cuboid(v(-0.17, -0.03, -0.08), v(0.08, 0.13, 0.1), leather);
    p.k(pv).cuboid(v(0.18, -0.02, 0.06), v(0.07, 0.12, 0.12), leather);
    p.k(pv).cuboid(v(-0.2, -0.08, 0.06), v(0.015, 0.22, 0.03), steel);
    p.k(pv).torus(v(-0.2, 0.04, 0.06), 0.012, 0.03, Quat::from_rotation_z(FRAC_PI_2), steel);
    p.k(pv).cyl(v(0.21, -0.08, -0.06), 0.012, 0.2, Quat::IDENTITY, leather);
    p.k(pv).cuboid(v(0.21, 0.03, -0.06), v(0.03, 0.03, 0.1), steel);
    // Hard hat with a headlamp, goggles with glowing lenses, beard.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.16, 0.0), v(0.12, 0.14, 0.13), skin);
    p.k(n).blob(v(0.0, 0.07, -0.05), v(0.11, 0.08, 0.09), c(0.42, 0.26, 0.14));
    p.k(n).blob(v(0.0, 0.135, -0.12), v(0.03, 0.025, 0.03), c(0.85, 0.62, 0.48));
    p.k(n).blob(v(0.0, 0.25, 0.0), v(0.145, 0.1, 0.155), yellow);
    p.k(n).cyl(v(0.0, 0.25, 0.0), 0.19, 0.015, Quat::IDENTITY, yellow);
    p.k(n).cuboid(v(0.0, 0.33, 0.0), v(0.04, 0.03, 0.26), c(0.85, 0.65, 0.08));
    p.k(n).cyl(v(0.0, 0.29, -0.15), 0.03, 0.04, Quat::from_rotation_x(FRAC_PI_2), dark);
    p.g(n).cyl(v(0.0, 0.29, -0.172), 0.024, 0.01, Quat::from_rotation_x(FRAC_PI_2), c(1.0, 1.0, 0.85));
    p.k(n).torus(v(0.0, 0.18, 0.0), 0.012, 0.135, Quat::IDENTITY, dark);
    for x in [-0.05, 0.05] {
        p.k(n).cyl(v(x, 0.18, -0.13), 0.038, 0.04, Quat::from_rotation_x(FRAC_PI_2), steel);
        p.g(n).cyl(v(x, 0.18, -0.152), 0.03, 0.01, Quat::from_rotation_x(FRAC_PI_2), c(0.3, 0.9, 1.0));
    }
    for side in 0..2 {
        let x = sx(side);
        // Rolled sleeves; the left forearm has a hydraulic brace.
        let u = Bone::UpperArm(side);
        p.k(u).torus(v(0.0, -0.24, 0.0), 0.02, 0.06, Quat::IDENTITY, shirt);
        let f = Bone::Forearm(side);
        p.k(f).capsule_between(v(0.0, -0.02, 0.0), v(0.0, -0.21, 0.0), 0.047, skin);
        p.k(f).frustum(v(0.0, -0.21, 0.0), 0.065, 0.05, 0.08, Quat::IDENTITY, c(0.55, 0.42, 0.25));
        if side == 0 {
            p.k(f).cuboid(v(0.0, -0.1, 0.0), v(0.11, 0.025, 0.11), steel);
            p.k(f).cuboid(v(0.0, -0.18, 0.0), v(0.11, 0.025, 0.11), steel);
            for z in [-0.05, 0.05] {
                p.k(f).cyl(v(x * 0.06, -0.14, z), 0.012, 0.11, Quat::IDENTITY, c(0.85, 0.75, 0.2));
            }
            p.g(f).cuboid(v(-0.056, -0.14, 0.0), v(0.01, 0.03, 0.04), c(0.3, 0.9, 1.0));
        }
        // Overall legs with knee pads and steel-capped boots.
        let n = Bone::Shin(side);
        p.k(n).blob(v(0.0, 0.0, -0.05), v(0.08, 0.08, 0.06), dark);
        p.k(n).cuboid(v(0.0, -0.475, -0.15), v(0.125, 0.07, 0.06), steel);
        let t = Bone::Thigh(side);
        p.k(t).cuboid(v(x * 0.07, -0.2, 0.02), v(0.03, 0.12, 0.1), c(0.24, 0.31, 0.4));
    }
}

fn blaze(p: &mut Parts) {
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
    p.k(s).cuboid(v(0.0, 0.34, -0.14), v(0.26, 0.2, 0.04), metal);
    p.k(s).cuboid(v(0.0, 0.34, -0.162), v(0.2, 0.05, 0.01), c(0.25, 0.22, 0.2));
    for y in [0.12, 0.2] {
        p.k(s).torus(v(0.0, y, 0.0), 0.012, 0.17, Quat::IDENTITY, orange);
    }
    for x in [-0.12, 0.12] {
        p.k(s).beam(v(x, 0.05, -0.15), v(x, 0.52, 0.0), Vec2::new(0.04, 0.02), c(0.12, 0.1, 0.08));
        p.k(s).beam(v(x, 0.52, 0.0), v(x, 0.12, 0.17), Vec2::new(0.04, 0.02), c(0.12, 0.1, 0.08));
    }
    // Twin fuel tanks with pipes, a gauge and a pilot light.
    for x in [-0.1, 0.1] {
        p.k(s).cyl(v(x, 0.3, 0.22), 0.085, 0.46, Quat::IDENTITY, tank);
        p.k(s).sphere(v(x, 0.53, 0.22), 0.085, tank);
        p.k(s).sphere(v(x, 0.07, 0.22), 0.085, tank);
        p.k(s).cyl(v(x, 0.62, 0.22), 0.025, 0.06, Quat::IDENTITY, metal);
        p.k(s).torus(v(x, 0.4, 0.22), 0.01, 0.088, Quat::IDENTITY, c(0.95, 0.85, 0.2));
    }
    p.k(s).cyl(v(0.0, 0.62, 0.22), 0.015, 0.22, Quat::from_rotation_z(FRAC_PI_2), metal);
    p.k(s).cyl(v(0.0, 0.42, 0.33), 0.04, 0.02, Quat::from_rotation_x(FRAC_PI_2), metal);
    p.k(s).cyl(v(0.0, 0.42, 0.342), 0.032, 0.005, Quat::from_rotation_x(FRAC_PI_2), c(0.95, 0.95, 0.9));
    p.k(s).capsule_between(v(0.1, 0.02, 0.2), v(0.25, 0.02, 0.0), 0.02, c(0.15, 0.15, 0.15));
    p.g(s).sphere(v(0.0, 0.02, 0.3), 0.03, c(1.0, 0.55, 0.1));
    // Belt with spare canisters.
    let pv = Bone::Pelvis;
    p.k(pv).cyl(v(0.0, 0.06, 0.0), 0.2, 0.06, Quat::IDENTITY, c(0.12, 0.1, 0.08));
    for a in [-0.6, 0.6, 2.6] {
        let d = v((a as f32).sin(), 0.0, -(a as f32).cos());
        p.k(pv).cyl(d * 0.21 + v(0.0, 0.0, 0.0), 0.035, 0.12, Quat::IDENTITY, tank);
    }
    // Hood and gas mask with big amber lenses and three filters.
    let n = Bone::Neck;
    p.k(n).blob(v(0.0, 0.18, 0.01), v(0.16, 0.17, 0.165), suit);
    p.k(n).torus(v(0.0, 0.17, -0.06), 0.02, 0.12, Quat::from_rotation_x(FRAC_PI_2 - 0.2), char_);
    p.k(n).blob(v(0.0, 0.15, -0.09), v(0.12, 0.12, 0.08), c(0.08, 0.08, 0.08));
    for x in [-0.055, 0.055] {
        p.k(n).cyl(v(x, 0.19, -0.155), 0.045, 0.03, Quat::from_rotation_x(FRAC_PI_2), metal);
        p.g(n).cyl(v(x, 0.19, -0.168), 0.036, 0.01, Quat::from_rotation_x(FRAC_PI_2), c(1.0, 0.6, 0.12));
    }
    p.k(n).cyl(v(0.0, 0.09, -0.19), 0.045, 0.08, Quat::from_rotation_x(FRAC_PI_2 - 0.4), metal);
    p.k(n).cyl(v(0.0, 0.08, -0.23), 0.05, 0.02, Quat::from_rotation_x(FRAC_PI_2 - 0.4), orange);
    for x in [-1.0, 1.0] {
        p.k(n).cyl(v(x * 0.1, 0.08, -0.13), 0.035, 0.06, Quat::from_rotation_y(x * 0.8) * Quat::from_rotation_x(FRAC_PI_2), metal);
    }
    p.k(n).cuboid(v(0.0, 0.3, -0.05), v(0.2, 0.025, 0.2), orange);
    for side in 0..2 {
        let x = sx(side);
        // Heat shields on the shoulders, long gauntlets.
        let u = Bone::UpperArm(side);
        p.k(u).blob(v(x * 0.02, 0.0, 0.0), v(0.1, 0.075, 0.11), metal);
        p.k(u).torus(v(0.0, -0.15, 0.0), 0.012, 0.064, Quat::IDENTITY, orange);
        let f = Bone::Forearm(side);
        p.k(f).frustum(v(0.0, -0.15, 0.0), 0.058, 0.075, 0.2, Quat::IDENTITY, glove);
        p.k(f).torus(v(0.0, -0.06, 0.0), 0.012, 0.073, Quat::IDENTITY, c(0.4, 0.26, 0.1));
        // Hazard stripes and heavy boots.
        let t = Bone::Thigh(side);
        p.k(t).torus(v(0.0, -0.25, 0.0), 0.012, 0.085, Quat::IDENTITY, orange);
        let n = Bone::Shin(side);
        p.k(n).torus(v(0.0, -0.2, 0.0), 0.012, 0.07, Quat::IDENTITY, orange);
        p.k(n).cuboid(v(0.0, -0.46, -0.05), v(0.135, 0.11, 0.27), c(0.1, 0.1, 0.1));
        p.k(n).cuboid(v(0.0, -0.46, -0.17), v(0.13, 0.07, 0.04), metal);
    }
}

fn finger_color(ch: Character) -> Color {
    match ch {
        Character::Striker => c(0.09, 0.09, 0.1),
        Character::Warden => c(0.85, 0.86, 0.88),
        Character::Ronin => c(0.07, 0.06, 0.07),
        Character::Tinker => c(0.55, 0.42, 0.25),
        Character::Blaze => c(0.62, 0.42, 0.2),
    }
}

pub fn model_parts(ch: Character) -> Vec<(Bone, Kit, bool)> {
    let mut p = Parts::default();
    match ch {
        Character::Striker => striker(&mut p),
        Character::Warden => warden(&mut p),
        Character::Ronin => ronin(&mut p),
        Character::Tinker => tinker(&mut p),
        Character::Blaze => blaze(&mut p),
    }
    let mut out = Vec::new();
    for (b, k) in p.solid {
        out.push((b, k, false));
    }
    for (b, k) in p.glow {
        out.push((b, k, true));
    }
    out
}

fn build_models(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut models = HashMap::new();
    let mut finger = HashMap::new();
    for ch in Character::ALL {
        let parts = model_parts(ch)
            .into_iter()
            .filter_map(|(b, k, glow)| k.build().map(|m| (b, meshes.add(m), glow)))
            .collect();
        models.insert(ch, parts);
        let mut k = Kit::new();
        k.capsule_between(v(0.0, -0.005, 0.0), v(0.0, -0.06, 0.0), 0.011, finger_color(ch));
        finger.insert(ch, meshes.add(k.build_or_empty()));
    }
    commands.insert_resource(RigAssets {
        models,
        finger,
        mat: materials.add(vertex_material(0.6, 0.15)),
        glow: materials.add(glow_material(2.5)),
    });
}

// ---------------------------------------------------------------------------
// Spawning
// ---------------------------------------------------------------------------

/// Animation state on the model's root.
#[derive(Component)]
#[allow(dead_code)]
pub struct Rig {
    pub character: Character,
    /// Look pitch (the gun and chest follow it).
    pub pitch: f32,
    /// 0 standing, 1 crouching, 2 sliding.
    pub stance: u8,
    /// Emote being played (0 none) and for how long.
    pub emote: u8,
    pub emote_t: f32,
    /// Holding a gun (else the arms hang).
    pub armed: bool,
    mount: Option<Entity>,
    phase: f32,
    last: Vec3,
    speed: f32,
    /// Smoothed blend into an emote pose.
    blend: f32,
}

impl Rig {
    pub fn play(&mut self, emote: u8) {
        if self.emote != emote {
            self.emote = emote;
            self.emote_t = 0.0;
        }
    }
}

#[derive(Component)]
struct Joint {
    owner: Entity,
    bone: Bone,
}

/// Builds a character under `root`. Returns the gun mount if a gun is given.
pub fn spawn_rig(
    commands: &mut Commands,
    assets: &RigAssets,
    root: Entity,
    character: Character,
    gun: Option<(u8, u8)>,
) -> Option<Entity> {
    let parts = &assets.models[&character];
    let mut joints: HashMap<Bone, Entity> = HashMap::new();
    let mut joint = |commands: &mut Commands, bone: Bone, parent: Entity, at: Vec3| {
        let e = commands
            .spawn((Joint { owner: root, bone }, Transform::from_translation(at), Visibility::default()))
            .id();
        commands.entity(parent).add_child(e);
        for (b, mesh, glow) in parts {
            if *b == bone {
                let mat = if *glow { assets.glow.clone() } else { assets.mat.clone() };
                commands.entity(e).with_child((Mesh3d(mesh.clone()), MeshMaterial3d(mat)));
            }
        }
        joints.insert(bone, e);
        e
    };
    let pelvis = joint(commands, Bone::Pelvis, root, Vec3::Y * HIP_Y);
    let spine = joint(commands, Bone::Spine, pelvis, Vec3::Y * SPINE_UP);
    joint(commands, Bone::Neck, spine, Vec3::Y * NECK_UP);
    for side in 0..2 {
        let x = sx(side);
        let u = joint(commands, Bone::UpperArm(side), spine, SHOULDER * v(x, 1.0, 1.0));
        let f = joint(commands, Bone::Forearm(side), u, -Vec3::Y * UPPER_ARM);
        let h = joint(commands, Bone::Hand(side), f, -Vec3::Y * FOREARM);
        for i in 0..4 {
            let fx = x * (0.03 - i as f32 * 0.02);
            let e = joint(commands, Bone::Finger(side, i), h, v(fx, -KNUCKLES, -0.004));
            commands
                .entity(e)
                .with_child((Mesh3d(assets.finger[&character].clone()), MeshMaterial3d(assets.mat.clone())));
        }
        let t = joint(commands, Bone::Thigh(side), pelvis, v(x * HIP_X, -0.02, 0.0));
        joint(commands, Bone::Shin(side), t, -Vec3::Y * THIGH);
    }
    let mount = gun.map(|(g, skin)| {
        let m = commands.spawn((GunMount::new(g, skin), Transform::default(), Visibility::default())).id();
        commands.entity(root).add_child(m);
        m
    });
    commands.entity(root).insert(Rig {
        character,
        pitch: 0.0,
        stance: 0,
        emote: 0,
        emote_t: 0.0,
        armed: gun.is_some(),
        mount,
        phase: 0.0,
        last: Vec3::ZERO,
        speed: 0.0,
        blend: 0.0,
    });
    mount
}

// ---------------------------------------------------------------------------
// Posing
// ---------------------------------------------------------------------------

/// Where a hand goes, in body space (as if the pelvis were at rest).
#[derive(Clone, Copy)]
struct Reach {
    target: Vec3,
    /// Which way the elbow should point.
    elbow: Vec3,
    /// Hand orientation, in body space (None follows the forearm).
    hand: Option<Quat>,
}

#[derive(Clone, Copy)]
struct Pose {
    pelvis_off: Vec3,
    pelvis: Quat,
    spine: Quat,
    neck: Quat,
    arms: [Reach; 2],
    /// Leg swing forward and knee bend (positive bends).
    thigh: [f32; 2],
    knee: [f32; 2],
    /// Leg spread out to the side.
    splay: [f32; 2],
    /// Finger curl per hand (0 straight, 1 fist), index to little.
    fingers: [[f32; 4]; 2],
}

fn shoulder_rest(side: usize) -> Vec3 {
    v(sx(side) * SHOULDER.x, HIP_Y + SPINE_UP + SHOULDER.y, 0.0)
}

fn hanging(side: usize) -> Reach {
    let x = sx(side);
    Reach {
        target: shoulder_rest(side) + v(x * 0.05, -0.62, -0.02),
        elbow: v(x * 0.3, -0.2, 1.0),
        hand: None,
    }
}

fn on_hip(side: usize) -> Reach {
    let x = sx(side);
    Reach {
        target: v(x * 0.25, HIP_Y + 0.08, -0.02),
        elbow: v(x, 0.0, 0.4),
        hand: Some(Quat::from_rotation_z(x * 2.2)),
    }
}

impl Pose {
    fn rest() -> Self {
        Pose {
            pelvis_off: Vec3::ZERO,
            pelvis: Quat::IDENTITY,
            spine: Quat::IDENTITY,
            neck: Quat::IDENTITY,
            arms: [hanging(0), hanging(1)],
            thigh: [0.0; 2],
            knee: [0.0; 2],
            splay: [0.0; 2],
            fingers: [[0.85; 4]; 2],
        }
    }

    fn blend(&self, o: &Pose, t: f32) -> Pose {
        let l = |a: f32, b: f32| a + (b - a) * t;
        let mut out = *self;
        out.pelvis_off = self.pelvis_off.lerp(o.pelvis_off, t);
        out.pelvis = self.pelvis.slerp(o.pelvis, t);
        out.spine = self.spine.slerp(o.spine, t);
        out.neck = self.neck.slerp(o.neck, t);
        for s in 0..2 {
            let (a, b) = (self.arms[s], o.arms[s]);
            out.arms[s] = Reach {
                target: a.target.lerp(b.target, t),
                elbow: a.elbow.lerp(b.elbow, t),
                hand: match (a.hand, b.hand) {
                    (Some(x), Some(y)) => Some(x.slerp(y, t)),
                    (_, h) if t > 0.5 => h,
                    (h, _) => h,
                },
            };
            out.thigh[s] = l(self.thigh[s], o.thigh[s]);
            out.knee[s] = l(self.knee[s], o.knee[s]);
            out.splay[s] = l(self.splay[s], o.splay[s]);
            for f in 0..4 {
                out.fingers[s][f] = l(self.fingers[s][f], o.fingers[s][f]);
            }
        }
        out
    }
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Walking, crouching and holding (or not holding) a gun.
fn base_pose(rig: &Rig, grip: Option<(Vec3, Vec3, bool)>) -> Pose {
    let mut p = Pose::rest();
    let amp = (rig.speed / 4.0).min(1.0);
    for side in 0..2 {
        let swing = (rig.phase + if side == 1 { 0.0 } else { PI }).sin();
        let lift = (rig.phase + if side == 1 { 0.0 } else { PI }).cos().max(0.0);
        p.thigh[side] = swing * 0.6 * amp;
        p.knee[side] = (0.1 + 0.9 * lift) * amp;
    }
    p.pelvis_off.y = -0.03 * amp * (rig.phase * 2.0).sin().abs();
    match rig.stance {
        1 => {
            p.pelvis_off.y -= 0.32;
            for s in 0..2 {
                p.thigh[s] = 1.15 + p.thigh[s] * 0.4;
                p.knee[s] = 1.55 + p.knee[s] * 0.3;
                p.splay[s] = 0.12;
            }
            p.spine = Quat::from_rotation_x(-0.2);
        }
        2 => {
            p.pelvis_off.y -= 0.6;
            p.thigh = [1.3, 0.6];
            p.knee = [0.3, 1.6];
            p.pelvis = Quat::from_rotation_x(0.35);
            p.spine = Quat::from_rotation_x(-0.2);
        }
        _ => {}
    }
    let pitch = rig.pitch.clamp(-1.2, 1.2);
    if let Some((right, left, dual)) = grip {
        // Turn the chest so the left shoulder comes forward to the gun.
        p.spine = p.spine * Quat::from_rotation_x(pitch * 0.35) * Quat::from_rotation_y(if dual { 0.0 } else { -0.45 });
        p.neck = Quat::from_rotation_y(if dual { 0.0 } else { 0.4 }) * Quat::from_rotation_x(pitch * 0.5);
        p.arms[1] = Reach {
            target: right,
            elbow: v(1.0, -1.0, 0.4),
            hand: None,
        };
        p.arms[0] = Reach {
            target: left,
            elbow: v(-1.0, -1.2, 0.0),
            hand: None,
        };
        p.fingers = [[0.9; 4], [0.95, 1.0, 1.0, 1.0]];
        if !dual {
            p.fingers[0] = [0.7, 0.8, 0.9, 0.9];
        }
    } else {
        p.neck = Quat::from_rotation_x(pitch * 0.6);
        let amp = (rig.speed / 4.0).min(1.0);
        for side in 0..2 {
            let swing = (rig.phase + if side == 1 { 0.0 } else { PI }).sin();
            let x = sx(side);
            p.arms[side].target += v(0.0, 0.03 * swing.abs() * amp, -swing * 0.25 * amp + 0.0 * x);
        }
    }
    p
}

fn emote_pose(emote: u8, t: f32, base: &Pose) -> Pose {
    let mut p = Pose::rest();
    p.thigh = [0.05; 2];
    p.knee = [0.08; 2];
    match emote {
        1 => {
            // Dance: bouncing, hip sway and disco arms.
            let w = 2.0 * PI * 2.0;
            let bounce = (w * t).sin().abs();
            let sway = (w * t / 2.0).sin();
            p.pelvis_off = v(0.05 * sway, -0.07 * bounce, 0.0);
            p.pelvis = Quat::from_rotation_y(0.35 * (w * t / 4.0).sin()) * Quat::from_rotation_z(0.12 * sway);
            p.spine = Quat::from_rotation_z(-0.18 * sway) * Quat::from_rotation_x(0.08 * bounce);
            p.neck = Quat::from_rotation_z(0.15 * sway) * Quat::from_rotation_x(0.15 * bounce - 0.05);
            let u = 0.5 - 0.5 * (PI * t * 2.0).cos();
            let up = v(0.5, 2.15, -0.15);
            let down = v(-0.15, 1.05, -0.32);
            p.arms[1] = Reach {
                target: up.lerp(down, u),
                elbow: v(1.0, -0.3, 0.5),
                hand: None,
            };
            p.arms[0] = Reach {
                target: v(-0.32, 1.22 + 0.12 * (w * t).sin(), -0.25),
                elbow: v(-1.0, -0.5, 0.4),
                hand: None,
            };
            p.fingers[1] = [0.0, 1.0, 1.0, 1.0];
            for s in 0..2 {
                let step = if s == 0 { sway.max(0.0) } else { (-sway).max(0.0) };
                p.thigh[s] = 0.3 * bounce + 0.35 * step;
                p.knee[s] = 0.55 * bounce + 0.6 * step;
                p.splay[s] = 0.08;
            }
        }
        2 => {
            // Wave with an open hand.
            p.arms[1] = Reach {
                target: v(0.52 + 0.08 * (9.0 * t).sin(), 1.95, -0.12),
                elbow: v(1.0, -1.0, 0.3),
                hand: Some(Quat::from_rotation_z(PI + 0.35 * (9.0 * t).sin())),
            };
            p.fingers[1] = [0.0; 4];
            p.spine = Quat::from_rotation_z(-0.06);
            p.neck = Quat::from_rotation_z(0.12) * Quat::from_rotation_x(-0.05);
        }
        3 => {
            // Flip off: arm thrust forward, middle finger up, a few pumps.
            let pump = if t > 0.4 { 0.05 * (12.0 * t).sin() } else { 0.0 };
            p.arms[1] = Reach {
                target: v(0.2, 1.62 + pump, -0.55),
                elbow: v(1.0, -1.0, 0.2),
                hand: Some(Quat::from_rotation_x(PI)),
            };
            p.fingers[1] = [1.0, 0.0, 1.0, 1.0];
            p.arms[0] = on_hip(0);
            p.spine = Quat::from_rotation_x(0.12) * Quat::from_rotation_y(-0.15);
            p.neck = Quat::from_rotation_x(-0.15) * Quat::from_rotation_z(0.1);
            p.splay = [0.1, 0.1];
        }
        4 => {
            // Point straight ahead.
            let sweep = 0.06 * (3.0 * t).sin();
            p.arms[1] = Reach {
                target: v(0.25 + sweep, 1.58, -0.62),
                elbow: v(1.0, -0.6, 0.3),
                hand: Some(Quat::from_rotation_z(PI) * Quat::from_rotation_x(FRAC_PI_2)),
            };
            p.fingers[1] = [0.0, 1.0, 1.0, 1.0];
            p.arms[0] = on_hip(0);
            p.spine = Quat::from_rotation_y(-0.12);
            p.neck = Quat::from_rotation_y(0.08);
        }
        5 => {
            // Backflip: crouch, launch, tuck and spin, land.
            let wind = smooth(t / 0.15) * (1.0 - smooth((t - 0.15) / 0.1));
            let spin = smooth((t - 0.15) / 0.85);
            let tuck = (spin * PI).sin();
            let land = smooth((t - 1.0) / 0.1) * (1.0 - smooth((t - 1.15) / 0.15));
            let crouch = wind.max(land);
            p.pelvis = Quat::from_rotation_x(spin * 2.0 * PI);
            p.pelvis_off.y = -0.3 * crouch + 0.1 * tuck;
            for s in 0..2 {
                p.thigh[s] = 1.1 * crouch + 1.7 * tuck;
                p.knee[s] = 1.5 * crouch + 2.1 * tuck;
                let x = sx(s);
                let back = v(x * 0.32, 1.2, 0.3);
                let up = v(x * 0.3, 2.2, -0.1);
                let knees = v(x * 0.16, 1.2, -0.35);
                let arm = if t < 0.15 {
                    back
                } else if t < 0.3 {
                    up
                } else if spin < 0.95 {
                    knees
                } else {
                    v(x * 0.45, 1.5, -0.2)
                };
                p.arms[s] = Reach {
                    target: arm,
                    elbow: v(x, -0.5, 0.3),
                    hand: None,
                };
            }
            p.spine = Quat::from_rotation_x(0.5 * tuck + 0.3 * crouch);
            p.neck = Quat::from_rotation_x(0.4 * tuck);
        }
        _ => return *base,
    }
    p
}

/// Two-bone IK: rotation of the upper arm and bend of the elbow that put
/// the palm at `target` (relative to the shoulder, in its parent's space).
fn solve_arm(target: Vec3, hint: Vec3) -> (Quat, f32) {
    let a = UPPER_ARM;
    let b = FOREARM + PALM;
    let d = target.length().clamp(0.08, a + b - 0.002);
    let dir = target.normalize_or(Vec3::NEG_Y);
    let cos_b = ((d * d - a * a - b * b) / (2.0 * a * b)).clamp(-1.0, 1.0);
    let bend = cos_b.acos();
    let reach = v(0.0, -a - b * bend.cos(), -b * bend.sin());
    let r1 = Quat::from_rotation_arc(reach.normalize(), dir);
    // Twist about the reach line so the elbow points towards the hint.
    let elbow = r1 * v(0.0, -a, 0.0);
    let e = elbow - dir * elbow.dot(dir);
    let h = hint - dir * hint.dot(dir);
    let twist = if e.length_squared() > 1e-6 && h.length_squared() > 1e-6 {
        dir.dot(e.cross(h)).atan2(e.dot(h))
    } else {
        0.0
    };
    (Quat::from_axis_angle(dir, twist) * r1, bend)
}

#[allow(clippy::type_complexity)]
fn animate(
    time: Res<Time>,
    guns: Option<Res<crate::gunmodels::GunAssets>>,
    mut rigs: Query<(Entity, &mut Rig, &GlobalTransform)>,
    mut joints: Query<(&Joint, &mut Transform), Without<GunMount>>,
    mut mounts: Query<(&GunMount, &mut Transform, &mut Visibility), Without<Joint>>,
) {
    let dt = time.delta_secs().max(1e-4);
    let mut poses: HashMap<Entity, Pose> = HashMap::new();
    for (entity, mut rig, gt) in &mut rigs {
        let pos = gt.translation();
        let moved = (pos - rig.last).with_y(0.0).length() / dt;
        rig.last = pos;
        let target = if moved > 30.0 { 0.0 } else { moved };
        rig.speed += (target - rig.speed) * (1.0 - (-10.0 * dt).exp());
        let speed = rig.speed;
        rig.phase += speed * dt * 1.7;
        if rig.emote != 0 {
            rig.emote_t += dt;
            rig.blend = (rig.blend + dt * 6.0).min(1.0);
        } else {
            rig.blend = (rig.blend - dt * 6.0).max(0.0);
        }

        // Hold the gun in front of the right shoulder, aimed with the pitch.
        let mut grip = None;
        if let Some(Ok((mount, mut tf, mut vis))) = rig.mount.map(|m| mounts.get_mut(m)) {
            let pitch = rig.pitch.clamp(-1.0, 1.0);
            let crouch = match rig.stance {
                1 => -0.32,
                2 => -0.6,
                _ => 0.0,
            };
            let pivot = v(0.17, 1.5 + crouch, 0.0);
            let rot = Quat::from_rotation_x(pitch);
            tf.translation = pivot + rot * v(0.0, -0.1, -0.2);
            tf.rotation = rot;
            tf.scale = Vec3::splat(GUN_SCALE);
            let show = rig.armed && mount.want.is_some() && rig.blend < 0.5;
            let want = if show { Visibility::Inherited } else { Visibility::Hidden };
            if *vis != want {
                *vis = want;
            }
            if show {
                let gun = guns.as_ref().zip(mount.want).map(|(g, id)| g.gun(id).rig);
                let (support, dual) = gun.map_or((v(0.0, -0.04, -0.25), false), |r| (r.support, r.dual));
                let right = tf.translation + rot * v(0.0, -0.04, 0.02);
                let left = if dual {
                    tf.translation + v(-0.3, -0.04, 0.0)
                } else {
                    tf.translation + rot * (support * GUN_SCALE + v(0.0, -0.02, 0.0))
                };
                grip = Some((right, left, dual));
            }
        }
        let base = base_pose(&rig, grip);
        let pose = if rig.blend > 0.0 {
            let e = if rig.emote != 0 { rig.emote } else { 0 };
            if e == 0 {
                base
            } else {
                let ep = emote_pose(e, rig.emote_t, &base);
                base.blend(&ep, smooth(rig.blend))
            }
        } else {
            base
        };
        poses.insert(entity, pose);
    }
    for (joint, mut tf) in &mut joints {
        let Some(p) = poses.get(&joint.owner) else { continue };
        apply(joint.bone, p, &mut tf);
    }
}

/// Body-space transform of the spine (pelvis offset and rotation applied).
fn spine_frame(p: &Pose) -> (Vec3, Quat) {
    let pelvis_pos = Vec3::Y * HIP_Y + p.pelvis_off;
    let spine_rot = p.pelvis * p.spine;
    (pelvis_pos + p.pelvis * Vec3::Y * SPINE_UP, spine_rot)
}

/// Turns a body-space point into the frame the pelvis rotation moves.
fn body_to_root(p: &Pose, at: Vec3) -> Vec3 {
    Vec3::Y * HIP_Y + p.pelvis_off + p.pelvis * (at - Vec3::Y * HIP_Y)
}

fn apply(bone: Bone, p: &Pose, tf: &mut Transform) {
    match bone {
        Bone::Pelvis => {
            tf.translation = Vec3::Y * HIP_Y + p.pelvis_off;
            tf.rotation = p.pelvis;
        }
        Bone::Spine => tf.rotation = p.spine,
        Bone::Neck => tf.rotation = p.neck,
        Bone::UpperArm(s) | Bone::Forearm(s) | Bone::Hand(s) => {
            let (spine_pos, spine_rot) = spine_frame(p);
            let shoulder = spine_pos + spine_rot * (SHOULDER * v(sx(s), 1.0, 1.0));
            let reach = p.arms[s];
            let target = body_to_root(p, reach.target);
            let local = spine_rot.inverse() * (target - shoulder);
            let hint = spine_rot.inverse() * (p.pelvis * reach.elbow);
            let (upper, bend) = solve_arm(local, hint);
            tf.rotation = match bone {
                Bone::UpperArm(_) => upper,
                Bone::Forearm(_) => Quat::from_rotation_x(bend),
                _ => match reach.hand {
                    Some(h) => (spine_rot * upper * Quat::from_rotation_x(bend)).inverse() * (p.pelvis * h),
                    None => Quat::IDENTITY,
                },
            };
        }
        Bone::Finger(s, i) => {
            let curl = p.fingers[s][i];
            tf.rotation = Quat::from_rotation_x(curl * 2.4);
            tf.scale = Vec3::splat(1.0 - 0.25 * curl);
        }
        Bone::Thigh(s) => {
            tf.rotation = Quat::from_rotation_z(-sx(s) * p.splay[s]) * Quat::from_rotation_x(p.thigh[s]);
        }
        Bone::Shin(s) => tf.rotation = Quat::from_rotation_x(-p.knee[s]),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ik_reaches() {
        for t in [v(-0.08, -0.19, -0.41), v(0.1, -0.5, -0.1), v(0.2, 0.3, -0.3)] {
            let (r, b) = solve_arm(t, v(1.0, -1.0, 0.4));
            let end = r * (v(0.0, -UPPER_ARM, 0.0) + Quat::from_rotation_x(b) * v(0.0, -(FOREARM + PALM), 0.0));
            println!("{t:?} -> {end:?} bend {b}");
            assert!(end.distance(t) < 0.01);
        }
    }
}
