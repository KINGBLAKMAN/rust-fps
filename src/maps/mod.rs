//! The three maps: Shipping Yard, Central Park and The Neighborhood. Each is
//! a floor plan of rooms, corridors and yards (see interiors.rs) behind doors
//! you buy open, furnished with modelled props (props.rs) merged into a few
//! meshes, with simple invisible boxes to collide with, a textured ground,
//! lamps, and the spots for spawns, the mystery box, perk machines and
//! extraction.

pub mod interiors;
pub mod nav;
pub mod props;
pub mod strips;

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::data::Perk;
use crate::kit::{c, glow_material, scale_uvs, vertex_material, Kit};
use interiors::{look, Finish, Floor, Plan, Room};
use props::{boxr, hash, shade, v, Art};
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

/// Half size of the whole map.
pub const OUTER: f32 = 58.0;

/// A wide doorway between areas. (These were doors bought open before v9;
/// now every area is open from the start.)
#[derive(Clone)]
pub struct DoorDef {
    pub pos: Vec3,
    /// The doorway runs along X (in a wall along X) or along Z.
    pub along_x: bool,
}

pub const DOOR_WIDTH: f32 = 4.0;

/// An ammo cache: a chalk board on a wall with a gun drawn on it.
#[derive(Clone)]
pub struct WallBuy {
    pub pos: Vec3,
    /// Which way the board faces (players stand on that side).
    pub yaw: f32,
    pub gun: u8,
    pub attach: crate::data::Attach,
}

impl WallBuy {
    /// Where you stand to buy it.
    pub fn stand(&self) -> Vec3 {
        self.pos + Quat::from_rotation_y(self.yaw) * Vec3::new(0.0, 0.0, 1.0)
    }

    pub fn near(&self, feet: Vec3) -> bool {
        feet.with_y(0.0).distance(self.stand()) < 2.2
    }
}

pub struct MapLayout {
    pub half: f32,
    pub ground: Color,
    pub ground_kind: Ground,
    pub sky: Color,
    pub sun: f32,
    pub solids: Vec<Solid>,
    pub art: Art,
    /// Small props (everything placed that fits in a 4.5 m box): drawn
    /// with outlines up close.
    pub prop_art: Art,
    /// Lamps: position, colour, brightness.
    pub lights: Vec<(Vec3, Color, f32)>,
    pub player_spawns: Vec<Vec3>,
    /// Enemy spawn points and the area each is in (only open areas spawn).
    pub enemy_spawns: Vec<(Vec3, u8)>,
    pub box_spots: [Vec3; 5],
    pub perk_spots: [Vec3; 5],
    /// Which way each perk machine faces.
    pub perk_yaws: [f32; 5],
    pub extraction: Vec3,
    pub doors: Vec<DoorDef>,
    pub wall_buys: Vec<WallBuy>,
}

impl MapLayout {
    fn new(ground: Color, ground_kind: Ground, sky: Color, sun: f32) -> Self {
        Self {
            half: OUTER,
            ground,
            ground_kind,
            sky,
            sun,
            solids: Vec::new(),
            art: Art::default(),
            prop_art: Art::default(),
            lights: Vec::new(),
            player_spawns: Vec::new(),
            enemy_spawns: Vec::new(),
            box_spots: [Vec3::ZERO; 5],
            perk_spots: [Vec3::ZERO; 5],
            perk_yaws: [0.0; 5],
            extraction: Vec3::ZERO,
            doors: Vec::new(),
            wall_buys: Vec::new(),
        }
    }

    /// Enemy spawn points in area `zone`.
    fn spawns(&mut self, zone: u8, at: &[(f32, f32)]) {
        for (x, z) in at {
            self.enemy_spawns.push((v(*x, 0.0, *z), zone));
        }
    }

    /// Flat paint on the ground (roads, paths, markings).
    pub(crate) fn ground_paint(
        &mut self,
        center: Vec3,
        size: Vec2,
        yaw: f32,
        color: Color,
        lift: f32,
    ) {
        self.art.paint.cuboid_rot(
            Vec3::new(center.x, lift, center.z),
            Vec3::new(size.x, 0.02, size.y),
            Quat::from_rotation_y(yaw),
            color,
        );
    }

}

pub fn layout(map: u8) -> MapLayout {
    match map {
        1 => central_park(),
        2 => neighborhood(),
        _ => shipping_yard(),
    }
}

// ---------------------------------------------------------------------------
// Shipping Yard
// ---------------------------------------------------------------------------

