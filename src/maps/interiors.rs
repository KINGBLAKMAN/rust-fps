//! The big buildings in the corners of each map. Each one joins the two
//! outer areas beside it: walk in from one area, through the rooms and out
//! into the other. The way through is blocked in the middle until someone
//! buys it open, which opens both areas.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::kit::c;
use crate::maps::{Blocker, DoorDef, MapLayout, DOOR_WIDTH, INNER, OUTER};
use crate::props::{boxr, hash, shade, v, Art};

/// What's inside a corner building.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Interior {
    Warehouse,
    MachineShop,
    Library,
    Museum,
    Mall,
    Diner,
}

impl Interior {
    fn name(self) -> &'static str {
        match self {
            Interior::Warehouse => "the Cold Store",
            Interior::MachineShop => "the Machine Shop",
            Interior::Library => "the Library",
            Interior::Museum => "the Museum",
            Interior::Mall => "the Mall",
            Interior::Diner => "the Diner",
        }
    }

    fn blocker(self) -> Blocker {
        match self {
            Interior::Warehouse | Interior::MachineShop => Blocker::Crates,
            Interior::Library => Blocker::Books,
            Interior::Museum => Blocker::Planks,
            Interior::Mall | Interior::Diner => Blocker::Furniture,
        }
    }

    /// Floor, lower wall and upper wall colours.
    fn colors(self) -> (Color, Color, Color) {
        match self {
            Interior::Warehouse => (c(0.42, 0.42, 0.43), c(0.55, 0.57, 0.6), c(0.68, 0.7, 0.72)),
            Interior::MachineShop => (c(0.35, 0.36, 0.34), c(0.3, 0.42, 0.35), c(0.62, 0.64, 0.6)),
            Interior::Library => (c(0.42, 0.12, 0.1), c(0.33, 0.2, 0.12), c(0.85, 0.8, 0.68)),
            Interior::Museum => (c(0.82, 0.8, 0.76), c(0.7, 0.68, 0.64), c(0.92, 0.9, 0.86)),
            Interior::Mall => (c(0.85, 0.83, 0.8), c(0.6, 0.62, 0.66), c(0.9, 0.9, 0.9)),
            Interior::Diner => (c(0.9, 0.9, 0.88), c(0.2, 0.55, 0.55), c(0.95, 0.93, 0.88)),
        }
    }

    /// Lamp colour inside.
    fn light(self) -> Color {
        match self {
            Interior::Warehouse | Interior::MachineShop => c(0.9, 0.95, 1.0),
            Interior::Mall | Interior::Museum => c(1.0, 0.97, 0.9),
            _ => c(1.0, 0.82, 0.55),
        }
    }
}

/// Height of the rooms inside.
const ROOM_H: f32 = 5.0;
/// Width and height of the openings in the outside walls.
const OPEN_W: f32 = 4.0;
const OPEN_H: f32 = 3.8;
const WALL: f32 = 0.4;

/// A room inside: x and z ranges, and spots to keep clear (doorways).
struct Room {
    x0: f32,
    x1: f32,
    z0: f32,
    z1: f32,
    keep_clear: Vec<Vec3>,
}

impl Room {
    fn free(&self, p: Vec3, r: f32) -> bool {
        p.x - r > self.x0 + 0.3
            && p.x + r < self.x1 - 0.3
            && p.z - r > self.z0 + 0.3
            && p.z + r < self.z1 - 0.3
            && self
                .keep_clear
                .iter()
                .all(|d| d.with_y(0.0).distance(p.with_y(0.0)) > r + 2.6)
    }
    fn center(&self) -> Vec3 {
        v((self.x0 + self.x1) / 2.0, 0.0, (self.z0 + self.z1) / 2.0)
    }
}

