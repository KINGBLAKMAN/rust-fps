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

use crate::data::{CastStyle, Character};
use crate::humanoid::GunMount;
use crate::kit::{c, glow_material, vertex_material, Kit};
use crate::Phase;

mod heroes;
mod zombie_models;

/// Every model built on the skeleton: the player characters and the zombies.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Model {
    Hero(Character),
    /// Walker zombie in one of three outfits.
    Walker(u8),
    Spitter,
    Brute,
}

impl From<Character> for Model {
    fn from(c: Character) -> Self {
        Model::Hero(c)
    }
}

impl Model {
    pub const ZOMBIES: [Model; 5] = [
        Model::Walker(0),
        Model::Walker(1),
        Model::Walker(2),
        Model::Spitter,
        Model::Brute,
    ];

    fn gait(self) -> Gait {
        match self {
            Model::Hero(_) => Gait::Hero,
            Model::Walker(_) => Gait::Shamble,
            Model::Spitter => Gait::Hunch,
            Model::Brute => Gait::Stomp,
        }
    }
}

/// How a model moves.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Gait {
    Hero,
    Shamble,
    Hunch,
    Stomp,
}

pub struct RigPlugin;

impl Plugin for RigPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, build_models)
            .add_systems(Update, animate.in_set(Phase::Present));
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
    EMOTES
        .get((emote as usize).wrapping_sub(1))
        .map_or(0.0, |e| e.1)
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
    models: HashMap<Model, Vec<(Bone, Handle<Mesh>, bool)>>,
    finger: HashMap<Model, Handle<Mesh>>,
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
        self.solid.entry(b).or_insert_with(Kit::fine)
    }
    fn g(&mut self, b: Bone) -> &mut Kit {
        self.glow.entry(b).or_insert_with(Kit::fine)
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
    p.k(Bone::Pelvis).blob(
        v(0.0, -0.02, 0.0),
        v(0.18, 0.12, 0.13) * bulk,
        pal.suit_dark,
    );
    // Torso: abdomen and chest.
    let s = Bone::Spine;
    p.k(s)
        .blob(v(0.0, 0.1, 0.0), v(0.16, 0.16, 0.12) * bulk, pal.suit);
    p.k(s)
        .blob(v(0.0, 0.33, 0.0), v(0.23, 0.2, 0.14) * bulk, pal.suit);
    p.k(s)
        .cyl(v(0.0, 0.52, 0.0), 0.06, 0.1, Quat::IDENTITY, pal.skin);
    for side in 0..2 {
        let x = sx(side);
        // Upper arm with a round shoulder.
        let u = Bone::UpperArm(side);
        p.k(u).sphere(v(0.0, -0.02, 0.0), 0.08 * bulk, pal.suit);
        p.k(u).capsule_between(
            v(0.0, -0.04, 0.0),
            v(0.0, -0.27, 0.0),
            0.058 * bulk,
            pal.suit,
        );
        // Forearm with a cuff.
        let f = Bone::Forearm(side);
        p.k(f).sphere(v(0.0, 0.0, 0.0), 0.052 * bulk, pal.suit);
        p.k(f).capsule_between(
            v(0.0, -0.02, 0.0),
            v(0.0, -0.22, 0.0),
            0.05 * bulk,
            pal.suit,
        );
        p.k(f)
            .cyl(v(0.0, -0.235, 0.0), 0.047, 0.05, Quat::IDENTITY, pal.glove);
        // Hand: palm, back and thumb. Fingers are separate joints.
        let h = Bone::Hand(side);
        p.k(h)
            .cuboid(v(0.0, -0.05, 0.0), v(0.085, 0.1, 0.035), pal.glove);
        p.k(h).capsule_between(
            v(-x * 0.045, -0.03, -0.015),
            v(-x * 0.05, -0.08, -0.04),
            0.014,
            pal.glove,
        );
        // Thigh.
        let t = Bone::Thigh(side);
        p.k(t).capsule_between(
            v(0.0, -0.02, 0.0),
            v(0.0, -0.42, 0.0),
            0.078 * bulk,
            pal.suit_dark,
        );
        // Shin and boot.
        let n = Bone::Shin(side);
        p.k(n).sphere(v(0.0, 0.0, 0.0), 0.066 * bulk, pal.suit_dark);
        p.k(n).capsule_between(
            v(0.0, -0.02, 0.0),
            v(0.0, -0.38, 0.0),
            0.062 * bulk,
            pal.suit_dark,
        );
        p.k(n)
            .cyl(v(0.0, -0.4, 0.0), 0.068, 0.12, Quat::IDENTITY, pal.boot);
        p.k(n)
            .cuboid(v(0.0, -0.47, -0.05), v(0.12, 0.08, 0.25), pal.boot);
        p.k(n).cuboid(
            v(0.0, -0.505, -0.05),
            v(0.13, 0.02, 0.27),
            c(0.08, 0.08, 0.08),
        );
    }
}

