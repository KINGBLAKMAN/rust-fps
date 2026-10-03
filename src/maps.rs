//! The three maps: Shipping Yard, Central Park and The Neighborhood. Each is
//! built from boxes (which double as colliders) plus decoration, along with
//! the spots for spawns, the mystery box, perk machines and extraction.

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashMap;
use std::f32::consts::FRAC_PI_4;

use crate::data::Perk;
use crate::{Collider, InGameEntity};

pub const MAP_NAMES: [&str; 3] = ["Shipping Yard", "Central Park", "The Neighborhood"];

pub fn map_name(id: u8) -> &'static str {
    MAP_NAMES[(id as usize).min(MAP_NAMES.len() - 1)]
}

pub struct Solid {
    pub pos: Vec3,
    pub size: Vec3,
    pub color: Color,
}

#[allow(dead_code)]
pub enum Shape {
    Box(Vec3),
    Ball(f32),
    Cylinder(f32, f32),
}

/// Non-colliding decoration.
pub struct Decor {
    pub shape: Shape,
    pub pos: Vec3,
    pub rot: Quat,
    pub color: Color,
    pub glow: f32,
}

pub struct MapLayout {
    pub half: f32,
    pub ground: Color,
    pub sky: Color,
    pub sun: f32,
    pub solids: Vec<Solid>,
    pub decor: Vec<Decor>,
    pub player_spawns: Vec<Vec3>,
    pub enemy_spawns: Vec<Vec3>,
    pub box_spots: [Vec3; 5],
    pub perk_spots: [Vec3; 5],
    pub extraction: Vec3,
}