impl MapLayout {
    /// A corner building at (`x`, `z`). `style` is the map (0 yard, 1 park,
    /// 2 neighbourhood) and `which` picks between its two kinds of building.
    pub(crate) fn corner_block(&mut self, x: f32, z: f32, style: u8, seed: f32, which: usize) {
        let inside = match (style, which % 2) {
            (0, 0) => Interior::Warehouse,
            (0, _) => Interior::MachineShop,
            (1, 0) => Interior::Library,
            (1, _) => Interior::Museum,
            (_, 0) => Interior::Mall,
            _ => Interior::Diner,
        };
        let size = OUTER - INNER - 1.0;
        let hs = size / 2.0;
        let (sx, sz) = (x.signum(), z.signum());
        let height = match style {
            0 => 9.0,
            1 => 16.0 + hash(seed, 3.0) * 8.0,
            _ => 7.0,
        };
        let (wall, trim) = match style {
            0 => (
                c(0.48 + hash(seed, 1.0) * 0.15, 0.5, 0.52),
                c(0.75, 0.55, 0.1),
            ),
            1 => (
                c(0.55 + hash(seed, 2.0) * 0.15, 0.3, 0.24),
                c(0.82, 0.8, 0.75),
            ),
            _ => (
                c(0.75, 0.68 - hash(seed, 4.0) * 0.15, 0.55),
                c(0.3, 0.32, 0.36),
            ),
        };
        // Face A looks into the north/south area, face B into the east/west
        // one. Room A is the half behind face A, room B the half behind B.
        let face_a = x - sx * hs;
        let face_b = z - sz * hs;
        let open_a = v(face_a, 0.0, z);
        let open_b = v(x + sx * hs / 2.0, 0.0, face_b);
        let door = v(x, 0.0, z + sz * 2.0);
        let zone_ns = if sz > 0.0 { 1 } else { 2 };
        let zone_ew = if sx > 0.0 { 3 } else { 4 };

        let mut a = Art::default();
        // Outside walls with their openings, the partition with its door,
        // the roof and the ceiling inside.
        let (x0, x1) = (x - hs, x + hs);
        let (z0, z1) = (z - hs, z + hs);
        self.wall_z(&mut a, face_a, z0, z1, height, wall, Some(z));
        self.wall_z(&mut a, x + sx * hs, z0, z1, height, wall, None);
        self.wall_x(&mut a, face_b, x0, x1, height, wall, Some(open_b.x));
        self.wall_x(&mut a, z + sz * hs, x0, x1, height, wall, None);
        let (_, lower, upper) = inside.colors();
        self.partition(&mut a, x, z0, z1, door.z, lower);
        boxr(
            &mut a.paint,
            v(x0 - 0.15, height, z0 - 0.15),
            v(x1 + 0.15, height + 0.4, z1 + 0.15),
            trim,
        );
        boxr(
            &mut a.paint,
            v(x0 + WALL / 2.0, ROOM_H, z0 + WALL / 2.0),
            v(x1 - WALL / 2.0, ROOM_H + 0.3, z1 - WALL / 2.0),
            shade(upper, 0.85),
        );
        // A darker plinth round the outside, broken at the openings.
        let plinth = shade(wall, 0.7);
        for (p0, p1) in [
            (v(x0 - 0.1, 0.0, z0 - 0.1), v(x1 + 0.1, 0.6, z0 + 0.0)),
            (v(x0 - 0.1, 0.0, z1 - 0.0), v(x1 + 0.1, 0.6, z1 + 0.1)),
            (v(x0 - 0.1, 0.0, z0 - 0.1), v(x0 + 0.0, 0.6, z1 + 0.1)),
            (v(x1 - 0.0, 0.0, z0 - 0.1), v(x1 + 0.1, 0.6, z1 + 0.1)),
        ] {
            let along_x = (p1.x - p0.x) > 1.0;
            let gap = if along_x && (p0.z - face_b).abs() < 0.2
                || along_x && (p1.z - face_b).abs() < 0.2
            {
                Some(open_b.x)
            } else if !along_x && ((p0.x - face_a).abs() < 0.2 || (p1.x - face_a).abs() < 0.2) {
                Some(z)
            } else {
                None
            };
            match gap {
                Some(g) if along_x => {
                    boxr(&mut a.paint, p0, v(g - OPEN_W / 2.0, p1.y, p1.z), plinth);
                    boxr(&mut a.paint, v(g + OPEN_W / 2.0, p0.y, p0.z), p1, plinth);
                }
                Some(g) => {
                    boxr(&mut a.paint, p0, v(p1.x, p1.y, g - OPEN_W / 2.0), plinth);
                    boxr(&mut a.paint, v(p0.x, p0.y, g + OPEN_W / 2.0), p1, plinth);
                }
                None => boxr(&mut a.paint, p0, p1, plinth),
            }
        }

        // Detail on the two faces that look into the play areas, kept clear
        // of the openings, and a lit entrance at each opening.
        // (The shell above is in world space; the detail is built around
        // the building's centre.)
        self.place(a, Vec3::ZERO, 0.0);
        let mut d = Art::default();
        for (n, gap) in [
            (v(-sx, 0.0, 0.0), 0.0f32),
            (v(0.0, 0.0, -sz), sx * hs / 2.0),
        ] {
            facade(&mut d, n, hs, size, height, style, wall, trim, seed, gap);
        }
        let ents = [(open_a, v(-sx, 0.0, 0.0)), (open_b, v(0.0, 0.0, -sz))];
        for (p, n) in ents {
            entrance(&mut d, p - v(x, 0.0, z), n, inside, trim);
            self.light(p + n * 1.2 + Vec3::Y * 3.6, inside.light(), 30_000.0);
        }
        self.place(d, v(x, 0.0, z), 0.0);

        // Inside: the two rooms, furnished.
        let in_x0 = x0 + WALL;
        let in_x1 = x1 - WALL;
        let in_z0 = z0 + WALL;
        let in_z1 = z1 - WALL;
        let (ax0, ax1) = if sx > 0.0 {
            (in_x0, x - WALL / 2.0)
        } else {
            (x + WALL / 2.0, in_x1)
        };
        let (bx0, bx1) = if sx > 0.0 {
            (x + WALL / 2.0, in_x1)
        } else {
            (in_x0, x - WALL / 2.0)
        };
        let clear = vec![open_a, open_b, door];
        let rooms = [
            Room {
                x0: ax0,
                x1: ax1,
                z0: in_z0,
                z1: in_z1,
                keep_clear: clear.clone(),
            },
            Room {
                x0: bx0,
                x1: bx1,
                z0: in_z0,
                z1: in_z1,
                keep_clear: clear,
            },
        ];
        self.room_shell(&rooms, inside);
        for (i, room) in rooms.iter().enumerate() {
            self.furnish(room, inside, seed + i as f32 * 17.0);
            let c0 = room.center();
            self.light(c0 + v(0.0, ROOM_H - 0.6, -2.5), inside.light(), 60_000.0);
            self.light(c0 + v(0.0, ROOM_H - 0.6, 2.5), inside.light(), 60_000.0);
        }

        self.doors.push(DoorDef {
            pos: door,
            along_x: false,
            cost: 1250,
            zone: zone_ns,
            zone2: zone_ew,
            name: inside.name(),
            blocker: inside.blocker(),
        });
    }

    /// An outside wall along Z at `x`, with an optional opening centred at
    /// `gap` (a lintel spans it).
    fn wall_z(
        &mut self,
        a: &mut Art,
        x: f32,
        z0: f32,
        z1: f32,
        h: f32,
        col: Color,
        gap: Option<f32>,
    ) {
        let mut parts = vec![(z0, z1)];
        if let Some(g) = gap {
            parts = vec![(z0, g - OPEN_W / 2.0), (g + OPEN_W / 2.0, z1)];
            boxr(
                &mut a.paint,
                v(x - WALL / 2.0, OPEN_H, g - OPEN_W / 2.0),
                v(x + WALL / 2.0, h, g + OPEN_W / 2.0),
                col,
            );
            self.collide(v(x, (OPEN_H + h) / 2.0, g), v(WALL, h - OPEN_H, OPEN_W));
        }
        for (s, e) in parts {
            boxr(
                &mut a.paint,
                v(x - WALL / 2.0, 0.0, s),
                v(x + WALL / 2.0, h, e),
                col,
            );
            self.collide(v(x, h / 2.0, (s + e) / 2.0), v(WALL, h, e - s));
        }
    }

    /// An outside wall along X at `z` (see `wall_z`).
    fn wall_x(
        &mut self,
        a: &mut Art,
        z: f32,
        x0: f32,
        x1: f32,
        h: f32,
        col: Color,
        gap: Option<f32>,
    ) {
        let mut parts = vec![(x0, x1)];
        if let Some(g) = gap {
            parts = vec![(x0, g - OPEN_W / 2.0), (g + OPEN_W / 2.0, x1)];
            boxr(
                &mut a.paint,
                v(g - OPEN_W / 2.0, OPEN_H, z - WALL / 2.0),
                v(g + OPEN_W / 2.0, h, z + WALL / 2.0),
                col,
            );
            self.collide(v(g, (OPEN_H + h) / 2.0, z), v(OPEN_W, h - OPEN_H, WALL));
        }
        for (s, e) in parts {
            boxr(
                &mut a.paint,
                v(s, 0.0, z - WALL / 2.0),
                v(e, h, z + WALL / 2.0),
                col,
            );
            self.collide(v((s + e) / 2.0, h / 2.0, z), v(e - s, h, WALL));
        }
    }

