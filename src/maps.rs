//! The three maps: Shipping Yard, Central Park and The Neighborhood. Each is
//! built from modelled props (see props.rs) merged into a few meshes, simple
//! invisible boxes to collide with, a textured ground, lamps, and the spots
//! for spawns, the mystery box, perk machines and extraction.

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use crate::data::Perk;
use crate::kit::{glow_material, scale_uvs, vertex_material, Kit};
use crate::props::{self, Art};
use crate::{Collider, InGameEntity};

pub const MAP_NAMES: [&str; 3] = ["Shipping Yard", "Central Park", "The Neighborhood"];

pub fn map_name(id: u8) -> &'static str {
    MAP_NAMES[(id as usize).min(MAP_NAMES.len() - 1)]
}

pub struct Solid {
    pub pos: Vec3,
    pub size: Vec3,
    pub color: Color,
    /// Drawn as a plain box (false when a prop's model covers it).
    pub show: bool,
}

/// Which procedural texture covers the ground.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ground {
    Asphalt,
    Grass,
}

pub struct MapLayout {
    pub half: f32,
    pub ground: Color,
    pub ground_kind: Ground,
    pub sky: Color,
    pub sun: f32,
    pub solids: Vec<Solid>,
    pub art: Art,
    /// Lamps: position, colour, brightness.
    pub lights: Vec<(Vec3, Color, f32)>,
    pub player_spawns: Vec<Vec3>,
    pub enemy_spawns: Vec<Vec3>,
    pub box_spots: [Vec3; 5],
    pub perk_spots: [Vec3; 5],
    pub extraction: Vec3,
}

impl MapLayout {
    fn new(half: f32, ground: Color, ground_kind: Ground, sky: Color, sun: f32) -> Self {
        let mut m = Self {
            half,
            ground,
            ground_kind,
            sky,
            sun,
            solids: Vec::new(),
            art: Art::default(),
            lights: Vec::new(),
            player_spawns: Vec::new(),
            enemy_spawns: Vec::new(),
            box_spots: [Vec3::ZERO; 5],
            perk_spots: [Vec3::ZERO; 5],
            extraction: Vec3::ZERO,
        };
        let h = half;
        for (x, z) in [
            (-h + 4.0, -h + 4.0),
            (h - 4.0, -h + 4.0),
            (-h + 4.0, h - 4.0),
            (h - 4.0, h - 4.0),
            (0.0, -h + 4.0),
            (0.0, h - 4.0),
            (-h + 4.0, 0.0),
            (h - 4.0, 0.0),
        ] {
            m.enemy_spawns.push(Vec3::new(x, 0.0, z));
        }
        m
    }

    /// Flat paint on the ground (roads, paths, markings).
    fn ground_paint(&mut self, center: Vec3, size: Vec2, yaw: f32, color: Color, lift: f32) {
        self.art.paint.cuboid_rot(
            Vec3::new(center.x, lift, center.z),
            Vec3::new(size.x, 0.02, size.y),
            Quat::from_rotation_y(yaw),
            color,
        );
    }

    /// Is this spot clear of the important places (spawns, box, perks)?
    fn clear(&self, x: f32, z: f32, room: f32) -> bool {
        let p = Vec3::new(x, 0.0, z);
        !self
            .box_spots
            .iter()
            .chain(self.perk_spots.iter())
            .chain(self.player_spawns.iter())
            .chain(self.enemy_spawns.iter())
            .chain(std::iter::once(&self.extraction))
            .any(|s| s.distance(p) < room)
    }
}

pub fn layout(map: u8) -> MapLayout {
    match map {
        1 => central_park(),
        2 => neighborhood(),
        _ => shipping_yard(),
    }
}