/// The port. You start in the customs hall with its gate yard. West are the
/// container stacks, east the warehouse and cold store, north the port
/// office (lockers, break room, harbour master, radio room, lobby) with the
/// crane yard behind it. Beyond those: the rail yard, the machine shop and
/// foundry, the truck depot, and the dockside where the ship waits.
fn shipping_yard() -> MapLayout {
    let mut m = MapLayout::new(
        Color::srgb(0.5, 0.5, 0.5),
        Ground::Asphalt,
        Color::srgb(0.62, 0.68, 0.75),
        8000.0,
    );
    m.player_spawns = spawn_line(v(0.0, 0.0, -1.0));
    m.extraction = v(0.0, 0.0, -50.0);
    m.box_spots = [
        v(-5.5, 0.0, -14.6),
        v(-23.0, 0.0, -13.0),
        v(29.5, 0.0, -0.5),
        v(0.0, 0.0, -32.0),
        v(-30.0, 0.0, 52.0),
    ];
    m.perk(0, 47.0, 18.6, PI);
    m.perk(1, -32.0, 18.6, PI);
    m.perk(2, 47.0, -42.5, 0.0);
    m.perk(3, -8.9, 23.0, FRAC_PI_2);
    m.perk(4, 56.0, 30.0, -FRAC_PI_2);
    m.boundary(0, 4.5);

    let hall = look(Finish::Block, c(0.8, 0.77, 0.68), c(0.22, 0.3, 0.38));
    let fence = look(Finish::Fence, c(0.55, 0.55, 0.53), c(0.6, 0.6, 0.62));
    let shed = look(Finish::Metal, c(0.42, 0.5, 0.58), c(0.85, 0.6, 0.1));
    let cold = look(Finish::Metal, c(0.75, 0.78, 0.8), c(0.2, 0.45, 0.7));
    let office = look(Finish::Plaster, c(0.86, 0.86, 0.8), c(0.3, 0.45, 0.42));
    let brick = look(Finish::Brick, c(0.55, 0.27, 0.2), c(0.72, 0.7, 0.64));
    let garage = look(Finish::Metal, c(0.72, 0.58, 0.25), c(0.25, 0.25, 0.27));

    // Doors: from the start, then between the wings.
    m.door_z(-10.0, 2.0);
    m.door_z(10.0, 2.0);
    m.door_x(0.0, -16.0);
    m.door_z(-10.0, 14.0);
    m.door_z(10.0, 14.0);
    m.door_x(16.0, -16.0);
    m.door_x(-24.0, 20.0);
    m.door_z(-10.0, 36.0);
    m.door_x(30.0, 20.0);
    m.door_z(10.0, 36.0);
    m.door_z(-36.0, 2.0);
    m.door_z(-36.0, -40.0);
    m.door_x(-47.0, 20.0);

    let mut p = Plan::default();
    // Customs hall and its gate yard (the start).
    p.wx(8.0, -10.0, 10.0, 5.0, hall);
    p.win_x(-6.0, 8.0, 3.0);
    p.win_x(6.0, 8.0, 3.0);
    p.wx(-4.0, -10.0, 10.0, 5.0, hall);
    p.open_x(-6.0, -4.0, 4.0);
    p.open_x(6.0, -4.0, 4.0);
    p.win_x(0.0, -4.0, 3.0);
    p.wz(-10.0, -4.0, 8.0, 7.0, hall);
    p.wz(10.0, -4.0, 8.0, 7.0, hall);
    p.win_z(-10.0, 6.0, 2.0);
    p.wz(-10.0, -16.0, -4.0, 4.5, fence);
    p.wz(10.0, -16.0, -4.0, 7.0, shed);
    p.wx(-16.0, -36.0, 10.0, 4.5, fence);
    // Container stacks.
    p.wz(-36.0, -58.0, 20.0, 4.5, fence);
    p.wx(20.0, -58.0, -10.0, 4.5, fence);
    // Warehouse, racking hall and cold store.
    p.wx(-16.0, 10.0, 57.8, 7.0, shed);
    p.wx(20.0, 10.0, 57.8, 7.0, shed);
    p.wz(23.0, -16.0, 20.0, 7.0, shed);
    p.open_z(23.0, -9.0, 4.5);
    p.open_z(23.0, 12.0, 4.5);
    p.wz(36.0, -16.0, 20.0, 7.0, cold);
    p.open_z(36.0, -7.0, 4.0);
    p.open_z(36.0, 11.0, 4.0);
    p.wx(2.0, 36.0, 57.8, 7.0, cold);
    p.open_x(50.0, 2.0, 4.0);
    p.wz(57.8, -16.0, 40.0, 7.0, cold);
    // Port office: corridor, four rooms and the lobby.
    p.wz(-10.0, 8.0, 40.0, 7.0, office);
    p.wz(10.0, 8.0, 40.0, 7.0, office);
    for x in [-2.0f32, 2.0] {
        p.wz(x, 8.0, 32.0, 4.5, office);
        p.open_z(x, 14.0, 3.0);
        p.open_z(x, 26.0, 3.0);
    }
    for (x0, x1) in [(-10.0f32, -2.0f32), (2.0, 10.0)] {
        p.wx(20.0, x0, x1, 4.5, office);
        p.wx(32.0, x0, x1, 4.5, office);
    }
    p.wx(40.0, -10.0, 10.0, 4.5, office);
    p.open_x(0.0, 40.0, 4.0);
    p.wz(-10.0, 40.0, 58.0, 4.5, fence);
    p.wz(10.0, 40.0, 58.0, 4.5, fence);
    // Machine shop and foundry, scrap yard behind.
    p.wz(34.0, 20.0, 40.0, 6.0, brick);
    p.open_z(34.0, 25.0, 4.0);
    p.open_z(34.0, 35.0, 4.0);
    p.wx(40.0, 10.0, 57.8, 6.0, brick);
    p.open_x(20.0, 40.0, 4.0);
    p.open_x(48.0, 40.0, 4.0);
    // Signal box in the rail yard.
    p.wx(50.0, -57.8, -46.0, 4.0, brick);
    p.open_x(-51.0, 50.0, 3.0);
    p.wz(-46.0, 50.0, 57.8, 4.0, brick);
    p.wz(-57.8, 50.0, 57.8, 4.0, brick);
    p.wx(57.8, -57.8, -46.0, 4.0, brick);
    // Truck depot fences and the mechanic's garage.
    p.wx(8.0, -57.8, -44.0, 5.0, garage);
    p.wx(-8.0, -57.8, -44.0, 5.0, garage);
    p.wz(-44.0, -8.0, 8.0, 5.0, garage);
    p.open_z(-44.0, -4.0, 3.5);
    p.open_z(-44.0, 4.0, 3.5);
    p.wz(-57.8, -8.0, 8.0, 5.0, garage);
    // Dockside: inspection shed and pump house.
    p.wx(-28.0, -6.0, 6.0, 4.0, hall);
    p.wx(-36.0, -6.0, 6.0, 4.0, hall);
    p.win_x(-3.0, -28.0, 2.5);
    p.win_x(3.0, -36.0, 2.5);
    p.wz(-6.0, -36.0, -28.0, 4.0, hall);
    p.wz(6.0, -36.0, -28.0, 4.0, hall);
    p.open_z(-6.0, -32.0, 3.5);
    p.open_z(6.0, -32.0, 3.5);
    p.wx(-30.0, 40.0, 54.0, 4.5, brick);
    p.wx(-44.0, 40.0, 54.0, 4.5, brick);
    p.open_x(47.0, -30.0, 3.5);
    p.wz(40.0, -44.0, -30.0, 4.5, brick);
    p.wz(54.0, -44.0, -30.0, 4.5, brick);
    p.open_z(40.0, -38.0, 3.0);
    m.build(&p);

    // Roofs, floors and lamps.
    let roof = c(0.32, 0.34, 0.36);
    m.roof(-10.0, -4.0, 10.0, 8.0, 5.0, roof);
    m.roof(10.0, -16.0, 57.8, 20.0, 7.0, c(0.38, 0.42, 0.46));
    m.roof(-10.0, 8.0, 10.0, 40.0, 4.5, roof);
    m.roof(10.0, 20.0, 57.8, 40.0, 6.0, c(0.4, 0.25, 0.2));
    m.roof(-57.8, 50.0, -46.0, 57.8, 4.0, c(0.3, 0.22, 0.18));
    m.roof(-57.8, -8.0, -44.0, 8.0, 5.0, roof);
    m.roof(-6.0, -36.0, 6.0, -28.0, 4.0, roof);
    m.roof(40.0, -44.0, 54.0, -30.0, 4.5, c(0.4, 0.25, 0.2));
    let warm = c(1.0, 0.93, 0.8);
    let cool = c(0.85, 0.93, 1.0);
    m.floor(-10.0, -4.0, 10.0, 8.0, Floor::Tiles(c(0.7, 0.68, 0.62)));
    m.floor(10.0, -16.0, 36.0, 20.0, Floor::Concrete(c(0.5, 0.5, 0.48)));
    m.floor(36.0, -16.0, 57.8, 20.0, Floor::Tiles(c(0.78, 0.82, 0.84)));
    m.floor(-10.0, 8.0, 10.0, 32.0, Floor::Tiles(c(0.62, 0.66, 0.6)));
    m.floor(-10.0, 32.0, 10.0, 40.0, Floor::Carpet(c(0.3, 0.38, 0.55)));
    m.floor(10.0, 20.0, 57.8, 40.0, Floor::Concrete(c(0.42, 0.42, 0.4)));
    m.floor(-57.8, -8.0, -44.0, 8.0, Floor::Concrete(c(0.45, 0.45, 0.44)));
    m.floor(-6.0, -36.0, 6.0, -28.0, Floor::Concrete(c(0.55, 0.55, 0.52)));
    m.floor(40.0, -44.0, 54.0, -30.0, Floor::Concrete(c(0.45, 0.45, 0.44)));
    m.floor(-57.8, 50.0, -46.0, 57.8, Floor::Boards(c(0.45, 0.32, 0.2)));
    for (x, z) in [(-5.0f32, 2.0f32), (5.0, 2.0)] {
        m.lamp(x, z, 5.0, warm, 70_000.0);
    }
    for (x, z) in [(16.5f32, -6.0f32), (16.5, 10.0), (29.5, -8.0), (29.5, 10.0)] {
        m.lamp(x, z, 7.0, cool, 90_000.0);
    }
    for (x, z) in [(47.0f32, -7.0f32), (47.0, 11.0)] {
        m.lamp(x, z, 7.0, c(0.7, 0.85, 1.0), 80_000.0);
    }
    for (x, z) in [(-6.0f32, 14.0f32), (6.0, 14.0), (-6.0, 26.0), (6.0, 26.0), (0.0, 36.0), (0.0, 20.0)] {
        m.lamp(x, z, 4.5, warm, 45_000.0);
    }
    for (x, z) in [(22.0f32, 30.0f32), (46.0, 30.0)] {
        m.lamp(x, z, 6.0, c(1.0, 0.85, 0.65), 80_000.0);
    }
    m.lamp(-51.0, 4.0, 5.0, cool, 50_000.0);
    m.lamp(0.0, -32.0, 4.0, cool, 40_000.0);
    m.lamp(47.0, -37.0, 4.5, warm, 40_000.0);
    m.lamp(-52.0, 54.0, 4.0, warm, 30_000.0);

    // Ammo caches.
    m.wall_gun(0.0, 8.0, PI, 3);
    m.wall_gun(-10.0, -1.8, -FRAC_PI_2, 10);
    m.wall_gun(23.0, 2.0, -FRAC_PI_2, 6);
    m.wall_gun(-2.0, 20.0, FRAC_PI_2, 2);
    m.wall_gun(26.0, -16.0, PI, 17);
    m.wall_gun(-46.0, 53.5, FRAC_PI_2, 13);
    m.wall_gun(34.0, 30.0, -FRAC_PI_2, 9);
    m.wall_gun(-51.0, 8.0, PI, 11);

    m.spawns(0, &[(-8.5, 6.5), (8.5, 6.5), (8.5, -14.0), (-8.5, -10.0)]);
    m.spawns(1, &[(-33.0, -13.0), (-14.0, -13.0), (-23.0, 17.0), (-33.0, 6.0)]);
    m.spawns(2, &[(34.5, -14.5), (34.5, 18.5), (55.0, -13.0), (40.0, 17.0), (13.0, 17.0)]);
    m.spawns(3, &[(7.0, 30.0), (-7.0, 34.5), (5.0, 55.0), (-8.0, 50.0)]);
    m.spawns(4, &[(-33.0, -55.0), (54.0, -52.0), (27.0, -53.0), (55.0, -20.0), (-20.0, -40.0)]);
    m.spawns(5, &[(-55.0, 23.0), (-55.0, 48.0), (-14.0, 55.0), (-30.0, 56.0), (-14.0, 24.0)]);
    m.spawns(6, &[(54.0, 54.0), (14.0, 55.0), (55.0, 23.0), (36.0, 55.0), (14.0, 23.0)]);
    m.spawns(7, &[(-55.0, -55.0), (-38.5, -56.0), (-55.0, 17.0), (-40.0, 17.0), (-50.0, -14.0)]);

    let colors = [
        c(0.6, 0.18, 0.12),
        c(0.12, 0.3, 0.6),
        c(0.15, 0.45, 0.25),
        c(0.85, 0.45, 0.1),
        c(0.5, 0.5, 0.52),
        c(0.75, 0.65, 0.15),
    ];
    let yellow = c(0.9, 0.75, 0.15);

    // Customs hall: the counter you go behind for the gun, an x-ray belt,
    // benches and flags.
    m.counter(0.0, 4.0, PI, 8.0, c(0.3, 0.38, 0.5));
    m.conveyor(-6.5, 5.0, 0.0, 3.0);
    m.bench(-2.4, -2.9, 0.0);
    m.bench(2.4, -2.9, 0.0);
    m.cabinets(4.8, 7.45, PI, 4);
    let mut a = Art::default();
    for (i, x) in [-8.0f32, 8.0].into_iter().enumerate() {
        let col = [c(0.15, 0.3, 0.65), c(0.75, 0.15, 0.15)][i];
        a.metal.cyl(v(x, 3.2, -3.75), 0.03, 1.4, Quat::from_rotation_x(FRAC_PI_2), c(0.6, 0.6, 0.6));
        a.paint.cuboid(v(x, 2.6, -3.6), v(1.0, 1.4, 0.02), col);
    }
    a.glow.cuboid(v(0.0, 3.6, 7.77), v(4.0, 0.5, 0.02), c(1.0, 0.85, 0.3));
    m.place(a, Vec3::ZERO, 0.0);

    // Gate yard: guard booth, barrier arm and a floodlight.
    m.booth(6.0, -9.5, 0.0, c(0.85, 0.85, 0.82));
    m.barriers(-4.0, -9.0, 0.0, 2, c(0.72, 0.72, 0.7));
    m.light_tower(-8.6, -15.0, PI * 0.25);
    for (x, z) in [(2.0, -13.0), (-2.0, -13.0)] {
        m.traffic_cone(x, z);
    }
    for i in 0..5 {
        m.ground_paint(v(-8.0 + i as f32 * 4.0, 0.0, -10.0), Vec2::new(2.0, 0.18), 0.0, yellow, 0.014);
    }

    // Container stacks: two rows of boxes make three lanes.
    let mut seed = 0.0;
    for (x, zs) in [
        (-18.0f32, [(-12.5f32, 1), (-6.4, 2), (8.0, 1), (14.1, 2)]),
        (-28.0, [(-9.0, 1), (0.5, 2), (11.0, 1), (17.1, 0)]),
    ] {
        for (z, stack) in zs {
            for l in 0..stack {
                seed += 1.0;
                let col = colors[(hash(seed, x) * 6.0) as usize % 6];
                m.container(x + l as f32 * 0.1, z, l as f32 * 2.6, true, col, seed);
            }
        }
    }
    m.forklift(-23.0, 4.0, 0.4, c(0.95, 0.7, 0.1));
    m.pallet_stack(-33.0, -6.0, 0.2, 1.0);
    m.drums(-14.0, 15.0, 3, 2.0);
    m.crates(-31.5, 14.0, 3.0);
    m.tires(-33.5, 10.0, 4);
    m.light_tower(-34.5, -14.5, PI * 0.25);
    m.lamp_post(-13.0, -8.0, FRAC_PI_2, c(1.0, 0.85, 0.6));

    // Warehouse floor: forklift, pallets and a parcel belt; racks beyond.
    m.forklift(15.0, 6.0, 0.4, c(0.95, 0.7, 0.1));
    m.pallet_stack(20.5, -12.0, 0.0, 11.0);
    m.pallet_stack(13.0, -13.0, 0.3, 12.0);
    m.pallet_stack(20.0, 17.5, 0.1, 13.0);
    m.conveyor(16.5, -3.5, 0.0, 8.0);
    for (i, z) in [-12.5f32, -5.5, 5.0, 16.5].into_iter().enumerate() {
        m.shelving(29.5, z, 0.0, i as f32 + 3.0);
    }
    for i in 0..4 {
        m.ground_paint(v(16.5, 0.0, -14.0 + i as f32 * 8.0), Vec2::new(10.0, 0.15), 0.0, yellow, 0.025);
    }

    // Cold store: crates on pallets, hanging carcasses and frost.
    for (i, (x, z)) in [(40.0f32, -13.0f32), (54.0, -4.0), (40.0, 6.0), (54.0, 6.5)].into_iter().enumerate() {
        m.crates(x, z, 20.0 + i as f32);
    }
    let mut a = Art::default();
    for i in 0..10 {
        let x = 41.0 + (i % 5) as f32 * 3.0;
        let z = if i < 5 { -9.0 } else { 14.0 };
        a.metal.cyl(v(x, 4.0, z), 0.015, 3.0, Quat::IDENTITY, c(0.5, 0.5, 0.52));
        a.paint.blob(v(x, 2.2, z), v(0.3, 0.6, 0.22), c(0.7, 0.35, 0.33));
    }
    a.metal.beam(v(40.0, 5.5, -9.0), v(54.0, 5.5, -9.0), Vec2::splat(0.12), c(0.5, 0.5, 0.52));
    a.metal.beam(v(40.0, 5.5, 14.0), v(54.0, 5.5, 14.0), Vec2::splat(0.12), c(0.5, 0.5, 0.52));
    m.place(a, Vec3::ZERO, 0.0);

    // Port office rooms.
    m.lockers(-6.0, 19.45, PI, 10, c(0.3, 0.45, 0.6));
    m.lockers(-9.45, 10.5, FRAC_PI_2, 3, c(0.3, 0.45, 0.6));
    m.kitchen(6.0, 19.5, PI, 6.0, c(0.65, 0.55, 0.4));
    m.table_set(5.5, 12.5, 0.0, c(0.85, 0.85, 0.8));
    m.fridge(9.3, 9.4, -FRAC_PI_2, false);
    m.vending(3.0, 9.4, 0.0, c(0.75, 0.12, 0.12));
    m.desk(-5.5, 29.0, 0.0);
    m.cabinets(-6.0, 20.6, 0.0, 5);
    m.console(9.4, 24.0, -FRAC_PI_2, 4.0);
    m.desk(5.0, 30.8, PI);
    m.counter(0.0, 37.0, PI, 5.0, c(0.45, 0.32, 0.2));
    m.sofa(-7.5, 38.8, PI, c(0.3, 0.35, 0.5));
    m.sofa(7.5, 38.8, PI, c(0.3, 0.35, 0.5));
    m.bush(-9.2, 33.0, 0.6, 1.0, None);

    // Crane yard.
    m.gantry(-5.0, 50.0, 12.0, 14.0);
    m.crates(5.0, 52.0, 30.0);
    m.drums(-6.0, 45.0, 3, 31.0);

    // Machine shop and foundry.
    m.machine_shop(&Room::new(10.4, 20.4, 33.8, 39.6, 6.0, &[(10.0, 36.0), (30.0, 20.0), (34.0, 25.0), (34.0, 35.0), (20.0, 40.0)]), 3.0);
    m.generator(46.0, 23.0, 0.0);
    m.pipes(40.0, 36.0, FRAC_PI_2);
    furnace(&mut m, 46.0, 33.0);
    // Scrap yard.
    m.tires(14.0, 46.0, 5);
    m.tires(15.5, 47.0, 3);
    m.drums(30.0, 55.0, 4, 40.0);
    m.crates(40.0, 52.0, 41.0);
    m.dumpster(25.0, 56.5, 0.0, c(0.25, 0.4, 0.3));
    m.car(52.0, 50.0, 0.7, c(0.45, 0.3, 0.25));
    m.light_tower(56.0, 56.0, PI * 1.25);

    // Rail yard: tracks with boxcars across them, a ticket booth.
    let cars = [c(0.55, 0.2, 0.12), c(0.2, 0.3, 0.45), c(0.35, 0.36, 0.3), c(0.6, 0.45, 0.15)];
    for (i, (z, xs)) in [(30.0f32, [-48.0f32, -20.0]), (44.0, [-52.0, -30.0])].into_iter().enumerate() {
        m.track(-57.5, -10.5, z);
        for (j, x) in xs.into_iter().enumerate() {
            m.boxcar(x, z, cars[(i * 2 + j) % 4], x);
        }
    }
    m.booth(-16.0, 50.0, 0.0, c(0.7, 0.6, 0.3));
    m.crates(-38.0, 37.0, 50.0);
    m.lamp_post(-34.0, 24.0, 0.0, c(1.0, 0.85, 0.6));
    m.lamp_post(-20.0, 37.0, 0.0, c(1.0, 0.85, 0.6));
    m.console(-56.9, 53.5, FRAC_PI_2, 3.0);

    // Truck depot.
    m.semi(-51.0, -30.0, 0.0, c(0.75, 0.12, 0.1), c(0.9, 0.9, 0.88));
    m.semi(-42.0, -48.0, PI, c(0.15, 0.3, 0.65), c(0.8, 0.8, 0.78));
    m.fuel_pumps(-47.0, 14.0, 0.0, true);
    m.car(-51.0, 0.0, FRAC_PI_2, c(0.2, 0.45, 0.3));
    m.tires(-56.5, -6.0, 4);
    m.generator(-55.5, 5.5, FRAC_PI_2);
    m.tires(-40.0, -18.0, 3);
    m.drums(-55.0, -16.0, 3, 21.0);

    // Dockside: the quay edge, bollards, a crane over it, the inspection
    // shed and the pump house.
    let mut a = Art::default();
    a.paint.cuboid(v(0.0, 0.02, -56.5), v(116.0, 0.04, 1.5), yellow);
    m.place(a, Vec3::ZERO, 0.0);
    let mut x = -34.0;
    while x <= 56.0 {
        if (x - 0.0f32).abs() > 4.0 {
            m.bollard(x, -55.5);
        }
        x += 9.0;
    }
    m.gantry(-20.0, -44.0, 16.0, 14.0);
    m.gantry(24.0, -44.0, 16.0, 14.0);
    for (i, (x, z, stack)) in [(-30.0f32, -22.0f32, 2), (-30.0, -28.5, 1), (20.0, -24.0, 2), (30.0, -24.0, 1)].into_iter().enumerate() {
        for l in 0..stack {
            m.container(x, z, l as f32 * 2.6, false, colors[(i + l) % 6], 200.0 + i as f32 + l as f32);
        }
    }
    m.crates(-14.0, -24.0, 60.0);
    m.crates(12.0, -46.0, 61.0);
    m.pallet_stack(-10.0, -40.0, 0.4, 62.0);
    m.table_set(0.0, -30.0, 0.0, c(0.6, 0.6, 0.62));
    m.conveyor(-3.0, -35.0, 0.0, 4.0);
    m.generator(44.5, -40.5, FRAC_PI_2);
    m.pipes(51.5, -38.0, 0.0);
    for x in [-26.0f32, 6.0, 38.0] {
        m.lamp_post(x, -54.5, PI, c(1.0, 0.8, 0.5));
    }
    m.ground_paint(v(0.0, 0.0, -50.0), Vec2::new(9.0, 9.0), 0.0, c(0.22, 0.23, 0.24), 0.011);
    for i in 0..6 {
        m.ground_paint(v(-4.0 + i as f32 * 1.6, 0.0, -50.0), Vec2::new(0.6, 8.0), 0.6, yellow, 0.015);
    }

    // Outside the fence: the ship at the quay and stacks of containers.
    m.ship(0.0, -80.0, 90.0);
    for (side, along_z) in [(1.0f32, false), (-1.0, true), (1.0, true)] {
        let mut t = -63.0;
        let mut i = 0.0;
        while t <= 63.0 {
            let stack = 1 + (hash(t, side) * 3.0) as i32;
            for l in 0..stack {
                let col = colors[((hash(t, l as f32 + side) * 6.0) as usize).min(5)];
                i += 1.0;
                let (x, z) = if along_z { (side * 64.0, t) } else { (t, 64.0) };
                m.container(x, z, l as f32 * 2.6, along_z, col, 100.0 + i);
            }
            t += 7.0;
        }
    }
    m
}