impl MapLayout {
    fn new(half: f32, ground: Color, sky: Color, sun: f32) -> Self {
        let mut m = Self {
            half,
            ground,
            sky,
            sun,
            solids: Vec::new(),
            decor: Vec::new(),
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

    fn solid(&mut self, pos: Vec3, size: Vec3, color: Color) {
        self.solids.push(Solid { pos, size, color });
    }

    /// A box sitting on the ground (pos.y is ignored).
    fn block(&mut self, x: f32, z: f32, size: Vec3, color: Color) {
        self.solid(Vec3::new(x, size.y / 2.0, z), size, color);
    }

    fn decor(&mut self, shape: Shape, pos: Vec3, color: Color) {
        self.decor.push(Decor {
            shape,
            pos,
            rot: Quat::IDENTITY,
            color,
            glow: 0.0,
        });
    }

    fn walls(&mut self, height: f32, color: Color) {
        let h = self.half;
        let len = h * 2.0 + 2.0;
        self.solid(Vec3::new(0.0, height / 2.0, -h - 0.5), Vec3::new(len, height, 1.0), color);
        self.solid(Vec3::new(0.0, height / 2.0, h + 0.5), Vec3::new(len, height, 1.0), color);
        self.solid(Vec3::new(-h - 0.5, height / 2.0, 0.0), Vec3::new(1.0, height, len), color);
        self.solid(Vec3::new(h + 0.5, height / 2.0, 0.0), Vec3::new(1.0, height, len), color);
    }

    fn tree(&mut self, x: f32, z: f32, size: f32) {
        let trunk = Color::srgb(0.36, 0.24, 0.14);
        let leaves = Color::srgb(0.16, 0.42 + size * 0.03, 0.15);
        self.block(x, z, Vec3::new(0.5, 3.0 * size, 0.5), trunk);
        self.decor(Shape::Ball(1.8 * size), Vec3::new(x, 3.6 * size, z), leaves);
        self.decor(
            Shape::Ball(1.3 * size),
            Vec3::new(x + 0.6, 4.6 * size, z - 0.3),
            leaves,
        );
    }

    fn lamp(&mut self, x: f32, z: f32) {
        self.block(x, z, Vec3::new(0.18, 4.0, 0.18), Color::srgb(0.15, 0.15, 0.17));
        self.decor.push(Decor {
            shape: Shape::Ball(0.25),
            pos: Vec3::new(x, 4.1, z),
            rot: Quat::IDENTITY,
            color: Color::srgb(1.0, 0.9, 0.6),
            glow: 4.0,
        });
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
        Color::srgb(0.26, 0.27, 0.28),
        Color::srgb(0.62, 0.68, 0.75),
        8000.0,
    );
    m.walls(4.5, Color::srgb(0.45, 0.47, 0.5));
    let mut rng = StdRng::seed_from_u64(7);
    let colors = [
        Color::srgb(0.6, 0.18, 0.12),
        Color::srgb(0.12, 0.3, 0.6),
        Color::srgb(0.15, 0.45, 0.25),
        Color::srgb(0.85, 0.45, 0.1),
        Color::srgb(0.5, 0.5, 0.52),
        Color::srgb(0.75, 0.65, 0.15),
    ];
    let container = Vec3::new(6.0, 2.6, 2.4);

    // Rows of containers running east-west, with gaps to run through.
    for z in [-28.0, -17.0, 17.0, 28.0] {
        let mut x = -31.0;
        while x <= 31.0 {
            if rng.gen_bool(0.78) {
                let c = colors[rng.gen_range(0..colors.len())];
                m.block(x, z, container, c);
                if rng.gen_bool(0.35) {
                    let c2 = colors[rng.gen_range(0..colors.len())];
                    m.solid(Vec3::new(x, 2.6 + 1.3, z), container, c2);
                }
            }
            x += 8.5;
        }
    }
    // Containers running north-south on the sides of the central yard.
    for x in [-22.0, 22.0] {
        for z in [-6.0, 5.0] {
            let c = colors[rng.gen_range(0..colors.len())];
            m.block(x, z, Vec3::new(2.4, 2.6, 6.0), c);
        }
    }
    // Loose cover in the middle.
    for (x, z) in [(-8.0, -6.0), (9.0, 4.0), (-3.0, 9.0)] {
        m.block(x, z, Vec3::new(2.4, 2.6, 2.4), colors[rng.gen_range(0..colors.len())]);
    }
    m.block(4.0, -7.0, Vec3::new(4.0, 1.2, 1.2), Color::srgb(0.55, 0.42, 0.25));

    // Two gantry cranes over the yard.
    let crane = Color::srgb(0.9, 0.7, 0.1);
    for x in [-12.0, 12.0] {
        for z in [-11.0, 11.0] {
            m.block(x, z, Vec3::new(0.9, 14.0, 0.9), crane);
        }
        m.solid(Vec3::new(x, 14.5, 0.0), Vec3::new(1.2, 1.2, 23.0), crane);
    }
    m.solid(Vec3::new(0.0, 14.5, -11.0), Vec3::new(24.0, 1.0, 1.0), crane);

    // Site office.
    m.block(-30.0, 8.0, Vec3::new(8.0, 3.2, 4.0), Color::srgb(0.85, 0.85, 0.8));
    for (x, z) in [(-34.0, -34.0), (34.0, -34.0), (-34.0, 34.0), (34.0, 34.0)] {
        m.lamp(x * 0.9, z * 0.9);
    }

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
    m
}

fn central_park() -> MapLayout {
    let mut m = MapLayout::new(
        40.0,
        Color::srgb(0.28, 0.5, 0.22),
        Color::srgb(0.5, 0.7, 0.95),
        10000.0,
    );
    m.walls(3.0, Color::srgb(0.5, 0.48, 0.44));
    let path = Color::srgb(0.68, 0.62, 0.5);
    // Paths (flat decoration).
    m.decor(Shape::Box(Vec3::new(80.0, 0.02, 4.0)), Vec3::new(0.0, 0.01, 0.0), path);
    m.decor(Shape::Box(Vec3::new(4.0, 0.02, 80.0)), Vec3::new(0.0, 0.01, 0.0), path);
    m.decor(
        Shape::Box(Vec3::new(3.0, 0.02, 60.0)),
        Vec3::new(0.0, 0.012, 0.0),
        path,
    );
    m.decor.last_mut().unwrap().rot = Quat::from_rotation_y(FRAC_PI_4);

    // Fountain in the middle.
    let stone = Color::srgb(0.62, 0.62, 0.6);
    for (x, z, sx, sz) in [
        (0.0, -3.5, 7.0, 0.6),
        (0.0, 3.5, 7.0, 0.6),
        (-3.5, 0.0, 0.6, 6.4),
        (3.5, 0.0, 0.6, 6.4),
    ] {
        m.block(x, z, Vec3::new(sx, 0.7, sz), stone);
    }
    m.block(0.0, 0.0, Vec3::new(1.2, 2.6, 1.2), stone);
    m.decor(
        Shape::Box(Vec3::new(6.4, 0.05, 6.4)),
        Vec3::new(0.0, 0.4, 0.0),
        Color::srgb(0.25, 0.5, 0.8),
    );

    // Pond with a stone rim (gaps let you walk in).
    let water = Color::srgb(0.2, 0.42, 0.7);
    m.decor(
        Shape::Box(Vec3::new(14.0, 0.03, 10.0)),
        Vec3::new(24.0, 0.02, -20.0),
        water,
    );
    for (x, z, sx, sz) in [
        (20.0, -25.3, 7.0, 0.5),
        (29.0, -25.3, 3.5, 0.5),
        (24.0, -14.7, 14.0, 0.5),
        (16.8, -20.0, 0.5, 6.0),
        (31.2, -20.0, 0.5, 10.6),
    ] {
        m.block(x, z, Vec3::new(sx, 0.5, sz), stone);
    }

    // Gazebo.
    let wood = Color::srgb(0.55, 0.38, 0.22);
    for (x, z) in [(-25.0, 18.0), (-19.0, 18.0), (-25.0, 24.0), (-19.0, 24.0)] {
        m.block(x, z, Vec3::new(0.4, 3.2, 0.4), wood);
    }
    m.solid(
        Vec3::new(-22.0, 3.4, 21.0),
        Vec3::new(7.4, 0.4, 7.4),
        Color::srgb(0.5, 0.2, 0.15),
    );

    // Trees, hedges, benches, rocks and lamps.
    let mut rng = StdRng::seed_from_u64(11);
    let mut placed = 0;
    while placed < 34 {
        let x = rng.gen_range(-36.0..36.0f32);
        let z = rng.gen_range(-36.0..36.0f32);
        let on_path = x.abs() < 4.0 || z.abs() < 4.0 || (x - z).abs() < 3.5;
        let busy = (x.abs() < 9.0 && z.abs() < 9.0)
            || (x > 14.0 && x < 34.0 && z > -28.0 && z < -12.0)
            || (x > -28.0 && x < -16.0 && z > 15.0 && z < 27.0)
            || m.enemy_spawns.iter().any(|s| s.distance(Vec3::new(x, 0.0, z)) < 5.0)
            || near_spot(x, z);
        if on_path || busy {
            continue;
        }
        m.tree(x, z, rng.gen_range(0.85..1.25));
        placed += 1;
    }
    let hedge = Color::srgb(0.13, 0.33, 0.12);
    for (x, z, sx, sz) in [
        (-14.0, -10.0, 8.0, 1.0),
        (14.0, 10.0, 8.0, 1.0),
        (-10.0, 14.0, 1.0, 6.0),
        (10.0, -14.0, 1.0, 6.0),
        (-30.0, -6.0, 6.0, 1.0),
    ] {
        m.block(x, z, Vec3::new(sx, 1.3, sz), hedge);
    }
    for (x, z, rot) in [(6.0, 3.0, false), (-6.0, -3.0, false), (3.0, 10.0, true), (-3.0, -10.0, true)] {
        let size = if rot {
            Vec3::new(0.6, 0.5, 2.0)
        } else {
            Vec3::new(2.0, 0.5, 0.6)
        };
        m.block(x, z, size, wood);
    }
    let rock = Color::srgb(0.45, 0.45, 0.47);
    for (x, z) in [(-28.0, -26.0), (26.0, 24.0), (-8.0, -30.0)] {
        m.block(x, z, Vec3::new(3.0, 1.6, 2.5), rock);
    }
    for (x, z) in [(5.0, 5.0), (-5.0, -5.0), (5.0, -5.0), (-5.0, 5.0)] {
        m.lamp(x * 1.5, z * 1.5);
    }

    m.player_spawns = spawn_line(Vec3::new(0.0, 0.0, 12.0));
    m.box_spots = BOX_SPOTS_OPEN;
    m.perk_spots = PERK_SPOTS_OPEN;
    m.extraction = Vec3::new(-22.0, 0.0, 21.0);
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

fn near_spot(x: f32, z: f32) -> bool {
    let p = Vec3::new(x, 0.0, z);
    BOX_SPOTS_OPEN
        .iter()
        .chain(PERK_SPOTS_OPEN.iter())
        .any(|s| s.distance(p) < 4.0)
        || p.distance(Vec3::new(0.0, 0.0, 12.0)) < 5.0
}

fn neighborhood() -> MapLayout {
    let mut m = MapLayout::new(
        40.0,
        Color::srgb(0.3, 0.48, 0.24),
        Color::srgb(0.95, 0.68, 0.5),
        7000.0,
    );
    m.walls(3.0, Color::srgb(0.55, 0.42, 0.3));
    // Street and sidewalks.
    m.decor(
        Shape::Box(Vec3::new(80.0, 0.02, 8.0)),
        Vec3::new(0.0, 0.01, 0.0),
        Color::srgb(0.18, 0.18, 0.2),
    );
    for z in [-5.0, 5.0] {
        m.decor(
            Shape::Box(Vec3::new(80.0, 0.06, 2.0)),
            Vec3::new(0.0, 0.03, z),
            Color::srgb(0.65, 0.65, 0.63),
        );
    }
    for i in -8..=8 {
        m.decor(
            Shape::Box(Vec3::new(2.0, 0.025, 0.2)),
            Vec3::new(i as f32 * 4.5, 0.02, 0.0),
            Color::srgb(0.95, 0.85, 0.3),
        );
    }

    // Houses on both sides of the street, with yards in front.
    let walls = [
        Color::srgb(0.85, 0.8, 0.7),
        Color::srgb(0.6, 0.72, 0.85),
        Color::srgb(0.9, 0.75, 0.6),
        Color::srgb(0.75, 0.85, 0.7),
        Color::srgb(0.8, 0.65, 0.65),
    ];
    let roof = Color::srgb(0.35, 0.2, 0.17);
    let mut i = 0;
    for side in [-1.0f32, 1.0] {
        for x in [-28.0, -14.0, 0.0, 14.0, 28.0] {
            let z = side * 17.0;
            let color = walls[i % walls.len()];
            i += 1;
            m.block(x, z, Vec3::new(9.0, 4.0, 8.0), color);
            // Pitched roof from two tilted slabs.
            for s in [-1.0f32, 1.0] {
                m.decor.push(Decor {
                    shape: Shape::Box(Vec3::new(9.6, 0.25, 5.2)),
                    pos: Vec3::new(x, 5.1, z + s * 2.05),
                    rot: Quat::from_rotation_x(s * 0.6),
                    color: roof,
                    glow: 0.0,
                });
            }
            // Door and windows facing the street.
            let front = z - side * 4.02;
            m.decor(
                Shape::Box(Vec3::new(1.2, 2.2, 0.05)),
                Vec3::new(x, 1.1, front),
                Color::srgb(0.35, 0.22, 0.15),
            );
            for wx in [-2.8, 2.8] {
                m.decor.push(Decor {
                    shape: Shape::Box(Vec3::new(1.6, 1.2, 0.05)),
                    pos: Vec3::new(x + wx, 2.2, front),
                    rot: Quat::IDENTITY,
                    color: Color::srgb(0.95, 0.85, 0.55),
                    glow: 0.6,
                });
            }
            // Low fence along the front yard, with a gate gap.
            let fz = side * 9.5;
            let picket = Color::srgb(0.95, 0.95, 0.92);
            m.block(x - 3.2, fz, Vec3::new(3.6, 0.9, 0.15), picket);
            m.block(x + 3.2, fz, Vec3::new(3.6, 0.9, 0.15), picket);
        }
    }
    // Parked cars.
    let car_colors = [
        Color::srgb(0.7, 0.1, 0.1),
        Color::srgb(0.1, 0.2, 0.6),
        Color::srgb(0.85, 0.85, 0.85),
        Color::srgb(0.15, 0.15, 0.15),
    ];
    for (k, (x, z)) in [(-22.0, -2.6), (-6.0, 2.6), (10.0, -2.6), (24.0, 2.6)]
        .into_iter()
        .enumerate()
    {
        let c = car_colors[k % car_colors.len()];
        m.block(x, z, Vec3::new(4.2, 0.9, 1.9), c);
        m.solid(Vec3::new(x - 0.3, 1.25, z), Vec3::new(2.3, 0.7, 1.7), c);
    }
    // Back-yard trees and sheds.
    for side in [-1.0f32, 1.0] {
        for x in [-21.0, -7.0, 7.0, 21.0] {
            m.tree(x, side * 30.0, 1.0);
        }
        m.block(-35.0, side * 26.0, Vec3::new(3.0, 2.4, 3.0), Color::srgb(0.5, 0.4, 0.3));
    }
    for x in [-30.0, -10.0, 10.0, 30.0] {
        m.lamp(x, 6.3);
        m.lamp(x + 10.0, -6.3);
    }

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

pub fn spawn_map(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    map: u8,
) -> MapLayout {
    let layout = layout(map);
    let mut cache: HashMap<[u32; 4], Handle<StandardMaterial>> = HashMap::new();
    let mut mat = |color: Color, glow: f32, materials: &mut Assets<StandardMaterial>| {
        let c = color.to_srgba();
        let key = [
            (c.red * 255.0) as u32,
            (c.green * 255.0) as u32,
            (c.blue * 255.0) as u32,
            (glow * 10.0) as u32,
        ];
        cache
            .entry(key)
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: color,
                    perceptual_roughness: 0.85,
                    emissive: LinearRgba::from(color) * glow,
                    ..default()
                })
            })
            .clone()
    };