fn shipping_yard() -> MapLayout {
    let mut m = MapLayout::new(
        40.0,
        Color::srgb(0.5, 0.5, 0.5),
        Ground::Asphalt,
        Color::srgb(0.62, 0.68, 0.75),
        8000.0,
    );
    m.player_spawns = spawn_line(Vec3::new(0.0, 0.0, 0.0));
    m.box_spots = [
        Vec3::new(0.0, 0.0, -9.0),
        Vec3::new(-14.0, 0.0, 22.5),
        Vec3::new(14.0, 0.0, -22.5),
        Vec3::new(31.0, 0.0, 0.0),
        Vec3::new(-31.0, 0.0, -2.0),
    ];
    m.perk_spots = [
        Vec3::new(-6.0, 0.0, 12.0),
        Vec3::new(6.0, 0.0, -12.0),
        Vec3::new(-27.0, 0.0, -10.0),
        Vec3::new(27.0, 0.0, 10.0),
        Vec3::new(0.0, 0.0, 22.5),
    ];
    m.extraction = Vec3::new(0.0, 0.0, 34.0);
    m.boundary(0, 4.5);

    let mut rng = StdRng::seed_from_u64(7);
    let colors = [
        Color::srgb(0.6, 0.18, 0.12),
        Color::srgb(0.12, 0.3, 0.6),
        Color::srgb(0.15, 0.45, 0.25),
        Color::srgb(0.85, 0.45, 0.1),
        Color::srgb(0.5, 0.5, 0.52),
        Color::srgb(0.75, 0.65, 0.15),
    ];
    // Rows of containers running east-west, with gaps to run through.
    let mut seed = 0.0;
    for z in [-28.0, -17.0, 17.0, 28.0] {
        let mut x = -31.0;
        while x <= 31.0 {
            if rng.gen_bool(0.78) {
                seed += 1.0;
                let col = colors[rng.gen_range(0..colors.len())];
                m.container(x, z, 0.0, false, col, seed);
                if rng.gen_bool(0.35) {
                    let c2 = colors[rng.gen_range(0..colors.len())];
                    m.container(x + 0.15, z, 2.6, false, c2, seed + 0.5);
                }
            }
            x += 8.5;
        }
    }
    // Containers running north-south on the sides of the central yard.
    for x in [-22.0, 22.0] {
        for z in [-6.0, 5.0] {
            seed += 1.0;
            let col = colors[rng.gen_range(0..colors.len())];
            m.container(x, z, 0.0, true, col, seed);
        }
    }
    // Cover in the middle: a forklift, pallets, drums and barriers.
    m.forklift(-8.0, -6.0, 0.6, Color::srgb(0.95, 0.7, 0.1));
    m.pallet_stack(9.0, 4.0, 0.2, 1.0);
    m.pallet_stack(10.4, 5.2, -0.3, 2.0);
    m.drums(-3.0, 9.0, 4, 3.0);
    m.barriers(4.0, -7.0, 0.0, 2, Color::srgb(0.72, 0.72, 0.7));
    m.barriers(-15.0, 13.0, FRAC_PI_2, 2, Color::srgb(0.72, 0.72, 0.7));
    m.barriers(15.0, -13.0, FRAC_PI_2, 2, Color::srgb(0.72, 0.72, 0.7));
    m.tires(17.0, 9.0, 4);
    m.tires(-17.5, -9.0, 3);
    m.drums(-35.0, 13.0, 3, 5.0);
    m.drums(35.0, -13.0, 2, 6.0);
    m.pallet_stack(-35.0, 22.0, 0.0, 4.0);
    m.pallet_stack(35.0, 22.0, 0.4, 5.0);
    m.pallet_stack(-10.0, -35.0, 0.0, 6.0);
    m.pallet_stack(12.0, 35.0, 0.3, 7.0);
    for (x, z) in [(2.0, -4.5), (3.5, -4.0), (-11.0, 3.0), (-11.5, 4.5), (18.0, 13.0), (-18.0, -13.0)] {
        m.traffic_cone(x, z);
    }

    // Two gantry cranes over the yard, with their rails.
    for x in [-12.0, 12.0] {
        m.gantry(x, 0.0, 22.0, 14.0);
        for s in [-1.2, 1.2] {
            m.ground_paint(Vec3::new(x + s, 0.0, 0.0), Vec2::new(0.25, 80.0), 0.0, Color::srgb(0.35, 0.33, 0.3), 0.012);
        }
    }

    // Site office and floodlights.
    m.site_office(-30.0, 8.0, 0.0);
    for (x, z) in [(-34.0f32, -34.0f32), (34.0, -34.0), (-34.0, 34.0), (34.0, 34.0)] {
        let yaw = (-x).atan2(-z) + std::f32::consts::PI;
        m.light_tower(x * 0.9, z * 0.9, yaw);
    }

    // Painted lanes and bay markings.
    let yellow = Color::srgb(0.9, 0.75, 0.15);
    let white = Color::srgb(0.85, 0.85, 0.82);
    for z in [-11.5, 11.5] {
        let mut x = -38.0;
        while x < 38.0 {
            m.ground_paint(Vec3::new(x, 0.0, z), Vec2::new(2.0, 0.18), 0.0, yellow, 0.014);
            x += 3.5;
        }
    }
    for x in [-30.0, -26.0, 26.0, 30.0] {
        m.ground_paint(Vec3::new(x, 0.0, 0.0), Vec2::new(0.15, 14.0), 0.0, white, 0.014);
    }
    m.ground_paint(Vec3::new(0.0, 0.0, 34.0), Vec2::new(9.0, 9.0), 0.0, Color::srgb(0.22, 0.23, 0.24), 0.011);
    for i in 0..6 {
        let x = -4.0 + i as f32 * 1.6;
        m.ground_paint(Vec3::new(x, 0.0, 34.0), Vec2::new(0.6, 8.0), 0.6, yellow, 0.015);
    }
    // Oil stains.
    for i in 0..14 {
        let x = rng.gen_range(-30.0..30.0f32);
        let z = rng.gen_range(-12.0..12.0f32);
        let r = rng.gen_range(0.6..1.6);
        m.art.paint.cyl(Vec3::new(x, 0.006, z), r, 0.01, Quat::IDENTITY, Color::srgb(0.13, 0.13, 0.14));
        let _ = i;
    }

    // Outside the fence: the ship at the quay and stacks of containers.
    m.ship(0.0, -58.0, 90.0);
    for (side, along_z) in [(1.0f32, false), (-1.0, true), (1.0, true)] {
        let mut t = -36.0;
        let mut i = 0.0;
        while t <= 36.0 {
            let stack = 1 + (props::hash(t, side) * 3.0) as i32;
            for l in 0..stack {
                let col = colors[((props::hash(t, l as f32 + side) * 6.0) as usize).min(5)];
                i += 1.0;
                let (x, z) = if along_z { (side * 47.0, t) } else { (t, 47.0) };
                m.container(x, z, l as f32 * 2.6, along_z, col, 100.0 + i);
            }
            t += 7.0;
        }
    }
    m
}