/// A brick furnace with a glowing mouth and a chimney (the foundry).
fn furnace(m: &mut MapLayout, x: f32, z: f32) {
    let mut a = Art::default();
    let brick = c(0.45, 0.22, 0.16);
    boxr(&mut a.paint, v(-2.0, 0.0, -1.5), v(2.0, 3.0, 1.5), brick);
    boxr(&mut a.paint, v(-2.1, 3.0, -1.6), v(2.1, 3.3, 1.6), shade(brick, 0.7));
    a.paint.cyl(v(0.0, 4.6, 0.0), 0.6, 2.6, Quat::IDENTITY, shade(brick, 0.8));
    a.glow.cuboid(v(0.0, 1.0, -1.52), v(1.4, 1.0, 0.04), c(1.0, 0.45, 0.1));
    a.paint.cuboid(v(0.0, 1.6, -1.6), v(1.8, 0.15, 0.2), c(0.2, 0.2, 0.2));
    for i in 0..3 {
        a.metal.cyl(v(-3.0 + i as f32 * 0.5, 0.45, -2.6), 0.25, 0.9, Quat::IDENTITY, c(0.25, 0.25, 0.27));
        a.glow.cyl(v(-3.0 + i as f32 * 0.5, 0.91, -2.6), 0.2, 0.02, Quat::IDENTITY, c(1.0, 0.55, 0.1));
    }
    m.place(a, v(x, 0.0, z), 0.0);
    m.collide(v(x, 1.5, z), v(4.0, 3.0, 3.0));
    m.collide(v(x - 2.5, 0.45, z - 2.6), v(1.6, 0.9, 0.6));
    m.light(v(x, 1.2, z - 2.5), c(1.0, 0.5, 0.15), 60_000.0);
}

// ---------------------------------------------------------------------------
// Central Park
// ---------------------------------------------------------------------------

