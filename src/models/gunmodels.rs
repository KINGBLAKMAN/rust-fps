//! Real models for all 23 guns, built from the modelling kit.
//!
//! Every gun is modelled in its own space: the origin is where the shooting
//! hand grips (top of the pistol grip), forward is -Z, up is +Y, and sizes are
//! real-world metres. A model is split into:
//! - `body`: the painted parts, drawn in the equipped skin,
//! - `detail`: metal, rubber, wood and brass parts in their own colours,
//! - `glow`: lit parts (sight dots, wonder weapon coils),
//! - `glass`: see-through sight lenses,
//! - `mag`: the magazine, separate so it can drop out on reload,
//! - `pump`: a shotgun's sliding forend.
//! The `GunRig` says where the hands go and where the muzzle is.

use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::data::{Attach, GUNS, SKINS};
use crate::kit::{c, glass_material, glow_material, vertex_material, Kit};

pub struct GunModelPlugin;

impl Plugin for GunModelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, build_all);
    }
}

/// How the support hand holds the gun.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Support {
    /// Cupping the shooting hand (pistols).
    Cup,
    /// Palm under the handguard.
    Under,
    /// Holding a vertical foregrip.
    Vertical,
}

/// Where hands go and things come out, in gun space.
#[derive(Clone, Copy, Debug)]
pub struct GunRig {
    /// Grip tilt: the grip leans back by this many radians.
    pub grip_tilt: f32,
    pub support: Vec3,
    pub support_style: Support,
    pub muzzle: Vec3,
    /// Where the magazine's top sits, and which way it comes out.
    pub mag_pos: Vec3,
    pub mag_out: Vec3,
    /// Where the support hand pumps (shotguns).
    pub pump_travel: f32,
    /// A second gun is held in the left hand.
    pub dual: bool,
    /// Revolver / double barrel / tube: loaded a round at a time.
    pub single_load: bool,
    pub glow_color: Color,
    /// Overall length, used to place the gun on the screen.
    pub length: f32,
    /// Height of the iron sight line above the grip (worked out from the
    /// model).
    pub sight_y: f32,
    /// Top of the rail where an optic sits (worked out from the model).
    pub optic_at: Vec3,
    /// Centre height of a sight built into the model, if it has one, and
    /// how far along the gun it sits.
    pub builtin_sight: Option<f32>,
    pub builtin_z: f32,
    /// How much the iron sight line drops per metre towards the muzzle (the
    /// rear sight usually sits a little higher than the front post).
    pub sight_slope: f32,
}

impl Default for GunRig {
    fn default() -> Self {
        Self {
            grip_tilt: 0.22,
            support: Vec3::new(0.0, -0.02, -0.25),
            support_style: Support::Under,
            muzzle: Vec3::new(0.0, 0.04, -0.6),
            mag_pos: Vec3::new(0.0, 0.0, -0.1),
            mag_out: Vec3::NEG_Y,
            pump_travel: 0.0,
            dual: false,
            single_load: false,
            glow_color: Color::WHITE,
            length: 0.8,
            sight_y: 0.06,
            optic_at: Vec3::new(0.0, 0.06, -0.08),
            builtin_sight: None,
            builtin_z: 0.0,
            sight_slope: 0.0,
        }
    }
}

pub struct GunHandles {
    pub body: Handle<Mesh>,
    pub detail: Handle<Mesh>,
    pub glow: Option<Handle<Mesh>>,
    pub glass: Option<Handle<Mesh>>,
    pub mag: Option<Handle<Mesh>>,
    pub pump: Option<Handle<Mesh>>,
    pub rig: GunRig,
}

/// One attachment's meshes: painted metal parts and lit parts.
pub struct AttachMesh {
    pub detail: Handle<Mesh>,
    pub glow: Option<Handle<Mesh>>,
    pub glass: Option<Handle<Mesh>>,
}

/// Every attachment model, by kind.
pub struct AttachModels {
    /// Red dot, holo, 3x scope (index = optic id - 1).
    pub optics: Vec<AttachMesh>,
    /// Suppressor, compensator.
    pub muzzles: Vec<AttachMesh>,
    /// Foregrip, laser.
    pub unders: Vec<AttachMesh>,
}

/// Every gun's meshes, plus the shared materials.
#[derive(Resource)]
pub struct GunAssets {
    pub guns: Vec<GunHandles>,
    pub attach: AttachModels,
    pub detail_mat: Handle<StandardMaterial>,
    pub glow_mat: Handle<StandardMaterial>,
    pub glass_mat: Handle<StandardMaterial>,
    pub skins: Vec<Handle<StandardMaterial>>,
}

impl GunAssets {
    pub fn gun(&self, id: u8) -> &GunHandles {
        &self.guns[(id as usize).min(self.guns.len() - 1)]
    }

    pub fn skin(&self, id: u8) -> Handle<StandardMaterial> {
        self.skins[(id as usize).min(self.skins.len() - 1)].clone()
    }
}

/// Marks a gun's magazine part (moved during reloads).
#[derive(Component)]
pub struct GunMag;

/// Marks a gun's pump forend.
#[derive(Component)]
pub struct GunPump;

/// Marks any part of a spawned gun.
#[derive(Component)]
pub struct GunPart;

fn build_all(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let guns = (0..GUNS.len() as u8)
        .map(|id| {
            let mut m = build_gun(id);
            let body = m.body.clone().build();
            let detail = m.detail.clone().build();
            let meshes_for_sight: Vec<&Mesh> = body.iter().chain(detail.iter()).collect();
            find_sights(&mut m.rig, &meshes_for_sight);
            GunHandles {
                body: meshes.add({
                    let mut body = m.body.build_or_empty();
                    crate::skins::project_uvs(&mut body);
                    body
                }),
                detail: meshes.add(m.detail.build_or_empty()),
                glow: m.glow.build().map(|g| meshes.add(g)),
                glass: m.glass.build().map(|g| meshes.add(g)),
                mag: m.mag.build().map(|g| meshes.add(g)),
                pump: m.pump.build().map(|g| meshes.add(g)),
                rig: m.rig,
            }
        })
        .collect();
    let skins = (0..SKINS.len() as u8)
        .map(|s| materials.add(crate::skins::material(s, &mut images)))
        .collect();
    let mut add = |(d, g, l): (Kit, Kit, Kit)| AttachMesh {
        detail: meshes.add(d.build_or_empty()),
        glow: g.build().map(|g| meshes.add(g)),
        glass: l.build().map(|g| meshes.add(g)),
    };
    let attach = AttachModels {
        optics: vec![add(att_red_dot()), add(att_holo()), add(att_scope())],
        muzzles: vec![add(att_suppressor()), add(att_compensator())],
        unders: vec![add(att_foregrip()), add(att_laser())],
    };
    commands.insert_resource(GunAssets {
        guns,
        attach,
        detail_mat: materials.add(vertex_material(0.45, 0.35)),
        glow_mat: materials.add(glow_material(1.0)),
        glass_mat: materials.add(glass_material()),
        skins,
    });
}

/// Spawns a gun's parts under `parent`. `body` is the painted material.
pub fn spawn_gun(
    parent: &mut ChildSpawnerCommands,
    assets: &GunAssets,
    id: u8,
    attach: Attach,
    body: Handle<StandardMaterial>,
    no_shadow: bool,
) {
    let g = assets.gun(id);
    let rig = g.rig;
    let mut part = |mesh: &Handle<Mesh>, mat: Handle<StandardMaterial>, tf: Transform, tag: u8| {
        let mut e = parent.spawn((GunPart, Mesh3d(mesh.clone()), MeshMaterial3d(mat), tf));
        if no_shadow {
            e.insert(NotShadowCaster);
        }
        match tag {
            1 => {
                e.insert(GunMag);
            }
            2 => {
                e.insert(GunPump);
            }
            3 => {
                e.insert(NotShadowCaster);
            }
            _ => {}
        }
    };
    part(&g.body, body, Transform::default(), 0);
    part(
        &g.detail,
        assets.detail_mat.clone(),
        Transform::default(),
        0,
    );
    if let Some(glow) = &g.glow {
        part(glow, assets.glow_mat.clone(), Transform::default(), 0);
    }
    if let Some(glass) = &g.glass {
        part(glass, assets.glass_mat.clone(), Transform::default(), 3);
    }
    if let Some(mag) = &g.mag {
        let long = if attach.ext_mag() { 1.45 } else { 1.0 };
        part(
            mag,
            assets.detail_mat.clone(),
            Transform::from_translation(g.rig.mag_pos).with_scale(Vec3::new(1.0, long, 1.0)),
            1,
        );
    }
    if let Some(pump) = &g.pump {
        part(pump, assets.detail_mat.clone(), Transform::default(), 2);
    }
    for (am, tf) in attachment_parts(assets, &rig, attach) {
        part(&am.detail, assets.detail_mat.clone(), tf, 0);
        if let Some(glow) = &am.glow {
            part(glow, assets.glow_mat.clone(), tf, 0);
        }
        if let Some(glass) = &am.glass {
            part(glass, assets.glass_mat.clone(), tf, 3);
        }
    }
}

/// Where each fitted attachment goes on a gun.
fn attachment_parts<'a>(
    assets: &'a GunAssets,
    rig: &GunRig,
    attach: Attach,
) -> Vec<(&'a AttachMesh, Transform)> {
    let mut v = Vec::new();
    if attach.optic() > 0 {
        v.push((
            &assets.attach.optics[attach.optic() as usize - 1],
            Transform::from_translation(rig.optic_at),
        ));
    }
    if attach.muzzle() > 0 {
        v.push((
            &assets.attach.muzzles[attach.muzzle() as usize - 1],
            Transform::from_translation(rig.muzzle),
        ));
    }
    if attach.under() > 0 {
        v.push((
            &assets.attach.unders[attach.under() as usize - 1],
            Transform::from_translation(under_mount(rig)),
        ));
    }
    v
}

/// Where an underbarrel attachment hangs: under the handguard, a little in
/// front of where the support hand normally goes.
pub fn under_mount(rig: &GunRig) -> Vec3 {
    rig.support + Vec3::new(0.0, 0.012, -0.03)
}

/// The rig with attachments fitted: the muzzle moves out past a suppressor,
/// a foregrip changes how the support hand holds on, and an optic raises the
/// sight line. Returns the rig and the sight height to aim along.
pub fn fitted_rig(rig: &GunRig, attach: Attach) -> (GunRig, f32) {
    let mut r = *rig;
    match attach.muzzle() {
        1 => r.muzzle.z -= 0.15,
        2 => r.muzzle.z -= 0.06,
        _ => {}
    }
    if attach.under() == 1 && !r.dual {
        r.support = under_mount(rig) + Vec3::new(0.0, -0.06, 0.0);
        r.support_style = Support::Vertical;
    }
    if attach.optic() > 0 {
        r.sight_slope = 0.0;
    }
    let sight = match attach.optic() {
        1 => rig.optic_at.y + 0.031,
        2 => rig.optic_at.y + 0.036,
        3 => rig.optic_at.y + 0.042,
        _ => rig.sight_y,
    };
    (r, sight)
}