fn central_park() -> MapLayout {
    let mut m = MapLayout::new(
        40.0,
        Color::srgb(0.85, 0.95, 0.8),
        Ground::Grass,
        Color::srgb(0.5, 0.7, 0.95),
        10000.0,
    );
    m.player_spawns = spawn_line(Vec3::new(0.0, 0.0, 12.0));
    m.box_spots = BOX_SPOTS_OPEN;
    m.perk_spots = PERK_SPOTS_OPEN;
    m.extraction = Vec3::new(-22.0, 0.0, 21.0);
    m.boundary(1, 3.0);
    m.skyline(40.0, 3.0);

    // Gravel paths with stone edging.
    let path = Color::srgb(0.7, 0.64, 0.52);
    let edge = Color::srgb(0.55, 0.53, 0.5);
    for (yaw, len, w) in [(0.0f32, 80.0f32, 4.0f32), (FRAC_PI_2, 80.0, 4.0), (FRAC_PI_4, 100.0, 3.0)] {
        m.ground_paint(Vec3::ZERO, Vec2::new(w, len), yaw, path, 0.008);
        for s in [-1.0f32, 1.0] {
            let off = Quat::from_rotation_y(yaw) * Vec3::new(s * (w / 2.0 + 0.1), 0.0, 0.0);
            m.art.paint.cuboid_rot(
                off + Vec3::Y * 0.05,
                Vec3::new(0.2, 0.1, len),
                Quat::from_rotation_y(yaw),
                edge,
            );
        }
    }
    // Paved plaza around the fountain.
    m.art.paint.cyl(Vec3::new(0.0, 0.012, 0.0), 7.0, 0.02, Quat::IDENTITY, Color::srgb(0.66, 0.63, 0.58));
    m.art.paint.torus(Vec3::new(0.0, 0.03, 0.0), 0.08, 7.0, Quat::IDENTITY, edge);
    m.fountain(0.0, 0.0);

    // Pond with a stone rim (gaps let you walk in), lily pads and reeds.
    let water = Color::srgb(0.2, 0.42, 0.62);
    m.art.glass.cuboid(Vec3::new(24.0, 0.04, -20.0), Vec3::new(14.0, 0.04, 10.0), water);
    m.art.paint.cuboid(Vec3::new(24.0, 0.01, -20.0), Vec3::new(14.2, 0.02, 10.2), Color::srgb(0.18, 0.25, 0.2));
    for (x, z, sx, sz) in [
        (20.0, -25.3, 7.0, 0.5),
        (29.0, -25.3, 3.5, 0.5),
        (24.0, -14.7, 14.0, 0.5),
        (16.8, -20.0, 0.5, 6.0),
        (31.2, -20.0, 0.5, 10.6),
    ] {
        m.collide(Vec3::new(x, 0.25, z), Vec3::new(sx, 0.5, sz));
        let n = ((sx.max(sz)) / 0.7) as i32;
        for i in 0..=n {
            let t = i as f32 / n.max(1) as f32 - 0.5;
            let p = if sx > sz { Vec3::new(x + t * sx, 0.22, z) } else { Vec3::new(x, 0.22, z + t * sz) };
            let r = 0.32 + props::hash(p.x, p.z) * 0.12;
            m.art.paint.blob(p, Vec3::new(r * 1.2, r * 0.8, r), Color::srgb(0.55 + r * 0.2, 0.55 + r * 0.2, 0.53 + r * 0.2));
        }
    }
    for i in 0..9 {
        let x = 19.0 + props::hash(i as f32, 1.0) * 10.0;
        let z = -24.0 + props::hash(i as f32, 2.0) * 8.0;
        m.art.paint.cyl(Vec3::new(x, 0.07, z), 0.35, 0.02, Quat::IDENTITY, Color::srgb(0.25, 0.55, 0.2));
        if i % 3 == 0 {
            m.art.paint.sphere(Vec3::new(x + 0.1, 0.12, z), 0.08, Color::srgb(0.95, 0.75, 0.85));
        }
    }
    for i in 0..14 {
        let x = 17.2 + (i % 2) as f32 * 0.3;
        let z = -24.5 + i as f32 * 0.65;
        let h = 1.0 + props::hash(i as f32, 3.0) * 0.6;
        m.art.paint.cyl(Vec3::new(x, h / 2.0, z), 0.02, h, Quat::IDENTITY, Color::srgb(0.3, 0.45, 0.2));
        m.art.paint.capsule_between(Vec3::new(x, h - 0.2, z), Vec3::new(x, h, z), 0.04, Color::srgb(0.35, 0.22, 0.12));
    }

    m.bandstand(-22.0, 21.0);
    m.statue(-12.0, -26.0, 0.3);
    m.playground(24.0, 10.0, 0.0);

    // Trees.
    let mut rng = StdRng::seed_from_u64(11);
    let mut placed = 0;
    let mut tries = 0;
    while placed < 30 && tries < 2000 {
        tries += 1;
        let x = rng.gen_range(-36.0..36.0f32);
        let z = rng.gen_range(-36.0..36.0f32);
        let on_path = x.abs() < 4.0 || z.abs() < 4.0 || (x - z).abs() < 3.5;
        let busy = (x.abs() < 9.0 && z.abs() < 9.0)
            || (x > 14.0 && x < 34.0 && z > -28.0 && z < -12.0)
            || (x > -28.0 && x < -16.0 && z > 15.0 && z < 27.0)
            || (x > 17.0 && x < 31.0 && z > 5.0 && z < 15.0)
            || (x > -16.0 && x < -8.0 && z > -30.0 && z < -22.0)
            || !m.clear(x, z, 5.0);
        if on_path || busy {
            continue;
        }
        let size = rng.gen_range(0.85..1.25);
        if rng.gen_bool(0.35) {
            m.pine(x, z, size, placed as f32);
        } else {
            m.oak(x, z, size, placed as f32);
        }
        placed += 1;
    }
    // Hedges with rounded tops.
    let hedge = Color::srgb(0.13, 0.33, 0.12);
    for (x, z, sx, sz) in [
        (-14.0, -10.0, 8.0, 1.0),
        (14.0, 10.0, 8.0, 1.0),
        (-10.0, 14.0, 1.0, 6.0),
        (10.0, -14.0, 1.0, 6.0),
        (-30.0, -6.0, 6.0, 1.0),
    ] {
        m.collide(Vec3::new(x, 0.65, z), Vec3::new(sx, 1.3, sz));
        m.art.paint.cuboid(Vec3::new(x, 0.55, z), Vec3::new(sx, 1.1, sz), hedge);
        let n = (sx.max(sz) / 0.8) as i32;
        for i in 0..=n {
            let t = i as f32 / n as f32 - 0.5;
            let p = if sx > sz { Vec3::new(x + t * (sx - 0.5), 1.1, z) } else { Vec3::new(x, 1.1, z + t * (sz - 0.5)) };
            m.art.paint.blob(p, Vec3::new(0.62, 0.35, 0.62), Color::srgb(0.15, 0.37 + props::hash(p.x, p.z) * 0.06, 0.13));
        }
    }
    // Benches facing the paths, bins, rocks, flower bushes and lamps.
    for (x, z, yaw) in [(6.0, 3.0, 0.0), (-6.0, -3.0, std::f32::consts::PI), (3.0, 10.0, FRAC_PI_2), (-3.0, -10.0, -FRAC_PI_2), (8.5, -2.6, 0.0), (-8.5, 2.6, std::f32::consts::PI)] {
        m.bench(x, z, yaw);
    }
    for (x, z) in [(7.5, 3.0), (-7.5, -3.0), (3.0, 12.0)] {
        m.bin(x, z);
    }
    for (x, z) in [(-28.0, -26.0), (26.0, 24.0), (-8.0, -30.0)] {
        m.rock(x, z, 1.4, x + z);
    }
    let flowers = [Color::srgb(0.95, 0.3, 0.4), Color::srgb(0.95, 0.85, 0.25), Color::srgb(0.65, 0.4, 0.95), Color::srgb(1.0, 1.0, 1.0)];
    for (i, (x, z)) in [(5.5, 6.5), (-5.5, 6.5), (5.5, -6.5), (-5.5, -6.5), (6.5, 18.0), (-6.5, -18.0), (18.0, -6.0), (-18.0, 6.5), (-26.0, 12.0), (12.0, 26.0)].into_iter().enumerate() {
        m.bush(x, z, 1.0, i as f32, Some(flowers[i % 4]));
    }
    for (x, z) in [(5.0, 5.0), (-5.0, -5.0), (5.0, -5.0), (-5.0, 5.0)] {
        m.park_lamp(x * 1.5, z * 1.5);
    }
    m.park_lamp(-6.0, 22.0);
    m.park_lamp(6.0, -22.0);
    m
}