    let size = layout.half * 2.0 + 2.0;
    commands.spawn((
        InGameEntity,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(size, size))),
        MeshMaterial3d(mat(layout.ground, 0.0, materials)),
    ));

    for s in &layout.solids {
        commands.spawn((
            InGameEntity,
            Mesh3d(meshes.add(Cuboid::from_size(s.size))),
            MeshMaterial3d(mat(s.color, 0.0, materials)),
            Transform::from_translation(s.pos),
            Collider { half: s.size / 2.0 },
        ));
    }
    for d in &layout.decor {
        let mesh = match d.shape {
            Shape::Box(size) => meshes.add(Cuboid::from_size(size)),
            Shape::Ball(r) => meshes.add(Sphere::new(r).mesh().ico(2).unwrap()),
            Shape::Cylinder(r, h) => meshes.add(Cylinder::new(r, h)),
        };
        commands.spawn((
            InGameEntity,
            Mesh3d(mesh),
            MeshMaterial3d(mat(d.color, d.glow, materials)),
            Transform::from_translation(d.pos).with_rotation(d.rot),
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

    // Perk machines: tall coloured cabinets with a glowing top.
    for (spot, perk) in layout.perk_spots.iter().zip(Perk::ALL) {
        let body = Vec3::new(1.2, 2.4, 1.0);
        commands
            .spawn((
                InGameEntity,
                PerkMachine(perk),
                Mesh3d(meshes.add(Cuboid::from_size(body))),
                MeshMaterial3d(mat(perk.color().darker(0.25), 0.0, materials)),
                Transform::from_translation(*spot + Vec3::Y * 1.2),
                Collider { half: body / 2.0 },
            ))
            .with_children(|p| {
                p.spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.25, 0.35, 1.05))),
                    MeshMaterial3d(mat(perk.color(), 3.0, materials)),
                    Transform::from_xyz(0.0, 1.05, 0.0),
                ));
                p.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.8, 0.8, 0.05))),
                    MeshMaterial3d(mat(Color::srgb(0.95, 0.95, 0.9), 1.0, materials)),
                    Transform::from_xyz(0.0, 0.2, 0.53),
                ));
                p.spawn((
                    PointLight {
                        intensity: 40_000.0,
                        color: perk.color(),
                        range: 6.0,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 1.6, 0.8),
                ));
            });
    }

    // Mystery box (moved between its spots during the match).
    let wood = mat(Color::srgb(0.4, 0.26, 0.14), 0.0, materials);
    commands
        .spawn((
            InGameEntity,
            MysteryBox,
            Transform::from_translation(layout.box_spots[0] + Vec3::Y * BOX_HALF.y),
            Visibility::default(),
            Collider { half: BOX_HALF },
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(meshes.add(Cuboid::from_size(BOX_HALF * 2.0))),
                MeshMaterial3d(wood.clone()),
            ));
            p.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.9, 0.12, 1.0))),
                MeshMaterial3d(mat(Color::srgb(0.25, 0.6, 1.0), 2.0, materials)),
                Transform::from_xyz(0.0, 0.5, 0.0),
            ));
            p.spawn((
                BoxGlow,
                PointLight {
                    intensity: 60_000.0,
                    color: Color::srgb(0.4, 0.7, 1.0),
                    range: 8.0,
                    ..default()
                },
                Transform::from_xyz(0.0, 1.55, 0.0),
            ));
        });

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
        });

    layout
}

pub const EXTRACT_RADIUS: f32 = 4.0;
pub const BOX_HALF: Vec3 = Vec3::new(0.9, 0.45, 0.45);