/// The park. You start in the visitor centre and its fountain court. West
/// is the hedge maze with a rose garden at its heart, east the lakeside café
/// and boathouse. North of the court is the natural history museum (hall,
/// dinosaur hall, gem room, gallery), flanked by the old zoo and the
/// conservatory. Beyond them: the chapel and its cemetery, and the
/// bandstand green with the tennis club.
fn central_park() -> MapLayout {
    let mut m = MapLayout::new(
        Color::srgb(0.85, 0.95, 0.8),
        Ground::Grass,
        Color::srgb(0.5, 0.7, 0.95),
        10000.0,
    );
    m.player_spawns = spawn_line(v(0.0, 0.0, -53.0));
    m.extraction = v(0.0, 0.0, 50.0);
    m.box_spots = [
        v(9.0, 0.0, -29.0),
        v(-54.0, 0.0, -47.0),
        v(7.0, 0.0, 12.0),
        v(29.5, 0.0, -17.5),
        v(14.0, 0.0, 52.0),
    ];
    m.perk(0, -13.0, 9.0, FRAC_PI_2);
    m.perk(1, -36.0, -45.0, 0.0);
    m.perk(2, 40.0, -56.9, 0.0);
    m.perk(3, -57.0, -5.0, FRAC_PI_2);
    m.perk(4, -26.9, 50.5, -FRAC_PI_2);
    m.boundary(1, 3.0);
    m.skyline(OUTER, 3.0);

    let stone = look(Finish::Stone, c(0.78, 0.74, 0.66), c(0.45, 0.4, 0.35));
    let museum = look(Finish::Stone, c(0.72, 0.7, 0.64), c(0.85, 0.82, 0.75));
    let hedge = look(Finish::Hedge, c(0.15, 0.36, 0.13), c(0.15, 0.36, 0.13));
    let wall = look(Finish::Stone, c(0.55, 0.53, 0.5), c(0.45, 0.43, 0.4));
    let glass = look(Finish::Glass, c(0.7, 0.85, 0.8), c(0.92, 0.93, 0.9));
    let wood = look(Finish::Siding, c(0.9, 0.9, 0.86), c(0.25, 0.4, 0.3));
    let brick = look(Finish::Brick, c(0.6, 0.32, 0.24), c(0.85, 0.82, 0.75));
    let chapel = look(Finish::Stone, c(0.62, 0.62, 0.6), c(0.4, 0.4, 0.42));
    let fence = look(Finish::Fence, c(0.55, 0.55, 0.53), c(0.6, 0.6, 0.62));

    m.door_z(-10.0, -51.0);
    m.door_z(10.0, -51.0);
    m.door_x(0.0, -26.0);
    m.door_x(-36.0, -26.0);
    m.door_x(36.0, -26.0);
    m.door_z(-14.0, -6.0);
    m.door_z(14.0, -6.0);
    m.door_x(-36.0, 14.0);
    m.door_x(-7.0, 14.0);
    m.door_x(36.0, 14.0);
    m.door_z(-10.0, 36.0);

    let mut p = Plan::default();
    // Visitor centre and the fountain court (the start).
    p.wx(-57.8, -10.0, 10.0, 5.0, stone);
    p.wx(-44.0, -10.0, 10.0, 5.0, stone);
    p.open_x(-4.0, -44.0, 4.0);
    p.open_x(4.0, -44.0, 4.0);
    p.win_x(0.0, -44.0, 2.0);
    p.wz(-10.0, -57.8, -44.0, 5.0, stone);
    p.wz(10.0, -57.8, -44.0, 5.0, stone);
    p.wx(-44.0, -14.0, -10.0, 4.5, hedge);
    p.wx(-44.0, 10.0, 14.0, 4.5, hedge);
    p.wz(-14.0, -44.0, -26.0, 4.5, hedge);
    p.wz(14.0, -44.0, -26.0, 4.5, hedge);
    // Hedge maze: two rings round the rose garden, and dead ends.
    p.wx(-26.0, -58.0, -14.0, 4.5, hedge);
    p.wx(-52.0, -50.0, -22.0, 3.2, hedge);
    p.open_x(-30.0, -52.0, 4.0);
    p.wx(-32.0, -50.0, -22.0, 3.2, hedge);
    p.open_x(-44.0, -32.0, 4.0);
    p.wz(-50.0, -52.0, -32.0, 3.2, hedge);
    p.wz(-22.0, -52.0, -32.0, 3.2, hedge);
    p.wx(-46.0, -42.0, -30.0, 3.2, hedge);
    p.wx(-38.0, -42.0, -30.0, 3.2, hedge);
    p.wz(-42.0, -46.0, -38.0, 3.2, hedge);
    p.open_z(-42.0, -42.0, 3.0);
    p.wz(-30.0, -46.0, -38.0, 3.2, hedge);
    p.open_z(-30.0, -42.0, 3.0);
    p.wx(-42.0, -58.0, -50.0, 3.2, hedge);
    // Lakeside: the boundary hedge and the café and boathouse.
    p.wx(-26.0, 14.0, 58.0, 4.5, hedge);
    p.wx(-44.0, 38.0, 57.8, 5.0, wood);
    p.open_x(43.0, -44.0, 3.5);
    p.open_x(53.0, -44.0, 3.5);
    p.wx(-57.8, 38.0, 57.8, 5.0, wood);
    p.wz(38.0, -57.8, -44.0, 5.0, wood);
    p.win_z(38.0, -51.0, 3.0);
    p.wz(48.0, -57.8, -44.0, 5.0, wood);
    p.open_z(48.0, -48.5, 3.5);
    p.wz(57.8, -57.8, -44.0, 5.0, wood);
    // Museum.
    p.wx(-26.0, -14.0, 14.0, 7.0, museum);
    p.win_x(-8.0, -26.0, 3.0);
    p.wz(-14.0, -26.0, 14.0, 7.0, museum);
    p.wz(14.0, -26.0, 14.0, 7.0, museum);
    p.wx(14.0, -14.0, 14.0, 7.0, museum);
    p.win_x(7.0, 14.0, 3.0);
    p.wx(-14.0, -14.0, 14.0, 7.0, museum);
    p.open_x(-8.0, -14.0, 4.0);
    p.open_x(8.0, -14.0, 4.0);
    p.wx(4.0, -14.0, 14.0, 7.0, museum);
    p.open_x(-7.0, 4.0, 4.0);
    p.open_x(7.0, 4.0, 4.0);
    p.wz(0.0, 4.0, 14.0, 7.0, museum);
    p.open_z(0.0, 9.0, 3.0);
    // Old zoo: the keeper's hut.
    p.wx(14.0, -58.0, -10.0, 3.5, wall);
    p.wz(-44.0, -8.0, 6.0, 4.5, brick);
    p.open_z(-44.0, -2.0, 3.5);
    p.wx(-8.0, -57.8, -44.0, 4.5, brick);
    p.wx(6.0, -57.8, -44.0, 4.5, brick);
    p.open_x(-50.0, 6.0, 3.5);
    p.wz(-57.8, -8.0, 6.0, 4.5, brick);
    // Conservatory: palm house and fern room.
    p.wx(14.0, 14.0, 58.0, 4.5, hedge);
    p.wx(-20.0, 22.0, 52.0, 6.0, glass);
    p.open_x(30.0, -20.0, 4.0);
    p.open_x(44.0, -20.0, 4.0);
    p.wx(6.0, 22.0, 52.0, 6.0, glass);
    p.open_x(30.0, 6.0, 4.0);
    p.wz(22.0, -20.0, 6.0, 6.0, glass);
    p.open_z(22.0, -6.0, 4.0);
    p.wz(52.0, -20.0, 6.0, 6.0, glass);
    p.wz(37.0, -20.0, 6.0, 6.0, glass);
    p.open_z(37.0, -13.0, 4.0);
    p.open_z(37.0, 0.0, 4.0);
    // Chapel: nave and vestry, in a walled cemetery.
    p.wz(-10.0, 14.0, 58.0, 3.5, wall);
    p.wx(28.0, -50.0, -26.0, 8.0, chapel);
    p.open_x(-38.0, 28.0, 4.0);
    p.win_x(-45.0, 28.0, 2.0);
    p.win_x(-31.0, 28.0, 2.0);
    p.wx(54.0, -50.0, -26.0, 8.0, chapel);
    p.wz(-50.0, 28.0, 54.0, 8.0, chapel);
    p.win_z(-50.0, 35.0, 2.0);
    p.win_z(-50.0, 41.0, 2.0);
    p.wz(-26.0, 28.0, 54.0, 8.0, chapel);
    p.open_z(-26.0, 36.0, 3.0);
    p.win_z(-26.0, 42.0, 2.0);
    p.wx(47.0, -50.0, -26.0, 8.0, chapel);
    p.open_x(-46.0, 47.0, 3.0);
    p.open_x(-30.0, 47.0, 3.0);
    // Bandstand green: the tennis enclosure and the clubhouse.
    p.wz(28.0, 14.0, 38.0, 3.5, fence);
    p.open_z(28.0, 27.0, 4.0);
    p.wx(38.0, 28.0, 58.0, 3.5, fence);
    p.open_x(31.5, 38.0, 4.0);
    p.wx(42.0, 34.0, 57.8, 5.0, wood);
    p.open_x(39.0, 42.0, 3.5);
    p.open_x(53.0, 42.0, 3.5);
    p.wz(34.0, 42.0, 57.8, 5.0, wood);
    p.win_z(34.0, 50.0, 3.0);
    p.wz(46.0, 42.0, 57.8, 5.0, wood);
    p.open_z(46.0, 50.0, 3.5);
    p.wz(57.8, 42.0, 57.8, 5.0, wood);
    p.wx(57.8, 34.0, 57.8, 5.0, wood);
    m.build(&p);

    let warm = c(1.0, 0.9, 0.72);
    m.roof(-10.0, -57.8, 10.0, -44.0, 5.0, c(0.45, 0.25, 0.2));
    m.roof(38.0, -57.8, 57.8, -44.0, 5.0, c(0.25, 0.35, 0.3));
    m.roof(-14.0, -26.0, 14.0, 14.0, 7.0, c(0.5, 0.5, 0.52));
    m.roof(-57.8, -8.0, -44.0, 6.0, 4.5, c(0.35, 0.22, 0.18));
    m.glass_roof(22.0, -20.0, 52.0, 6.0, 6.0);
    m.roof(-50.0, 28.0, -26.0, 54.0, 8.0, c(0.3, 0.3, 0.34));
    m.roof(34.0, 42.0, 57.8, 57.8, 5.0, c(0.25, 0.35, 0.3));
    m.floor(-10.0, -57.8, 10.0, -44.0, Floor::Tiles(c(0.8, 0.74, 0.62)));
    m.floor(-14.0, -44.0, 14.0, -26.0, Floor::Tiles(c(0.66, 0.63, 0.58)));
    m.floor(38.0, -57.8, 48.0, -44.0, Floor::Checker(c(0.9, 0.9, 0.88), c(0.15, 0.15, 0.16)));
    m.floor(48.0, -57.8, 57.8, -44.0, Floor::Boards(c(0.5, 0.36, 0.22)));
    m.floor(-14.0, -26.0, 14.0, -14.0, Floor::Checker(c(0.85, 0.83, 0.78), c(0.35, 0.33, 0.32)));
    m.floor(-14.0, -14.0, 14.0, 4.0, Floor::Tiles(c(0.8, 0.78, 0.72)));
    m.floor(-14.0, 4.0, 0.0, 14.0, Floor::Carpet(c(0.2, 0.25, 0.45)));
    m.floor(0.0, 4.0, 14.0, 14.0, Floor::Boards(c(0.55, 0.38, 0.22)));
    m.floor(-57.8, -8.0, -44.0, 6.0, Floor::Boards(c(0.45, 0.32, 0.2)));
    m.floor(22.0, -20.0, 52.0, 6.0, Floor::Tiles(c(0.6, 0.55, 0.48)));
    m.floor(-50.0, 28.0, -26.0, 54.0, Floor::Tiles(c(0.55, 0.53, 0.5)));
    m.floor(-40.0, 29.0, -36.0, 46.0, Floor::Carpet(c(0.6, 0.12, 0.12)));
    m.floor(34.0, 42.0, 57.8, 57.8, Floor::Boards(c(0.6, 0.45, 0.3)));
    for (x, z) in [(-5.0f32, -51.0f32), (5.0, -51.0)] {
        m.lamp(x, z, 5.0, warm, 60_000.0);
    }
    m.lamp(43.0, -51.0, 5.0, warm, 50_000.0);
    m.lamp(53.0, -51.0, 5.0, warm, 40_000.0);
    for (x, z) in [(0.0f32, -20.0f32), (-7.0, -5.0), (7.0, -5.0), (-7.0, 9.0), (7.0, 9.0)] {
        m.lamp(x, z, 7.0, warm, 80_000.0);
    }
    m.lamp(-51.0, -1.0, 4.5, warm, 40_000.0);
    m.lamp(-38.0, 36.0, 8.0, c(1.0, 0.8, 0.55), 90_000.0);
    m.lamp(-38.0, 50.5, 8.0, c(1.0, 0.8, 0.55), 40_000.0);
    m.lamp(40.0, 50.0, 5.0, warm, 50_000.0);
    m.lamp(52.0, 50.0, 5.0, warm, 40_000.0);
    m.light(v(30.0, 4.5, -7.0), c(0.85, 1.0, 0.85), 60_000.0);
    m.light(v(44.5, 4.5, -7.0), c(0.85, 1.0, 0.85), 60_000.0);

    m.wall_gun(0.0, -57.8, 0.0, 2);
    m.wall_gun(-10.0, -46.5, -FRAC_PI_2, 12);
    m.wall_gun(8.0, -26.0, 0.0, 7);
    m.wall_gun(-44.0, 3.5, FRAC_PI_2, 4);
    m.wall_gun(48.0, -55.0, FRAC_PI_2, 11);
    m.wall_gun(52.0, -7.0, -FRAC_PI_2, 16);
    m.wall_gun(-38.0, 54.0, PI, 18);
    m.wall_gun(34.0, 46.0, FRAC_PI_2, 14);

    m.spawns(0, &[(-12.0, -28.0), (12.0, -28.0), (-8.5, -56.0), (8.5, -46.5)]);
    m.spawns(1, &[(-55.0, -55.0), (-55.0, -29.0), (-17.0, -29.0), (-36.0, -55.0), (-12.0, -55.0)]);
    m.spawns(2, &[(55.0, -29.0), (16.0, -55.0), (30.0, -55.0), (35.0, -45.0), (17.0, -29.0)]);
    m.spawns(3, &[(-11.0, -11.0), (11.0, 1.0), (-11.0, 12.0), (11.0, -23.0)]);
    m.spawns(4, &[(-55.0, -23.0), (-17.0, -23.0), (-17.0, 11.0), (-45.0, 11.0), (-30.0, -5.0)]);
    m.spawns(5, &[(55.0, -23.0), (17.0, -23.0), (50.0, 11.0), (17.0, 11.0), (45.0, 10.0)]);
    m.spawns(6, &[(-55.0, 17.0), (-56.0, 50.0), (-13.0, 55.0), (-13.0, 17.0), (-20.0, 40.0)]);
    m.spawns(7, &[(-7.0, 55.0), (20.0, 55.0), (-7.0, 17.0), (20.0, 17.0), (55.0, 40.0)]);

    let path = c(0.7, 0.64, 0.52);
    let flowers = [
        c(0.95, 0.3, 0.4),
        c(0.95, 0.85, 0.25),
        c(0.65, 0.4, 0.95),
        c(1.0, 1.0, 1.0),
    ];

    // Visitor centre: information desk, gift shop and a park map.
    m.counter(0.0, -48.5, 0.0, 6.0, c(0.45, 0.32, 0.2));
    m.shelving(6.5, -55.5, 0.0, 1.0);
    m.vending(-9.2, -47.0, FRAC_PI_2, c(0.2, 0.45, 0.25));
    m.bench(-6.0, -54.5, 0.0);
    let mut a = Art::default();
    a.paint.cuboid(v(-6.0, 2.2, -57.55), v(3.0, 1.8, 0.06), c(0.3, 0.25, 0.2));
    a.paint.cuboid(v(-6.0, 2.2, -57.5), v(2.7, 1.5, 0.04), c(0.45, 0.65, 0.35));
    a.paint.cuboid(v(-6.0, 2.2, -57.47), v(0.5, 1.5, 0.02), c(0.7, 0.64, 0.52));
    a.glass.cuboid(v(-6.6, 2.4, -57.47), v(0.8, 0.5, 0.02), c(0.25, 0.45, 0.7));
    m.place(a, Vec3::ZERO, 0.0);
    // Fountain court.
    m.fountain(0.0, -35.0);
    for (x, z) in [(-9.0f32, -40.0f32), (9.0, -40.0), (-9.0, -30.0)] {
        m.flower_bed(x, z, 4.0, x + z);
    }
    for (x, z, yaw) in [(-6.0f32, -30.0f32, PI), (6.0, -40.0, 0.0)] {
        m.bench(x, z, yaw);
    }
    for (x, z) in [(-12.0f32, -35.0f32), (12.0, -35.0)] {
        m.park_lamp(x, z);
    }
    // Hedge maze: the rose garden and a gardener's barrow.
    m.statue(-36.0, -41.0, 0.0);
    m.flower_bed(-33.0, -39.0, 2.0, 1.0);
    m.flower_bed(-39.0, -39.0, 2.0, 2.0);
    m.park_lamp(-46.0, -29.0);
    m.park_lamp(-18.0, -48.0);
    m.bench(-46.0, -48.0, FRAC_PI_2);
    m.ground_paint(v(-36.0, 0.0, -42.0), Vec2::new(11.0, 7.0), 0.0, path, 0.008);

    // Lakeside: a wading pond with a boardwalk, the café and boat shed.
    pond(&mut m, 18.0, -42.0, 34.0, -30.0);
    m.rowboat(23.0, -36.0, 0.3, c(0.85, 0.85, 0.82));
    m.rowboat(29.0, -34.0, -0.4, c(0.7, 0.15, 0.12));
    let mut a = Art::default();
    let plank = c(0.55, 0.4, 0.27);
    let mut x = 18.0;
    while x < 34.0 {
        a.paint.cuboid(v(x, 0.12, -28.5), v(0.28, 0.08, 2.2), shade(plank, 0.9 + hash(x, 1.0) * 0.2));
        x += 0.32;
    }
    m.place(a, Vec3::ZERO, 0.0);
    for (x, z) in [(20.0f32, -50.0f32), (32.0, -52.0), (52.0, -38.0)] {
        m.oak(x, z, 1.0, x);
    }
    m.park_lamp(36.0, -36.0);
    m.park_lamp(16.0, -46.0);
    m.diner(&Room::new(38.2, -57.6, 47.8, -44.2, 5.0, &[(43.0, -44.0), (48.0, -48.5), (40.0, -56.9)]), 3.0);
    for (i, z) in [-56.0f32, -52.0].into_iter().enumerate() {
        m.rowboat(53.0, z, FRAC_PI_2, [c(0.2, 0.35, 0.6), c(0.85, 0.75, 0.2)][i]);
    }
    m.crates(55.5, -47.5, 5.0);

    // Museum: info desk, a dinosaur, gems and paintings.
    m.counter(-6.0, -20.0, 0.0, 5.0, c(0.4, 0.25, 0.15));
    m.bench(0.0, -18.0, 0.0);
    dinosaur(&mut m, 0.0, -5.0);
    m.museum(&Room::new(-13.8, 4.2, -0.2, 13.8, 7.0, &[(-7.0, 14.0), (-7.0, 4.0), (0.0, 9.0), (-13.0, 9.0)]), 1.0);
    m.museum(&Room::new(0.2, 4.2, 13.8, 13.8, 7.0, &[(7.0, 4.0), (0.0, 9.0), (7.0, 12.0)]), 2.0);

    // Old zoo: cages, a keeper's hut, paths and trees.
    m.cage(-24.0, -20.0, 7.0, 4.0);
    m.cage(-50.0, -20.0, 7.0, 4.0);
    m.cage(-24.0, 10.0, 7.0, 4.0);
    m.desk(-50.0, -6.0, PI);
    m.lockers(-47.0, 5.6, PI, 4, c(0.4, 0.5, 0.35));
    m.cabinets(-57.3, 1.5, FRAC_PI_2, 3);
    m.ground_paint(v(-36.0, 0.0, -6.0), Vec2::new(3.0, 38.0), 0.0, path, 0.008);
    m.ground_paint(v(-30.0, 0.0, -2.0), Vec2::new(28.0, 3.0), 0.0, path, 0.008);
    for (x, z) in [(-20.0f32, -4.0f32), (-40.0, -14.0), (-30.0, 8.0)] {
        m.oak(x, z, 1.1, x + z);
    }
    m.park_lamp(-33.0, -12.0);
    m.park_lamp(-38.0, 4.0);
    m.bench(-27.0, -4.5, 0.0);

    // Conservatory: palms and beds under glass, a potting shed outside.
    m.oak(29.0, -6.0, 1.15, 3.0);
    m.pine(33.0, 1.0, 0.9, 4.0);
    for (x, z) in [(26.0f32, -16.0f32), (33.0, -16.0)] {
        m.flower_bed(x, z, 3.0, x);
    }
    for (x, z) in [(41.0f32, -16.0f32), (48.0, -16.0), (41.0, 2.5), (48.0, 2.5)] {
        m.bush(x, z, 1.2, x + z, Some(flowers[(x as usize) % 4]));
    }
    m.bench(44.5, -7.0, FRAC_PI_2);
    m.shed(54.5, 10.0, PI);
    m.park_lamp(18.0, -12.0);
    m.park_lamp(55.0, -14.0);

    // Chapel: pews either side of the aisle, an altar; graves outside.
    let mut z = 31.5;
    while z < 41.0 {
        for x in [-44.5f32, -31.5] {
            m.pew(x, z, PI, 6.0);
        }
        z += 2.2;
    }
    m.altar(-38.0, 44.5, PI);
    m.cabinets(-48.0, 53.4, PI, 3);
    for (i, (x, z)) in [(-54.0f32, 22.0f32), (-44.0, 20.0), (-34.0, 22.0), (-20.0, 20.0), (-18.0, 30.0), (-18.0, 48.0), (-54.0, 56.0), (-30.0, 57.0)].into_iter().enumerate() {
        m.graves(x, z, if i % 2 == 0 { 0.0 } else { PI }, i as f32);
    }
    m.pine(-15.0, 39.0, 1.2, 5.0);
    m.park_lamp(-38.0, 24.0);

    // Bandstand green: the bandstand, benches, a tennis court.
    m.bandstand(8.0, 32.0);
    m.tennis_court(43.0, 26.0, c(0.2, 0.45, 0.6));
    for (x, z, yaw) in [(-4.0f32, 30.0f32, FRAC_PI_2), (20.0, 32.0, -FRAC_PI_2), (8.0, 22.0, 0.0)] {
        m.bench(x, z, yaw);
    }
    m.park_lamp(-4.0, 44.0);
    m.park_lamp(20.0, 44.0);
    m.park_lamp(22.0, 20.0);
    m.counter(40.0, 54.5, PI, 5.0, c(0.35, 0.25, 0.15));
    m.table_set(39.0, 47.0, 0.0, c(0.85, 0.85, 0.82));
    m.sofa(42.5, 44.0, 0.0, c(0.25, 0.4, 0.3));
    m.lockers(52.0, 57.3, PI, 8, c(0.6, 0.3, 0.25));
    m.bench(52.0, 54.5, 0.0);
    m.ground_paint(v(8.0, 0.0, 32.0), Vec2::new(3.0, 34.0), 0.0, path, 0.008);
    m.ground_paint(v(8.0, 0.0, 48.0), Vec2::new(30.0, 3.0), 0.0, path, 0.008);
    for (x, z) in [(-6.0f32, 20.0f32), (24.0, 56.0), (-6.0, 40.0)] {
        m.oak(x, z, 1.0, x + z);
    }
    for (i, (x, z)) in [(2.0f32, 40.0f32), (14.0, 40.0), (2.0, 24.0), (14.0, 24.0)].into_iter().enumerate() {
        m.bush(x, z, 0.9, i as f32, Some(flowers[i]));
    }
    m
}