fn finger_color(model: Model) -> Color {
    let Model::Hero(ch) = model else {
        return match model {
            Model::Walker(0) => c(0.45, 0.55, 0.4),
            Model::Walker(1) => c(0.62, 0.63, 0.55),
            Model::Walker(_) => c(0.52, 0.47, 0.42),
            Model::Spitter => c(0.55, 0.48, 0.58),
            _ => c(0.5, 0.4, 0.36),
        };
    };
    match ch {
        Character::Striker => c(0.09, 0.09, 0.1),
        Character::Warden => c(0.85, 0.86, 0.88),
        Character::Ronin => c(0.07, 0.06, 0.07),
        Character::Tinker => c(0.55, 0.42, 0.25),
        Character::Blaze => c(0.62, 0.42, 0.2),
        Character::Valkyrie => c(0.75, 0.78, 0.85),
    }
}

pub fn model_parts(model: Model) -> Vec<(Bone, Kit, bool)> {
    let mut p = Parts::default();
    match model {
        Model::Hero(Character::Striker) => heroes::striker(&mut p),
        Model::Hero(Character::Warden) => heroes::warden(&mut p),
        Model::Hero(Character::Ronin) => heroes::ronin(&mut p),
        Model::Hero(Character::Tinker) => heroes::tinker(&mut p),
        Model::Hero(Character::Blaze) => heroes::blaze(&mut p),
        Model::Hero(Character::Valkyrie) => heroes::valkyrie(&mut p),
        Model::Walker(v) => zombie_models::walker(&mut p, v),
        Model::Spitter => zombie_models::spitter(&mut p),
        Model::Brute => zombie_models::brute(&mut p),
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
    let all = Character::ALL
        .iter()
        .map(|c| Model::Hero(*c))
        .chain(Model::ZOMBIES);
    for model in all {
        let parts = model_parts(model)
            .into_iter()
            .filter_map(|(b, k, glow)| k.build().map(|m| (b, meshes.add(m), glow)))
            .collect();
        models.insert(model, parts);
        let mut k = Kit::fine();
        k.capsule_between(
            v(0.0, -0.005, 0.0),
            v(0.0, -0.06, 0.0),
            0.011,
            finger_color(model),
        );
        if !matches!(model, Model::Hero(_)) {
            // Cracked, dirty claws.
            k.cone(
                v(0.0, -0.075, 0.0),
                0.008,
                0.03,
                Quat::from_rotation_x(PI),
                c(0.25, 0.22, 0.15),
            );
        }
        finger.insert(model, meshes.add(k.build_or_empty()));
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
pub struct Rig {
    gait: Gait,
    /// Zombies: crawling on the ground (legs shot out).
    pub crawl: bool,
    /// Zombies: seconds since the last swing (big = not attacking).
    pub swing: f32,
    /// Jolt from being hit, fading.
    pub flinch: f32,
    /// Dying: seconds since death and which way it falls.
    pub dying: Option<(f32, u8)>,
    /// Look pitch (the gun and chest follow it).
    pub pitch: f32,
    /// 0 standing, 1 crouching, 2 sliding.
    pub stance: u8,
    /// Emote being played (0 none) and for how long.
    pub emote: u8,
    pub emote_t: f32,
    /// Holding a gun (else the arms hang).
    pub armed: bool,
    /// Using an ability: how the body moves and seconds since.
    pub cast: Option<(CastStyle, f32)>,
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
    model: impl Into<Model>,
    gun: Option<(u8, u8)>,
) -> Option<Entity> {
    spawn_rig_with(commands, assets, root, model.into(), gun, None)
}

/// Like `spawn_rig`, with its own body material (so it can be tinted).
pub fn spawn_rig_with(
    commands: &mut Commands,
    assets: &RigAssets,
    root: Entity,
    model: Model,
    gun: Option<(u8, u8)>,
    body: Option<Handle<StandardMaterial>>,
) -> Option<Entity> {
    let parts = &assets.models[&model];
    let body = body.unwrap_or_else(|| assets.mat.clone());
    let mut joints: HashMap<Bone, Entity> = HashMap::new();
    let mut joint = |commands: &mut Commands, bone: Bone, parent: Entity, at: Vec3| {
        let e = commands
            .spawn((
                Joint { owner: root, bone },
                Transform::from_translation(at),
                Visibility::default(),
            ))
            .id();
        commands.entity(parent).add_child(e);
        for (b, mesh, glow) in parts {
            if *b == bone {
                let mat = if *glow {
                    assets.glow.clone()
                } else {
                    body.clone()
                };
                commands
                    .entity(e)
                    .with_child((Mesh3d(mesh.clone()), MeshMaterial3d(mat)));
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
        let u = joint(
            commands,
            Bone::UpperArm(side),
            spine,
            SHOULDER * v(x, 1.0, 1.0),
        );
        let f = joint(commands, Bone::Forearm(side), u, -Vec3::Y * UPPER_ARM);
        let h = joint(commands, Bone::Hand(side), f, -Vec3::Y * FOREARM);
        for i in 0..4 {
            let fx = x * (0.03 - i as f32 * 0.02);
            let e = joint(commands, Bone::Finger(side, i), h, v(fx, -KNUCKLES, -0.004));
            commands.entity(e).with_child((
                Mesh3d(assets.finger[&model].clone()),
                MeshMaterial3d(body.clone()),
            ));
        }
        let t = joint(
            commands,
            Bone::Thigh(side),
            pelvis,
            v(x * HIP_X, -0.02, 0.0),
        );
        joint(commands, Bone::Shin(side), t, -Vec3::Y * THIGH);
    }
    let mount = gun.map(|(g, skin)| {
        let m = commands
            .spawn((
                GunMount::new(g, skin),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        commands.entity(root).add_child(m);
        m
    });
    commands.entity(root).insert(Rig {
        gait: model.gait(),
        crawl: false,
        swing: 9.0,
        flinch: 0.0,
        dying: None,
        pitch: 0.0,
        stance: 0,
        emote: 0,
        emote_t: 0.0,
        armed: gun.is_some(),
        cast: None,
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
        let lift = (rig.phase + if side == 1 { 0.0 } else { PI })
            .cos()
            .max(0.0);
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
        p.spine = p.spine
            * Quat::from_rotation_x(pitch * 0.35)
            * Quat::from_rotation_y(if dual { 0.0 } else { -0.45 });
        p.neck = Quat::from_rotation_y(if dual { 0.0 } else { 0.4 })
            * Quat::from_rotation_x(pitch * 0.5);
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
            p.pelvis = Quat::from_rotation_y(0.35 * (w * t / 4.0).sin())
                * Quat::from_rotation_z(0.12 * sway);
            p.spine = Quat::from_rotation_z(-0.18 * sway) * Quat::from_rotation_x(0.08 * bounce);
            p.neck =
                Quat::from_rotation_z(0.15 * sway) * Quat::from_rotation_x(0.15 * bounce - 0.05);
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
                let step = if s == 0 {
                    sway.max(0.0)
                } else {
                    (-sway).max(0.0)
                };
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
            let pump = if t > 0.4 {
                0.05 * (12.0 * t).sin()
            } else {
                0.0
            };
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

/// Root-space point to body space (undoing the pelvis movement).
fn from_root(p: &Pose, at: Vec3) -> Vec3 {
    Vec3::Y * HIP_Y + p.pelvis.inverse() * (at - Vec3::Y * HIP_Y - p.pelvis_off)
}

/// Zombies walking: a limping shamble with arms out, a hunched lope, or a
/// heavy stomp. Attacks swipe, spit or smash.
fn zombie_pose(rig: &Rig, gait: Gait) -> Pose {
    let mut p = Pose::rest();
    let amp = (rig.speed / 3.0).min(1.0);
    let ph = rig.phase;
    for side in 0..2 {
        let off = if side == 1 { 0.0 } else { PI };
        let swing = (ph + off).sin();
        let lift = (ph + off).cos().max(0.0);
        match gait {
            Gait::Hunch => {
                p.thigh[side] = swing * 0.5 * amp + 0.3;
                p.knee[side] = 0.5 + 0.8 * lift * amp;
            }
            Gait::Stomp => {
                p.thigh[side] = swing * 0.45 * amp + 0.05;
                p.knee[side] = (0.2 + 0.9 * lift) * amp + 0.15;
                p.splay[side] = 0.14;
            }
            _ => {
                p.thigh[side] = swing * 0.45 * amp;
                // The left leg is stiff: a limp.
                let stiff = if side == 0 { 0.35 } else { 1.0 };
                p.knee[side] = ((0.15 + 0.75 * lift) * amp + 0.08) * stiff;
            }
        }
    }
    let bob = (ph * 2.0).sin().abs();
    let sway = ph.sin();
    match gait {
        Gait::Hunch => {
            p.pelvis_off.y = -0.14 - 0.03 * bob * amp;
            p.spine = Quat::from_rotation_x(-0.55) * Quat::from_rotation_z(0.06 * sway);
            p.neck = Quat::from_rotation_x(0.5);
        }
        Gait::Stomp => {
            p.pelvis_off.y = -0.06 * bob * amp - 0.04;
            p.pelvis = Quat::from_rotation_z(0.06 * sway * amp);
            p.spine = Quat::from_rotation_x(-0.2) * Quat::from_rotation_z(-0.08 * sway * amp);
            p.neck = Quat::from_rotation_x(0.12);
        }
        _ => {
            p.pelvis_off.y = -0.035 * bob * amp;
            p.pelvis = Quat::from_rotation_z(0.1 * sway * amp);
            p.spine = Quat::from_rotation_x(-0.25) * Quat::from_rotation_z(-0.12 * sway * amp);
            p.neck = Quat::from_rotation_z(0.3) * Quat::from_rotation_x(0.18);
        }
    }
    for side in 0..2 {
        let x = sx(side);
        let sh = shoulder_rest(side);
        let wave = (ph + side as f32 * 1.7).sin();
        p.arms[side] = match gait {
            Gait::Hunch => Reach {
                target: sh + v(x * 0.06, -0.5, -0.3 + 0.15 * wave * amp),
                elbow: v(x, -0.3, 1.0),
                hand: None,
            },
            Gait::Stomp => Reach {
                target: sh + v(x * 0.25, -0.5, -0.12 + 0.2 * wave * amp),
                elbow: v(x, -0.2, 0.6),
                hand: None,
            },
            _ => Reach {
                target: sh
                    + v(
                        x * 0.02 - x * 0.05,
                        -0.1 + 0.05 * wave + if side == 1 { 0.06 } else { 0.0 },
                        -0.55,
                    ),
                elbow: v(x, -1.0, 0.2),
                hand: Some(Quat::from_rotation_x(-FRAC_PI_2) * Quat::from_rotation_y(x * 0.3)),
            },
        };
    }
    p.fingers = match gait {
        Gait::Stomp => [[0.95; 4]; 2],
        _ => [[0.35, 0.45, 0.5, 0.55]; 2],
    };
    // Attacking.
    if rig.swing < 0.6 {
        let k = smooth(rig.swing / 0.35);
        let back = 1.0 - smooth((rig.swing - 0.35) / 0.25);
        match gait {
            Gait::Hunch => {
                // Rears back, then thrusts the head forward to spit.
                p.spine = Quat::from_rotation_x(-0.55 + 0.4 * (1.0 - k) * back - 0.4 * k * back);
                p.neck = Quat::from_rotation_x(0.5 + 0.5 * k * back);
            }
            Gait::Stomp => {
                // Two-handed overhead smash.
                for side in 0..2 {
                    let x = sx(side);
                    let up = v(x * 0.2, HIP_Y + 1.25, 0.05);
                    let down = v(x * 0.15, HIP_Y + 0.15, -0.65);
                    p.arms[side] = Reach {
                        target: up.lerp(down, k).lerp(p.arms[side].target, 1.0 - back),
                        elbow: v(x, 0.0, 0.5),
                        hand: None,
                    };
                }
                p.spine = Quat::from_rotation_x(0.3 * (1.0 - k) - 0.55 * k);
            }
            _ => {
                // Right-handed claw swipe across the body.
                let from = v(0.5, HIP_Y + 1.0, -0.15);
                let to = v(-0.25, HIP_Y + 0.3, -0.55);
                p.arms[1] = Reach {
                    target: from.lerp(to, k).lerp(p.arms[1].target, 1.0 - back),
                    elbow: v(1.0, 0.2, 0.4),
                    hand: None,
                };
                p.fingers[1] = [0.1; 4];
                p.spine = p.spine * Quat::from_rotation_y((-0.45 + 0.9 * k) * back);
            }
        }
    }
    p
}

/// Legs shot out: face down, dragging itself along with its arms.
fn crawl_pose(rig: &Rig, gait: Gait) -> Pose {
    let mut p = Pose::rest();
    p.pelvis = Quat::from_rotation_x(-1.35);
    p.pelvis_off.y = -(HIP_Y - 0.2);
    p.spine = Quat::from_rotation_x(0.22);
    p.neck = Quat::from_rotation_x(0.85);
    p.thigh = [0.05, -0.1];
    p.knee = [0.15 + 0.1 * (rig.phase).sin().abs(), 0.35];
    p.splay = [0.15, -0.05];
    let reach = if gait == Gait::Stomp { 1.2 } else { 1.0 };
    for side in 0..2 {
        let x = sx(side);
        let c = rig.phase * 0.8 + side as f32 * PI;
        let target = v(
            x * 0.3,
            0.03 + 0.14 * c.sin().max(0.0),
            (-0.8 + 0.28 * c.cos()) * reach,
        );
        p.arms[side] = Reach {
            target: from_root(&p, target),
            elbow: p.pelvis.inverse() * v(x, 1.0, 0.3),
            hand: None,
        };
    }
    p.fingers = [[0.5; 4]; 2];
    if rig.swing < 0.5 {
        // Lunges up at your ankles.
        let k = (rig.swing / 0.5 * PI).sin();
        p.spine = Quat::from_rotation_x(0.22 + 0.4 * k);
        p.arms[1].target = from_root(&p, v(0.15, 0.35 + 0.2 * k, -1.0));
    }
    p
}

/// Falling over dead: backwards, onto the knees and forwards, or crumpling
/// sideways. Crawlers just slump.
fn death_pose(base: &Pose, t: f32, kind: u8, crawl: bool) -> Pose {
    let mut d = Pose::rest();
    d.fingers = [[0.4; 4]; 2];
    let twitch = if (1.2..2.0).contains(&t) {
        (t * 40.0).sin() * 0.06 * (2.0 - t)
    } else {
        0.0
    };
    if crawl {
        let k = smooth(t / 0.4);
        d = *base;
        d.pelvis = Quat::from_rotation_x(-1.35 - 0.2 * k);
        d.pelvis_off.y = -(HIP_Y - 0.2) - 0.07 * k;
        d.spine = Quat::from_rotation_x(0.22 * (1.0 - k));
        d.neck = Quat::from_rotation_x(0.85 * (1.0 - k) - 0.1 + twitch);
        for side in 0..2 {
            let x = sx(side);
            d.arms[side].target = from_root(&d, v(x * 0.5, 0.05, -0.45));
            d.arms[side].elbow = d.pelvis.inverse() * v(x, 0.5, 0.0);
        }
        return d;
    }
    match kind % 3 {
        0 => {
            // Knocked flat on its back.
            let buckle = smooth(t / 0.3);
            let fall = ((t - 0.15) / 0.75).clamp(0.0, 1.0).powi(2);
            d.pelvis = Quat::from_rotation_x(1.45 * fall);
            d.pelvis_off = v(
                0.0,
                -(HIP_Y - 0.16) * fall - 0.1 * buckle * (1.0 - fall),
                0.25 * fall,
            );
            d.thigh = [0.5 * buckle * (1.0 - fall) + 0.1, 0.3 * (1.0 - fall) + 0.05];
            d.knee = [0.7 * buckle * (1.0 - fall) + 0.15, 0.4 * (1.0 - fall) + 0.1];
            d.spine = Quat::from_rotation_x(0.15 * fall);
            d.neck = Quat::from_rotation_x(0.3 * fall + twitch);
            for side in 0..2 {
                let x = sx(side);
                d.arms[side] = Reach {
                    target: v(
                        x * (0.35 + 0.25 * fall),
                        HIP_Y + 0.55 + 0.35 * fall,
                        -0.1 + 0.25 * fall,
                    ),
                    elbow: v(x, 0.0, 0.0),
                    hand: None,
                };
            }
        }
        1 => {
            // Drops to its knees, then falls on its face.
            let kneel = smooth(t / 0.35);
            let fwd = ((t - 0.35) / 0.6).clamp(0.0, 1.0).powi(2);
            d.thigh = [1.1 * kneel * (1.0 - fwd); 2];
            d.knee = [1.6 * kneel * (1.0 - fwd) + 0.1; 2];
            d.pelvis_off = v(
                0.0,
                -0.5 * kneel * (1.0 - fwd) - (HIP_Y - 0.16) * fwd,
                -0.35 * fwd,
            );
            d.pelvis = Quat::from_rotation_x(-1.5 * fwd);
            d.spine = Quat::from_rotation_x(-0.3 * kneel * (1.0 - fwd));
            d.neck = Quat::from_rotation_x(-0.4 * kneel + twitch);
            for side in 0..2 {
                let x = sx(side);
                d.arms[side] = Reach {
                    target: v(x * 0.4, HIP_Y + 0.1 + 0.6 * fwd, -0.2 - 0.25 * fwd),
                    elbow: v(x, 0.0, 0.5),
                    hand: None,
                };
            }
        }
        _ => {
            // Legs give out and it rolls onto its side.
            let buck = smooth(t / 0.4);
            let roll = ((t - 0.3) / 0.6).clamp(0.0, 1.0).powi(2);
            d.thigh = [1.3 * buck * (1.0 - roll * 0.6); 2];
            d.knee = [2.0 * buck * (1.0 - roll * 0.5); 2];
            d.pelvis_off = v(0.3 * roll, -0.45 * buck - (HIP_Y - 0.45 - 0.2) * roll, 0.0);
            d.pelvis = Quat::from_rotation_z(-1.45 * roll) * Quat::from_rotation_x(-0.3 * buck);
            d.spine = Quat::from_rotation_x(-0.5 * buck * (1.0 - roll))
                * Quat::from_rotation_z(0.15 * roll);
            d.neck = Quat::from_rotation_z(0.4 * roll) * Quat::from_rotation_x(-0.3 + twitch);
            for side in 0..2 {
                let x = sx(side);
                d.arms[side] = Reach {
                    target: v(x * 0.3, HIP_Y + 0.25, -0.15),
                    elbow: v(x, -0.5, 0.3),
                    hand: None,
                };
            }
        }
    }
    base.blend(&d, smooth(t / 0.12))
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
            // Sword and spear casts put the gun away for a moment.
            let stowed = rig
                .cast
                .is_some_and(|(s, t)| matches!(s, CastStyle::Sword | CastStyle::Spear) && t < 0.6);
            let show = rig.armed && mount.want.is_some() && rig.blend < 0.5 && !stowed;
            let want = if show {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *vis != want {
                *vis = want;
            }
            if show {
                let gun = guns.as_ref().zip(mount.want).map(|(g, id)| g.gun(id).rig);
                let (support, dual) =
                    gun.map_or((v(0.0, -0.04, -0.25), false), |r| (r.support, r.dual));
                let right = tf.translation + rot * v(0.0, -0.04, 0.02);
                let left = if dual {
                    tf.translation + v(-0.3, -0.04, 0.0)
                } else {
                    tf.translation + rot * (support * GUN_SCALE + v(0.0, -0.02, 0.0))
                };
                grip = Some((right, left, dual));
            }
        }
        rig.swing += dt;
        rig.flinch = (rig.flinch - dt * 5.0).max(0.0);
        let base = match rig.gait {
            Gait::Hero => base_pose(&rig, grip),
            gait if rig.crawl => crawl_pose(&rig, gait),
            gait => zombie_pose(&rig, gait),
        };
        let mut pose = if rig.blend > 0.0 {
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
        if let Some((style, t)) = rig
            .cast
            .filter(|_| rig.gait == Gait::Hero && rig.blend < 0.5)
        {
            pose = cast_pose(&pose, style, t);
        }
        if rig.flinch > 0.0 {
            let k = rig.flinch;
            pose.spine =
                pose.spine * Quat::from_rotation_x(0.35 * k) * Quat::from_rotation_y(0.2 * k);
            pose.neck = pose.neck * Quat::from_rotation_x(0.4 * k);
        }
        if let Some((t, kind)) = rig.dying {
            pose = death_pose(&pose, t, kind, rig.crawl);
        }
        poses.insert(entity, pose);
    }
    for (joint, mut tf) in &mut joints {
        let Some(p) = poses.get(&joint.owner) else {
            continue;
        };
        apply(joint.bone, p, &mut tf);
    }
}

/// Arms (and a lean) for using an ability, blended over the normal pose.
fn cast_pose(base: &Pose, style: CastStyle, t: f32) -> Pose {
    let w = smooth(t / 0.08) * (1.0 - smooth((t - 0.45) / 0.25));
    if w <= 0.0 || style == CastStyle::Move {
        return *base;
    }
    let mut p = *base;
    let reach = |target: Vec3, elbow: Vec3| Reach {
        target,
        elbow,
        hand: None,
    };
    // Keyframes: before, during and after the moment of release.
    let keys = |a: Vec3, b: Vec3, c: Vec3| {
        if t < 0.15 {
            a.lerp(b, smooth(t / 0.15))
        } else {
            b.lerp(c, smooth((t - 0.15) / 0.12))
        }
    };
    match style {
        CastStyle::Throw => {
            // Left arm: back behind the head, then over and out.
            let hand = keys(
                v(-0.32, 1.85, 0.25),
                v(-0.3, 1.9, 0.15),
                v(-0.12, 1.45, -0.62),
            );
            p.arms[0] = reach(hand, v(-1.0, -0.4, 0.6));
            p.spine = p.spine
                * Quat::from_rotation_y(if t < 0.15 { 0.35 } else { -0.3 })
                * Quat::from_rotation_x(if t < 0.15 { 0.1 } else { -0.15 });
            p.fingers[0] = if t < 0.17 { [0.9; 4] } else { [0.1; 4] };
        }
        CastStyle::Spear => {
            let hand = keys(v(0.35, 1.8, 0.35), v(0.33, 1.85, 0.3), v(0.12, 1.5, -0.65));
            p.arms[1] = reach(hand, v(1.0, -0.4, 0.6));
            p.arms[0] = reach(v(-0.3, 1.5, -0.5), v(-1.0, -0.5, 0.3));
            p.spine = p.spine * Quat::from_rotation_y(if t < 0.15 { -0.45 } else { 0.3 });
            p.fingers[1] = [1.0; 4];
            p.fingers[0] = [0.0; 4];
        }
        CastStyle::Sword => {
            // Draw from the left hip and cut across to the right.
            let hand = keys(
                v(-0.28, 1.05, -0.15),
                v(-0.35, 1.25, -0.4),
                v(0.6, 1.35, -0.35),
            );
            p.arms[1] = reach(hand, v(1.0, -0.6, 0.3));
            p.arms[0] = reach(v(-0.25, 1.05, -0.12), v(-1.0, -0.3, 0.5));
            p.spine = p.spine * Quat::from_rotation_y(if t < 0.15 { 0.5 } else { -0.45 });
            p.fingers[1] = [1.0; 4];
            p.pelvis_off.y -= 0.12;
            for s in 0..2 {
                p.knee[s] += 0.35;
                p.thigh[s] += 0.2;
            }
        }
        CastStyle::Push => {
            p.arms[0] = reach(v(-0.12, 1.45, -0.62), v(-1.0, -0.6, 0.2));
            p.arms[0].hand = Some(Quat::from_rotation_x(-FRAC_PI_2));
            p.fingers[0] = [0.0; 4];
        }
        CastStyle::Sky => {
            p.arms[0] = reach(v(-0.3, 2.25, -0.15), v(-1.0, 0.0, 0.3));
            p.fingers[0] = [0.05; 4];
            p.neck = p.neck * Quat::from_rotation_x(0.35);
        }
        CastStyle::Deploy => {
            p.arms[0] = reach(v(-0.15, 0.75, -0.5), v(-1.0, 0.0, 0.3));
            p.spine = p.spine * Quat::from_rotation_x(-0.45);
            p.pelvis_off.y -= 0.1;
            p.fingers[0] = [0.5; 4];
        }
        CastStyle::Ground => {
            // Both palms slammed down at the floor in a crouch.
            let drop = keys(v(0.0, 1.9, -0.2), v(0.0, 1.95, -0.25), v(0.0, 0.55, -0.45));
            p.arms[0] = reach(drop + v(-0.22, 0.0, 0.0), v(-1.0, 0.2, 0.3));
            p.arms[1] = reach(drop + v(0.22, 0.0, 0.0), v(1.0, 0.2, 0.3));
            p.arms[0].hand = Some(Quat::from_rotation_x(FRAC_PI_2));
            p.arms[1].hand = Some(Quat::from_rotation_x(FRAC_PI_2));
            p.fingers = [[0.0; 4]; 2];
            if t > 0.15 {
                p.spine = p.spine * Quat::from_rotation_x(-0.6);
                p.pelvis_off.y -= 0.3;
                for s in 0..2 {
                    p.knee[s] += 0.9;
                    p.thigh[s] += 0.6;
                }
            }
        }
        CastStyle::Move => {}
    }
    base.blend(&p, w)
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
                    Some(h) => {
                        (spine_rot * upper * Quat::from_rotation_x(bend)).inverse() * (p.pelvis * h)
                    }
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
            tf.rotation =
                Quat::from_rotation_z(-sx(s) * p.splay[s]) * Quat::from_rotation_x(p.thigh[s]);
        }
        Bone::Shin(s) => tf.rotation = Quat::from_rotation_x(-p.knee[s]),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ik_reaches() {
        for t in [
            v(-0.08, -0.19, -0.41),
            v(0.1, -0.5, -0.1),
            v(0.2, 0.3, -0.3),
        ] {
            let (r, b) = solve_arm(t, v(1.0, -1.0, 0.4));
            let end = r
                * (v(0.0, -UPPER_ARM, 0.0)
                    + Quat::from_rotation_x(b) * v(0.0, -(FOREARM + PALM), 0.0));
            println!("{t:?} -> {end:?} bend {b}");
            assert!(end.distance(t) < 0.01);
        }
    }
}