/// Spots used by maps whose layout keeps these areas clear.
const BOX_SPOTS_OPEN: [Vec3; 5] = [
    Vec3::new(0.0, 0.0, -14.0),
    Vec3::new(-30.0, 0.0, 30.0),
    Vec3::new(30.0, 0.0, 30.0),
    Vec3::new(-30.0, 0.0, -30.0),
    Vec3::new(14.0, 0.0, 0.0),
];
const PERK_SPOTS_OPEN: [Vec3; 5] = [
    Vec3::new(-14.0, 0.0, 6.0),
    Vec3::new(14.0, 0.0, -6.0),
    Vec3::new(-20.0, 0.0, -20.0),
    Vec3::new(20.0, 0.0, 20.0),
    Vec3::new(0.0, 0.0, 26.0),
];

fn neighborhood() -> MapLayout {
    let mut m = MapLayout::new(
        40.0,
        Color::srgb(0.9, 1.0, 0.85),
        Ground::Grass,
        Color::srgb(0.95, 0.68, 0.5),
        7000.0,
    );
    m.player_spawns = spawn_line(Vec3::new(0.0, 0.0, -1.0));
    m.box_spots = [
        Vec3::new(-7.0, 0.0, -12.0),
        Vec3::new(21.0, 0.0, 12.0),
        Vec3::new(-21.0, 0.0, 30.0),
        Vec3::new(14.0, 0.0, -30.0),
        Vec3::new(-35.0, 0.0, 0.0),
    ];
    m.perk_spots = [
        Vec3::new(7.0, 0.0, 12.0),
        Vec3::new(-21.0, 0.0, -12.0),
        Vec3::new(21.0, 0.0, -12.0),
        Vec3::new(-28.0, 0.0, 30.0),
        Vec3::new(35.0, 0.0, 0.0),
    ];
    m.extraction = Vec3::new(0.0, 0.0, -33.0);
    m.boundary(2, 2.2);

    // Street: asphalt, centre dashes, curbs and sidewalks with seams.
    m.ground_paint(Vec3::ZERO, Vec2::new(82.0, 8.0), 0.0, Color::srgb(0.17, 0.17, 0.19), 0.006);
    for i in -9..=9 {
        m.ground_paint(Vec3::new(i as f32 * 4.5, 0.0, 0.0), Vec2::new(2.0, 0.15), 0.0, Color::srgb(0.95, 0.82, 0.25), 0.018);
    }
    for z in [-3.85, 3.85] {
        m.ground_paint(Vec3::new(0.0, 0.0, z), Vec2::new(82.0, 0.12), 0.0, Color::srgb(0.9, 0.9, 0.88), 0.017);
    }
    for s in [-1.0f32, 1.0] {
        m.art.paint.cuboid(Vec3::new(0.0, 0.06, s * 4.05), Vec3::new(82.0, 0.12, 0.2), Color::srgb(0.6, 0.6, 0.58));
        m.art.paint.cuboid(Vec3::new(0.0, 0.05, s * 5.15), Vec3::new(82.0, 0.1, 2.0), Color::srgb(0.68, 0.68, 0.65));
        let mut x = -40.0;
        while x < 40.0 {
            m.art.paint.cuboid(Vec3::new(x, 0.1, s * 5.15), Vec3::new(0.04, 0.012, 2.0), Color::srgb(0.5, 0.5, 0.48));
            x += 1.6;
        }
        // Grass verge strip trees are at the back; crosswalk near the middle.
    }
    for i in 0..8 {
        m.ground_paint(Vec3::new(-12.0, 0.0, -3.2 + i as f32 * 0.9), Vec2::new(2.6, 0.45), 0.0, Color::srgb(0.92, 0.92, 0.9), 0.019);
    }

    // Houses on both sides of the street, with yards in front.
    let walls = [
        Color::srgb(0.85, 0.8, 0.7),
        Color::srgb(0.6, 0.72, 0.85),
        Color::srgb(0.9, 0.75, 0.6),
        Color::srgb(0.75, 0.85, 0.7),
        Color::srgb(0.8, 0.65, 0.65),
    ];
    let roofs = [
        Color::srgb(0.35, 0.2, 0.17),
        Color::srgb(0.25, 0.27, 0.32),
        Color::srgb(0.42, 0.28, 0.2),
    ];
    let mut i = 0;
    for side in [-1.0f32, 1.0] {
        for x in [-28.0f32, -14.0, 0.0, 14.0, 28.0] {
            let z = side * 17.0;
            let yaw = if side < 0.0 { 0.0 } else { std::f32::consts::PI };
            m.house(x, z, yaw, walls[i % walls.len()], roofs[i % roofs.len()], i as f32);
            i += 1;
            m.picket_fence(Vec3::new(x - 5.0, 0.0, side * 9.5), Vec3::new(x + 5.0, 0.0, side * 9.5), 2.6);
            // Front walk and driveway.
            m.ground_paint(Vec3::new(x - 1.0 * if side < 0.0 { 1.0 } else { -1.0 }, 0.0, side * 8.0), Vec2::new(1.2, 4.2), 0.0, Color::srgb(0.68, 0.66, 0.62), 0.01);
            let dx = x + 7.0;
            if dx < 38.0 {
                m.ground_paint(Vec3::new(dx, 0.0, side * 9.0), Vec2::new(3.2, 7.0), 0.0, Color::srgb(0.45, 0.45, 0.45), 0.009);
            }
            let mx = x - 1.0 * if side < 0.0 { 1.0 } else { -1.0 } + 1.2;
            m.mailbox(mx, side * 6.8, if side < 0.0 { 0.0 } else { std::f32::consts::PI }, Color::srgb(0.15, 0.2, 0.45));
        }
    }
    // Parked cars along the street and in a couple of driveways.
    let car_colors = [
        Color::srgb(0.7, 0.1, 0.1),
        Color::srgb(0.1, 0.2, 0.6),
        Color::srgb(0.85, 0.85, 0.85),
        Color::srgb(0.15, 0.15, 0.15),
        Color::srgb(0.2, 0.45, 0.3),
    ];
    for (k, (x, z)) in [(-22.0, -2.6), (-6.0, 2.6), (10.0, -2.6), (24.0, 2.6)].into_iter().enumerate() {
        m.car(x, z, if z < 0.0 { 0.0 } else { std::f32::consts::PI }, car_colors[k % car_colors.len()]);
    }
    m.car(-7.0, 11.0, FRAC_PI_2, car_colors[4]);
    m.car(7.0, -11.5, -FRAC_PI_2, car_colors[2]);
    for (x, z) in [(-17.5, 6.6), (17.0, -6.6), (31.0, 6.6)] {
        m.hydrant(x, z);
    }
    for (x, z) in [(-25.0, 11.0), (25.0, -11.0), (3.5, -10.5)] {
        if m.clear(x, z, 2.5) {
            m.trash_cans(x, z);
        }
    }
    m.hoop(-21.0, -11.0, 0.0);
    // Street lights on alternating sides.
    for x in [-30.0f32, -10.0, 10.0, 30.0] {
        m.street_light(x, 6.6, 0.0);
        m.street_light(x + 10.0, -6.6, std::f32::consts::PI);
    }

    // Back yards: trees, bushes and sheds.
    for side in [-1.0f32, 1.0] {
        for x in [-21.0f32, -7.0, 7.0, 21.0] {
            if m.clear(x, side * 30.0, 3.0) {
                m.oak(x, side * 30.0, 1.0, x + side);
            } else {
                m.bush(x + 3.0, side * 31.0, 1.2, x, None);
            }
        }
        m.shed(-35.0, side * 26.0, if side < 0.0 { 0.0 } else { std::f32::consts::PI });
        for x in [-32.0f32, -18.0, 4.0, 18.0, 32.0] {
            if m.clear(x, side * 34.5, 3.0) {
                m.bush(x, side * 35.5, 1.1, x * side, None);
            }
        }
    }
    for (i, (x, z)) in [(-24.0f32, -10.6), (-33.0, 10.6), (33.0, -10.6), (12.0, 10.6), (-3.5, 10.6)].into_iter().enumerate() {
        if m.clear(x, z, 2.0) {
            m.bush(x, z, 0.8, i as f32, Some(Color::srgb(0.95, 0.4, 0.55)));
        }
    }
    // Trees beyond the fence.
    for i in 0..48 {
        let ang = i as f32 / 48.0 * std::f32::consts::TAU;
        let r = 52.0 + props::hash(i as f32, 9.0) * 10.0;
        let (x, z) = (ang.cos() * r, ang.sin() * r);
        if i % 3 == 0 {
            m.oak(x, z, 1.5, i as f32);
        } else {
            m.pine(x, z, 1.6 + props::hash(i as f32, 1.0), i as f32);
        }
    }
    m
}