/// A shallow pond you can wade through, with stones round it (gaps let
/// you walk in).
fn pond(m: &mut MapLayout, x0: f32, z0: f32, x1: f32, z1: f32) {
    let mut a = Art::default();
    let (cx, cz) = ((x0 + x1) / 2.0, (z0 + z1) / 2.0);
    a.glass.cuboid(v(cx, 0.04, cz), v(x1 - x0, 0.04, z1 - z0), c(0.2, 0.42, 0.62));
    a.paint.cuboid(v(cx, 0.01, cz), v(x1 - x0 + 0.2, 0.02, z1 - z0 + 0.2), c(0.18, 0.25, 0.2));
    // Each side in two halves with a 3m gap in the middle.
    for (along_x, line, a0, a1) in [
        (true, z0, x0, x1),
        (true, z1, x0, x1),
        (false, x0, z0, z1),
        (false, x1, z0, z1),
    ] {
        let mid = (a0 + a1) / 2.0;
        for (s, e) in [(a0, mid - 1.5), (mid + 1.5, a1)] {
            let len = e - s;
            let c0 = (s + e) / 2.0;
            let (pos, size) = if along_x {
                (v(c0, 0.25, line), v(len, 0.5, 0.5))
            } else {
                (v(line, 0.25, c0), v(0.5, 0.5, len))
            };
            m.collide(pos, size);
            let n = (len / 0.7) as i32;
            for i in 0..=n {
                let t = s + len * i as f32 / n.max(1) as f32;
                let p = if along_x { v(t, 0.22, line) } else { v(line, 0.22, t) };
                let r = 0.32 + hash(p.x, p.z) * 0.12;
                a.paint.blob(p, v(r * 1.2, r * 0.8, r), c(0.55 + r * 0.2, 0.55 + r * 0.2, 0.53 + r * 0.2));
            }
        }
    }
    for i in 0..8 {
        let x = x0 + 1.0 + hash(i as f32, 1.0) * (x1 - x0 - 2.0);
        let z = z0 + 1.0 + hash(i as f32, 2.0) * (z1 - z0 - 2.0);
        a.paint.cyl(v(x, 0.07, z), 0.35, 0.02, Quat::IDENTITY, c(0.25, 0.55, 0.2));
    }
    m.place(a, Vec3::ZERO, 0.0);
}

/// A dinosaur skeleton on a plinth, its long side along X.
fn dinosaur(m: &mut MapLayout, x: f32, z: f32) {
    let mut a = Art::default();
    let bone = c(0.88, 0.84, 0.72);
    boxr(&mut a.paint, v(-5.5, 0.0, -1.6), v(5.5, 0.6, 1.6), c(0.55, 0.52, 0.48));
    // Spine as a curve from tail to skull.
    let spine: Vec<Vec3> = (0..=14)
        .map(|i| {
            let t = i as f32 / 14.0;
            v(-5.0 + t * 9.5, 2.6 + (t * PI).sin() * 1.3 + t * 0.8, 0.0)
        })
        .collect();
    for w in spine.windows(2) {
        a.paint.capsule_between(w[0], w[1], 0.12, bone);
    }
    // Ribs, hanging off the middle of the spine.
    for p in spine.iter().skip(5).take(6) {
        for s in [-1.0f32, 1.0] {
            a.paint.capsule_between(*p, *p + v(0.1, -1.1, s * 0.7), 0.05, bone);
        }
    }
    // Skull and jaw.
    let head = *spine.last().unwrap_or(&Vec3::ZERO);
    a.paint.blob(head + v(0.5, 0.0, 0.0), v(0.8, 0.45, 0.35), bone);
    a.paint.capsule_between(head + v(0.1, -0.35, 0.0), head + v(1.2, -0.45, 0.0), 0.08, bone);
    // Legs and the steel posts that hold it up.
    for (lx, s) in [(-1.2f32, -1.0f32), (-1.2, 1.0), (2.0, -1.0), (2.0, 1.0)] {
        let hip = v(lx, 3.0, s * 0.4);
        let knee = v(lx + 0.5, 1.7, s * 0.6);
        a.paint.capsule_between(hip, knee, 0.1, bone);
        a.paint.capsule_between(knee, v(lx, 0.65, s * 0.6), 0.08, bone);
    }
    for px in [-3.5f32, 0.5, 3.5] {
        a.metal.cyl(v(px, 1.6, 0.0), 0.04, 2.0, Quat::IDENTITY, c(0.3, 0.3, 0.32));
    }
    m.place(a, v(x, 0.0, z), 0.0);
    m.collide(v(x, 0.8, z), v(11.0, 1.6, 3.2));
}

// ---------------------------------------------------------------------------
// The Neighborhood
// ---------------------------------------------------------------------------