    /// The wall between the two rooms, with the bought doorway in it.
    fn partition(&mut self, a: &mut Art, x: f32, z0: f32, z1: f32, door_z: f32, col: Color) {
        let half = DOOR_WIDTH / 2.0 + 0.3;
        for (s, e) in [(z0, door_z - half), (door_z + half, z1)] {
            boxr(
                &mut a.paint,
                v(x - WALL / 2.0, 0.0, s),
                v(x + WALL / 2.0, ROOM_H, e),
                col,
            );
            self.collide(v(x, ROOM_H / 2.0, (s + e) / 2.0), v(WALL, ROOM_H, e - s));
        }
        boxr(
            &mut a.paint,
            v(x - WALL / 2.0, 3.6, door_z - half),
            v(x + WALL / 2.0, ROOM_H, door_z + half),
            col,
        );
        // Door frame.
        for s in [-1.0f32, 1.0] {
            boxr(
                &mut a.paint,
                v(x - 0.3, 0.0, door_z + s * half - 0.15),
                v(x + 0.3, 3.7, door_z + s * half + 0.15),
                shade(col, 0.7),
            );
        }
        boxr(
            &mut a.paint,
            v(x - 0.3, 3.6, door_z - half - 0.15),
            v(x + 0.3, 3.85, door_z + half + 0.15),
            shade(col, 0.7),
        );
    }

    /// Floors and the inside faces of the walls, two-tone.
    fn room_shell(&mut self, rooms: &[Room], inside: Interior) {
        let (floor, lower, upper) = inside.colors();
        let mut a = Art::default();
        for r in rooms {
            boxr(&mut a.paint, v(r.x0, 0.0, r.z0), v(r.x1, 0.03, r.z1), floor);
            // Floor pattern.
            match inside {
                Interior::Diner => {
                    let mut i = 0;
                    let mut x = r.x0;
                    while x < r.x1 - 0.1 {
                        let mut z = r.z0;
                        let mut j = 0;
                        while z < r.z1 - 0.1 {
                            if (i + j) % 2 == 0 {
                                boxr(
                                    &mut a.paint,
                                    v(x, 0.031, z),
                                    v((x + 0.8).min(r.x1), 0.035, (z + 0.8).min(r.z1)),
                                    c(0.12, 0.12, 0.13),
                                );
                            }
                            z += 0.8;
                            j += 1;
                        }
                        x += 0.8;
                        i += 1;
                    }
                }
                Interior::Library => {
                    // A rug down the middle.
                    let m = r.center();
                    boxr(
                        &mut a.paint,
                        v(m.x - 1.4, 0.031, r.z0 + 1.0),
                        v(m.x + 1.4, 0.04, r.z1 - 1.0),
                        c(0.55, 0.38, 0.15),
                    );
                    boxr(
                        &mut a.paint,
                        v(m.x - 1.2, 0.04, r.z0 + 1.2),
                        v(m.x + 1.2, 0.045, r.z1 - 1.2),
                        c(0.45, 0.1, 0.1),
                    );
                }
                Interior::Mall | Interior::Museum => {
                    let mut x = r.x0 + 1.5;
                    while x < r.x1 {
                        boxr(
                            &mut a.paint,
                            v(x - 0.02, 0.031, r.z0),
                            v(x + 0.02, 0.034, r.z1),
                            shade(floor, 0.85),
                        );
                        x += 1.5;
                    }
                    let mut z = r.z0 + 1.5;
                    while z < r.z1 {
                        boxr(
                            &mut a.paint,
                            v(r.x0, 0.031, z - 0.02),
                            v(r.x1, 0.034, z + 0.02),
                            shade(floor, 0.85),
                        );
                        z += 1.5;
                    }
                }
                _ => {
                    // Painted safety lines.
                    let m = r.center();
                    for s in [-1.0f32, 1.0] {
                        boxr(
                            &mut a.paint,
                            v(m.x + s * 1.6 - 0.06, 0.031, r.z0 + 0.6),
                            v(m.x + s * 1.6 + 0.06, 0.034, r.z1 - 0.6),
                            c(0.9, 0.75, 0.15),
                        );
                    }
                }
            }
            // Two-tone skin on the walls, cut away round the doorways.
            let t = 0.03;
            let gaps = |along_x: bool, line: f32, lo: f32, hi: f32| -> Vec<(f32, f32)> {
                let mut cuts: Vec<f32> = r
                    .keep_clear
                    .iter()
                    .filter(|p| {
                        if along_x {
                            (p.z - line).abs() < 0.8
                        } else {
                            (p.x - line).abs() < 0.8
                        }
                    })
                    .map(|p| if along_x { p.x } else { p.z })
                    .filter(|c| *c > lo && *c < hi)
                    .collect();
                cuts.sort_by(f32::total_cmp);
                let mut out = Vec::new();
                let mut start = lo;
                for c in cuts {
                    out.push((start, c - OPEN_W / 2.0 - 0.3));
                    start = c + OPEN_W / 2.0 + 0.3;
                }
                out.push((start, hi));
                out
            };
            for (along_x, line, inward) in [
                (true, r.z0, t),
                (true, r.z1, -t),
                (false, r.x0, t),
                (false, r.x1, -t),
            ] {
                let (lo, hi) = if along_x { (r.x0, r.x1) } else { (r.z0, r.z1) };
                let parts = gaps(along_x, line, lo, hi);
                let mut spans: Vec<(f32, f32, f32, f32)> =
                    parts.iter().map(|(a, b)| (*a, *b, 0.0, ROOM_H)).collect();
                // Above each doorway.
                for w in parts.windows(2) {
                    spans.push((w[0].1, w[1].0, OPEN_H, ROOM_H));
                }
                for (a0, a1, y0, y1) in spans {
                    if a1 - a0 < 0.05 {
                        continue;
                    }
                    for (b0, b1, col) in
                        [(y0, y0.max(1.2).min(y1), lower), (y0.max(1.2), y1, upper)]
                    {
                        if b1 - b0 < 0.01 {
                            continue;
                        }
                        let (p0, p1) = if along_x {
                            (v(a0, b0, line), v(a1, b1, line + inward))
                        } else {
                            (v(line, b0, a0), v(line + inward, b1, a1))
                        };
                        boxr(&mut a.paint, p0.min(p1), p0.max(p1), col);
                    }
                }
            }
            // Ceiling lights.
            let m = r.center();
            for dz in [-2.5f32, 2.5] {
                a.glow.cuboid(
                    m + v(0.0, ROOM_H - 0.05, dz),
                    v(1.6, 0.08, 0.4),
                    c(1.0, 0.95, 0.85),
                );
            }
        }
        self.place(a, Vec3::ZERO, 0.0);
    }