fn spawn_line(center: Vec3) -> Vec<Vec3> {
    (0..8)
        .map(|i| center + Vec3::new((i % 4) as f32 * 2.0 - 3.0, 0.0, (i / 4) as f32 * 2.0))
        .collect()
}

/// Where a player (re)spawns on this map.
pub fn player_spawn(layout: &MapLayout, id: u8) -> Vec3 {
    layout.player_spawns[id as usize % layout.player_spawns.len()]
}

// ---------------------------------------------------------------------------
// Spawning the map into the world
// ---------------------------------------------------------------------------

/// The loaded map's layout, available during a match.
#[derive(Resource)]
pub struct CurrentMap(pub MapLayout);

#[derive(Component)]
pub struct MysteryBox;

#[derive(Component)]
pub struct BoxGlow;

#[derive(Component)]
#[allow(dead_code)]
pub struct PerkMachine(pub Perk);

#[derive(Component)]
pub struct ExtractionBeacon;

/// The mystery box lid (hinged at the back edge).
#[derive(Component)]
pub struct BoxLid;

/// The light pillar over the box that helps you find it.
#[derive(Component)]
pub struct BoxPillar;

/// Tileable value noise in 0..1 with the given period (in cells).
pub fn vnoise(x: f32, y: f32, period: i32) -> f32 {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let h = |a: i32, b: i32| props::hash(a.rem_euclid(period) as f32, b.rem_euclid(period) as f32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(fx), s(fy));
    let top = h(ix, iy) + (h(ix + 1, iy) - h(ix, iy)) * sx;
    let bottom = h(ix, iy + 1) + (h(ix + 1, iy + 1) - h(ix, iy + 1)) * sx;
    top + (bottom - top) * sy
}