/// The iron sight line, found from the model itself: a thin slice down the
/// gun's centre line gives its top profile. The line rests on the highest
/// point of the front half (the front post) and tilts up just enough to clear
/// everything behind it (so it runs through the rear notch or aperture).
/// Returns the line's height at the grip and its slope.
fn iron_sight_line(meshes: &[&Mesh], front: f32, short: bool) -> Option<(f32, f32)> {
    use bevy::render::mesh::{Indices, VertexAttributeValues};
    // The eye sits behind the gun; anything behind that doesn't count.
    let eye = if short { 0.25 } else { 0.07 };
    let split = front * 0.5;
    let mut profile: Vec<Vec2> = Vec::new(); // (z, y) points on the slice
    for m in meshes {
        let Some(VertexAttributeValues::Float32x3(p)) = m.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            continue;
        };
        let tris: Vec<[usize; 3]> = match m.indices() {
            Some(Indices::U32(i)) => i
                .chunks(3)
                .map(|c| [c[0] as usize, c[1] as usize, c[2] as usize])
                .collect(),
            Some(Indices::U16(i)) => i
                .chunks(3)
                .map(|c| [c[0] as usize, c[1] as usize, c[2] as usize])
                .collect(),
            None => (0..p.len() / 3)
                .map(|t| [t * 3, t * 3 + 1, t * 3 + 2])
                .collect(),
        };
        for t in tris {
            let v = t.map(|i| Vec3::from_array(p[i]));
            for (a, b) in [(v[0], v[1]), (v[1], v[2]), (v[2], v[0])] {
                if a.x.abs() < 1e-4 {
                    profile.push(Vec2::new(a.z, a.y));
                }
                if (a.x < 0.0) != (b.x < 0.0) && (a.x - b.x).abs() > 1e-6 {
                    let k = a.x / (a.x - b.x);
                    let q = a.lerp(b, k);
                    profile.push(Vec2::new(q.z, q.y));
                }
            }
        }
    }
    // Front post: the highest point in the front half.
    let post = profile
        .iter()
        .filter(|q| q.x <= split && q.x >= front - 0.01)
        .max_by(|a, b| a.y.total_cmp(&b.y))?;
    // Tilt so every point between the post and the eye stays under the line.
    let slope = profile
        .iter()
        .filter(|q| q.x > post.x + 0.02 && q.x < eye)
        .map(|q| (q.y - post.y) / (q.x - post.x))
        .fold(0.0f32, f32::max);
    // A sliver above the profile so the post tip sits on the line.
    let y0 = post.y - slope * post.x + 0.0005;
    Some((y0, slope))
}

/// Works out the iron sight line and where an optic sits from the model.
fn find_sights(rig: &mut GunRig, meshes: &[&Mesh]) {
    let mut pts: Vec<Vec3> = Vec::new();
    for m in meshes {
        if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(p)) =
            m.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            pts.extend(p.iter().map(|a| Vec3::from_array(*a)));
        }
    }
    let top = |z0: f32, z1: f32, half_w: f32| {
        pts.iter()
            .filter(|p| p.x.abs() < half_w && p.z >= z0.min(z1) && p.z <= z0.max(z1))
            .map(|p| p.y)
            .fold(f32::MIN, f32::max)
    };
    let front = rig.muzzle.z;
    if let Some(b) = rig.builtin_sight {
        rig.sight_y = b;
    } else if let Some((y, slope)) = iron_sight_line(meshes, front, rig.length < 0.4) {
        rig.sight_y = y;
        rig.sight_slope = slope;
    }
    // Optics sit over the receiver: just ahead of the grip on long guns,
    // over the slide on pistols.
    let short = rig.length < 0.4;
    let oz = if short {
        -0.06
    } else {
        (front * 0.22).max(-0.16)
    };
    let rail = top(oz - 0.035, oz + 0.035, 0.02);
    rig.optic_at = Vec3::new(0.0, if rail > f32::MIN { rail } else { rig.sight_y }, oz);
}

// ---------------------------------------------------------------------------
// Attachment models (origin at the mounting point, gun forward is -Z)
// ---------------------------------------------------------------------------

const LENS: Color = c(0.55, 0.85, 0.95);

fn att_red_dot() -> (Kit, Kit, Kit) {
    let (mut k, mut g) = (Kit::new(), Kit::new());
    // Mount, then an open tube (two rings joined by thin rails) so you can
    // see through it, with the dot projected in the middle.
    boxr(
        &mut k,
        v(-0.012, 0.0, -0.025),
        v(0.012, 0.012, 0.02),
        GUNMETAL,
    );
    let cy = 0.031;
    let rx = Quat::from_rotation_x(FRAC_PI_2);
    k.torus(v(0.0, cy, 0.016), 0.0025, 0.018, rx, BLACK);
    k.torus(v(0.0, cy, -0.024), 0.003, 0.019, rx, BLACK);
    for (x, y) in [(0.0, 0.019), (0.019, 0.0), (-0.019, 0.0)] {
        boxr(
            &mut k,
            v(x - 0.002, cy + y - 0.002, -0.024),
            v(x + 0.002, cy + y + 0.002, 0.016),
            BLACK,
        );
    }
    boxr(
        &mut k,
        v(0.018, cy - 0.008, -0.01),
        v(0.026, cy + 0.006, 0.008),
        GUNMETAL,
    );
    g.sphere(v(0.0, cy, -0.02), 0.0016, c(1.0, 0.1, 0.1));
    let mut l = Kit::new();
    l.cyl_z(v(0.0, cy, -0.024), 0.017, 0.0015, LENS);
    (k, g, l)
}

fn att_holo() -> (Kit, Kit, Kit) {
    let (mut k, mut g) = (Kit::new(), Kit::new());
    // Body under a rectangular window hood.
    boxr(&mut k, v(-0.018, 0.0, -0.03), v(0.018, 0.016, 0.035), BLACK);
    boxr(
        &mut k,
        v(-0.022, 0.016, -0.03),
        v(-0.018, 0.056, -0.022),
        BLACK,
    );
    boxr(
        &mut k,
        v(0.018, 0.016, -0.03),
        v(0.022, 0.056, -0.022),
        BLACK,
    );
    boxr(
        &mut k,
        v(-0.022, 0.056, -0.03),
        v(0.022, 0.06, -0.022),
        BLACK,
    );
    boxr(&mut k, v(0.02, 0.004, 0.0), v(0.03, 0.014, 0.025), GUNMETAL);
    // Ring reticle with a centre dot, just behind the glass.
    g.torus(
        v(0.0, 0.036, -0.022),
        0.0008,
        0.008,
        Quat::from_rotation_x(FRAC_PI_2),
        c(1.0, 0.25, 0.15),
    );
    g.sphere(v(0.0, 0.036, -0.022), 0.0013, c(1.0, 0.25, 0.15));
    let mut l = Kit::new();
    l.cuboid(v(0.0, 0.036, -0.027), v(0.036, 0.04, 0.0015), LENS);
    (k, g, l)
}

fn att_scope() -> (Kit, Kit, Kit) {
    let (mut k, g) = (Kit::new(), Kit::new());
    // Two rings on the rail holding a tube with bells at both ends.
    for z in [-0.04, 0.035] {
        boxr(
            &mut k,
            v(-0.01, 0.0, z - 0.01),
            v(0.01, 0.02, z + 0.01),
            GUNMETAL,
        );
        k.torus(
            v(0.0, 0.042, z),
            0.004,
            0.017,
            Quat::from_rotation_x(FRAC_PI_2),
            GUNMETAL,
        );
    }
    k.cyl_z(v(0.0, 0.042, -0.005), 0.014, 0.13, BLACK);
    k.frustum(
        v(0.0, 0.042, -0.085),
        0.022,
        0.015,
        0.04,
        Quat::from_rotation_x(-FRAC_PI_2),
        BLACK,
    );
    k.frustum(
        v(0.0, 0.042, 0.075),
        0.018,
        0.014,
        0.03,
        Quat::from_rotation_x(FRAC_PI_2),
        BLACK,
    );
    k.cyl(v(0.0, 0.06, -0.005), 0.006, 0.012, Quat::IDENTITY, GUNMETAL);
    k.cyl(
        v(0.018, 0.042, -0.005),
        0.006,
        0.012,
        Quat::from_rotation_z(FRAC_PI_2),
        GUNMETAL,
    );
    let mut l = Kit::new();
    l.cyl_z(v(0.0, 0.042, -0.106), 0.02, 0.002, LENS);
    l.cyl_z(v(0.0, 0.042, 0.09), 0.015, 0.002, LENS);
    (k, g, l)
}

fn att_suppressor() -> (Kit, Kit, Kit) {
    let mut k = Kit::new();
    k.cyl_z(v(0.0, 0.0, -0.078), 0.019, 0.15, BLACK);
    k.cyl_z(v(0.0, 0.0, -0.004), 0.021, 0.012, GUNMETAL);
    k.torus(
        v(0.0, 0.0, -0.153),
        0.003,
        0.016,
        Quat::from_rotation_x(FRAC_PI_2),
        GUNMETAL,
    );
    (k, Kit::new(), Kit::new())
}

fn att_compensator() -> (Kit, Kit, Kit) {
    let mut k = Kit::new();
    boxr(
        &mut k,
        v(-0.014, -0.012, -0.06),
        v(0.014, 0.014, 0.0),
        GUNMETAL,
    );
    // Ports cut in the top and sides.
    for i in 0..3 {
        let z = -0.012 - i as f32 * 0.016;
        boxr(
            &mut k,
            v(-0.0145, 0.002, z - 0.004),
            v(0.0145, 0.009, z + 0.004),
            BLACK,
        );
        boxr(
            &mut k,
            v(-0.008, 0.0141, z - 0.004),
            v(0.008, 0.0145, z + 0.004),
            BLACK,
        );
    }
    (k, Kit::new(), Kit::new())
}

fn att_foregrip() -> (Kit, Kit, Kit) {
    let mut k = Kit::new();
    boxr(
        &mut k,
        v(-0.012, -0.008, -0.03),
        v(0.012, 0.0, 0.03),
        GUNMETAL,
    );
    k.cyl(
        v(0.0, -0.05, 0.0),
        0.014,
        0.085,
        Quat::from_rotation_x(0.12),
        POLY,
    );
    for i in 0..4 {
        k.torus(
            v(0.0, -0.025 - i as f32 * 0.016, 0.002 * i as f32),
            0.0025,
            0.0145,
            Quat::from_rotation_x(0.12),
            RUBBER,
        );
    }
    k.cyl(
        v(0.0, -0.094, 0.006),
        0.017,
        0.008,
        Quat::from_rotation_x(0.12),
        POLY,
    );
    (k, Kit::new(), Kit::new())
}

fn att_laser() -> (Kit, Kit, Kit) {
    let (mut k, mut g) = (Kit::new(), Kit::new());
    boxr(
        &mut k,
        v(-0.01, -0.006, -0.02),
        v(0.01, 0.0, 0.02),
        GUNMETAL,
    );
    boxr(
        &mut k,
        v(-0.014, -0.032, -0.05),
        v(0.014, -0.006, 0.02),
        POLY,
    );
    k.cyl_z(v(-0.005, -0.019, -0.052), 0.006, 0.006, BLACK);
    boxr(
        &mut k,
        v(0.008, -0.03, -0.02),
        v(0.015, -0.022, 0.0),
        c(0.6, 0.1, 0.1),
    );
    g.sphere(v(-0.005, -0.019, -0.056), 0.0035, c(1.0, 0.1, 0.1));
    // A short visible beam.
    g.cyl_z(v(-0.005, -0.019, -1.06), 0.0012, 2.0, c(1.0, 0.1, 0.1));
    (k, g, Kit::new())
}

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

/// Body parts are white so the skin colour shows through; greys shade them.
const W: Color = c(1.0, 1.0, 1.0);
const W2: Color = c(0.78, 0.78, 0.78);
const W3: Color = c(0.6, 0.6, 0.6);
const BLACK: Color = c(0.06, 0.06, 0.065);
const GUNMETAL: Color = c(0.17, 0.18, 0.2);
const STEEL: Color = c(0.42, 0.43, 0.45);
const RUBBER: Color = c(0.035, 0.035, 0.035);
const WOOD: Color = c(0.46, 0.25, 0.11);
const WOOD_D: Color = c(0.3, 0.16, 0.07);
const BRASS: Color = c(0.8, 0.62, 0.25);
const POLY: Color = c(0.12, 0.12, 0.11);
const TAN: Color = c(0.55, 0.47, 0.33);

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// Box from its two corners.
fn boxr(k: &mut Kit, a: Vec3, b: Vec3, col: Color) {
    let min = a.min(b);
    let max = a.max(b);
    k.cuboid((min + max) / 2.0, max - min, col);
}

struct Model {
    body: Kit,
    detail: Kit,
    glow: Kit,
    glass: Kit,
    mag: Kit,
    pump: Kit,
    rig: GunRig,
}