    fn furnish(&mut self, room: &Room, inside: Interior, seed: f32) {
        match inside {
            Interior::Warehouse => {
                let m = room.center();
                for dz in [-5.0f32, -1.0, 3.0, 6.5] {
                    let p = v(m.x, 0.0, m.z + dz);
                    if room.free(p, 2.2) {
                        self.shelving(p.x, p.z, FRAC_PI_2 * 0.0, seed + dz);
                    }
                }
                for (i, (dx, dz)) in [(-2.0f32, 0.5f32), (2.2, -3.0), (-2.4, 5.5)]
                    .into_iter()
                    .enumerate()
                {
                    let p = v(m.x + dx, 0.0, m.z + dz);
                    if room.free(p, 1.0) {
                        self.pallet_stack(p.x, p.z, hash(seed, i as f32), seed + i as f32);
                    }
                }
            }
            Interior::MachineShop => self.machine_shop(room, seed),
            Interior::Library => self.library(room, seed),
            Interior::Museum => self.museum(room, seed),
            Interior::Mall => self.mall(room, seed),
            Interior::Diner => self.diner(room, seed),
        }
    }

    // -----------------------------------------------------------------------
    // Furniture
    // -----------------------------------------------------------------------

    fn library(&mut self, room: &Room, seed: f32) {
        let m = room.center();
        let wood = c(0.36, 0.22, 0.13);
        // Rows of bookcases across the room, an aisle down the middle.
        let mut zz = room.z0 + 1.6;
        while zz < room.z1 - 1.2 {
            for side in [-1.0f32, 1.0] {
                let p = v(m.x + side * 2.1, 0.0, zz);
                if !room.free(p, 1.0) {
                    continue;
                }
                let mut a = Art::default();
                bookcase(&mut a, 2.4, seed + zz + side);
                a.paint
                    .cuboid(v(0.0, 1.25, 0.0), v(2.5, 2.5, 0.12), shade(wood, 0.8));
                let mut b = Art::default();
                bookcase(&mut b, 2.4, seed + zz - side);
                a.append(b, Transform::from_rotation(Quat::from_rotation_y(PI)));
                self.place(a, p, 0.0);
                self.collide(p + v(0.0, 1.25, 0.0), v(2.5, 2.5, 0.8));
            }
            zz += 2.6;
        }
        // Reading tables with green lamps in the aisle ends.
        for dz in [-6.0f32, 6.0] {
            let p = m + v(0.0, 0.0, dz);
            if room.free(p, 1.0) {
                let mut a = Art::default();
                boxr(&mut a.paint, v(-0.9, 0.72, -0.5), v(0.9, 0.78, 0.5), wood);
                for (lx, lz) in [(-0.8f32, -0.4f32), (0.8, -0.4), (-0.8, 0.4), (0.8, 0.4)] {
                    a.paint
                        .cuboid(v(lx, 0.36, lz), v(0.08, 0.72, 0.08), shade(wood, 0.8));
                }
                for s in [-0.45f32, 0.45] {
                    a.metal.cyl(
                        v(s, 0.85, 0.0),
                        0.015,
                        0.14,
                        Quat::IDENTITY,
                        c(0.7, 0.6, 0.3),
                    );
                    a.glass
                        .cuboid(v(s, 0.95, 0.0), v(0.3, 0.06, 0.14), c(0.1, 0.5, 0.25));
                    a.glow
                        .cuboid(v(s, 0.91, 0.0), v(0.26, 0.02, 0.1), c(1.0, 0.9, 0.6));
                }
                self.place(a, p, 0.0);
                self.collide(p + v(0.0, 0.4, 0.0), v(1.8, 0.8, 1.0));
            }
        }
    }

    fn museum(&mut self, room: &Room, seed: f32) {
        let m = room.center();
        let marble = c(0.88, 0.86, 0.82);
        // Plinths with exhibits, glass cases, paintings on the walls.
        let mut i = 0;
        for dz in [-5.5f32, -2.0, 2.0, 5.5] {
            for side in [-1.0f32, 1.0] {
                let p = v(m.x + side * 1.8, 0.0, m.z + dz);
                if !room.free(p, 0.7) {
                    continue;
                }
                let mut a = Art::default();
                boxr(&mut a.paint, v(-0.4, 0.0, -0.4), v(0.4, 1.0, 0.4), marble);
                match i % 3 {
                    0 => {
                        // A vase.
                        a.paint
                            .blob(v(0.0, 1.3, 0.0), v(0.2, 0.28, 0.2), c(0.2, 0.35, 0.6));
                        a.paint.cyl(
                            v(0.0, 1.6, 0.0),
                            0.08,
                            0.12,
                            Quat::IDENTITY,
                            c(0.2, 0.35, 0.6),
                        );
                    }
                    1 => {
                        // A bust.
                        a.paint
                            .blob(v(0.0, 1.55, 0.0), v(0.13, 0.17, 0.15), c(0.8, 0.78, 0.72));
                        a.paint
                            .blob(v(0.0, 1.22, 0.0), v(0.25, 0.14, 0.15), c(0.8, 0.78, 0.72));
                    }
                    _ => {
                        // Gold idol in a glass case.
                        a.metal
                            .blob(v(0.0, 1.3, 0.0), v(0.1, 0.22, 0.1), c(0.9, 0.7, 0.2));
                        a.glass
                            .cuboid(v(0.0, 1.4, 0.0), v(0.7, 0.8, 0.7), c(0.6, 0.75, 0.85));
                    }
                }
                self.place(a, p, 0.0);
                self.collide(p + v(0.0, 0.7, 0.0), v(0.8, 1.4, 0.8));
                i += 1;
            }
        }
        // Velvet ropes along the aisle.
        let mut a = Art::default();
        for side in [-1.0f32, 1.0] {
            let mut z = room.z0 + 1.0;
            while z < room.z1 - 1.0 {
                let p = v(m.x + side * 0.9, 0.0, z);
                if room.free(p, 0.1) {
                    a.metal.cyl(
                        p + v(0.0, 0.5, 0.0),
                        0.03,
                        1.0,
                        Quat::IDENTITY,
                        c(0.85, 0.7, 0.25),
                    );
                    a.paint.cyl_between(
                        p + v(0.0, 0.85, 0.0),
                        p + v(0.0, 0.8, 1.5),
                        0.025,
                        c(0.6, 0.05, 0.1),
                    );
                }
                z += 1.5;
            }
        }
        // Paintings: framed glowing canvases on the long walls.
        for (k, z) in [room.z0 + 0.05, room.z1 - 0.05].into_iter().enumerate() {
            let mut x = room.x0 + 1.5;
            let mut j = 0;
            while x < room.x1 - 1.2 {
                let p = v(x, 2.4, z);
                if room.free(v(x, 0.0, z + if k == 0 { 1.0 } else { -1.0 }), 0.0) {
                    a.paint.cuboid(p, v(1.4, 1.1, 0.06), c(0.7, 0.55, 0.2));
                    let art = [
                        c(0.3, 0.5, 0.75),
                        c(0.75, 0.45, 0.25),
                        c(0.3, 0.6, 0.35),
                        c(0.6, 0.3, 0.5),
                    ][(j + seed as usize) % 4];
                    a.glow.cuboid(p, v(1.2, 0.9, 0.08), shade(art, 0.6));
                }
                x += 2.4;
                j += 1;
            }
        }
        self.place(a, Vec3::ZERO, 0.0);
    }