/// Procedural, seamlessly tiling ground texture.
fn ground_texture(kind: Ground) -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const N: usize = 256;
    let mut data = vec![0u8; N * N * 4];
    for y in 0..N {
        for x in 0..N {
            let (fx, fy) = (x as f32, y as f32);
            let mut f = 0.0;
            let mut amp = 0.5;
            for period in [4, 8, 16, 32] {
                let scale = period as f32 / N as f32;
                f += vnoise(fx * scale, fy * scale, period) * amp;
                amp *= 0.5;
            }
            let grain = props::hash(fx, fy);
            let rgb = match kind {
                Ground::Asphalt => {
                    let mut g = 0.2 + f * 0.12 + (grain - 0.5) * 0.05;
                    if grain > 0.985 {
                        g += 0.12;
                    }
                    // Faint patching.
                    let patch = vnoise(fx / 40.0, fy / 40.0, 6);
                    if patch > 0.72 {
                        g -= 0.04;
                    }
                    [g, g, g * 1.04]
                }
                Ground::Grass => {
                    let blade = (grain - 0.5) * 0.08;
                    let patch = vnoise(fx / 32.0, fy / 32.0, 8);
                    let r = 0.2 + f * 0.1 + blade + patch * 0.06;
                    let g = 0.4 + f * 0.14 + blade * 1.5 + patch * 0.04;
                    let b = 0.13 + f * 0.05;
                    if grain > 0.993 {
                        [0.85, 0.8, 0.35]
                    } else {
                        [r, g, b]
                    }
                }
            };
            let i = (y * N + x) * 4;
            for c in 0..3 {
                data[i + c] = (rgb[c].clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8;
            }
            data[i + 3] = 255;
        }
    }
    // Cracks in asphalt.
    if kind == Ground::Asphalt {
        for crack in 0..6 {
            let mut px = props::hash(crack as f32, 1.0) * N as f32;
            let mut py = props::hash(crack as f32, 2.0) * N as f32;
            let mut ang = props::hash(crack as f32, 3.0) * 6.28;
            for step in 0..120 {
                ang += (props::hash(crack as f32, step as f32) - 0.5) * 0.9;
                px += ang.cos();
                py += ang.sin();
                let (ix, iy) = ((px as i32).rem_euclid(N as i32) as usize, (py as i32).rem_euclid(N as i32) as usize);
                let i = (iy * N + ix) * 4;
                for c in 0..3 {
                    data[i + c] = (data[i + c] as f32 * 0.55) as u8;
                }
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

/// A glowing "?" in the XY plane, facing +Z.
fn question_mark(k: &mut Kit, size: f32, color: Color) {
    let r = 0.32 * size;
    let cy = 0.3 * size;
    let mut pts = Vec::new();
    for i in 0..=8 {
        let a = std::f32::consts::PI - i as f32 / 8.0 * 4.2;
        pts.push(Vec3::new(a.cos() * r, cy + a.sin() * r, 0.0));
    }
    pts.push(Vec3::new(0.0, -0.18 * size, 0.0));
    for w in pts.windows(2) {
        k.capsule_between(w[0], w[1], 0.07 * size, color);
    }
    k.sphere(Vec3::new(0.0, -0.48 * size, 0.0), 0.1 * size, color);
}

fn box_art() -> (Art, Art) {
    let mut body = Art::default();
    let mut lid = Art::default();
    let (hx, hz) = (BOX_HALF.x, BOX_HALF.z);
    let bottom = -BOX_HALF.y;
    let top = BOX_HALF.y - 0.18;
    let woods = [Color::srgb(0.42, 0.27, 0.14), Color::srgb(0.36, 0.22, 0.11), Color::srgb(0.46, 0.3, 0.16)];
    let metal = Color::srgb(0.3, 0.3, 0.32);
    let blue = Color::srgb(0.45, 0.8, 1.0);
    // Planks.
    let rows = 4;
    let ph = (top - bottom) / rows as f32;
    for i in 0..rows {
        let y0 = bottom + i as f32 * ph;
        props::boxr(&mut body.paint, Vec3::new(-hx, y0 + 0.005, -hz), Vec3::new(hx, y0 + ph - 0.005, hz), woods[i % 3]);
        props::boxr(&mut body.paint, Vec3::new(-hx - 0.004, y0 + ph - 0.012, -hz - 0.004), Vec3::new(hx + 0.004, y0 + ph, hz + 0.004), Color::srgb(0.2, 0.12, 0.06));
    }
    // Metal corners and bands.
    for sx in [-1.0f32, 1.0] {
        for sz in [-1.0f32, 1.0] {
            props::boxr(&mut body.metal, Vec3::new(sx * hx - 0.06, bottom, sz * hz - 0.06), Vec3::new(sx * hx + 0.02, top, sz * hz + 0.02), metal);
        }
        props::boxr(&mut body.metal, Vec3::new(sx * 0.55 - 0.05, bottom, -hz - 0.015), Vec3::new(sx * 0.55 + 0.05, top, hz + 0.015), metal);
        // Handles on the ends.
        body.metal.torus(Vec3::new(sx * (hx + 0.03), 0.0, 0.0), 0.015, 0.09, Quat::from_rotation_z(FRAC_PI_2), metal);
    }
    // Glowing "?" on every side.
    for (pos, rot) in [
        (Vec3::new(0.0, -0.05, hz + 0.02), Quat::IDENTITY),
        (Vec3::new(0.0, -0.05, -hz - 0.02), Quat::from_rotation_y(std::f32::consts::PI)),
        (Vec3::new(hx + 0.02, -0.05, 0.0), Quat::from_rotation_y(FRAC_PI_2)),
        (Vec3::new(-hx - 0.02, -0.05, 0.0), Quat::from_rotation_y(-FRAC_PI_2)),
    ] {
        let mut q = Kit::new();
        question_mark(&mut q, 0.42, blue);
        body.glow.append(q, Transform::from_translation(pos).with_rotation(rot));
    }
    // The glow inside, seen when the lid opens.
    body.glow.cuboid(Vec3::new(0.0, top - 0.02, 0.0), Vec3::new(hx * 1.9, 0.02, hz * 1.8), Color::srgb(0.75, 0.92, 1.0));
    // Lid (pivot at the back top edge).
    let d = hz * 2.0;
    props::boxr(&mut lid.paint, Vec3::new(-hx - 0.02, 0.0, 0.0), Vec3::new(hx + 0.02, 0.18, d + 0.02), woods[0]);
    for i in 0..4 {
        let z = 0.02 + i as f32 * d / 4.0;
        props::boxr(&mut lid.paint, Vec3::new(-hx - 0.024, 0.17, z), Vec3::new(hx + 0.024, 0.19, z + 0.01), Color::srgb(0.2, 0.12, 0.06));
    }
    for sx in [-1.0f32, 1.0] {
        props::boxr(&mut lid.metal, Vec3::new(sx * 0.55 - 0.05, -0.005, -0.01), Vec3::new(sx * 0.55 + 0.05, 0.195, d + 0.03), metal);
        props::boxr(&mut lid.metal, Vec3::new(sx * hx - 0.08, -0.005, -0.01), Vec3::new(sx * hx + 0.03, 0.195, d + 0.03), metal);
    }
    let mut q = Kit::new();
    question_mark(&mut q, 0.5, blue);
    lid.glow.append(q, Transform::from_xyz(0.0, 0.2, d / 2.0).with_rotation(Quat::from_rotation_x(-FRAC_PI_2)));
    lid.metal.cuboid(Vec3::new(0.0, 0.06, d + 0.03), Vec3::new(0.2, 0.08, 0.04), metal);
    (body, lid)
}

/// Vending-machine style perk machine, front facing +Z, base at y = 0.
fn perk_art(perk: Perk) -> Art {
    let mut a = Art::default();
    let col = perk.color();
    let s = col.to_srgba();
    let body = Color::srgb(s.red * 0.55, s.green * 0.55, s.blue * 0.55);
    let black = Color::srgb(0.05, 0.05, 0.06);
    let chrome = Color::srgb(0.7, 0.7, 0.72);
    props::boxr(&mut a.paint, Vec3::new(-0.65, 0.0, -0.52), Vec3::new(0.65, 0.12, 0.52), black);
    props::boxr(&mut a.paint, Vec3::new(-0.6, 0.12, -0.5), Vec3::new(0.6, 2.3, 0.5), body);
    for sx in [-1.0f32, 1.0] {
        props::boxr(&mut a.paint, Vec3::new(sx * 0.6 - 0.01, 0.3, -0.2), Vec3::new(sx * 0.6 + 0.01, 2.1, 0.2), col);
        props::boxr(&mut a.metal, Vec3::new(sx * 0.6 - 0.03, 0.12, 0.47), Vec3::new(sx * 0.6 + 0.03, 2.3, 0.53), chrome);
    }
    // Lit sign on top with an emblem.
    props::boxr(&mut a.glow, Vec3::new(-0.6, 2.33, -0.48), Vec3::new(0.6, 2.72, 0.5), col);
    props::boxr(&mut a.metal, Vec3::new(-0.64, 2.3, -0.52), Vec3::new(0.64, 2.34, 0.54), chrome);
    props::boxr(&mut a.metal, Vec3::new(-0.64, 2.72, -0.52), Vec3::new(0.64, 2.76, 0.54), chrome);
    a.glow.cyl(Vec3::new(0.0, 2.52, 0.51), 0.15, 0.02, Quat::from_rotation_x(FRAC_PI_2), Color::WHITE);
    a.glow.cyl(Vec3::new(0.0, 2.52, 0.525), 0.1, 0.02, Quat::from_rotation_x(FRAC_PI_2), col);
    // Window full of bottles, standing out from the cabinet front.
    props::boxr(&mut a.paint, Vec3::new(-0.54, 0.88, 0.5), Vec3::new(0.26, 2.14, 0.52), Color::srgb(0.15, 0.15, 0.17));
    for sx in [-0.54f32, 0.26] {
        props::boxr(&mut a.metal, Vec3::new(sx - 0.025, 0.88, 0.5), Vec3::new(sx + 0.025, 2.14, 0.66), chrome);
    }
    for y in [0.88f32, 2.14] {
        props::boxr(&mut a.metal, Vec3::new(-0.565, y - 0.025, 0.5), Vec3::new(0.285, y + 0.025, 0.66), chrome);
    }
    for row in 0..3 {
        let y = 1.0 + row as f32 * 0.38;
        props::boxr(&mut a.metal, Vec3::new(-0.52, y - 0.02, 0.52), Vec3::new(0.24, y, 0.64), chrome);
        for i in 0..4 {
            let x = -0.42 + i as f32 * 0.18;
            a.glow.cyl(Vec3::new(x, y + 0.12, 0.58), 0.045, 0.2, Quat::IDENTITY, col);
            a.glow.cyl(Vec3::new(x, y + 0.26, 0.58), 0.018, 0.08, Quat::IDENTITY, Color::WHITE);
        }
    }
    a.glass.cuboid(Vec3::new(-0.14, 1.51, 0.655), Vec3::new(0.78, 1.24, 0.01), Color::srgb(0.75, 0.85, 0.95));
    // Buttons, screen and coin slot.
    a.glow.cuboid(Vec3::new(0.42, 1.9, 0.505), Vec3::new(0.18, 0.12, 0.02), Color::srgb(0.85, 1.0, 0.9));
    for row in 0..4 {
        for c in 0..2 {
            a.glow.cuboid(Vec3::new(0.37 + c as f32 * 0.1, 1.65 - row as f32 * 0.1, 0.505), Vec3::new(0.06, 0.05, 0.02), if (row + c) % 2 == 0 { col } else { Color::WHITE });
        }
    }
    props::boxr(&mut a.metal, Vec3::new(0.38, 1.1, 0.5), Vec3::new(0.46, 1.25, 0.52), chrome);
    a.paint.cuboid(Vec3::new(0.42, 1.18, 0.525), Vec3::new(0.012, 0.08, 0.01), black);
    // Dispenser.
    props::boxr(&mut a.paint, Vec3::new(-0.45, 0.25, 0.48), Vec3::new(0.2, 0.62, 0.52), black);
    props::boxr(&mut a.metal, Vec3::new(-0.43, 0.42, 0.51), Vec3::new(0.18, 0.6, 0.53), Color::srgb(0.25, 0.25, 0.27));
    a
}

fn art_meshes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &[Handle<StandardMaterial>; 4],
    art: Art,
    parent: Option<Entity>,
    marker: impl Fn(&mut EntityCommands),
) {
    for (kit, mat) in [art.paint, art.metal, art.glow, art.glass].into_iter().zip(mats.iter()) {
        if let Some(mesh) = kit.build() {
            let mut e = commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(mat.clone()), Transform::default()));
            marker(&mut e);
            if let Some(p) = parent {
                let id = e.id();
                commands.entity(p).add_child(id);
            } else {
                e.insert(InGameEntity);
            }
        }
    }
}

pub fn spawn_map(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    map: u8,
) -> MapLayout {
    let mut layout = layout(map);
    let mats: [Handle<StandardMaterial>; 4] = [
        materials.add(vertex_material(0.85, 0.0)),
        materials.add(vertex_material(0.42, 0.3)),
        materials.add(glow_material(1.0)),
        materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.55),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.08,
            reflectance: 0.6,
            ..default()
        }),
    ];

    // Textured ground, extending past the walls under the scenery.
    let size = layout.half * 2.0 + 140.0;
    let mut ground = Plane3d::default().mesh().size(size, size).build();
    scale_uvs(&mut ground, Vec2::splat(size / 7.0));
    commands.spawn((
        InGameEntity,
        Mesh3d(meshes.add(ground)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: layout.ground,
            base_color_texture: Some(images.add(ground_texture(layout.ground_kind))),
            perceptual_roughness: 0.95,
            ..default()
        })),
    ));

    for s in &layout.solids {
        let mut e = commands.spawn((
            InGameEntity,
            Transform::from_translation(s.pos),
            Collider { half: s.size / 2.0 },
        ));
        if s.show {
            e.insert((
                Mesh3d(meshes.add(Cuboid::from_size(s.size))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: s.color,
                    perceptual_roughness: 0.85,
                    ..default()
                })),
            ));
        }
    }
    let art = std::mem::take(&mut layout.art);
    art_meshes(commands, meshes, &mats, art, None, |_| {});
    for (pos, color, intensity) in &layout.lights {
        commands.spawn((
            InGameEntity,
            PointLight {
                intensity: *intensity,
                color: *color,
                range: 18.0,
                ..default()
            },
            Transform::from_translation(*pos),
        ));
    }

    commands.spawn((
        InGameEntity,
        DirectionalLight {
            illuminance: layout.sun,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(20.0, 40.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Perk machines, turned to face the middle of the map.
    for (spot, perk) in layout.perk_spots.iter().zip(Perk::ALL) {
        let to_center = -*spot;
        let quarter = (to_center.x.atan2(to_center.z) / FRAC_PI_2).round();
        let yaw = quarter * FRAC_PI_2;
        let body = if (quarter as i32).rem_euclid(2) == 1 { Vec3::new(1.0, 2.4, 1.2) } else { Vec3::new(1.2, 2.4, 1.0) };
        let root = commands
            .spawn((
                InGameEntity,
                PerkMachine(perk),
                Transform::from_translation(*spot + Vec3::Y * 1.2),
                Visibility::default(),
                Collider { half: body / 2.0 },
            ))
            .id();
        let holder = commands
            .spawn((
                Transform::from_xyz(0.0, -1.2, 0.0).with_rotation(Quat::from_rotation_y(yaw)),
                Visibility::default(),
            ))
            .id();
        commands.entity(root).add_child(holder);
        art_meshes(commands, meshes, &mats, perk_art(perk), Some(holder), |_| {});
        let light = commands
            .spawn((
                PointLight {
                    intensity: 40_000.0,
                    color: perk.color(),
                    range: 6.0,
                    ..default()
                },
                Transform::from_xyz(0.0, 2.6, 1.0),
            ))
            .id();
        commands.entity(holder).add_child(light);
    }

    // Mystery box: chest with a hinged lid and a light pillar over it.
    let (body, lid) = box_art();
    let root = commands
        .spawn((
            InGameEntity,
            MysteryBox,
            Transform::from_translation(layout.box_spots[0] + Vec3::Y * BOX_HALF.y),
            Visibility::default(),
            Collider { half: BOX_HALF },
        ))
        .id();
    art_meshes(commands, meshes, &mats, body, Some(root), |_| {});
    let hinge = commands
        .spawn((
            BoxLid,
            Transform::from_xyz(0.0, BOX_HALF.y - 0.18, -BOX_HALF.z),
            Visibility::default(),
        ))
        .id();
    commands.entity(root).add_child(hinge);
    art_meshes(commands, meshes, &mats, lid, Some(hinge), |_| {});
    let extras = [
        commands
            .spawn((
                BoxPillar,
                Mesh3d(meshes.add(Cylinder::new(0.35, 40.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(0.4, 0.75, 1.0, 0.18),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                })),
                Transform::from_xyz(0.0, 20.5, 0.0),
                bevy::pbr::NotShadowCaster,
            ))
            .id(),
        commands
            .spawn((
                BoxGlow,
                PointLight {
                    intensity: 60_000.0,
                    color: Color::srgb(0.4, 0.7, 1.0),
                    range: 8.0,
                    ..default()
                },
                Transform::from_xyz(0.0, 1.55, 0.0),
            ))
            .id(),
    ];
    commands.entity(root).add_children(&extras);

    // Extraction beacon (shown only when extraction is available).
    commands
        .spawn((
            InGameEntity,
            ExtractionBeacon,
            Transform::from_translation(layout.extraction),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(meshes.add(Cylinder::new(EXTRACT_RADIUS, 0.05))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(0.2, 1.0, 0.4, 0.35),
                    emissive: LinearRgba::rgb(0.5, 3.0, 1.0),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.05, 0.0),
            ));
            p.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.3, 30.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(0.3, 1.0, 0.5, 0.4),
                    emissive: LinearRgba::rgb(1.0, 6.0, 2.0),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_xyz(0.0, 15.0, 0.0),
            ));
            p.spawn((
                Mesh3d(meshes.add(Torus::new(EXTRACT_RADIUS - 0.15, EXTRACT_RADIUS))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.4, 1.0, 0.6),
                    unlit: true,
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.1, 0.0),
            ));
        });

    layout
}

pub const EXTRACT_RADIUS: f32 = 4.0;
pub const BOX_HALF: Vec3 = Vec3::new(0.9, 0.45, 0.45);