/// The street. You start in the Hendersons' house (living room, kitchen,
/// hall, bedroom, garage and back yard). Its side doors lead into the diner
/// and the school; the front door opens onto Main Street. Across the street
/// are the police station, the community centre and Maple Court's two
/// houses, and behind them all runs the back alley.
fn neighborhood() -> MapLayout {
    let mut m = MapLayout::new(
        Color::srgb(0.9, 1.0, 0.85),
        Ground::Grass,
        Color::srgb(0.95, 0.68, 0.5),
        7000.0,
    );
    m.player_spawns = spawn_line(v(-5.0, 0.0, -15.0));
    m.extraction = v(-6.0, 0.0, 51.0);
    m.box_spots = [
        v(-4.0, 0.0, -50.0),
        v(18.0, 0.0, 6.0),
        v(52.0, 0.0, -14.0),
        v(-23.0, 0.0, 30.8),
        v(40.0, 0.0, 48.0),
    ];
    m.perk(0, 13.0, -28.0, FRAC_PI_2);
    m.perk(1, 12.9, 40.0, FRAC_PI_2);
    m.perk(2, -56.9, -21.0, FRAC_PI_2);
    m.perk(3, -31.0, 29.0, -FRAC_PI_2);
    m.perk(4, 11.2, 40.0, -FRAC_PI_2);
    m.boundary(2, 2.2);

    let home = look(Finish::Siding, c(0.6, 0.72, 0.85), c(0.95, 0.95, 0.92));
    let inside = look(Finish::Plaster, c(0.92, 0.88, 0.78), c(0.55, 0.42, 0.3));
    let wood = look(Finish::Wood, c(0.55, 0.4, 0.27), c(0.55, 0.4, 0.27));
    let diner = look(Finish::Plaster, c(0.92, 0.9, 0.84), c(0.2, 0.55, 0.55));
    let store = look(Finish::Block, c(0.85, 0.8, 0.7), c(0.75, 0.15, 0.12));
    let school = look(Finish::Brick, c(0.6, 0.3, 0.22), c(0.85, 0.82, 0.75));
    let police = look(Finish::Block, c(0.7, 0.74, 0.8), c(0.15, 0.22, 0.45));
    let fence = look(Finish::Fence, c(0.55, 0.55, 0.53), c(0.6, 0.6, 0.62));
    let hall = look(Finish::Brick, c(0.55, 0.35, 0.25), c(0.9, 0.88, 0.8));
    let yellow = look(Finish::Siding, c(0.92, 0.82, 0.5), c(0.95, 0.95, 0.92));
    let pink = look(Finish::Siding, c(0.85, 0.65, 0.65), c(0.95, 0.95, 0.92));

    m.door_x(-5.0, -8.0);
    m.door_z(-12.0, -21.0);
    m.door_z(12.0, -21.0);
    m.door_x(-24.0, -8.0);
    m.door_x(-41.0, -8.0);
    m.door_x(29.0, -8.0);
    m.door_x(-38.0, 8.0);
    m.door_x(0.0, 8.0);
    m.door_x(36.0, 8.0);
    m.door_x(-38.0, 44.0);
    m.door_x(0.0, 44.0);
    m.door_x(24.0, 44.0);

    let mut p = Plan::default();
    // The Hendersons' house and back yard (the start).
    p.wx(-8.0, -12.0, 12.0, 4.0, home);
    p.win_x(-9.5, -8.0, 2.0);
    p.win_x(5.0, -8.0, 2.5);
    p.wx(-30.0, -12.0, 12.0, 4.0, home);
    p.open_x(6.0, -30.0, 3.5);
    p.win_x(-7.0, -30.0, 2.0);
    p.wz(-12.0, -30.0, -8.0, 4.5, home);
    p.wz(12.0, -30.0, -8.0, 4.5, home);
    p.wz(12.0, -34.0, -30.0, 4.5, school);
    p.wz(2.0, -18.0, -8.0, 4.0, inside);
    p.open_z(2.0, -13.0, 3.5);
    p.wx(-18.0, -12.0, 12.0, 4.0, inside);
    p.open_x(-5.0, -18.0, 3.5);
    p.open_x(7.0, -18.0, 3.5);
    p.wx(-24.0, -12.0, 12.0, 4.0, inside);
    p.open_x(-7.0, -24.0, 3.0);
    p.open_x(6.0, -24.0, 3.5);
    p.wz(-2.0, -30.0, -24.0, 4.0, inside);
    p.wz(-12.0, -58.0, -30.0, 2.2, wood);
    p.wz(12.0, -58.0, -34.0, 2.2, wood);
    // Diner, the alley beside it and the corner store.
    p.wx(-8.0, -30.0, -12.0, 4.5, diner);
    p.win_x(-17.0, -8.0, 2.5);
    p.win_x(-28.3, -8.0, 2.0);
    p.wz(-30.0, -24.0, -8.0, 4.5, diner);
    p.win_z(-30.0, -16.0, 3.0);
    p.wx(-24.0, -30.0, -12.0, 4.5, diner);
    p.open_x(-15.0, -24.0, 3.0);
    p.open_x(-26.0, -24.0, 3.0);
    p.wz(-18.0, -24.0, -8.0, 4.5, diner);
    p.open_z(-18.0, -12.0, 3.0);
    p.wx(-8.0, -34.0, -30.0, 4.5, store);
    p.wx(-8.0, -57.8, -34.0, 4.5, store);
    p.win_x(-37.0, -8.0, 3.0);
    p.win_x(-52.0, -8.0, 3.0);
    p.wz(-34.0, -24.0, -8.0, 4.5, store);
    p.wz(-57.8, -24.0, -8.0, 4.5, store);
    p.wx(-24.0, -57.8, -34.0, 4.5, store);
    p.open_x(-41.0, -24.0, 3.5);
    p.wz(-48.0, -24.0, -8.0, 4.5, store);
    p.open_z(-48.0, -16.0, 3.0);
    // School: classrooms and lobby off a corridor, the lab, the library and
    // the gym.
    p.wx(-8.0, 12.0, 46.0, 4.5, school);
    p.win_x(18.0, -8.0, 3.0);
    p.win_x(40.0, -8.0, 3.0);
    p.wx(-8.0, 46.0, 57.8, 7.0, school);
    p.wx(-34.0, 12.0, 46.0, 4.5, school);
    p.open_x(37.0, -34.0, 3.5);
    p.win_x(20.0, -34.0, 3.0);
    p.wx(-34.0, 46.0, 57.8, 7.0, school);
    p.open_x(52.0, -34.0, 4.0);
    p.wz(57.8, -34.0, -8.0, 7.0, school);
    p.wz(46.0, -34.0, -8.0, 7.0, school);
    p.open_z(46.0, -20.0, 3.5);
    p.wx(-18.0, 12.0, 46.0, 4.5, inside);
    p.open_x(18.0, -18.0, 3.0);
    p.open_x(29.0, -18.0, 4.0);
    p.open_x(40.0, -18.0, 3.0);
    p.wx(-22.0, 12.0, 46.0, 4.5, inside);
    p.open_x(20.0, -22.0, 3.0);
    p.open_x(37.0, -22.0, 3.0);
    p.wz(24.0, -18.0, -8.0, 4.5, inside);
    p.wz(34.0, -18.0, -8.0, 4.5, inside);
    p.wz(28.0, -34.0, -22.0, 4.5, inside);
    // Police station: lobby, offices, briefing room and cells.
    p.wx(8.0, -57.8, -20.0, 4.5, police);
    p.win_x(-51.0, 8.0, 3.0);
    p.win_x(-25.0, 8.0, 2.5);
    p.wx(8.0, -20.0, -12.0, 3.5, fence);
    p.wz(-57.8, 8.0, 32.0, 4.5, police);
    p.wz(-20.0, 8.0, 32.0, 4.5, police);
    p.wx(32.0, -57.8, -20.0, 4.5, police);
    p.open_x(-38.0, 32.0, 3.5);
    p.wz(-46.0, 8.0, 32.0, 4.5, inside);
    p.open_z(-46.0, 13.0, 3.0);
    p.open_z(-46.0, 26.0, 3.0);
    p.wx(18.0, -46.0, -30.0, 4.5, inside);
    p.open_x(-38.0, 18.0, 3.5);
    p.wz(-30.0, 8.0, 32.0, 4.5, police);
    p.open_z(-30.0, 13.0, 3.0);
    p.open_z(-30.0, 22.0, 3.0);
    p.wz(-26.0, 8.0, 32.0, 4.5, police);
    p.win_z(-26.0, 12.0, 5.0);
    p.win_z(-26.0, 20.0, 5.0);
    p.open_z(-26.0, 28.0, 3.0);
    p.wx(16.0, -26.0, -20.0, 4.5, police);
    p.wx(24.0, -26.0, -20.0, 4.5, police);
    p.wx(44.0, -58.0, -12.0, 3.5, fence);
    // Community centre: front hall, library, games room, kitchen.
    p.wx(8.0, -12.0, 12.0, 5.0, hall);
    p.win_x(-8.0, 8.0, 3.0);
    p.win_x(8.0, 8.0, 3.0);
    p.wz(-12.0, 8.0, 44.0, 5.0, hall);
    p.wz(12.0, 8.0, 44.0, 5.0, hall);
    p.wx(44.0, -12.0, 12.0, 5.0, hall);
    p.win_x(-7.0, 44.0, 3.0);
    p.wx(20.0, -12.0, 12.0, 5.0, inside);
    p.open_x(-6.0, 20.0, 3.5);
    p.open_x(6.0, 20.0, 3.5);
    p.wz(0.0, 20.0, 34.0, 5.0, inside);
    p.open_z(0.0, 27.0, 3.0);
    p.wx(34.0, -12.0, 12.0, 5.0, inside);
    p.open_x(-6.0, 34.0, 3.5);
    p.open_x(6.0, 34.0, 3.5);
    // Maple Court: two houses, front fence and back yards.
    p.wx(8.0, 12.0, 58.0, 2.2, wood);
    p.wx(12.0, 16.0, 34.0, 4.0, yellow);
    p.win_x(20.0, 12.0, 2.0);
    p.open_x(25.0, 12.0, 2.5);
    p.win_x(30.0, 12.0, 2.0);
    p.wx(30.0, 16.0, 34.0, 4.0, yellow);
    p.open_x(20.0, 30.0, 3.0);
    p.wz(16.0, 12.0, 30.0, 4.0, yellow);
    p.win_z(16.0, 20.0, 2.0);
    p.wz(34.0, 12.0, 30.0, 4.0, yellow);
    p.open_z(34.0, 16.0, 3.0);
    p.win_z(34.0, 26.0, 2.0);
    p.wx(21.0, 16.0, 34.0, 4.0, inside);
    p.open_x(20.0, 21.0, 3.0);
    p.open_x(30.0, 21.0, 3.0);
    p.wz(25.0, 21.0, 30.0, 4.0, inside);
    p.wx(12.0, 38.0, 57.8, 4.0, pink);
    p.win_x(44.0, 12.0, 2.0);
    p.win_x(52.0, 12.0, 2.0);
    p.wx(30.0, 38.0, 57.8, 4.0, pink);
    p.open_x(52.0, 30.0, 3.0);
    p.wz(38.0, 12.0, 30.0, 4.0, pink);
    p.open_z(38.0, 16.0, 3.0);
    p.win_z(38.0, 26.0, 2.0);
    p.wz(57.8, 12.0, 30.0, 4.0, pink);
    p.wx(21.0, 38.0, 57.8, 4.0, inside);
    p.open_x(43.0, 21.0, 3.0);
    p.open_x(52.0, 21.0, 3.0);
    p.wz(47.0, 21.0, 30.0, 4.0, inside);
    p.wx(44.0, 12.0, 58.0, 2.2, wood);
    p.wz(36.0, 30.0, 44.0, 2.2, wood);
    p.open_z(36.0, 37.0, 3.0);
    m.build(&p);

    let warm = c(1.0, 0.88, 0.68);
    m.gable_roof(-12.0, -30.0, 12.0, -8.0, 4.0, c(0.6, 0.72, 0.85), c(0.35, 0.2, 0.17));
    m.roof(-30.0, -24.0, -12.0, -8.0, 4.5, c(0.2, 0.5, 0.5));
    m.roof(-57.8, -24.0, -34.0, -8.0, 4.5, c(0.6, 0.15, 0.12));
    m.roof(12.0, -34.0, 46.0, -8.0, 4.5, c(0.35, 0.33, 0.32));
    m.roof(46.0, -34.0, 57.8, -8.0, 7.0, c(0.35, 0.33, 0.32));
    m.roof(-57.8, 8.0, -20.0, 32.0, 4.5, c(0.3, 0.32, 0.36));
    m.gable_roof(-12.0, 8.0, 12.0, 44.0, 5.0, c(0.55, 0.35, 0.25), c(0.25, 0.27, 0.32));
    m.gable_roof(16.0, 12.0, 34.0, 30.0, 4.0, c(0.92, 0.82, 0.5), c(0.42, 0.28, 0.2));
    m.gable_roof(38.0, 12.0, 57.8, 30.0, 4.0, c(0.85, 0.65, 0.65), c(0.25, 0.27, 0.32));
    m.floor(-12.0, -18.0, 2.0, -8.0, Floor::Carpet(c(0.55, 0.35, 0.3)));
    m.floor(2.0, -18.0, 12.0, -8.0, Floor::Checker(c(0.9, 0.9, 0.86), c(0.4, 0.55, 0.6)));
    m.floor(-12.0, -24.0, 12.0, -18.0, Floor::Boards(c(0.55, 0.38, 0.22)));
    m.floor(-12.0, -30.0, -2.0, -24.0, Floor::Carpet(c(0.35, 0.45, 0.6)));
    m.floor(-2.0, -30.0, 12.0, -24.0, Floor::Concrete(c(0.5, 0.5, 0.48)));
    m.floor(-30.0, -24.0, -18.0, -8.0, Floor::Checker(c(0.92, 0.92, 0.9), c(0.12, 0.12, 0.13)));
    m.floor(-18.0, -24.0, -12.0, -8.0, Floor::Tiles(c(0.8, 0.8, 0.78)));
    m.floor(-57.8, -24.0, -34.0, -8.0, Floor::Tiles(c(0.85, 0.85, 0.82)));
    m.floor(12.0, -34.0, 46.0, -8.0, Floor::Tiles(c(0.75, 0.72, 0.62)));
    m.floor(46.0, -34.0, 57.8, -8.0, Floor::Boards(c(0.75, 0.55, 0.3)));
    m.floor(-57.8, 8.0, -20.0, 32.0, Floor::Tiles(c(0.7, 0.72, 0.74)));
    m.floor(-12.0, 8.0, 12.0, 44.0, Floor::Boards(c(0.6, 0.42, 0.25)));
    m.floor(16.0, 12.0, 34.0, 30.0, Floor::Boards(c(0.6, 0.45, 0.3)));
    m.floor(38.0, 12.0, 57.8, 30.0, Floor::Carpet(c(0.45, 0.5, 0.4)));
    for (x, z) in [(-5.0f32, -13.0f32), (7.0, -13.0), (0.0, -21.0), (-7.0, -27.0), (5.0, -27.0)] {
        m.lamp(x, z, 4.0, warm, 35_000.0);
    }
    for (x, z) in [(-24.0f32, -16.0f32), (-15.0, -16.0), (-41.0, -16.0), (-53.0, -16.0)] {
        m.lamp(x, z, 4.5, c(1.0, 0.95, 0.85), 45_000.0);
    }
    for (x, z) in [(18.0f32, -13.0f32), (29.0, -13.0), (40.0, -13.0), (29.0, -20.0), (20.0, -28.0), (37.0, -28.0), (52.0, -21.0)] {
        m.lamp(x, z, if x > 46.0 { 7.0 } else { 4.5 }, c(0.95, 0.97, 1.0), 45_000.0);
    }
    for (x, z) in [(-51.0f32, 20.0f32), (-38.0, 13.0), (-38.0, 25.0), (-28.0, 20.0)] {
        m.lamp(x, z, 4.5, c(0.92, 0.96, 1.0), 40_000.0);
    }
    for (x, z) in [(0.0f32, 14.0f32), (-6.0, 27.0), (6.0, 27.0), (0.0, 39.0)] {
        m.lamp(x, z, 5.0, warm, 45_000.0);
    }
    for (x, z) in [(25.0f32, 16.0f32), (25.0, 25.5), (48.0, 16.0), (48.0, 25.5)] {
        m.lamp(x, z, 4.0, warm, 30_000.0);
    }

    m.wall_gun(-9.5, -18.0, 0.0, 12);
    m.wall_gun(8.0, -8.0, 0.0, 9);
    m.wall_gun(-30.0, -11.0, FRAC_PI_2, 5);
    m.wall_gun(34.0, -18.0, PI, 8);
    m.wall_gun(-43.0, 18.0, PI, 18);
    m.wall_gun(12.0, 27.0, -FRAC_PI_2, 6);
    m.wall_gun(48.0, 21.0, PI, 3);
    m.wall_gun(-7.0, 44.0, 0.0, 13);

    m.spawns(0, &[(-2.0, -56.0), (10.0, -56.0), (0.0, -40.0), (-10.0, -28.0)]);
    m.spawns(1, &[(-56.0, 0.0), (56.0, 0.0), (-20.0, 0.0), (20.0, 0.0), (0.0, 5.0)]);
    m.spawns(2, &[(-56.0, -56.0), (-14.0, -56.0), (-32.0, -16.0), (-53.0, -15.0)]);
    m.spawns(3, &[(56.0, -56.0), (15.0, -56.0), (55.0, -26.0), (15.0, -26.0)]);
    m.spawns(4, &[(-56.0, 42.0), (-14.0, 42.0), (-14.0, 10.0), (-55.0, 30.0)]);
    m.spawns(5, &[(18.0, 42.0), (56.0, 42.0), (14.0, 10.0), (52.0, 33.0)]);
    m.spawns(6, &[(-56.0, 48.0), (56.0, 48.0), (-14.0, 48.0), (14.0, 48.0)]);
    m.spawns(7, &[(-10.0, 42.0), (8.0, 41.0), (-10.0, 32.0)]);

    let red = c(0.75, 0.12, 0.12);
    let cars = [
        c(0.7, 0.1, 0.1),
        c(0.1, 0.2, 0.6),
        c(0.85, 0.85, 0.85),
        c(0.15, 0.15, 0.15),
        c(0.2, 0.45, 0.3),
    ];

    // Main Street: road, dashes, curbs, sidewalks, parked cars, lights.
    m.ground_paint(Vec3::ZERO, Vec2::new(116.0, 9.0), 0.0, c(0.17, 0.17, 0.19), 0.006);
    for i in -12..=12 {
        m.ground_paint(v(i as f32 * 4.5, 0.0, 0.0), Vec2::new(2.0, 0.15), 0.0, c(0.95, 0.82, 0.25), 0.018);
    }
    for s in [-1.0f32, 1.0] {
        let mut a = Art::default();
        a.paint.cuboid(v(0.0, 0.06, s * 4.6), v(116.0, 0.12, 0.2), c(0.6, 0.6, 0.58));
        a.paint.cuboid(v(0.0, 0.05, s * 6.2), v(116.0, 0.1, 3.0), c(0.68, 0.68, 0.65));
        m.place(a, Vec3::ZERO, 0.0);
    }
    for i in 0..8 {
        m.ground_paint(v(-12.0, 0.0, -3.2 + i as f32 * 0.9), Vec2::new(2.6, 0.45), 0.0, c(0.92, 0.92, 0.9), 0.019);
    }
    for (k, (x, z)) in [(-48.0f32, -2.4f32), (-30.0, 2.4), (-14.0, -2.4), (14.0, 2.4), (24.0, -2.4), (46.0, 2.4)].into_iter().enumerate() {
        m.car(x, z, if z < 0.0 { 0.0 } else { PI }, cars[k % cars.len()]);
    }
    m.school_bus(44.0, -2.0, FRAC_PI_2);
    for x in [-44.0f32, -16.0, 16.0, 50.0] {
        m.street_light(x, 6.6, 0.0);
    }
    for x in [-30.0f32, 2.0, 32.0] {
        m.street_light(x, -6.6, PI);
    }
    m.hydrant(-20.0, 6.4);
    m.hydrant(20.0, -6.4);
    m.mailbox(-9.0, -6.6, PI, c(0.15, 0.2, 0.45));
    m.trash_cans(-52.0, 6.3);

    // The Hendersons' house: living room, kitchen, hall, bedroom, garage.
    m.sofa(-9.0, -10.0, PI, c(0.4, 0.3, 0.55));
    m.tv(-4.5, -16.2, 0.0);
    m.kitchen(7.0, -8.6, PI, 6.0, c(0.85, 0.85, 0.8));
    m.fridge(11.3, -17.0, -FRAC_PI_2, false);
    m.table_set(7.0, -14.0, 0.0, c(0.6, 0.42, 0.25));
    m.cabinets(-11.4, -26.5, FRAC_PI_2, 2);
    m.bed(-7.0, -28.2, 0.0, c(0.3, 0.45, 0.7));
    m.car(1.5, -27.5, FRAC_PI_2, cars[2]);
    m.tires(10.8, -25.0, 3);
    // Back yard: shed, swings, barbecue table.
    m.shed(-8.0, -54.0, 0.0);
    m.playground(4.0, -48.0, 0.0);
    m.table_set(-6.0, -38.0, 0.0, c(0.85, 0.85, 0.82));
    m.oak(9.0, -40.0, 1.0, 1.0);

    // Diner, store and the parking lot.
    m.diner(&Room::new(-29.8, -23.8, -18.2, -8.2, 4.5, &[(-24.0, -8.0), (-18.0, -12.0), (-26.0, -24.0), (-30.0, -11.0)]), 2.0);
    m.kitchen(-15.0, -8.6, PI, 5.0, c(0.75, 0.75, 0.77));
    m.fridge(-12.7, -14.0, -FRAC_PI_2, false);
    m.fridge(-12.7, -15.0, -FRAC_PI_2, false);
    for (i, z) in [-12.0f32, -18.5].into_iter().enumerate() {
        m.shelving(-41.0, z, 0.0, i as f32 + 7.0);
    }
    m.counter(-37.0, -12.5, FRAC_PI_2, 3.0, red);
    m.fridge(-35.0, -22.6, PI, true);
    m.fridge(-36.0, -22.6, PI, true);
    m.crates(-55.0, -11.0, 70.0);
    m.pallet_stack(-51.0, -22.0, 0.2, 71.0);
    for (k, (x, z)) in [(-50.0f32, -34.0f32), (-40.0, -34.0), (-24.0, -42.0), (-50.0, -50.0)].into_iter().enumerate() {
        m.car(x, z, FRAC_PI_2, cars[(k + 2) % cars.len()]);
    }
    m.dumpster(-20.0, -27.0, 0.0, c(0.2, 0.4, 0.25));
    m.trash_cans(-32.0, -27.0);
    m.street_light(-34.0, -40.0, 0.0);
    for i in 0..6 {
        m.ground_paint(v(-55.0 + i as f32 * 6.0, 0.0, -42.0), Vec2::new(0.15, 5.0), 0.0, c(0.9, 0.9, 0.88), 0.012);
    }
    m.ground_paint(v(-35.0, 0.0, -41.0), Vec2::new(46.0, 34.0), 0.0, c(0.22, 0.22, 0.24), 0.005);

    // School: desks, the lab, the library, the gym and the yard.
    for (rx, flip) in [(18.0f32, 0.0f32), (40.0, 0.0)] {
        for (dx, dz) in [(-3.0f32, -15.0f32), (3.0, -15.0), (-3.0, -12.0), (3.0, -12.0)] {
            m.desk(rx + dx, dz, flip);
        }
        let mut a = Art::default();
        a.paint.cuboid(v(rx, 2.0, -8.25), v(4.0, 1.4, 0.06), c(0.15, 0.25, 0.18));
        a.paint.cuboid(v(rx, 1.28, -8.3), v(4.0, 0.06, 0.12), c(0.5, 0.36, 0.22));
        m.place(a, Vec3::ZERO, 0.0);
    }
    m.counter(29.0, -14.0, 0.0, 4.0, c(0.45, 0.3, 0.2));
    for x in [15.5f32, 20.0, 24.5] {
        m.lab_bench(x, -30.0, 0.0, x);
    }
    m.library(&Room::new(28.2, -33.8, 45.8, -22.2, 4.5, &[(37.0, -22.0), (37.0, -34.0)]), 4.0);
    m.bleachers(52.0, -32.0, 0.0);
    m.hoop(52.0, -9.0, PI);
    let mut a = Art::default();
    a.paint.cuboid(v(52.0, 0.025, -21.0), v(10.0, 0.01, 20.0), c(0.75, 0.55, 0.3));
    a.paint.torus(v(52.0, 0.03, -21.0), 0.05, 1.8, Quat::IDENTITY, c(0.95, 0.95, 0.95));
    a.paint.cuboid(v(52.0, 0.03, -21.0), v(10.0, 0.01, 0.1), c(0.95, 0.95, 0.95));
    m.place(a, Vec3::ZERO, 0.0);
    m.school_bus(24.0, -46.0, FRAC_PI_2);
    m.bleachers(40.0, -55.0, 0.0);
    m.hoop(50.0, -46.0, -FRAC_PI_2);
    m.ground_paint(v(44.0, 0.0, -46.0), Vec2::new(14.0, 12.0), 0.0, c(0.55, 0.25, 0.2), 0.006);

    // Police station: front desk, offices, briefing room, cells, cruisers.
    m.counter(-38.0, 12.0, PI, 6.0, c(0.25, 0.3, 0.45));
    m.bench(-44.0, 9.0, PI);
    for (x, z) in [(-52.0f32, 12.0f32), (-52.0, 18.0), (-52.0, 24.0)] {
        m.desk(x, z, FRAC_PI_2);
    }
    m.cabinets(-57.3, 29.0, FRAC_PI_2, 4);
    m.table_set(-38.0, 25.0, 0.0, c(0.6, 0.6, 0.62));
    m.lockers(-43.0, 31.5, PI, 4, c(0.25, 0.3, 0.45));
    for z in [12.0f32, 20.0] {
        m.bed(-21.2, z, -FRAC_PI_2, c(0.45, 0.45, 0.4));
    }
    m.car(-48.0, 38.0, 0.0, c(0.1, 0.1, 0.15));
    m.car(-30.0, 38.0, 0.0, c(0.9, 0.9, 0.92));
    m.street_light(-16.0, 30.0, -FRAC_PI_2);

    // Community centre: lobby, library, games room, kitchen.
    m.counter(-6.0, 11.0, 0.0, 4.0, c(0.45, 0.3, 0.2));
    m.bench(6.0, 18.0, PI);
    m.library(&Room::new(-11.8, 20.2, -0.2, 33.8, 5.0, &[(-6.0, 20.0), (-6.0, 34.0), (0.0, 27.0)]), 5.0);
    m.table_set(6.0, 24.0, 0.0, c(0.15, 0.45, 0.25));
    m.table_set(6.0, 30.5, 0.3, c(0.15, 0.45, 0.25));
    m.vending(11.2, 21.5, -FRAC_PI_2, c(0.15, 0.3, 0.7));
    m.kitchen(-4.0, 43.4, PI, 6.0, c(0.7, 0.7, 0.72));
    m.fridge(-11.3, 40.0, FRAC_PI_2, false);
    m.table_set(4.0, 38.0, 0.0, c(0.85, 0.85, 0.82));

    // Maple Court: furnished houses, a paddling pool, a trampoline.
    m.sofa(25.0, 13.0, 0.0, c(0.55, 0.45, 0.3));
    m.tv(25.0, 20.4, PI);
    m.kitchen(20.5, 29.4, PI, 5.0, c(0.6, 0.45, 0.3));
    m.bed(31.0, 27.0, PI, c(0.7, 0.35, 0.4));
    m.sofa(48.0, 13.0, 0.0, c(0.3, 0.4, 0.35));
    m.bed(42.5, 27.0, PI, c(0.35, 0.5, 0.65));
    m.kitchen(52.5, 29.4, PI, 5.0, c(0.85, 0.85, 0.82));
    m.table_set(52.0, 24.5, 0.0, c(0.6, 0.42, 0.25));
    pond(&mut m, 20.0, 34.0, 30.0, 40.0);
    m.playground(48.0, 37.0, PI);
    m.oak(56.0, 33.0, 1.0, 2.0);
    for x in [14.0f32, 56.0] {
        m.bush(x, 10.0, 0.9, x, Some(c(0.95, 0.4, 0.55)));
    }

    // Back alley: garages, bins, power lines.
    m.ground_paint(v(0.0, 0.0, 51.0), Vec2::new(116.0, 14.0), 0.0, c(0.2, 0.2, 0.22), 0.006);
    let cols = [c(0.75, 0.7, 0.6), c(0.6, 0.65, 0.7), c(0.7, 0.55, 0.45), c(0.65, 0.7, 0.6)];
    for (i, x) in [-50.0f32, -38.0, -26.0, 18.0, 30.0, 52.0].into_iter().enumerate() {
        m.garage(x, 54.8, PI, cols[i % 4]);
    }
    m.dumpster(-14.0, 56.5, 0.0, c(0.25, 0.3, 0.55));
    m.dumpster(42.0, 56.5, 0.0, c(0.2, 0.4, 0.25));
    m.trash_cans(8.0, 56.8);
    m.power_line(-50.0, 50.0, 45.5);
    m.street_light(-30.0, 46.0, 0.0);
    m.street_light(30.0, 46.0, 0.0);

    // Trees beyond the fence.
    for i in 0..48 {
        let ang = i as f32 / 48.0 * std::f32::consts::TAU;
        let r = (63.0 + hash(i as f32, 9.0) * 10.0) / ang.cos().abs().max(ang.sin().abs());
        let (x, z) = (ang.cos() * r, ang.sin() * r);
        if i % 3 == 0 {
            m.oak(x, z, 1.5, i as f32);
        } else {
            m.pine(x, z, 1.6 + hash(i as f32, 1.0), i as f32);
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
                let (ix, iy) = (
                    (px as i32).rem_euclid(N as i32) as usize,
                    (py as i32).rem_euclid(N as i32) as usize,
                );
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
    let woods = [
        Color::srgb(0.42, 0.27, 0.14),
        Color::srgb(0.36, 0.22, 0.11),
        Color::srgb(0.46, 0.3, 0.16),
    ];
    let metal = Color::srgb(0.3, 0.3, 0.32);
    let blue = Color::srgb(0.45, 0.8, 1.0);
    // Planks.
    let rows = 4;
    let ph = (top - bottom) / rows as f32;
    for i in 0..rows {
        let y0 = bottom + i as f32 * ph;
        props::boxr(
            &mut body.paint,
            Vec3::new(-hx, y0 + 0.005, -hz),
            Vec3::new(hx, y0 + ph - 0.005, hz),
            woods[i % 3],
        );
        props::boxr(
            &mut body.paint,
            Vec3::new(-hx - 0.004, y0 + ph - 0.012, -hz - 0.004),
            Vec3::new(hx + 0.004, y0 + ph, hz + 0.004),
            Color::srgb(0.2, 0.12, 0.06),
        );
    }
    // Metal corners and bands.
    for sx in [-1.0f32, 1.0] {
        for sz in [-1.0f32, 1.0] {
            props::boxr(
                &mut body.metal,
                Vec3::new(sx * hx - 0.06, bottom, sz * hz - 0.06),
                Vec3::new(sx * hx + 0.02, top, sz * hz + 0.02),
                metal,
            );
        }
        props::boxr(
            &mut body.metal,
            Vec3::new(sx * 0.55 - 0.05, bottom, -hz - 0.015),
            Vec3::new(sx * 0.55 + 0.05, top, hz + 0.015),
            metal,
        );
        // Handles on the ends.
        body.metal.torus(
            Vec3::new(sx * (hx + 0.03), 0.0, 0.0),
            0.015,
            0.09,
            Quat::from_rotation_z(FRAC_PI_2),
            metal,
        );
    }
    // Glowing "?" on every side.
    for (pos, rot) in [
        (Vec3::new(0.0, -0.05, hz + 0.02), Quat::IDENTITY),
        (
            Vec3::new(0.0, -0.05, -hz - 0.02),
            Quat::from_rotation_y(std::f32::consts::PI),
        ),
        (
            Vec3::new(hx + 0.02, -0.05, 0.0),
            Quat::from_rotation_y(FRAC_PI_2),
        ),
        (
            Vec3::new(-hx - 0.02, -0.05, 0.0),
            Quat::from_rotation_y(-FRAC_PI_2),
        ),
    ] {
        let mut q = Kit::new();
        question_mark(&mut q, 0.42, blue);
        body.glow
            .append(q, Transform::from_translation(pos).with_rotation(rot));
    }
    // The glow inside, seen when the lid opens.
    body.glow.cuboid(
        Vec3::new(0.0, top - 0.02, 0.0),
        Vec3::new(hx * 1.9, 0.02, hz * 1.8),
        Color::srgb(0.75, 0.92, 1.0),
    );
    // Lid (pivot at the back top edge).
    let d = hz * 2.0;
    props::boxr(
        &mut lid.paint,
        Vec3::new(-hx - 0.02, 0.0, 0.0),
        Vec3::new(hx + 0.02, 0.18, d + 0.02),
        woods[0],
    );
    for i in 0..4 {
        let z = 0.02 + i as f32 * d / 4.0;
        props::boxr(
            &mut lid.paint,
            Vec3::new(-hx - 0.024, 0.17, z),
            Vec3::new(hx + 0.024, 0.19, z + 0.01),
            Color::srgb(0.2, 0.12, 0.06),
        );
    }
    for sx in [-1.0f32, 1.0] {
        props::boxr(
            &mut lid.metal,
            Vec3::new(sx * 0.55 - 0.05, -0.005, -0.01),
            Vec3::new(sx * 0.55 + 0.05, 0.195, d + 0.03),
            metal,
        );
        props::boxr(
            &mut lid.metal,
            Vec3::new(sx * hx - 0.08, -0.005, -0.01),
            Vec3::new(sx * hx + 0.03, 0.195, d + 0.03),
            metal,
        );
    }
    let mut q = Kit::new();
    question_mark(&mut q, 0.5, blue);
    lid.glow.append(
        q,
        Transform::from_xyz(0.0, 0.2, d / 2.0).with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
    );
    lid.metal.cuboid(
        Vec3::new(0.0, 0.06, d + 0.03),
        Vec3::new(0.2, 0.08, 0.04),
        metal,
    );
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
    props::boxr(
        &mut a.paint,
        Vec3::new(-0.65, 0.0, -0.52),
        Vec3::new(0.65, 0.12, 0.52),
        black,
    );
    props::boxr(
        &mut a.paint,
        Vec3::new(-0.6, 0.12, -0.5),
        Vec3::new(0.6, 2.3, 0.5),
        body,
    );
    for sx in [-1.0f32, 1.0] {
        props::boxr(
            &mut a.paint,
            Vec3::new(sx * 0.6 - 0.01, 0.3, -0.2),
            Vec3::new(sx * 0.6 + 0.01, 2.1, 0.2),
            col,
        );
        props::boxr(
            &mut a.metal,
            Vec3::new(sx * 0.6 - 0.03, 0.12, 0.47),
            Vec3::new(sx * 0.6 + 0.03, 2.3, 0.53),
            chrome,
        );
    }
    // Lit sign on top with an emblem.
    props::boxr(
        &mut a.glow,
        Vec3::new(-0.6, 2.33, -0.48),
        Vec3::new(0.6, 2.72, 0.5),
        col,
    );
    props::boxr(
        &mut a.metal,
        Vec3::new(-0.64, 2.3, -0.52),
        Vec3::new(0.64, 2.34, 0.54),
        chrome,
    );
    props::boxr(
        &mut a.metal,
        Vec3::new(-0.64, 2.72, -0.52),
        Vec3::new(0.64, 2.76, 0.54),
        chrome,
    );
    a.glow.cyl(
        Vec3::new(0.0, 2.52, 0.51),
        0.15,
        0.02,
        Quat::from_rotation_x(FRAC_PI_2),
        Color::WHITE,
    );
    a.glow.cyl(
        Vec3::new(0.0, 2.52, 0.525),
        0.1,
        0.02,
        Quat::from_rotation_x(FRAC_PI_2),
        col,
    );
    // Window full of bottles, standing out from the cabinet front.
    props::boxr(
        &mut a.paint,
        Vec3::new(-0.54, 0.88, 0.5),
        Vec3::new(0.26, 2.14, 0.52),
        Color::srgb(0.15, 0.15, 0.17),
    );
    for sx in [-0.54f32, 0.26] {
        props::boxr(
            &mut a.metal,
            Vec3::new(sx - 0.025, 0.88, 0.5),
            Vec3::new(sx + 0.025, 2.14, 0.66),
            chrome,
        );
    }
    for y in [0.88f32, 2.14] {
        props::boxr(
            &mut a.metal,
            Vec3::new(-0.565, y - 0.025, 0.5),
            Vec3::new(0.285, y + 0.025, 0.66),
            chrome,
        );
    }
    for row in 0..3 {
        let y = 1.0 + row as f32 * 0.38;
        props::boxr(
            &mut a.metal,
            Vec3::new(-0.52, y - 0.02, 0.52),
            Vec3::new(0.24, y, 0.64),
            chrome,
        );
        for i in 0..4 {
            let x = -0.42 + i as f32 * 0.18;
            a.glow.cyl(
                Vec3::new(x, y + 0.12, 0.58),
                0.045,
                0.2,
                Quat::IDENTITY,
                col,
            );
            a.glow.cyl(
                Vec3::new(x, y + 0.26, 0.58),
                0.018,
                0.08,
                Quat::IDENTITY,
                Color::WHITE,
            );
        }
    }
    a.glass.cuboid(
        Vec3::new(-0.14, 1.51, 0.655),
        Vec3::new(0.78, 1.24, 0.01),
        Color::srgb(0.75, 0.85, 0.95),
    );
    // Buttons, screen and coin slot.
    a.glow.cuboid(
        Vec3::new(0.42, 1.9, 0.505),
        Vec3::new(0.18, 0.12, 0.02),
        Color::srgb(0.85, 1.0, 0.9),
    );
    for row in 0..4 {
        for c in 0..2 {
            a.glow.cuboid(
                Vec3::new(0.37 + c as f32 * 0.1, 1.65 - row as f32 * 0.1, 0.505),
                Vec3::new(0.06, 0.05, 0.02),
                if (row + c) % 2 == 0 {
                    col
                } else {
                    Color::WHITE
                },
            );
        }
    }
    props::boxr(
        &mut a.metal,
        Vec3::new(0.38, 1.1, 0.5),
        Vec3::new(0.46, 1.25, 0.52),
        chrome,
    );
    a.paint.cuboid(
        Vec3::new(0.42, 1.18, 0.525),
        Vec3::new(0.012, 0.08, 0.01),
        black,
    );
    // Dispenser.
    props::boxr(
        &mut a.paint,
        Vec3::new(-0.45, 0.25, 0.48),
        Vec3::new(0.2, 0.62, 0.52),
        black,
    );
    props::boxr(
        &mut a.metal,
        Vec3::new(-0.43, 0.42, 0.51),
        Vec3::new(0.18, 0.6, 0.53),
        Color::srgb(0.25, 0.25, 0.27),
    );
    a
}

/// `art_meshes` marker: outlines on the solid parts of a prop.
fn outline_solid(e: &mut EntityCommands, solid: bool) {
    if solid {
        e.insert(crate::outline::Outline::Prop);
    }
}

fn art_meshes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &[Handle<StandardMaterial>; 4],
    art: Art,
    parent: Option<Entity>,
    marker: impl Fn(&mut EntityCommands, bool),
) {
    for (i, (kit, mat)) in [art.paint, art.metal, art.glow, art.glass]
        .into_iter()
        .zip(mats.iter())
        .enumerate()
    {
        if let Some(mesh) = kit.build() {
            let mut e = commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(mat.clone()),
                Transform::default(),
            ));
            // The marker also learns if this is a solid part (not glow or glass).
            marker(&mut e, i < 2);
            if let Some(p) = parent {
                let id = e.id();
                commands.entity(p).add_child(id);
            } else {
                e.insert(InGameEntity);
            }
        }
    }
}