    fn machine_shop(&mut self, room: &Room, seed: f32) {
        let m = room.center();
        let green = c(0.25, 0.4, 0.3);
        let steel = c(0.55, 0.57, 0.6);
        for (i, dz) in [-5.5f32, -1.8, 1.8, 5.5].into_iter().enumerate() {
            for side in [-1.0f32, 1.0] {
                let p = v(m.x + side * 2.0, 0.0, m.z + dz);
                if !room.free(p, 1.0) {
                    continue;
                }
                let mut a = Art::default();
                if (i + side as usize) % 2 == 0 {
                    // Lathe.
                    boxr(&mut a.metal, v(-1.1, 0.0, -0.35), v(1.1, 0.9, 0.35), green);
                    boxr(&mut a.metal, v(-1.0, 0.9, -0.15), v(1.0, 1.05, 0.15), steel);
                    boxr(&mut a.metal, v(-1.1, 0.9, -0.3), v(-0.6, 1.5, 0.3), green);
                    a.metal.cyl(
                        v(-0.45, 1.25, 0.0),
                        0.12,
                        0.2,
                        Quat::from_rotation_z(FRAC_PI_2),
                        steel,
                    );
                    a.metal.cuboid(v(0.4, 1.25, 0.0), v(0.3, 0.25, 0.3), green);
                    a.glow
                        .cuboid(v(-0.85, 1.35, 0.31), v(0.1, 0.1, 0.02), c(1.0, 0.3, 0.2));
                } else {
                    // Workbench with tools and a vice.
                    boxr(
                        &mut a.paint,
                        v(-1.1, 0.85, -0.4),
                        v(1.1, 0.95, 0.4),
                        c(0.5, 0.36, 0.22),
                    );
                    for (lx, lz) in [(-1.0f32, -0.35f32), (1.0, -0.35), (-1.0, 0.35), (1.0, 0.35)] {
                        a.metal
                            .cuboid(v(lx, 0.42, lz), v(0.06, 0.85, 0.06), c(0.25, 0.25, 0.27));
                    }
                    a.metal
                        .cuboid(v(0.7, 1.05, 0.0), v(0.2, 0.18, 0.25), c(0.2, 0.35, 0.65));
                    for t in 0..4 {
                        a.metal.cuboid_rot(
                            v(-0.6 + t as f32 * 0.25, 0.97, 0.1),
                            v(0.04, 0.03, 0.3),
                            Quat::from_rotation_y(hash(seed, t as f32)),
                            steel,
                        );
                    }
                    a.paint
                        .cuboid(v(-0.2, 0.47, 0.0), v(1.6, 0.06, 0.6), c(0.3, 0.3, 0.32));
                }
                self.place(a, p, if side > 0.0 { PI } else { 0.0 });
                self.collide(p + v(0.0, 0.6, 0.0), v(2.2, 1.2, 0.8));
            }
        }
        // Hanging lamps and drums in the corners.
        let mut a = Art::default();
        for dz in [-4.0f32, 0.0, 4.0] {
            let p = m + v(0.0, 0.0, dz);
            a.metal.cyl_between(
                p + v(0.0, ROOM_H, 0.0),
                p + v(0.0, ROOM_H - 1.2, 0.0),
                0.01,
                c(0.1, 0.1, 0.1),
            );
            a.metal.cone(
                p + v(0.0, ROOM_H - 1.3, 0.0),
                0.35,
                0.25,
                Quat::IDENTITY,
                green,
            );
            a.glow
                .sphere(p + v(0.0, ROOM_H - 1.45, 0.0), 0.1, c(1.0, 0.95, 0.8));
        }
        self.place(a, Vec3::ZERO, 0.0);
        for (dx, dz) in [(-1.0f32, -1.0f32), (1.0, 1.0)] {
            let p = v(
                if dx < 0.0 {
                    room.x0 + 0.9
                } else {
                    room.x1 - 0.9
                },
                0.0,
                if dz < 0.0 {
                    room.z0 + 0.9
                } else {
                    room.z1 - 0.9
                },
            );
            if room.free(p, 0.5) {
                self.drums(p.x, p.z, 2, seed + dx);
            }
        }
    }