impl Model {
    fn new() -> Self {
        Self {
            body: Kit::new(),
            detail: Kit::new(),
            glow: Kit::new(),
            glass: Kit::new(),
            mag: Kit::new(),
            pump: Kit::new(),
            rig: GunRig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Parts
// ---------------------------------------------------------------------------

/// Pistol grip hanging down from the origin, leaning back by `tilt`.
fn grip(k: &mut Kit, tilt: f32, len: f32, w: f32, d: f32, col: Color, grooves: bool) {
    let mut g = Kit::new();
    g.cuboid(v(0.0, -len / 2.0, 0.0), v(w, len, d), col);
    // Rounded back strap and front.
    g.cyl(
        v(0.0, -len / 2.0, d / 2.0 - 0.002),
        w / 2.0,
        len,
        Quat::IDENTITY,
        col,
    );
    if grooves {
        for i in 0..3 {
            let y = -0.025 - i as f32 * 0.024;
            g.cyl(
                v(0.0, y, -d / 2.0),
                0.007,
                w * 0.98,
                Quat::from_rotation_z(FRAC_PI_2),
                col,
            );
        }
        // Stippled side panels.
        for s in [-1.0, 1.0] {
            g.cuboid(
                v(s * w / 2.0, -len * 0.5, 0.002),
                v(0.003, len * 0.7, d * 0.7),
                RUBBER,
            );
        }
    }
    // Base.
    g.cuboid(v(0.0, -len + 0.004, 0.003), v(w * 1.08, 0.01, d * 1.1), col);
    k.append(g, Transform::from_rotation(Quat::from_rotation_x(-tilt)));
}

/// Trigger guard and trigger in front of the grip.
fn trigger(k: &mut Kit, z: f32, y: f32) {
    k.torus(
        v(0.0, y, z),
        0.0035,
        0.022,
        Quat::from_rotation_z(FRAC_PI_2),
        BLACK,
    );
    k.cuboid_rot(
        v(0.0, y + 0.002, z + 0.004),
        v(0.006, 0.022, 0.006),
        Quat::from_rotation_x(0.35),
        STEEL,
    );
}

/// Picatinny rail from `z0` (back) to `z1` (front) at height `y`.
fn rail(k: &mut Kit, z0: f32, z1: f32, y: f32, w: f32, col: Color) {
    boxr(k, v(-w / 2.0, y, z1), v(w / 2.0, y + 0.006, z0), col);
    let mut z = z0 - 0.004;
    while z > z1 + 0.006 {
        boxr(
            k,
            v(-w / 2.0 - 0.002, y + 0.006, z - 0.005),
            v(w / 2.0 + 0.002, y + 0.011, z),
            col,
        );
        z -= 0.011;
    }
}

fn barrel(k: &mut Kit, z_start: f32, len: f32, y: f32, r: f32, col: Color) {
    k.cyl_z(v(0.0, y, z_start - len / 2.0), r, len, col);
}

/// Slotted flash hider.
fn flash_hider(k: &mut Kit, z: f32, y: f32, r: f32) {
    k.cyl_z(v(0.0, y, z - 0.025), r * 1.35, 0.05, BLACK);
    for i in 0..4 {
        let a = i as f32 * FRAC_PI_2 + 0.4;
        k.cuboid_rot(
            v(a.cos() * r * 1.36, y + a.sin() * r * 1.36, z - 0.03),
            v(0.004, 0.004, 0.03),
            Quat::IDENTITY,
            c(0.02, 0.02, 0.02),
        );
    }
}

/// Muzzle brake with side ports.
fn brake(k: &mut Kit, z: f32, y: f32, r: f32) {
    boxr(
        k,
        v(-r * 1.6, y - r * 1.1, z),
        v(r * 1.6, y + r * 1.1, z - 0.06),
        BLACK,
    );
    for i in 0..3 {
        let zz = z - 0.012 - i as f32 * 0.017;
        for s in [-1.0, 1.0] {
            boxr(
                k,
                v(s * r * 1.6, y - r * 0.7, zz),
                v(s * r * 1.75, y + r * 0.7, zz - 0.009),
                c(0.01, 0.01, 0.01),
            );
        }
    }
}

fn suppressor(k: &mut Kit, z: f32, y: f32, r: f32, len: f32) {
    k.cyl_z(v(0.0, y, z - len / 2.0), r, len, GUNMETAL);
    k.cyl_z(v(0.0, y, z - 0.01), r * 1.05, 0.02, BLACK);
    k.cyl_z(v(0.0, y, z - len + 0.006), r * 1.05, 0.012, BLACK);
}

fn front_post(k: &mut Kit, z: f32, y: f32, h: f32) {
    // A-frame front sight.
    boxr(
        k,
        v(-0.012, y, z + 0.01),
        v(0.012, y + 0.012, z - 0.01),
        BLACK,
    );
    k.beam(
        v(-0.01, y + 0.01, z),
        v(-0.004, y + h, z),
        Vec2::splat(0.005),
        BLACK,
    );
    k.beam(
        v(0.01, y + 0.01, z),
        v(0.004, y + h, z),
        Vec2::splat(0.005),
        BLACK,
    );
    k.cuboid(v(0.0, y + h - 0.006, z), v(0.003, 0.014, 0.003), BLACK);
}

fn rear_sight(k: &mut Kit, z: f32, y: f32) {
    boxr(
        k,
        v(-0.012, y, z + 0.012),
        v(0.012, y + 0.016, z - 0.004),
        BLACK,
    );
    boxr(
        k,
        v(-0.012, y + 0.016, z + 0.004),
        v(-0.004, y + 0.024, z - 0.004),
        BLACK,
    );
    boxr(
        k,
        v(0.004, y + 0.016, z + 0.004),
        v(0.012, y + 0.024, z - 0.004),
        BLACK,
    );
}

/// Tube red dot sight on a rail at height `y`.
fn red_dot(m: &mut Model, z: f32, y: f32, dot: Color) {
    let d = &mut m.detail;
    boxr(
        d,
        v(-0.014, y, z + 0.02),
        v(0.014, y + 0.014, z - 0.02),
        BLACK,
    );
    let cy = y + 0.032;
    d.tube_z(v(0.0, cy, z), 0.0165, 0.019, 0.05, BLACK);
    d.tube_z(v(0.0, cy, z - 0.026), 0.0165, 0.021, 0.006, GUNMETAL);
    d.cyl(
        v(0.014, cy, z + 0.004),
        0.007,
        0.012,
        Quat::from_rotation_z(FRAC_PI_2),
        GUNMETAL,
    );
    d.cyl(
        v(0.0, cy + 0.02, z + 0.004),
        0.007,
        0.01,
        Quat::IDENTITY,
        GUNMETAL,
    );
    m.rig.builtin_sight = Some(cy);
    m.rig.builtin_z = z;
    // Lens and dot.
    m.glass.cyl_z(v(0.0, cy, z - 0.024), 0.0166, 0.0015, LENS);
    m.glow.sphere(v(0.0, cy, z - 0.02), 0.0013, dot);
}

/// Open holographic sight.
fn holo(m: &mut Model, z: f32, y: f32) {
    let d = &mut m.detail;
    boxr(
        d,
        v(-0.02, y, z + 0.035),
        v(0.02, y + 0.018, z - 0.035),
        BLACK,
    );
    // Hood.
    boxr(
        d,
        v(-0.022, y + 0.018, z - 0.03),
        v(-0.018, y + 0.05, z - 0.01),
        BLACK,
    );
    boxr(
        d,
        v(0.018, y + 0.018, z - 0.03),
        v(0.022, y + 0.05, z - 0.01),
        BLACK,
    );
    boxr(
        d,
        v(-0.022, y + 0.05, z - 0.03),
        v(0.022, y + 0.054, z - 0.01),
        BLACK,
    );
    d.cyl(
        v(0.025, y + 0.01, z + 0.02),
        0.006,
        0.008,
        Quat::from_rotation_z(FRAC_PI_2),
        GUNMETAL,
    );
    m.rig.builtin_sight = Some(y + 0.034);
    m.rig.builtin_z = z;
    // Glass.
    m.glass
        .cuboid(v(0.0, y + 0.034, z - 0.022), v(0.034, 0.03, 0.0015), LENS);
    m.glow.torus(
        v(0.0, y + 0.034, z - 0.018),
        0.0008,
        0.0055,
        Quat::from_rotation_x(FRAC_PI_2),
        c(1.0, 0.2, 0.15),
    );
    m.glow
        .sphere(v(0.0, y + 0.034, z - 0.018), 0.001, c(1.0, 0.2, 0.15));
}

/// Telescopic sight.
fn scope(m: &mut Model, z: f32, y: f32, len: f32, r: f32, big: bool) {
    let cy = y + r + 0.022;
    m.rig.builtin_sight = Some(cy);
    m.rig.builtin_z = z;
    let d = &mut m.detail;
    d.cyl_z(v(0.0, cy, z), r, len, BLACK);
    let bell = if big { r * 1.75 } else { r * 1.4 };
    let front = z - len / 2.0;
    let back = z + len / 2.0;
    let rx = Quat::from_rotation_x(FRAC_PI_2);
    d.frustum(v(0.0, cy, front - 0.025), bell, r, 0.05, rx, BLACK);
    d.cyl_z(v(0.0, cy, front - 0.065), bell, 0.03, BLACK);
    d.frustum(v(0.0, cy, back + 0.02), r, r * 1.3, 0.04, rx, BLACK);
    d.cyl_z(v(0.0, cy, back + 0.055), r * 1.3, 0.03, RUBBER);
    // Turrets.
    d.cyl(
        v(0.0, cy + r + 0.006, z + 0.01),
        r * 0.55,
        0.018,
        Quat::IDENTITY,
        GUNMETAL,
    );
    d.cyl(
        v(r + 0.006, cy, z + 0.01),
        r * 0.55,
        0.018,
        Quat::from_rotation_z(FRAC_PI_2),
        GUNMETAL,
    );
    // Rings and mounts.
    for zz in [z - len * 0.3, z + len * 0.3] {
        d.torus(v(0.0, cy, zz), 0.004, r + 0.003, rx, GUNMETAL);
        boxr(
            d,
            v(-0.01, y, zz - 0.008),
            v(0.01, cy - r, zz + 0.008),
            GUNMETAL,
        );
    }
    m.glass
        .cyl_z(v(0.0, cy, front - 0.081), bell * 0.85, 0.002, LENS);
    m.glass
        .cyl_z(v(0.0, cy, back + 0.071), r * 1.1, 0.002, LENS);
}

/// Prism scope (boxy, ACOG-like).
fn prism(m: &mut Model, z: f32, y: f32) {
    m.rig.builtin_sight = Some(y + 0.034);
    m.rig.builtin_z = z;
    let d = &mut m.detail;
    boxr(
        d,
        v(-0.018, y, z + 0.04),
        v(0.018, y + 0.012, z - 0.04),
        BLACK,
    );
    boxr(
        d,
        v(-0.022, y + 0.012, z + 0.03),
        v(0.022, y + 0.05, z - 0.03),
        TAN,
    );
    let rx = Quat::from_rotation_x(FRAC_PI_2);
    d.frustum(v(0.0, y + 0.034, z - 0.05), 0.026, 0.02, 0.04, rx, TAN);
    d.cyl_z(v(0.0, y + 0.034, z + 0.045), 0.017, 0.03, TAN);
    boxr(
        d,
        v(-0.008, y + 0.05, z + 0.01),
        v(0.008, y + 0.062, z - 0.012),
        BLACK,
    );
    m.glass
        .cyl_z(v(0.0, y + 0.034, z - 0.071), 0.022, 0.002, c(0.9, 0.7, 0.4));
    m.glass
        .cyl_z(v(0.0, y + 0.034, z + 0.061), 0.013, 0.002, LENS);
}

/// Collapsible M4-style stock behind the receiver.
fn stock_tube(m: &mut Model, z: f32, y: f32) {
    let d = &mut m.detail;
    d.cyl_z(v(0.0, y, z + 0.09), 0.014, 0.18, BLACK);
    // Ribs on the tube.
    for i in 0..5 {
        d.cyl_z(
            v(0.0, y, z + 0.06 + i as f32 * 0.02),
            0.0155,
            0.006,
            GUNMETAL,
        );
    }
    let b = &mut m.body;
    boxr(
        b,
        v(-0.019, y - 0.035, z + 0.13),
        v(0.019, y + 0.022, z + 0.24),
        W2,
    );
    boxr(
        b,
        v(-0.017, y + 0.02, z + 0.12),
        v(0.017, y + 0.034, z + 0.22),
        W2,
    );
    boxr(
        b,
        v(-0.02, y - 0.08, z + 0.2),
        v(0.02, y - 0.03, z + 0.24),
        W2,
    );
    boxr(
        &mut m.detail,
        v(-0.021, y - 0.083, z + 0.24),
        v(0.021, y + 0.026, z + 0.255),
        RUBBER,
    );
}

/// Solid stock (wood or polymer) going back from `z`.
fn stock_solid(k: &mut Kit, z: f32, y: f32, len: f32, drop: f32, col: Color, pad: Color) {
    let back = z + len;
    let tilt = (drop / len).atan();
    let mut s = Kit::new();
    // Comb and belly taper toward the wrist.
    s.cuboid(v(0.0, 0.0, len / 2.0), v(0.034, 0.045, len), col);
    s.wedge(
        v(0.0, -0.045, len * 0.62),
        v(0.06, 0.05, 0.03),
        Quat::from_rotation_y(FRAC_PI_2) * Quat::from_rotation_z(PI),
        col,
    );
    s.cuboid_rot(
        v(0.0, -0.045, len * 0.7),
        v(0.032, 0.07, len * 0.6),
        Quat::from_rotation_x(-0.2),
        col,
    );
    s.cuboid(v(0.0, -0.035, len - 0.006), v(0.036, 0.12, 0.014), pad);
    k.append(
        s,
        Transform::from_translation(v(0.0, y, z)).with_rotation(Quat::from_rotation_x(-tilt)),
    );
    let _ = back;
}

/// Folding/skeleton stock out of struts.
fn stock_skeleton(k: &mut Kit, z: f32, y: f32, len: f32, col: Color) {
    let t = Vec2::splat(0.012);
    let back = z + len;
    k.beam(v(0.0, y + 0.015, z), v(0.0, y + 0.02, back), t, col);
    k.beam(v(0.0, y - 0.03, z), v(0.0, y - 0.09, back), t, col);
    k.beam(
        v(0.0, y + 0.02, back - 0.06),
        v(0.0, y - 0.08, back - 0.06),
        t,
        col,
    );
    boxr(
        k,
        v(-0.02, y - 0.1, back),
        v(0.02, y + 0.03, back + 0.015),
        RUBBER,
    );
}

/// Folded bipod under the barrel.
fn bipod(k: &mut Kit, z: f32, y: f32, len: f32) {
    boxr(
        k,
        v(-0.018, y - 0.012, z + 0.015),
        v(0.018, y, z - 0.015),
        BLACK,
    );
    for s in [-1.0, 1.0] {
        k.beam(
            v(s * 0.012, y - 0.01, z),
            v(s * 0.014, y - 0.02, z + len),
            Vec2::splat(0.008),
            GUNMETAL,
        );
        k.cuboid(
            v(s * 0.014, y - 0.022, z + len),
            v(0.012, 0.01, 0.02),
            RUBBER,
        );
    }
}

/// Straight or curved box magazine in magazine space (top at origin).
fn box_mag(k: &mut Kit, len: f32, w: f32, d: f32, curve: f32, col: Color, base: Color) {
    let segs = if curve.abs() > 0.0 { 6 } else { 1 };
    let seg = len / segs as f32;
    let mut p = Vec3::ZERO;
    let mut ang = 0.0f32;
    for _ in 0..segs {
        let rot = Quat::from_rotation_x(ang);
        let dir = rot * Vec3::NEG_Y;
        k.cuboid_rot(p + dir * seg / 2.0, v(w, seg + 0.003, d), rot, col);
        // Witness ribs.
        k.cuboid_rot(
            p + dir * seg / 2.0 + rot * v(0.0, 0.0, -d * 0.1),
            v(w + 0.003, seg * 0.5, d * 0.5),
            rot,
            col,
        );
        p += dir * seg;
        ang += curve / segs as f32;
    }
    let rot = Quat::from_rotation_x(ang);
    k.cuboid_rot(p, v(w * 1.25, 0.012, d * 1.15), rot, base);
    // Top round.
    k.cyl_z(v(0.0, 0.004, 0.0), w * 0.28, d * 0.8, BRASS);
}

fn drum(k: &mut Kit, r: f32, w: f32, col: Color) {
    let rz = Quat::from_rotation_z(FRAC_PI_2);
    k.cuboid(v(0.0, -0.02, 0.0), v(w * 0.5, 0.04, 0.035), col);
    k.cyl(v(0.0, -0.02 - r, 0.0), r, w, rz, col);
    k.cyl(v(0.0, -0.02 - r, 0.0), r * 0.35, w + 0.008, rz, STEEL);
    k.torus(v(w / 2.0, -0.02 - r, 0.0), 0.004, r * 0.9, rz, GUNMETAL);
    k.torus(v(-w / 2.0, -0.02 - r, 0.0), 0.004, r * 0.9, rz, GUNMETAL);
}

/// Ventilated handguard section (round with slots).
fn vented_shroud(k: &mut Kit, z0: f32, z1: f32, y: f32, r: f32, col: Color) {
    let len = z0 - z1;
    k.cyl_z(v(0.0, y, (z0 + z1) / 2.0), r, len, col);
    let n = (len / 0.03) as i32;
    for i in 0..n {
        let z = z0 - 0.02 - i as f32 * 0.03;
        for a in [0.0f32, PI] {
            let x = a.cos() * r;
            k.cuboid(v(x, y, z), v(0.004, r * 0.8, 0.014), c(0.01, 0.01, 0.01));
        }
    }
}

/// Quad-rail handguard.
fn quad_rail(m: &mut Model, z0: f32, z1: f32, y: f32, h: f32, w: f32) {
    boxr(
        &mut m.body,
        v(-w / 2.0, y - h / 2.0, z0),
        v(w / 2.0, y + h / 2.0, z1),
        W,
    );
    rail(&mut m.detail, z0, z1, y + h / 2.0, w * 0.55, BLACK);
    let mut side = Kit::new();
    rail(&mut side, z0 - 0.02, z1 + 0.04, 0.0, w * 0.5, BLACK);
    m.detail.append(
        side,
        Transform::from_translation(v(w / 2.0, y, 0.0))
            .with_rotation(Quat::from_rotation_z(-FRAC_PI_2)),
    );
    let mut side = Kit::new();
    rail(&mut side, z0 - 0.02, z1 + 0.04, 0.0, w * 0.5, BLACK);
    m.detail.append(
        side,
        Transform::from_translation(v(-w / 2.0, y, 0.0))
            .with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
    );
    let mut under = Kit::new();
    rail(&mut under, z0 - 0.02, z1 + 0.02, 0.0, w * 0.5, BLACK);
    m.detail.append(
        under,
        Transform::from_translation(v(0.0, y - h / 2.0, 0.0))
            .with_rotation(Quat::from_rotation_z(PI)),
    );
}

fn vertical_grip(k: &mut Kit, z: f32, y: f32) {
    boxr(
        k,
        v(-0.014, y - 0.008, z + 0.02),
        v(0.014, y, z - 0.02),
        BLACK,
    );
    k.cyl(
        v(0.0, y - 0.055, z),
        0.016,
        0.095,
        Quat::from_rotation_x(0.1),
        POLY,
    );
    for i in 0..4 {
        k.torus(
            v(0.0, y - 0.02 - i as f32 * 0.02, z),
            0.003,
            0.016,
            Quat::IDENTITY,
            RUBBER,
        );
    }
}

fn ejection_port(k: &mut Kit, z: f32, y: f32, x: f32) {
    boxr(
        k,
        v(x, y - 0.01, z + 0.02),
        v(x + 0.002, y + 0.01, z - 0.02),
        c(0.02, 0.02, 0.02),
    );
    k.cyl_z(v(x - 0.002, y, z), 0.005, 0.03, BRASS);
}

fn charging_handle(k: &mut Kit, z: f32, y: f32, x: f32) {
    k.cyl(
        v(x + 0.015, y, z),
        0.005,
        0.03,
        Quat::from_rotation_z(FRAC_PI_2),
        STEEL,
    );
    k.sphere(v(x + 0.03, y, z), 0.008, BLACK);
}

// ---------------------------------------------------------------------------
// Pistols
// ---------------------------------------------------------------------------

struct PistolSpec {
    len: f32,
    slide_h: f32,
    w: f32,
    ported: bool,
    light: bool,
    long_mag: f32,
    comp: bool,
}

fn pistol(m: &mut Model, s: PistolSpec) {
    let back = 0.04;
    let front = back - s.len;
    let sy = 0.012;
    // Slide with serrations.
    boxr(
        &mut m.body,
        v(-s.w / 2.0, sy, back),
        v(s.w / 2.0, sy + s.slide_h, front),
        W,
    );
    boxr(
        &mut m.body,
        v(-s.w * 0.35, sy + s.slide_h, back - 0.005),
        v(s.w * 0.35, sy + s.slide_h + 0.005, front + 0.01),
        W2,
    );
    for i in 0..6 {
        let z = back - 0.008 - i as f32 * 0.006;
        for x in [-s.w / 2.0, s.w / 2.0] {
            boxr(
                &mut m.detail,
                v(x - 0.001, sy + 0.006, z),
                v(x + 0.001, sy + s.slide_h - 0.004, z - 0.0025),
                BLACK,
            );
        }
    }
    if s.ported {
        for i in 0..3 {
            let z = front + 0.025 + i as f32 * 0.018;
            boxr(
                &mut m.detail,
                v(-0.006, sy + s.slide_h, z),
                v(0.006, sy + s.slide_h + 0.002, z - 0.01),
                BLACK,
            );
        }
    }
    ejection_port(&mut m.detail, back - 0.07, sy + s.slide_h * 0.6, s.w / 2.0);
    // Frame and dust cover.
    boxr(
        &mut m.detail,
        v(-s.w * 0.45, -0.008, back - 0.01),
        v(s.w * 0.45, sy, front + 0.01),
        POLY,
    );
    boxr(
        &mut m.detail,
        v(-s.w * 0.4, -0.022, back - 0.07),
        v(s.w * 0.4, -0.008, front + 0.012),
        POLY,
    );
    // Barrel tip and sights.
    m.detail.cyl_z(
        v(0.0, sy + s.slide_h * 0.55, front - 0.002),
        0.0065,
        0.008,
        STEEL,
    );
    m.detail.cyl_z(
        v(0.0, sy + s.slide_h * 0.55, front - 0.006),
        0.004,
        0.002,
        c(0.0, 0.0, 0.0),
    );
    rear_sight(&mut m.detail, back - 0.012, sy + s.slide_h);
    boxr(
        &mut m.detail,
        v(-0.002, sy + s.slide_h, front + 0.012),
        v(0.002, sy + s.slide_h + 0.008, front + 0.004),
        BLACK,
    );
    m.glow.sphere(
        v(0.0, sy + s.slide_h + 0.007, front + 0.008),
        0.0018,
        c(0.3, 1.0, 0.3),
    );
    if s.comp {
        brake(&mut m.detail, front, sy + s.slide_h * 0.55, 0.009);
    }
    // Hammer and grip.
    boxr(
        &mut m.detail,
        v(-0.004, sy, back + 0.006),
        v(0.004, sy + 0.016, back),
        BLACK,
    );
    grip(&mut m.detail, 0.28, 0.11, s.w * 0.95, 0.045, POLY, true);
    trigger(&mut m.detail, -0.035, -0.012);
    if s.light {
        boxr(
            &mut m.detail,
            v(-0.014, -0.045, front + 0.06),
            v(0.014, -0.02, front + 0.005),
            BLACK,
        );
        m.glow.cyl_z(
            v(0.0, -0.032, front + 0.004),
            0.009,
            0.002,
            c(1.0, 1.0, 0.85),
        );
    }
    // Magazine in the grip, base plate showing (longer mags stick out).
    let rot = Quat::from_rotation_x(-0.28);
    let mlen = 0.11 + s.long_mag;
    m.mag.cuboid_rot(
        rot * v(0.0, -mlen / 2.0, 0.0),
        v(s.w * 0.75, mlen, 0.035),
        rot,
        GUNMETAL,
    );
    m.mag.cuboid_rot(
        rot * v(0.0, -mlen - 0.004, 0.0),
        v(s.w * 0.95, 0.012, 0.046),
        rot,
        BLACK,
    );
    m.rig.mag_pos = v(0.0, -0.002, 0.0);
    m.rig.grip_tilt = 0.28;
    m.rig.support = v(0.0, -0.06, -0.03);
    m.rig.support_style = Support::Cup;
    m.rig.muzzle = v(0.0, sy + s.slide_h * 0.55, front - 0.02);
    m.rig.length = s.len + 0.05;
}

fn revolver(m: &mut Model) {
    let y = 0.03;
    // Frame.
    boxr(
        &mut m.body,
        v(-0.014, -0.004, 0.04),
        v(0.014, 0.05, -0.065),
        W,
    );
    boxr(
        &mut m.body,
        v(-0.01, 0.045, 0.03),
        v(0.01, 0.056, -0.06),
        W2,
    );
    // Cylinder (the "mag", swings out to the left on reload).
    let rz = Quat::from_rotation_x(FRAC_PI_2);
    m.mag.cyl(Vec3::ZERO, 0.021, 0.042, rz, STEEL);
    for i in 0..6 {
        let a = i as f32 / 6.0 * std::f32::consts::TAU;
        m.mag.cuboid(
            v(a.cos() * 0.021, a.sin() * 0.021, 0.0),
            v(0.006, 0.006, 0.036),
            GUNMETAL,
        );
        m.mag.cyl_z(
            v(a.cos() * 0.012, a.sin() * 0.012, 0.0215),
            0.0045,
            0.002,
            BRASS,
        );
    }
    m.rig.mag_pos = v(0.0, y, -0.025);
    m.rig.mag_out = v(-1.0, -0.3, 0.0);
    // Long barrel with a vent rib and an ejector rod shroud.
    barrel(&mut m.detail, -0.065, 0.2, y + 0.006, 0.0095, STEEL);
    boxr(
        &mut m.body,
        v(-0.011, y - 0.016, -0.065),
        v(0.011, y - 0.002, -0.26),
        W,
    );
    boxr(
        &mut m.body,
        v(-0.005, y + 0.014, -0.065),
        v(0.005, y + 0.022, -0.265),
        W2,
    );
    for i in 0..7 {
        let z = -0.075 - i as f32 * 0.026;
        boxr(
            &mut m.detail,
            v(-0.006, y + 0.017, z),
            v(0.006, y + 0.023, z - 0.01),
            BLACK,
        );
    }
    boxr(
        &mut m.detail,
        v(-0.002, y + 0.022, -0.25),
        v(0.002, y + 0.036, -0.262),
        c(0.9, 0.3, 0.1),
    );
    // Hammer.
    m.detail.cuboid_rot(
        v(0.0, 0.055, 0.045),
        v(0.008, 0.03, 0.008),
        Quat::from_rotation_x(0.6),
        STEEL,
    );
    // Wooden grip.
    grip(&mut m.detail, 0.35, 0.11, 0.032, 0.048, WOOD, false);
    m.detail.cyl(
        v(0.016, -0.05, 0.02),
        0.004,
        0.003,
        Quat::from_rotation_z(FRAC_PI_2),
        BRASS,
    );
    trigger(&mut m.detail, -0.03, -0.012);
    m.rig.grip_tilt = 0.35;
    m.rig.support = v(0.0, -0.06, -0.03);
    m.rig.support_style = Support::Cup;
    m.rig.muzzle = v(0.0, y + 0.006, -0.27);
    m.rig.single_load = true;
    m.rig.length = 0.32;
}

// ---------------------------------------------------------------------------
// Long guns
// ---------------------------------------------------------------------------

/// Standard rifle receiver: upper and lower with mag well. Returns the
/// receiver's top height.
fn receiver(m: &mut Model, back: f32, front: f32, h: f32, w: f32) -> f32 {
    let top = 0.012 + h;
    // Lower.
    boxr(
        &mut m.body,
        v(-w * 0.48, -0.012, back - 0.01),
        v(w * 0.48, 0.012, front + 0.05),
        W2,
    );
    // Upper.
    boxr(
        &mut m.body,
        v(-w / 2.0, 0.012, back),
        v(w / 2.0, top, front),
        W,
    );
    // Forward assist / bolt detail.
    ejection_port(&mut m.detail, back - 0.07, 0.012 + h * 0.55, w / 2.0);
    trigger(&mut m.detail, -0.035, -0.012);
    top
}

fn mag_well(m: &mut Model, z: f32, w: f32, d: f32) {
    boxr(
        &mut m.body,
        v(-w / 2.0, -0.045, z + d / 2.0),
        v(w / 2.0, -0.006, z - d / 2.0),
        W2,
    );
    m.rig.mag_pos = v(0.0, -0.04, z);
}

fn hornet(m: &mut Model) {
    // Compact roller-delayed SMG: round receiver tube, slim handguard.
    let y = 0.035;
    m.body.cyl_z(v(0.0, y, -0.06), 0.022, 0.26, W);
    boxr(&mut m.body, v(-0.017, -0.012, 0.06), v(0.017, y, -0.04), W2);
    boxr(
        &mut m.detail,
        v(-0.02, y - 0.025, -0.19),
        v(0.02, y + 0.008, -0.31),
        POLY,
    );
    for i in 0..5 {
        let z = -0.2 - i as f32 * 0.022;
        boxr(
            &mut m.detail,
            v(-0.021, y - 0.02, z),
            v(0.021, y + 0.004, z - 0.008),
            RUBBER,
        );
    }
    barrel(&mut m.detail, -0.31, 0.05, y, 0.009, BLACK);
    m.detail.cyl_z(v(0.0, y, -0.365), 0.012, 0.012, BLACK);
    // Cocking tube and handle.
    m.detail
        .cyl_z(v(0.0, y + 0.03, -0.15), 0.008, 0.2, GUNMETAL);
    m.detail.cyl(
        v(-0.012, y + 0.03, -0.24),
        0.004,
        0.025,
        Quat::from_rotation_z(FRAC_PI_2),
        BLACK,
    );
    // Drum rear sight and hooded front sight.
    m.detail.cyl(
        v(0.0, y + 0.03, 0.05),
        0.012,
        0.02,
        Quat::from_rotation_z(FRAC_PI_2),
        BLACK,
    );
    m.detail.torus(
        v(0.0, y + 0.04, -0.27),
        0.003,
        0.012,
        Quat::from_rotation_x(FRAC_PI_2),
        BLACK,
    );
    m.detail.cyl(
        v(0.0, y + 0.035, -0.27),
        0.0025,
        0.02,
        Quat::IDENTITY,
        BLACK,
    );
    // Retractable stock rails.
    for s in [-1.0, 1.0] {
        m.detail.beam(
            v(s * 0.014, y, 0.07),
            v(s * 0.014, y - 0.015, 0.25),
            Vec2::splat(0.007),
            GUNMETAL,
        );
    }
    boxr(
        &mut m.detail,
        v(-0.022, y - 0.06, 0.24),
        v(0.022, y + 0.01, 0.26),
        RUBBER,
    );
    grip(&mut m.detail, 0.22, 0.1, 0.03, 0.045, POLY, true);
    trigger(&mut m.detail, -0.035, -0.012);
    box_mag(&mut m.mag, 0.15, 0.022, 0.035, 0.45, GUNMETAL, BLACK);
    mag_well(m, -0.075, 0.026, 0.045);
    m.rig.support = v(0.0, y - 0.03, -0.24);
    m.rig.muzzle = v(0.0, y, -0.38);
    m.rig.length = 0.45;
}

fn kestrel(m: &mut Model) {
    // Angular SMG with a slanted lower and a folding stock.
    let y = 0.03;
    boxr(
        &mut m.body,
        v(-0.022, 0.0, 0.07),
        v(0.022, y + 0.03, -0.2),
        W,
    );
    let mut lower = Kit::new();
    lower.cuboid(v(0.0, -0.03, 0.0), v(0.04, 0.07, 0.12), W2);
    m.body.append(
        lower,
        Transform::from_xyz(0.0, 0.0, -0.11).with_rotation(Quat::from_rotation_x(0.5)),
    );
    boxr(
        &mut m.body,
        v(-0.024, y + 0.03, 0.06),
        v(0.024, y + 0.034, -0.19),
        W3,
    );
    rail(&mut m.detail, 0.06, -0.19, y + 0.034, 0.02, BLACK);
    red_dot(m, -0.02, y + 0.045, c(1.0, 0.15, 0.1));
    barrel(&mut m.detail, -0.2, 0.08, y + 0.005, 0.009, BLACK);
    suppressor(&mut m.detail, -0.27, y + 0.005, 0.018, 0.1);
    // Side vents.
    for i in 0..4 {
        let z = -0.07 - i as f32 * 0.025;
        boxr(
            &mut m.detail,
            v(0.022, y, z),
            v(0.0235, y + 0.022, z - 0.012),
            BLACK,
        );
    }
    stock_skeleton(&mut m.detail, 0.07, y, 0.2, BLACK);
    grip(&mut m.detail, 0.2, 0.1, 0.03, 0.045, POLY, true);
    trigger(&mut m.detail, -0.035, -0.012);
    box_mag(&mut m.mag, 0.17, 0.024, 0.032, 0.0, GUNMETAL, BLACK);
    m.rig.mag_pos = v(0.0, -0.03, -0.15);
    vertical_grip(&mut m.detail, -0.15, -0.03);
    m.rig.support = v(0.0, -0.08, -0.15);
    m.rig.support_style = Support::Vertical;
    // Move the mag behind the foregrip so they don't overlap.
    m.rig.mag_pos = v(0.0, -0.01, -0.075);
    m.rig.muzzle = v(0.0, y + 0.005, -0.38);
    m.rig.length = 0.48;
}

fn wasp(m: &mut Model) {
    // Bullpup PDW with a top-mounted see-through magazine.
    let mut shell = Kit::new();
    shell.blob(v(0.0, 0.0, 0.0), v(0.034, 0.05, 0.2), W);
    shell.cuboid(v(0.0, 0.0, 0.03), v(0.06, 0.07, 0.3), W);
    m.body.append(shell, Transform::from_xyz(0.0, 0.03, 0.0));
    // Thumb hole grip area (dark recess on each side).
    for s in [-1.0, 1.0] {
        boxr(
            &mut m.detail,
            v(s * 0.0305, -0.02, 0.06),
            v(s * 0.0315, 0.02, -0.07),
            RUBBER,
        );
    }
    boxr(
        &mut m.body,
        v(-0.031, -0.08, -0.05),
        v(0.031, -0.005, -0.1),
        W2,
    );
    boxr(
        &mut m.body,
        v(-0.031, -0.09, 0.08),
        v(0.031, -0.0, 0.18),
        W2,
    );
    boxr(
        &mut m.detail,
        v(-0.032, -0.095, 0.17),
        v(0.032, 0.06, 0.185),
        RUBBER,
    );
    // Top mag (translucent look) on rails.
    m.mag
        .cuboid(v(0.0, 0.0, 0.0), v(0.05, 0.016, 0.24), c(0.35, 0.4, 0.45));
    for i in 0..10 {
        m.mag.cyl(
            v(0.0, 0.0, -0.1 + i as f32 * 0.022),
            0.006,
            0.048,
            Quat::from_rotation_z(FRAC_PI_2),
            BRASS,
        );
    }
    m.rig.mag_pos = v(0.0, 0.074, 0.01);
    m.rig.mag_out = v(0.0, 0.3, 1.0);
    holo(m, -0.05, 0.086);
    barrel(&mut m.detail, -0.17, 0.05, 0.03, 0.009, BLACK);
    flash_hider(&mut m.detail, -0.21, 0.03, 0.009);
    grip(&mut m.detail, 0.1, 0.09, 0.028, 0.04, POLY, true);
    trigger(&mut m.detail, -0.03, -0.012);
    m.rig.support = v(0.0, -0.045, -0.12);
    m.rig.muzzle = v(0.0, 0.03, -0.26);
    m.rig.length = 0.45;
}

fn mamba(m: &mut Model) {
    pistol(
        m,
        PistolSpec {
            len: 0.2,
            slide_h: 0.032,
            w: 0.028,
            ported: false,
            light: false,
            long_mag: 0.08,
            comp: true,
        },
    );
    // Folding brace and a small foregrip under the frame.
    vertical_grip(&mut m.detail, -0.12, -0.022);
    red_dot(m, -0.02, 0.046, c(0.2, 1.0, 0.3));
    m.rig.support = v(0.0, -0.07, -0.12);
    m.rig.support_style = Support::Vertical;
}

fn falcon(m: &mut Model) {
    // M4-pattern carbine.
    let top = receiver(m, 0.08, -0.18, 0.05, 0.034);
    rail(&mut m.detail, 0.07, -0.18, top, 0.022, BLACK);
    quad_rail(m, -0.18, -0.4, top - 0.025, 0.05, 0.05);
    barrel(&mut m.detail, -0.4, 0.13, top - 0.025, 0.0085, BLACK);
    front_post(&mut m.detail, -0.42, top - 0.012, 0.045);
    flash_hider(&mut m.detail, -0.53, top - 0.025, 0.0085);
    red_dot(m, -0.02, top + 0.011, c(1.0, 0.15, 0.1));
    rear_sight(&mut m.detail, 0.05, top + 0.011);
    stock_tube(m, 0.08, top - 0.03);
    charging_handle(&mut m.detail, 0.07, top - 0.01, 0.0);
    grip(&mut m.detail, 0.3, 0.1, 0.03, 0.045, POLY, true);
    mag_well(m, -0.1, 0.03, 0.06);
    box_mag(&mut m.mag, 0.19, 0.024, 0.06, 0.25, GUNMETAL, BLACK);
    m.rig.support = v(0.0, top - 0.06, -0.3);
    m.rig.muzzle = v(0.0, top - 0.025, -0.59);
    m.rig.length = 0.8;
}

fn ranger(m: &mut Model) {
    // AK-pattern rifle with wood furniture.
    let y = 0.0;
    boxr(
        &mut m.body,
        v(-0.018, y - 0.015, 0.08),
        v(0.018, y + 0.045, -0.18),
        W,
    );
    // Ribbed dust cover.
    boxr(
        &mut m.body,
        v(-0.017, y + 0.045, 0.08),
        v(0.017, y + 0.058, -0.14),
        W2,
    );
    for i in 0..3 {
        let z = 0.05 - i as f32 * 0.03;
        boxr(
            &mut m.body,
            v(-0.012, y + 0.058, z),
            v(0.012, y + 0.061, z - 0.01),
            W3,
        );
    }
    // Wood handguard and gas tube.
    boxr(
        &mut m.detail,
        v(-0.022, y - 0.02, -0.18),
        v(0.022, y + 0.03, -0.38),
        WOOD,
    );
    for i in 0..3 {
        let z = -0.21 - i as f32 * 0.05;
        boxr(
            &mut m.detail,
            v(-0.023, y - 0.005, z),
            v(0.023, y + 0.02, z - 0.012),
            WOOD_D,
        );
    }
    m.detail.cyl_z(v(0.0, y + 0.045, -0.28), 0.012, 0.22, WOOD);
    m.detail
        .cyl_z(v(0.0, y + 0.045, -0.4), 0.008, 0.03, GUNMETAL);
    barrel(&mut m.detail, -0.38, 0.2, y + 0.015, 0.009, GUNMETAL);
    boxr(
        &mut m.detail,
        v(-0.01, y + 0.015, -0.525),
        v(0.01, y + 0.055, -0.54),
        BLACK,
    );
    m.detail.cyl_z(v(0.0, y + 0.015, -0.6), 0.012, 0.035, BLACK);
    // Rear tangent sight.
    boxr(
        &mut m.detail,
        v(-0.012, y + 0.058, -0.14),
        v(0.012, y + 0.068, -0.18),
        BLACK,
    );
    // Selector lever.
    boxr(
        &mut m.detail,
        v(0.018, y + 0.01, 0.0),
        v(0.02, y + 0.03, -0.12),
        STEEL,
    );
    stock_solid(&mut m.detail, 0.08, y + 0.02, 0.28, 0.05, WOOD, BLACK);
    grip(&mut m.detail, 0.3, 0.1, 0.03, 0.045, WOOD_D, false);
    trigger(&mut m.detail, -0.035, -0.015);
    box_mag(
        &mut m.mag,
        0.2,
        0.026,
        0.06,
        0.65,
        c(0.32, 0.12, 0.06),
        BLACK,
    );
    m.rig.mag_pos = v(0.0, -0.015, -0.1);
    m.rig.support = v(0.0, y - 0.03, -0.29);
    m.rig.muzzle = v(0.0, y + 0.015, -0.62);
    m.rig.length = 0.85;
}

fn tempest(m: &mut Model) {
    // Angular battle rifle firing in bursts.
    let top = receiver(m, 0.08, -0.22, 0.055, 0.04);
    boxr(
        &mut m.body,
        v(-0.024, top - 0.06, -0.22),
        v(0.024, top, -0.42),
        W,
    );
    // Chamfered top edges.
    for s in [-1.0, 1.0] {
        m.body.cuboid_rot(
            v(s * 0.02, top - 0.004, -0.17),
            v(0.012, 0.012, 0.5),
            Quat::from_rotation_z(0.8),
            W2,
        );
    }
    rail(&mut m.detail, 0.07, -0.4, top, 0.022, BLACK);
    holo(m, -0.04, top + 0.011);
    barrel(&mut m.detail, -0.42, 0.1, top - 0.03, 0.009, BLACK);
    brake(&mut m.detail, -0.5, top - 0.03, 0.01);
    // Side charging handle and vents.
    m.detail.cyl(
        v(-0.03, top - 0.02, -0.15),
        0.005,
        0.02,
        Quat::from_rotation_z(FRAC_PI_2),
        BLACK,
    );
    for i in 0..5 {
        let z = -0.25 - i as f32 * 0.03;
        boxr(
            &mut m.detail,
            v(0.024, top - 0.045, z),
            v(0.0255, top - 0.015, z - 0.015),
            BLACK,
        );
    }
    stock_skeleton(&mut m.body, 0.08, top - 0.025, 0.24, W3);
    grip(&mut m.detail, 0.3, 0.1, 0.03, 0.045, POLY, true);
    mag_well(m, -0.1, 0.03, 0.06);
    box_mag(&mut m.mag, 0.17, 0.026, 0.062, 0.0, POLY, BLACK);
    vertical_grip(&mut m.detail, -0.32, top - 0.06);
    m.rig.support = v(0.0, top - 0.11, -0.32);
    m.rig.support_style = Support::Vertical;
    m.rig.muzzle = v(0.0, top - 0.03, -0.57);
    m.rig.length = 0.82;
}

fn bulldog(m: &mut Model) {
    // Bullpup carbine with a built-in short scope and rounded stock.
    let mut shell = Kit::new();
    shell.cuboid(v(0.0, 0.02, 0.06), v(0.05, 0.08, 0.36), W);
    shell.blob(v(0.0, 0.02, 0.22), v(0.026, 0.045, 0.06), W);
    shell.cuboid(v(0.0, -0.035, 0.12), v(0.048, 0.04, 0.2), W2);
    m.body.append(shell, Transform::IDENTITY);
    boxr(
        &mut m.detail,
        v(-0.026, -0.06, 0.24),
        v(0.026, 0.065, 0.255),
        RUBBER,
    );
    // Big trigger guard covering the hand.
    boxr(&mut m.body, v(-0.02, -0.11, -0.08), v(0.02, -0.1, 0.03), W2);
    boxr(
        &mut m.body,
        v(-0.02, -0.11, -0.09),
        v(0.02, -0.02, -0.08),
        W2,
    );
    scope(m, -0.02, 0.06, 0.16, 0.017, false);
    m.detail.cyl_z(v(0.0, 0.03, -0.2), 0.02, 0.1, BLACK);
    barrel(&mut m.detail, -0.25, 0.14, 0.03, 0.009, BLACK);
    flash_hider(&mut m.detail, -0.39, 0.03, 0.009);
    vertical_grip(&mut m.detail, -0.17, -0.02);
    grip(&mut m.detail, 0.1, 0.09, 0.028, 0.04, POLY, true);
    trigger(&mut m.detail, -0.03, -0.012);
    box_mag(&mut m.mag, 0.18, 0.026, 0.06, 0.3, c(0.3, 0.32, 0.3), BLACK);
    m.rig.mag_pos = v(0.0, -0.02, 0.1);
    m.rig.support = v(0.0, -0.07, -0.17);
    m.rig.support_style = Support::Vertical;
    m.rig.muzzle = v(0.0, 0.03, -0.44);
    m.rig.length = 0.7;
}

fn breacher(m: &mut Model) {
    // Pump-action shotgun.
    let y = 0.02;
    boxr(
        &mut m.body,
        v(-0.02, -0.015, 0.07),
        v(0.02, y + 0.03, -0.17),
        W,
    );
    ejection_port(&mut m.detail, -0.06, y + 0.012, 0.02);
    barrel(&mut m.detail, -0.17, 0.42, y + 0.03, 0.012, BLACK);
    m.detail
        .cyl_z(v(0.0, y - 0.005, -0.37), 0.011, 0.4, GUNMETAL);
    m.detail.cyl_z(v(0.0, y + 0.012, -0.58), 0.007, 0.02, BLACK);
    m.detail.sphere(v(0.0, y + 0.045, -0.585), 0.003, BRASS);
    // Pump forend (slides back after each shot).
    m.pump.cyl_z(v(0.0, y - 0.005, -0.31), 0.022, 0.15, POLY);
    for i in 0..6 {
        m.pump.torus(
            v(0.0, y - 0.005, -0.25 - i as f32 * 0.022),
            0.003,
            0.022,
            Quat::from_rotation_x(FRAC_PI_2),
            RUBBER,
        );
    }
    m.rig.pump_travel = 0.07;
    stock_solid(&mut m.body, 0.07, y + 0.01, 0.3, 0.05, W2, RUBBER);
    grip(&mut m.detail, 0.3, 0.1, 0.03, 0.045, POLY, true);
    trigger(&mut m.detail, -0.035, -0.012);
    // Side saddle shells.
    for i in 0..4 {
        m.detail.cyl(
            v(-0.024, y + 0.005, -0.04 - i as f32 * 0.02),
            0.008,
            0.045,
            Quat::IDENTITY,
            c(0.8, 0.12, 0.1),
        );
        m.detail.cyl(
            v(-0.024, y - 0.02, -0.04 - i as f32 * 0.02),
            0.0085,
            0.008,
            Quat::IDENTITY,
            BRASS,
        );
    }
    m.rig.support = v(0.0, y - 0.03, -0.3);
    m.rig.muzzle = v(0.0, y + 0.03, -0.6);
    m.rig.single_load = true;
    m.rig.mag_pos = v(0.0, y - 0.005, -0.18);
    m.rig.length = 0.9;
}

fn stormfront(m: &mut Model) {
    // Fully automatic drum-fed shotgun.
    let y = 0.02;
    boxr(
        &mut m.body,
        v(-0.03, -0.02, 0.08),
        v(0.03, y + 0.06, -0.38),
        W,
    );
    boxr(
        &mut m.body,
        v(-0.031, y + 0.02, 0.08),
        v(0.031, y + 0.025, -0.38),
        W3,
    );
    // Carry handle.
    m.detail.beam(
        v(0.0, y + 0.06, -0.02),
        v(0.0, y + 0.1, -0.05),
        Vec2::splat(0.014),
        BLACK,
    );
    m.detail.beam(
        v(0.0, y + 0.06, -0.2),
        v(0.0, y + 0.1, -0.17),
        Vec2::splat(0.014),
        BLACK,
    );
    boxr(
        &mut m.detail,
        v(-0.008, y + 0.093, -0.04),
        v(0.008, y + 0.107, -0.18),
        BLACK,
    );
    barrel(&mut m.detail, -0.38, 0.1, y + 0.025, 0.014, BLACK);
    brake(&mut m.detail, -0.47, y + 0.025, 0.014);
    for i in 0..6 {
        let z = -0.2 - i as f32 * 0.028;
        boxr(
            &mut m.detail,
            v(-0.0315, y, z),
            v(0.0315, y + 0.015, z - 0.012),
            BLACK,
        );
    }
    boxr(
        &mut m.detail,
        v(-0.032, y - 0.05, 0.08),
        v(0.032, y + 0.04, 0.32),
        POLY,
    );
    boxr(
        &mut m.detail,
        v(-0.033, y - 0.06, 0.31),
        v(0.033, y + 0.045, 0.33),
        RUBBER,
    );
    grip(&mut m.detail, 0.25, 0.1, 0.032, 0.05, POLY, true);
    trigger(&mut m.detail, -0.035, -0.015);
    drum(&mut m.mag, 0.07, 0.07, POLY);
    m.rig.mag_pos = v(0.0, -0.02, -0.12);
    vertical_grip(&mut m.detail, -0.3, -0.02);
    m.rig.support = v(0.0, -0.07, -0.3);
    m.rig.support_style = Support::Vertical;
    m.rig.muzzle = v(0.0, y + 0.025, -0.54);
    m.rig.length = 0.85;
}

fn double_barrel(m: &mut Model) {
    let y = 0.025;
    boxr(
        &mut m.detail,
        v(-0.022, -0.01, 0.05),
        v(0.022, y + 0.02, -0.06),
        STEEL,
    );
    for s in [-1.0, 1.0] {
        barrel(&mut m.body, -0.06, 0.5, y + 0.005, 0.013, W);
        m.body
            .cyl_z(v(s * 0.0135, y + 0.005, -0.31), 0.0125, 0.5, W);
        m.detail.cyl_z(
            v(s * 0.0135, y + 0.005, -0.561),
            0.009,
            0.002,
            c(0.0, 0.0, 0.0),
        );
    }
    boxr(
        &mut m.body,
        v(-0.004, y + 0.015, -0.06),
        v(0.004, y + 0.02, -0.56),
        W2,
    );
    m.detail.sphere(v(0.0, y + 0.024, -0.55), 0.003, BRASS);
    // Hinge and top lever.
    m.detail.cyl(
        v(0.0, -0.005, -0.065),
        0.01,
        0.046,
        Quat::from_rotation_z(FRAC_PI_2),
        STEEL,
    );
    m.detail.cuboid_rot(
        v(0.006, y + 0.024, 0.02),
        v(0.008, 0.006, 0.04),
        Quat::from_rotation_y(0.4),
        STEEL,
    );
    // Wooden forend and stock.
    boxr(
        &mut m.detail,
        v(-0.022, -0.015, -0.08),
        v(0.022, y - 0.002, -0.27),
        WOOD,
    );
    stock_solid(&mut m.detail, 0.05, y - 0.005, 0.33, 0.07, WOOD, WOOD_D);
    grip(&mut m.detail, 0.45, 0.06, 0.03, 0.04, WOOD, false);
    trigger(&mut m.detail, -0.03, -0.01);
    boxr(
        &mut m.detail,
        v(-0.002, -0.018, -0.025),
        v(0.002, -0.006, -0.03),
        STEEL,
    );
    // Shells as the "mag" when breaking open.
    for s in [-1.0, 1.0] {
        m.mag
            .cyl_z(v(s * 0.0135, 0.0, 0.0), 0.011, 0.06, c(0.8, 0.12, 0.1));
        m.mag.cyl_z(v(s * 0.0135, 0.0, 0.032), 0.012, 0.008, BRASS);
    }
    m.rig.mag_pos = v(0.0, y + 0.005, -0.09);
    m.rig.mag_out = v(0.0, 0.6, 1.0);
    m.rig.support = v(0.0, -0.025, -0.18);
    m.rig.muzzle = v(0.0, y + 0.005, -0.58);
    m.rig.single_load = true;
    m.rig.length = 0.9;
}

fn goliath(m: &mut Model) {
    // Belt-fed light machine gun.
    let top = receiver(m, 0.1, -0.2, 0.07, 0.05);
    // Feed cover with ribs and a carry handle.
    boxr(
        &mut m.body,
        v(-0.027, top, 0.05),
        v(0.027, top + 0.018, -0.15),
        W2,
    );
    for i in 0..4 {
        let z = 0.02 - i as f32 * 0.04;
        boxr(
            &mut m.body,
            v(-0.022, top + 0.018, z),
            v(0.022, top + 0.023, z - 0.012),
            W3,
        );
    }
    m.detail.beam(
        v(0.0, top, -0.25),
        v(0.0, top + 0.06, -0.22),
        Vec2::splat(0.012),
        BLACK,
    );
    m.detail.beam(
        v(0.0, top + 0.06, -0.22),
        v(0.0, top + 0.06, -0.12),
        Vec2::splat(0.012),
        BLACK,
    );
    // Heat shield over the barrel.
    vented_shroud(&mut m.detail, -0.2, -0.48, top - 0.03, 0.022, GUNMETAL);
    barrel(&mut m.detail, -0.48, 0.12, top - 0.03, 0.011, BLACK);
    flash_hider(&mut m.detail, -0.6, top - 0.03, 0.011);
    front_post(&mut m.detail, -0.55, top - 0.02, 0.04);
    bipod(&mut m.detail, -0.5, top - 0.055, 0.22);
    rail(&mut m.detail, 0.08, -0.06, top + 0.023, 0.022, BLACK);
    stock_solid(&mut m.body, 0.1, top - 0.035, 0.27, 0.03, W, RUBBER);
    grip(&mut m.detail, 0.3, 0.1, 0.03, 0.045, POLY, true);
    // Box mag with a belt feeding into the gun.
    m.mag.cuboid(
        v(0.0, -0.06, 0.0),
        v(0.075, 0.12, 0.11),
        c(0.28, 0.32, 0.22),
    );
    m.mag.cuboid(
        v(0.0, -0.002, 0.0),
        v(0.077, 0.008, 0.112),
        c(0.2, 0.23, 0.16),
    );
    for i in 0..6 {
        let p = v(0.025 + i as f32 * 0.002, 0.004 + i as f32 * 0.012, 0.0);
        m.mag.cyl_z(p, 0.006, 0.05, BRASS);
        m.mag
            .cuboid(p + v(0.0, 0.0, 0.0), v(0.012, 0.004, 0.02), BLACK);
    }
    m.rig.mag_pos = v(-0.04, -0.01, -0.08);
    m.rig.mag_out = v(-0.5, -1.0, 0.0);
    m.rig.support = v(0.0, top - 0.06, -0.3);
    m.rig.muzzle = v(0.0, top - 0.03, -0.66);
    m.rig.length = 0.95;
}

fn ripsaw(m: &mut Model) {
    // High rate LMG with a cooling shroud and drum.
    let top = receiver(m, 0.1, -0.18, 0.065, 0.05);
    boxr(
        &mut m.body,
        v(-0.03, top - 0.02, 0.08),
        v(0.03, top + 0.012, -0.18),
        W,
    );
    rail(&mut m.detail, 0.07, -0.16, top + 0.012, 0.022, BLACK);
    red_dot(m, -0.04, top + 0.023, c(1.0, 0.5, 0.1));
    let cy = top - 0.03;
    m.body.cyl_z(v(0.0, cy, -0.36), 0.032, 0.36, W2);
    for i in 0..8 {
        m.detail.torus(
            v(0.0, cy, -0.2 - i as f32 * 0.045),
            0.005,
            0.033,
            Quat::from_rotation_x(FRAC_PI_2),
            BLACK,
        );
    }
    for a in 0..6 {
        let ang = a as f32 / 6.0 * std::f32::consts::TAU;
        m.detail.cyl_z(
            v(ang.cos() * 0.022, cy + ang.sin() * 0.022, -0.6),
            0.006,
            0.1,
            GUNMETAL,
        );
    }
    m.detail.cyl_z(v(0.0, cy, -0.62), 0.03, 0.03, BLACK);
    stock_skeleton(&mut m.detail, 0.1, top - 0.03, 0.25, BLACK);
    grip(&mut m.detail, 0.3, 0.1, 0.03, 0.045, POLY, true);
    drum(&mut m.mag, 0.08, 0.09, GUNMETAL);
    m.rig.mag_pos = v(0.0, -0.01, -0.1);
    m.rig.support = v(0.0, cy - 0.035, -0.3);
    m.rig.muzzle = v(0.0, cy, -0.66);
    m.rig.length = 0.95;
}

fn longbow(m: &mut Model) {
    // Bolt-action sniper with a big scope.
    let y = 0.02;
    m.body.cyl_z(v(0.0, y + 0.02, -0.05), 0.02, 0.24, W);
    boxr(
        &mut m.body,
        v(-0.026, -0.03, 0.08),
        v(0.026, y + 0.01, -0.45),
        W2,
    );
    // Bolt.
    m.detail.cyl_z(v(0.0, y + 0.02, 0.08), 0.012, 0.03, STEEL);
    m.detail.cyl_between(
        v(0.0, y + 0.025, 0.07),
        v(0.05, y - 0.005, 0.07),
        0.004,
        STEEL,
    );
    m.detail.sphere(v(0.05, y - 0.005, 0.07), 0.011, BLACK);
    barrel(&mut m.detail, -0.17, 0.55, y + 0.02, 0.011, BLACK);
    // Fluting.
    for a in 0..4 {
        let ang = a as f32 * FRAC_PI_2;
        m.detail.cyl_z(
            v(ang.cos() * 0.011, y + 0.02 + ang.sin() * 0.011, -0.55),
            0.0025,
            0.25,
            GUNMETAL,
        );
    }
    brake(&mut m.detail, -0.72, y + 0.02, 0.013);
    scope(m, -0.04, y + 0.04, 0.3, 0.02, true);
    bipod(&mut m.detail, -0.42, -0.03, 0.25);
    // Thumbhole stock.
    boxr(
        &mut m.body,
        v(-0.026, -0.03, 0.08),
        v(0.026, y + 0.04, 0.38),
        W,
    );
    boxr(
        &mut m.body,
        v(-0.028, -0.11, 0.25),
        v(0.028, -0.03, 0.38),
        W,
    );
    boxr(
        &mut m.detail,
        v(-0.03, -0.115, 0.38),
        v(0.03, y + 0.045, 0.4),
        RUBBER,
    );
    boxr(
        &mut m.body,
        v(-0.02, y + 0.04, 0.2),
        v(0.02, y + 0.06, 0.34),
        W2,
    );
    grip(&mut m.detail, 0.2, 0.1, 0.032, 0.05, POLY, true);
    trigger(&mut m.detail, -0.03, -0.012);
    box_mag(&mut m.mag, 0.06, 0.03, 0.08, 0.0, BLACK, BLACK);
    m.rig.mag_pos = v(0.0, -0.025, -0.11);
    m.rig.support = v(0.0, -0.045, -0.32);
    m.rig.muzzle = v(0.0, y + 0.02, -0.78);
    m.rig.single_load = false;
    m.rig.length = 1.1;
}

fn arbiter(m: &mut Model) {
    // Semi-auto DMR with a mid scope and a suppressor.
    let top = receiver(m, 0.08, -0.2, 0.055, 0.036);
    rail(&mut m.detail, 0.07, -0.42, top, 0.022, BLACK);
    quad_rail(m, -0.2, -0.44, top - 0.026, 0.05, 0.052);
    barrel(&mut m.detail, -0.44, 0.06, top - 0.026, 0.009, BLACK);
    suppressor(&mut m.detail, -0.48, top - 0.026, 0.02, 0.17);
    scope(m, -0.06, top + 0.011, 0.22, 0.017, false);
    stock_tube(m, 0.08, top - 0.03);
    charging_handle(&mut m.detail, 0.07, top - 0.01, 0.0);
    grip(&mut m.detail, 0.3, 0.1, 0.03, 0.045, POLY, true);
    mag_well(m, -0.1, 0.032, 0.07);
    box_mag(&mut m.mag, 0.15, 0.028, 0.07, 0.12, GUNMETAL, BLACK);
    bipod(&mut m.detail, -0.4, top - 0.052, 0.18);
    m.rig.support = v(0.0, top - 0.06, -0.3);
    m.rig.muzzle = v(0.0, top - 0.026, -0.66);
    m.rig.length = 0.95;
}

fn sentinel(m: &mut Model) {
    // Marksman rifle: long slim handguard, prism scope, skeleton stock.
    let top = receiver(m, 0.08, -0.2, 0.052, 0.034);
    rail(&mut m.detail, 0.07, -0.2, top, 0.022, BLACK);
    let cy = top - 0.025;
    m.body.cyl_z(v(0.0, cy, -0.34), 0.026, 0.28, W);
    for i in 0..6 {
        let z = -0.23 - i as f32 * 0.04;
        for s in [-1.0, 1.0] {
            m.detail
                .cuboid(v(s * 0.024, cy, z), v(0.006, 0.012, 0.024), BLACK);
        }
    }
    barrel(&mut m.detail, -0.48, 0.16, cy, 0.009, BLACK);
    brake(&mut m.detail, -0.63, cy, 0.01);
    prism(m, -0.03, top + 0.011);
    stock_skeleton(&mut m.body, 0.08, top - 0.025, 0.26, W2);
    grip(&mut m.detail, 0.3, 0.1, 0.03, 0.045, POLY, true);
    mag_well(m, -0.1, 0.03, 0.065);
    box_mag(&mut m.mag, 0.14, 0.026, 0.065, 0.0, POLY, BLACK);
    m.rig.support = v(0.0, cy - 0.03, -0.33);
    m.rig.muzzle = v(0.0, cy, -0.7);
    m.rig.length = 1.0;
}

fn twin_fang(m: &mut Model) {
    // Compact open-bolt SMG (carried in pairs).
    let y = 0.03;
    boxr(
        &mut m.body,
        v(-0.022, -0.01, 0.05),
        v(0.022, y + 0.03, -0.2),
        W,
    );
    boxr(
        &mut m.body,
        v(-0.015, y + 0.03, 0.03),
        v(0.015, y + 0.036, -0.18),
        W2,
    );
    for i in 0..4 {
        let z = -0.05 - i as f32 * 0.03;
        boxr(
            &mut m.detail,
            v(-0.023, y, z),
            v(0.023, y + 0.012, z - 0.012),
            BLACK,
        );
    }
    m.detail
        .cyl_z(v(0.0, y + 0.01, -0.215), 0.016, 0.03, GUNMETAL);
    barrel(&mut m.detail, -0.23, 0.04, y + 0.01, 0.008, BLACK);
    rear_sight(&mut m.detail, 0.03, y + 0.036);
    front_post(&mut m.detail, -0.18, y + 0.036, 0.02);
    // Mag goes through the grip.
    grip(&mut m.detail, 0.1, 0.1, 0.034, 0.05, POLY, true);
    trigger(&mut m.detail, -0.04, -0.012);
    let rot = Quat::from_rotation_x(-0.1);
    m.mag.cuboid_rot(
        rot * v(0.0, -0.08, 0.0),
        v(0.022, 0.16, 0.035),
        rot,
        GUNMETAL,
    );
    m.mag
        .cuboid_rot(rot * v(0.0, -0.16, 0.0), v(0.026, 0.01, 0.04), rot, BLACK);
    m.rig.mag_pos = v(0.0, -0.01, 0.0);
    m.rig.grip_tilt = 0.1;
    m.rig.dual = true;
    m.rig.support_style = Support::Cup;
    m.rig.support = v(0.0, -0.06, -0.03);
    m.rig.muzzle = v(0.0, y + 0.01, -0.26);
    m.rig.length = 0.3;
}

fn ray_blaster(m: &mut Model) {
    // Retro ray gun: rounded body, glowing rings and fins.
    let green = c(0.3, 1.0, 0.4);
    m.body.blob(v(0.0, 0.035, 0.0), v(0.035, 0.04, 0.09), W);
    m.body.blob(v(0.0, 0.035, 0.07), v(0.03, 0.03, 0.03), W2);
    let rx = Quat::from_rotation_x(FRAC_PI_2);
    m.body
        .frustum(v(0.0, 0.035, -0.13), 0.016, 0.03, 0.1, rx, W2);
    m.detail.cyl_z(v(0.0, 0.035, -0.22), 0.012, 0.08, STEEL);
    for i in 0..3 {
        let z = -0.1 - i as f32 * 0.045;
        m.glow
            .torus(v(0.0, 0.035, z), 0.005, 0.03 - i as f32 * 0.004, rx, green);
    }
    m.glow.sphere(v(0.0, 0.035, -0.265), 0.014, green);
    m.glow.sphere(v(0.0, 0.08, 0.02), 0.015, green);
    m.detail
        .cyl(v(0.0, 0.065, 0.02), 0.008, 0.02, Quat::IDENTITY, STEEL);
    // Fins.
    for s in [-1.0, 1.0] {
        m.body.cuboid_rot(
            v(s * 0.04, 0.035, 0.05),
            v(0.03, 0.05, 0.004),
            Quat::from_rotation_y(s * 1.2),
            W3,
        );
    }
    m.body
        .cuboid_rot(v(0.0, 0.08, 0.05), v(0.004, 0.04, 0.05), Quat::IDENTITY, W3);
    grip(
        &mut m.detail,
        0.3,
        0.1,
        0.03,
        0.045,
        c(0.2, 0.05, 0.05),
        true,
    );
    trigger(&mut m.detail, -0.03, -0.006);
    // Energy cell.
    m.mag
        .cyl(v(0.0, -0.04, 0.0), 0.012, 0.08, Quat::IDENTITY, STEEL);
    m.mag
        .cyl(v(0.0, -0.04, 0.0), 0.013, 0.05, Quat::IDENTITY, green);
    m.rig.mag_pos = v(0.0, 0.0, 0.04);
    m.rig.mag_out = v(0.0, -1.0, 0.5);
    m.rig.support_style = Support::Cup;
    m.rig.support = v(0.0, -0.06, -0.03);
    m.rig.muzzle = v(0.0, 0.035, -0.28);
    m.rig.glow_color = green;
    m.rig.length = 0.35;
}

fn thunder_cannon(m: &mut Model) {
    // Tesla cannon: coils, capacitors and two electrode prongs.
    let blue = c(0.45, 0.6, 1.0);
    let y = 0.04;
    boxr(
        &mut m.body,
        v(-0.04, -0.01, 0.12),
        v(0.04, y + 0.05, -0.12),
        W,
    );
    m.body.cyl_z(v(0.0, y + 0.01, -0.2), 0.04, 0.18, W2);
    let rx = Quat::from_rotation_x(FRAC_PI_2);
    for i in 0..6 {
        let z = -0.13 - i as f32 * 0.025;
        m.detail
            .torus(v(0.0, y + 0.01, z), 0.006, 0.045, rx, c(0.75, 0.45, 0.2));
    }
    for s in [-1.0, 1.0] {
        m.detail
            .cyl_z(v(s * 0.05, y + 0.04, -0.02), 0.014, 0.16, STEEL);
        m.glow
            .cyl_z(v(s * 0.05, y + 0.04, -0.02), 0.0145, 0.1, blue);
        // Prongs.
        m.detail.beam(
            v(s * 0.025, y + 0.01, -0.29),
            v(s * 0.035, y + 0.03, -0.4),
            Vec2::splat(0.01),
            STEEL,
        );
        m.glow.sphere(v(s * 0.035, y + 0.03, -0.405), 0.012, blue);
    }
    m.glow.sphere(v(0.0, y + 0.01, -0.3), 0.022, blue);
    // Handle on top and stock.
    m.detail.beam(
        v(0.0, y + 0.05, 0.02),
        v(0.0, y + 0.1, 0.0),
        Vec2::splat(0.014),
        BLACK,
    );
    m.detail.beam(
        v(0.0, y + 0.05, -0.1),
        v(0.0, y + 0.1, -0.08),
        Vec2::splat(0.014),
        BLACK,
    );
    boxr(
        &mut m.detail,
        v(-0.008, y + 0.093, 0.0),
        v(0.008, y + 0.107, -0.09),
        BLACK,
    );
    stock_solid(&mut m.body, 0.12, y, 0.2, 0.02, W2, RUBBER);
    grip(&mut m.detail, 0.25, 0.1, 0.034, 0.05, POLY, true);
    trigger(&mut m.detail, -0.035, -0.012);
    // Battery pack.
    m.mag
        .cuboid(v(0.0, -0.04, 0.0), v(0.05, 0.08, 0.06), GUNMETAL);
    m.mag
        .cuboid(v(0.0, -0.04, -0.031), v(0.03, 0.05, 0.004), blue);
    m.rig.mag_pos = v(0.0, -0.01, -0.09);
    vertical_grip(&mut m.detail, -0.2, -0.04);
    m.rig.support = v(0.0, -0.09, -0.2);
    m.rig.support_style = Support::Vertical;
    m.rig.muzzle = v(0.0, y + 0.01, -0.42);
    m.rig.glow_color = blue;
    m.rig.length = 0.7;
}

fn build_gun(id: u8) -> Model {
    let mut m = Model::new();
    match id {
        0 => pistol(
            &mut m,
            PistolSpec {
                len: 0.19,
                slide_h: 0.032,
                w: 0.028,
                ported: false,
                light: false,
                long_mag: 0.0,
                comp: false,
            },
        ),
        1 => pistol(
            &mut m,
            PistolSpec {
                len: 0.21,
                slide_h: 0.036,
                w: 0.031,
                ported: true,
                light: true,
                long_mag: 0.0,
                comp: false,
            },
        ),
        2 => hornet(&mut m),
        3 => kestrel(&mut m),
        4 => wasp(&mut m),
        5 => mamba(&mut m),
        6 => falcon(&mut m),
        7 => ranger(&mut m),
        8 => tempest(&mut m),
        9 => bulldog(&mut m),
        10 => breacher(&mut m),
        11 => stormfront(&mut m),
        12 => double_barrel(&mut m),
        13 => goliath(&mut m),
        14 => ripsaw(&mut m),
        15 => longbow(&mut m),
        16 => arbiter(&mut m),
        17 => sentinel(&mut m),
        18 => revolver(&mut m),
        19 => {
            pistol(
                &mut m,
                PistolSpec {
                    len: 0.26,
                    slide_h: 0.042,
                    w: 0.034,
                    ported: true,
                    light: false,
                    long_mag: 0.0,
                    comp: true,
                },
            );
            rail(&mut m.detail, 0.02, -0.18, 0.054, 0.016, BLACK);
        }
        20 => twin_fang(&mut m),
        21 => ray_blaster(&mut m),
        _ => thunder_cannon(&mut m),
    }
    m
}

/// A grenade model (for the held and thrown grenade).
pub fn grenade_kit(k: &mut Kit, at: Vec3) {
    // A classic "pineapple" frag: segmented olive body, steel fuze on top,
    // the spoon down the side and a pull ring.
    let olive = c(0.3, 0.36, 0.19);
    let groove = c(0.17, 0.2, 0.1);
    k.blob(at, v(0.03, 0.038, 0.03), groove);
    for row in 0..5 {
        let lat = -0.75 + row as f32 * 0.375;
        for col in 0..8 {
            let lon = col as f32 / 8.0 * std::f32::consts::TAU + row as f32 * 0.2;
            let n = v(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin());
            let p = at + n * v(0.03, 0.038, 0.03);
            let rot = Quat::from_rotation_arc(Vec3::Y, n);
            k.cuboid_rot(p, v(0.019, 0.007, 0.016), rot, olive);
        }
    }
    // Fuze: collar, body and the striker housing.
    k.cyl(
        at + v(0.0, 0.04, 0.0),
        0.012,
        0.01,
        Quat::IDENTITY,
        c(0.5, 0.5, 0.48),
    );
    k.cyl(at + v(0.0, 0.052, 0.0), 0.01, 0.016, Quat::IDENTITY, STEEL);
    k.cuboid(at + v(0.0, 0.062, 0.004), v(0.012, 0.008, 0.02), STEEL);
    // Spoon: curved lever down the side.
    k.cuboid_rot(
        at + v(0.012, 0.058, 0.0),
        v(0.026, 0.004, 0.012),
        Quat::from_rotation_z(-0.2),
        STEEL,
    );
    k.cuboid_rot(
        at + v(0.031, 0.03, 0.0),
        v(0.004, 0.05, 0.012),
        Quat::from_rotation_z(0.25),
        STEEL,
    );
    k.cuboid_rot(
        at + v(0.034, -0.002, 0.0),
        v(0.004, 0.018, 0.01),
        Quat::from_rotation_z(-0.15),
        STEEL,
    );
    // Pin and ring.
    k.cyl_z(at + v(-0.004, 0.054, 0.0), 0.002, 0.03, c(0.75, 0.75, 0.7));
    k.torus(
        at + v(-0.012, 0.054, -0.024),
        0.0018,
        0.011,
        Quat::from_rotation_x(FRAC_PI_2),
        c(0.8, 0.8, 0.75),
    );
}