/// Daytime sun strength over each map's own value, and the cool fill
/// light's share (the ambient light is much lower than before v8).
const SUN_BOOST: f32 = 0.7;
const FILL: f32 = 0.14;

pub fn spawn_map(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    map: u8,
    night: bool,
) -> MapLayout {
    let mut layout = layout(map);
    if night {
        layout.sky = Color::srgb(0.025, 0.03, 0.065);
    }
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
    art_meshes(commands, meshes, &mats, art, None, |_, _| {});
    let props = std::mem::take(&mut layout.prop_art);
    art_meshes(commands, meshes, &mats, props, None, outline_solid);
    // At night the lamps do the work: brighter and reaching further.
    let (lamp, reach) = if night { (3.0, 24.0) } else { (1.0, 18.0) };
    for (pos, color, intensity) in &layout.lights {
        commands.spawn((
            InGameEntity,
            PointLight {
                intensity: *intensity * lamp,
                color: *color,
                range: reach,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(*pos),
        ));
    }

    // Sun by day, a pale moon by night. The sun is a little warm; a weak
    // cool light from the opposite side of the sky fills the shadows.
    let (sun, sun_color) = if night {
        (layout.sun * 0.06, Color::srgb(0.6, 0.7, 1.0))
    } else {
        (layout.sun * SUN_BOOST, Color::srgb(1.0, 0.95, 0.86))
    };
    let sun_pos = Vec3::new(20.0, 40.0, 15.0);
    commands.spawn((
        InGameEntity,
        DirectionalLight {
            illuminance: sun,
            color: sun_color,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(sun_pos).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    if !night {
        commands.spawn((
            InGameEntity,
            crate::graphics::FillLight,
            DirectionalLight {
                illuminance: layout.sun * FILL,
                color: Color::srgb(0.82, 0.87, 1.0),
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(-20.0, 30.0, -15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
    }
    crate::graphics::spawn_sky(commands, meshes, materials, layout.sky, night, sun_pos);

    // Perk machines, turned the way the map says.
    for ((spot, yaw), perk) in layout
        .perk_spots
        .iter()
        .zip(layout.perk_yaws)
        .zip(Perk::ALL)
    {
        let quarter = (yaw / FRAC_PI_2).round();
        let body = if (quarter as i32).rem_euclid(2) == 1 {
            Vec3::new(1.0, 2.4, 1.2)
        } else {
            Vec3::new(1.2, 2.4, 1.0)
        };
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
        art_meshes(
            commands,
            meshes,
            &mats,
            perk_art(perk),
            Some(holder),
            outline_solid,
        );
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
    art_meshes(commands, meshes, &mats, body, Some(root), outline_solid);
    let hinge = commands
        .spawn((
            BoxLid,
            Transform::from_xyz(0.0, BOX_HALF.y - 0.18, -BOX_HALF.z),
            Visibility::default(),
        ))
        .id();
    commands.entity(root).add_child(hinge);
    art_meshes(commands, meshes, &mats, lid, Some(hinge), outline_solid);
    let extras = [
        // A wide soft beam and a bright core, tall enough to see from
        // anywhere on the map.
        commands
            .spawn((
                BoxPillar,
                Mesh3d(meshes.add(Cylinder::new(0.75, 120.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(0.35, 0.7, 1.0, 0.28),
                    emissive: LinearRgba::rgb(0.6, 1.4, 3.0),
                    alpha_mode: AlphaMode::Add,
                    unlit: true,
                    ..default()
                })),
                Transform::from_xyz(0.0, 60.5, 0.0),
                bevy::pbr::NotShadowCaster,
            ))
            .id(),
        commands
            .spawn((
                BoxPillar,
                Mesh3d(meshes.add(Cylinder::new(0.2, 120.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(0.8, 0.95, 1.0, 0.9),
                    emissive: LinearRgba::rgb(2.0, 4.0, 8.0),
                    alpha_mode: AlphaMode::Add,
                    unlit: true,
                    ..default()
                })),
                Transform::from_xyz(0.0, 60.5, 0.0),
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