    fn mall(&mut self, room: &Room, seed: f32) {
        let m = room.center();
        // Shop fronts along both long walls.
        let mut a = Art::default();
        for (k, z) in [room.z0 + 0.06, room.z1 - 0.06].into_iter().enumerate() {
            let n = if k == 0 { 1.0 } else { -1.0 };
            let mut x = room.x0 + 0.4;
            let mut j = 0;
            while x + 3.6 < room.x1 {
                let cx = x + 1.8;
                if room.free(v(cx, 0.0, z + n * 1.0), -1.5) {
                    let sign = [
                        c(1.0, 0.4, 0.5),
                        c(0.4, 0.8, 1.0),
                        c(1.0, 0.85, 0.3),
                        c(0.5, 1.0, 0.6),
                    ][(j + seed as usize) % 4];
                    a.glass
                        .cuboid(v(cx, 1.4, z), v(3.2, 2.6, 0.06), c(0.45, 0.55, 0.6));
                    a.paint.cuboid(
                        v(cx, 2.95, z + n * 0.05),
                        v(3.6, 0.5, 0.1),
                        c(0.2, 0.2, 0.22),
                    );
                    a.glow
                        .cuboid(v(cx, 2.95, z + n * 0.11), v(2.4, 0.3, 0.02), sign);
                    a.glow.cuboid(
                        v(cx, 1.4, z - n * 0.05),
                        v(3.0, 2.4, 0.02),
                        shade(sign, 0.25),
                    );
                    for s in [-1.0f32, 1.0] {
                        a.paint.cuboid(
                            v(cx + s * 1.75, 1.6, z + n * 0.05),
                            v(0.12, 3.2, 0.12),
                            c(0.75, 0.75, 0.78),
                        );
                    }
                }
                x += 3.8;
                j += 1;
            }
        }
        self.place(a, Vec3::ZERO, 0.0);
        // A kiosk, planters and benches down the middle.
        let p = m + v(0.0, 0.0, -3.5);
        if room.free(p, 1.0) {
            let mut k = Art::default();
            k.paint.cyl(
                v(0.0, 0.55, 0.0),
                0.9,
                1.1,
                Quat::IDENTITY,
                c(0.9, 0.9, 0.92),
            );
            k.metal.cyl(
                v(0.0, 1.1, 0.0),
                0.95,
                0.06,
                Quat::IDENTITY,
                c(0.3, 0.3, 0.32),
            );
            k.metal.cyl(
                v(0.0, 2.0, 0.0),
                0.05,
                1.8,
                Quat::IDENTITY,
                c(0.3, 0.3, 0.32),
            );
            k.paint.cone(
                v(0.0, 2.9, 0.0),
                1.2,
                0.5,
                Quat::IDENTITY,
                c(0.85, 0.25, 0.3),
            );
            k.glow.cyl(
                v(0.0, 0.95, 0.0),
                0.92,
                0.12,
                Quat::IDENTITY,
                c(1.0, 0.75, 0.4),
            );
            self.place(k, p, 0.0);
            self.collide(p + v(0.0, 0.55, 0.0), v(1.8, 1.1, 1.8));
        }
        for (i, dz) in [0.5f32, 4.5].into_iter().enumerate() {
            let p = m + v(0.0, 0.0, dz);
            if room.free(p, 0.8) {
                let mut pl = Art::default();
                boxr(
                    &mut pl.paint,
                    v(-0.6, 0.0, -0.6),
                    v(0.6, 0.6, 0.6),
                    c(0.55, 0.5, 0.45),
                );
                boxr(
                    &mut pl.paint,
                    v(-0.55, 0.6, -0.55),
                    v(0.55, 0.62, 0.55),
                    c(0.3, 0.2, 0.12),
                );
                self.place(pl, p, 0.0);
                self.collide(p + v(0.0, 0.3, 0.0), v(1.2, 0.6, 1.2));
                self.bush(p.x, p.z, 0.8, seed + i as f32, Some(c(0.95, 0.5, 0.6)));
            }
        }
        for dz in [-6.5f32, 6.5] {
            let p = m + v(0.0, 0.0, dz);
            if room.free(p, 0.8) {
                self.bench(p.x, p.z, 0.0);
            }
        }
    }

    fn diner(&mut self, room: &Room, seed: f32) {
        let m = room.center();
        let red = c(0.75, 0.12, 0.12);
        let chrome = c(0.8, 0.82, 0.85);
        // Booths along one long wall, a counter with stools along the other.
        let mut z = room.z0 + 1.6;
        while z < room.z1 - 1.5 {
            let p = v(room.x0 + 1.4, 0.0, z);
            let q = v(room.x1 - 1.4, 0.0, z);
            for (i, p) in [p, q].into_iter().enumerate() {
                if !room.free(p, 1.0) {
                    continue;
                }
                let mut a = Art::default();
                for s in [-1.0f32, 1.0] {
                    boxr(
                        &mut a.paint,
                        v(-0.9, 0.0, s * 0.75 - 0.25),
                        v(0.9, 0.45, s * 0.75 + 0.25),
                        red,
                    );
                    boxr(
                        &mut a.paint,
                        v(-0.9, 0.45, s * 0.95 - 0.08),
                        v(0.9, 1.2, s * 0.95 + 0.08),
                        red,
                    );
                }
                boxr(
                    &mut a.paint,
                    v(-0.8, 0.72, -0.4),
                    v(0.8, 0.77, 0.4),
                    c(0.92, 0.92, 0.9),
                );
                a.metal
                    .cyl(v(0.0, 0.36, 0.0), 0.05, 0.72, Quat::IDENTITY, chrome);
                a.paint.cyl(
                    v(0.3, 0.82, 0.0),
                    0.06,
                    0.1,
                    Quat::IDENTITY,
                    c(0.95, 0.85, 0.3),
                );
                self.place(a, p, if i == 0 { 0.0 } else { PI });
                self.collide(p + v(0.0, 0.6, 0.0), v(1.8, 1.2, 2.2));
            }
            z += 2.6;
        }
        let p = m + v(0.0, 0.0, 0.0);
        if room.free(p, 0.2) {
            let mut a = Art::default();
            let len = (room.z1 - room.z0 - 6.0).max(2.0);
            boxr(
                &mut a.paint,
                v(-0.4, 0.0, -len / 2.0),
                v(0.4, 1.0, len / 2.0),
                c(0.2, 0.55, 0.55),
            );
            boxr(
                &mut a.metal,
                v(-0.5, 1.0, -len / 2.0 - 0.1),
                v(0.5, 1.08, len / 2.0 + 0.1),
                chrome,
            );
            let mut sz = -len / 2.0 + 0.5;
            while sz < len / 2.0 {
                a.metal
                    .cyl(v(-0.85, 0.35, sz), 0.04, 0.7, Quat::IDENTITY, chrome);
                a.paint
                    .cyl(v(-0.85, 0.72, sz), 0.2, 0.08, Quat::IDENTITY, red);
                sz += 0.9;
            }
            // Pie case and a neon sign hanging above.
            a.glass
                .cuboid(v(0.0, 1.3, 0.0), v(0.5, 0.4, 0.8), c(0.7, 0.85, 0.9));
            a.paint
                .blob(v(0.0, 1.18, 0.0), v(0.18, 0.06, 0.18), c(0.85, 0.55, 0.25));
            a.glow.cuboid(
                v(0.0, ROOM_H - 1.2, 0.0),
                v(0.06, 0.4, 2.5),
                c(1.0, 0.3, 0.55),
            );
            a.glow.cuboid(
                v(0.0, ROOM_H - 1.65, 0.0),
                v(0.06, 0.15, 1.6),
                c(0.3, 0.9, 1.0),
            );
            self.place(a, p, 0.0);
            self.collide(p + v(0.0, 0.55, 0.0), v(1.0, 1.1, len + 0.2));
        }
        // Jukebox in a corner.
        let p = v(room.x0 + 0.6, 0.0, room.z1 - 0.6);
        if room.free(p, 0.3) {
            let mut a = Art::default();
            boxr(
                &mut a.paint,
                v(-0.4, 0.0, -0.3),
                v(0.4, 1.4, 0.3),
                c(0.55, 0.3, 0.15),
            );
            a.paint
                .blob(v(0.0, 1.4, 0.0), v(0.4, 0.3, 0.3), c(0.55, 0.3, 0.15));
            a.glow
                .cuboid(v(0.0, 1.0, 0.31), v(0.6, 0.6, 0.02), c(1.0, 0.6, 0.2));
            a.glow
                .cuboid(v(0.0, 0.4, 0.31), v(0.5, 0.2, 0.02), c(0.4, 0.8, 1.0));
            self.place(a, p, PI * 0.75 + seed * 0.0);
            self.collide(p + v(0.0, 0.8, 0.0), v(0.8, 1.6, 0.8));
        }
    }
}

/// Shelves of books (2.5 tall, `len` long), faced towards +Z.
fn bookcase(a: &mut Art, len: f32, seed: f32) {
    let wood = c(0.36, 0.22, 0.13);
    boxr(
        &mut a.paint,
        v(-len / 2.0, 0.0, 0.0),
        v(len / 2.0, 2.5, 0.06),
        wood,
    );
    for s in [-1.0f32, 1.0] {
        boxr(
            &mut a.paint,
            v(s * len / 2.0 - 0.04, 0.0, 0.0),
            v(s * len / 2.0 + 0.04, 2.5, 0.38),
            wood,
        );
    }
    for row in 0..5 {
        let y = 0.1 + row as f32 * 0.48;
        boxr(
            &mut a.paint,
            v(-len / 2.0, y, 0.0),
            v(len / 2.0, y + 0.04, 0.38),
            shade(wood, 1.15),
        );
        let mut x = -len / 2.0 + 0.05;
        let mut i = 0;
        while x < len / 2.0 - 0.1 {
            let bw = 0.05 + 0.05 * hash(x + seed, row as f32);
            let bh = 0.28 + 0.12 * hash(row as f32 + seed, x);
            if hash(x * 3.0, seed + row as f32) > 0.08 {
                let col = [
                    c(0.6, 0.12, 0.1),
                    c(0.15, 0.3, 0.55),
                    c(0.2, 0.42, 0.25),
                    c(0.75, 0.6, 0.3),
                    c(0.3, 0.2, 0.35),
                    c(0.85, 0.82, 0.72),
                ][(i * 7 + row) % 6];
                boxr(
                    &mut a.paint,
                    v(x, y + 0.04, 0.08),
                    v(x + bw, y + 0.04 + bh, 0.34),
                    col,
                );
            }
            x += bw + 0.008;
            i += 1;
        }
    }
}

/// Detail on an outside face (normal `n`), kept clear of the opening at
/// `gap` along the face.
#[allow(clippy::too_many_arguments)]
fn facade(
    a: &mut Art,
    n: Vec3,
    hs: f32,
    size: f32,
    height: f32,
    style: u8,
    wall: Color,
    trim: Color,
    seed: f32,
    gap: f32,
) {
    let along = v(n.z.abs(), 0.0, n.x.abs());
    let face = n * (hs + 0.02);
    // Does something from `t - w/2` to `t + w/2`, starting at `bottom`,
    // cover the opening?
    let blocked = |t: f32, w: f32, bottom: f32| {
        (t - gap).abs() < w / 2.0 + OPEN_W / 2.0 + 0.3 && bottom < OPEN_H + 0.6
    };
    match style {
        0 => {
            // Corrugated cladding, roller doors, and a strip of windows.
            let mut t = -hs + 0.3;
            while t < hs {
                if !blocked(t, 0.12, 0.4) {
                    a.paint.cuboid(
                        face + along * t + v(0.0, height / 2.0, 0.0),
                        along * 0.12 + n * 0.06 + v(0.0, height - 0.8, 0.0),
                        shade(wall, 0.85),
                    );
                } else {
                    a.paint.cuboid(
                        face + along * t + v(0.0, (height + OPEN_H + 0.6) / 2.0, 0.0),
                        along * 0.12 + n * 0.06 + v(0.0, height - OPEN_H - 1.0, 0.0),
                        shade(wall, 0.85),
                    );
                }
                t += 0.6;
            }
            for k in [-4.5f32, 0.0, 4.5] {
                if blocked(k, 3.4, 0.0) {
                    continue;
                }
                let at = face + n * 0.05 + along * k;
                a.metal.cuboid(
                    at + v(0.0, 2.0, 0.0),
                    along * 3.4 + n * 0.08 + v(0.0, 4.0, 0.0),
                    c(0.6, 0.6, 0.6),
                );
                for y in 0..12 {
                    a.metal.cuboid(
                        at + n * 0.05 + v(0.0, 0.2 + y as f32 * 0.32, 0.0),
                        along * 3.4 + n * 0.02 + v(0.0, 0.04, 0.0),
                        c(0.45, 0.45, 0.45),
                    );
                }
                a.paint.cuboid(
                    at + v(0.0, 4.15, 0.0),
                    along * 3.8 + n * 0.3 + v(0.0, 0.3, 0.0),
                    trim,
                );
                a.glow.cuboid(
                    at + n * 0.2 + v(0.0, 4.6, 0.0),
                    along * 0.6 + n * 0.1 + v(0.0, 0.25, 0.0),
                    c(1.0, 0.9, 0.6),
                );
            }
            a.glass.cuboid(
                face + v(0.0, 6.8, 0.0),
                along * (size - 2.0) + n * 0.06 + v(0.0, 1.0, 0.0),
                c(0.35, 0.45, 0.55),
            );
        }
        1 => {
            // Rows of windows with sills, and balconies.
            let mut y = 3.2;
            let mut row = 0;
            while y < height - 1.5 {
                let mut t = -hs + 1.6;
                let mut col = 0;
                while t < hs - 1.0 {
                    if !blocked(t, 1.3, y - 0.8) {
                        let at = face + along * t + v(0.0, y, 0.0);
                        a.glass.cuboid(
                            at,
                            along * 1.1 + n * 0.06 + v(0.0, 1.4, 0.0),
                            c(0.3, 0.38, 0.46),
                        );
                        a.paint.cuboid(
                            at - v(0.0, 0.8, 0.0) + n * 0.08,
                            along * 1.3 + n * 0.18 + v(0.0, 0.1, 0.0),
                            trim,
                        );
                        if (row + col) % 3 == 0 {
                            a.glow.cuboid(
                                at + n * 0.035,
                                along * 1.0 + n * 0.02 + v(0.0, 1.3, 0.0),
                                c(1.0, 0.85, 0.55),
                            );
                        }
                        if row % 2 == 1 && col % 3 == 1 {
                            a.metal.cuboid(
                                at - v(0.0, 0.75, 0.0) + n * 0.6,
                                along * 2.6 + n * 1.2 + v(0.0, 0.1, 0.0),
                                c(0.3, 0.3, 0.32),
                            );
                            a.metal.cuboid(
                                at - v(0.0, 0.3, 0.0) + n * 1.18,
                                along * 2.6 + n * 0.04 + v(0.0, 0.9, 0.0),
                                c(0.25, 0.25, 0.27),
                            );
                        }
                    }
                    t += 2.6;
                    col += 1;
                }
                y += 3.0;
                row += 1;
            }
            let _ = seed;
        }
        _ => {
            // Shop windows with awnings and signs either side of the way in.
            for (i, k) in [-5.5f32, 0.0, 5.5].into_iter().enumerate() {
                if blocked(k, 4.0, 0.4) {
                    continue;
                }
                let at = face + along * k;
                a.glass.cuboid(
                    at + v(0.0, 1.5, 0.0),
                    along * 4.0 + n * 0.06 + v(0.0, 2.2, 0.0),
                    c(0.35, 0.45, 0.5),
                );
                a.paint.cuboid(
                    at + v(0.0, 1.5, 0.0) + n * 0.04,
                    along * 0.12 + n * 0.08 + v(0.0, 2.2, 0.0),
                    trim,
                );
                let awn = [c(0.8, 0.15, 0.12), c(0.15, 0.45, 0.2), c(0.2, 0.3, 0.7)]
                    [(i + seed as usize) % 3];
                a.paint.cuboid(
                    at + v(0.0, 3.0, 0.0) + n * 0.6,
                    along * 4.4 + n * 1.2 + v(0.0, 0.12, 0.0),
                    awn,
                );
                a.glow.cuboid(
                    at + v(0.0, 3.7, 0.0) + n * 0.06,
                    along * 3.0 + n * 0.06 + v(0.0, 0.6, 0.0),
                    [c(1.0, 0.85, 0.3), c(0.4, 1.0, 0.6), c(1.0, 0.45, 0.4)][(i + 1) % 3],
                );
                a.glass.cuboid(
                    at + v(0.0, 5.2, 0.0),
                    along * 3.0 + n * 0.06 + v(0.0, 1.4, 0.0),
                    c(0.3, 0.38, 0.46),
                );
            }
        }
    }
}

/// A lit entrance round an opening at `p` (relative to the building) in a
/// face with normal `n`: a frame, a canopy and a glowing sign.
fn entrance(a: &mut Art, p: Vec3, n: Vec3, inside: Interior, trim: Color) {
    let along = v(n.z.abs(), 0.0, n.x.abs());
    let out = p + n * (WALL / 2.0 + 0.05);
    let (canopy, sign) = match inside {
        Interior::Warehouse => (c(0.3, 0.45, 0.65), c(0.5, 0.85, 1.0)),
        Interior::MachineShop => (c(0.3, 0.4, 0.3), c(1.0, 0.75, 0.3)),
        Interior::Library => (c(0.35, 0.15, 0.1), c(1.0, 0.85, 0.55)),
        Interior::Museum => (c(0.75, 0.72, 0.68), c(1.0, 0.95, 0.8)),
        Interior::Mall => (c(0.2, 0.3, 0.55), c(0.5, 1.0, 0.9)),
        Interior::Diner => (c(0.75, 0.12, 0.12), c(1.0, 0.35, 0.6)),
    };
    for s in [-1.0f32, 1.0] {
        a.paint.cuboid(
            out + along * s * (OPEN_W / 2.0 + 0.15) + v(0.0, OPEN_H / 2.0, 0.0),
            along * 0.3 + n * 0.2 + v(0.0, OPEN_H, 0.0),
            shade(trim, 0.9),
        );
    }
    a.paint.cuboid(
        out + v(0.0, OPEN_H + 0.1, 0.0),
        along * (OPEN_W + 0.6) + n * 0.2 + v(0.0, 0.25, 0.0),
        shade(trim, 0.9),
    );
    if inside == Interior::Museum || inside == Interior::Library {
        // Columns and a pediment.
        for s in [-1.0f32, 1.0] {
            a.paint.cyl(
                out + n * 0.8 + along * s * (OPEN_W / 2.0 + 0.5) + v(0.0, 2.1, 0.0),
                0.25,
                4.2,
                Quat::IDENTITY,
                canopy,
            );
        }
        a.paint.cuboid(
            out + n * 0.8 + v(0.0, 4.35, 0.0),
            along * (OPEN_W + 2.0) + n * 1.2 + v(0.0, 0.3, 0.0),
            canopy,
        );
        a.paint.cuboid(
            out + n * 0.8 + v(0.0, 4.75, 0.0),
            along * (OPEN_W + 1.0) + n * 1.0 + v(0.0, 0.5, 0.0),
            shade(canopy, 0.9),
        );
    } else {
        a.paint.cuboid(
            out + n * 0.8 + v(0.0, OPEN_H + 0.35, 0.0),
            along * (OPEN_W + 1.2) + n * 1.6 + v(0.0, 0.15, 0.0),
            canopy,
        );
    }
    a.glow.cuboid(
        out + n * 0.12 + v(0.0, OPEN_H + 0.75, 0.0),
        along * (OPEN_W - 0.6) + n * 0.04 + v(0.0, 0.45, 0.0),
        sign,
    );
    a.glow.cuboid(
        out + n * 0.6 + v(0.0, OPEN_H + 0.25, 0.0),
        along * 0.8 + n * 0.1 + v(0.0, 0.05, 0.0),
        c(1.0, 0.95, 0.85),
    );
}
