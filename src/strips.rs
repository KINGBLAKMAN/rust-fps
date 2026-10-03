//! The bigger maps: every map has a starting area inside an inner fence and
//! four more areas around it (north, south, east and west), each behind a
//! door you buy open. The corners are filled with big buildings. Two of the
//! areas have a gun on the wall to buy, and one has a perk machine.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::kit::c;
use crate::maps::{DoorDef, MapLayout, WallBuy, DOOR_WIDTH, INNER, OUTER};
use crate::props::{boxr, hash, shade, v, Art};

/// Inner fence line (the old map edge).
const FENCE: f32 = INNER + 0.5;
/// Gap left in the fence for a door (its pillars fill the edges).
const GAP: f32 = DOOR_WIDTH / 2.0 + 0.4;

impl MapLayout {
    /// Adds the doors, the four new areas and the corner buildings.
    pub fn expand(&mut self, map: u8) {
        match map {
            1 => self.expand_park(),
            2 => self.expand_neighborhood(),
            _ => self.expand_yard(),
        }
        let (style, height, block) = match map {
            1 => (1, 3.0, 1),
            2 => (2, 2.2, 2),
            _ => (0, 4.5, 0),
        };
        self.ring(style, height);
        let mid = (INNER + 1.0 + OUTER) / 2.0;
        for (i, (sx, sz)) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)].into_iter().enumerate() {
            self.corner_block(sx * mid, sz * mid, block, i as f32 + map as f32 * 4.0);
        }
    }

    fn door(&mut self, x: f32, z: f32, zone: u8, cost: u32, name: &'static str) {
        let along_x = zone == 1 || zone == 2;
        let pos = if along_x { v(x, 0.0, if zone == 1 { FENCE } else { -FENCE }) } else { v(if zone == 3 { FENCE } else { -FENCE }, 0.0, z) };
        self.doors.push(DoorDef { pos, along_x, cost, zone, name });
    }

    /// A gun board on a concrete panel, facing `yaw`, with its back to the
    /// inner fence.
    fn wall_buy(&mut self, pos: Vec3, yaw: f32, gun: u8) {
        let mut a = Art::default();
        let concrete = c(0.62, 0.62, 0.6);
        boxr(&mut a.paint, v(-1.5, 0.0, -0.3), v(1.5, 2.6, 0.0), concrete);
        boxr(&mut a.paint, v(-1.6, 2.6, -0.35), v(1.6, 2.75, 0.05), shade(concrete, 0.8));
        // Chalk outline of the gun and a lamp over it.
        let chalk = c(0.95, 0.95, 0.85);
        for (p0, p1) in [
            (v(-1.1, 0.9, 0.01), v(1.1, 0.9, 0.01)),
            (v(-1.1, 2.1, 0.01), v(1.1, 2.1, 0.01)),
            (v(-1.1, 0.9, 0.01), v(-1.1, 2.1, 0.01)),
            (v(1.1, 0.9, 0.01), v(1.1, 2.1, 0.01)),
        ] {
            a.glow.beam(p0, p1, Vec2::new(0.04, 0.01), chalk);
        }
        a.metal.beam(v(0.0, 2.75, -0.1), v(0.0, 2.85, 0.35), Vec2::splat(0.05), c(0.2, 0.2, 0.2));
        a.metal.cone(v(0.0, 2.8, 0.42), 0.18, 0.15, Quat::IDENTITY, c(0.2, 0.2, 0.2));
        a.glow.cyl(v(0.0, 2.72, 0.42), 0.12, 0.02, Quat::IDENTITY, c(1.0, 0.95, 0.8));
        self.place(a, pos, yaw);
        self.collide_local(pos, yaw, v(0.0, 1.3, -0.15), v(3.0, 2.6, 0.35));
        self.light(pos + Quat::from_rotation_y(yaw) * v(0.0, 2.5, 0.8), c(1.0, 0.92, 0.75), 40_000.0);
        self.wall_buys.push(WallBuy { pos, yaw, gun });
    }

    /// The inner fence, with gaps and pillars for the doors.
    fn ring(&mut self, style: u8, height: f32) {
        let mut a = Art::default();
        for side in 0..4 {
            // Positions along the side where doors are.
            let mut cuts: Vec<f32> = self
                .doors
                .iter()
                .filter(|d| {
                    let z = d.pos.z.abs() > INNER;
                    match side {
                        0 => z && d.pos.z < 0.0,
                        1 => z && d.pos.z > 0.0,
                        2 => !z && d.pos.x < 0.0,
                        _ => !z && d.pos.x > 0.0,
                    }
                })
                .map(|d| if side < 2 { d.pos.x } else { d.pos.z })
                .collect();
            cuts.sort_by(f32::total_cmp);
            let point = |t: f32| match side {
                0 => v(t, 0.0, -FENCE),
                1 => v(t, 0.0, FENCE),
                2 => v(-FENCE, 0.0, t),
                _ => v(FENCE, 0.0, t),
            };
            let mut start = -FENCE;
            for cut in cuts.iter().copied().chain(std::iter::once(FENCE + GAP)) {
                let end = cut - GAP;
                if end - start > 1.2 {
                    self.fence_run(&mut a, point(start + 0.5), point(end - 0.5), style, height);
                }
                start = cut + GAP;
            }
        }
        // Door pillars and a lintel with a warning lamp.
        let doors = self.doors.clone();
        for d in &doors {
            let yaw = if d.along_x { 0.0 } else { FRAC_PI_2 };
            let mut p = Art::default();
            let (col, cap) = match style {
                0 => (c(0.55, 0.55, 0.53), c(0.9, 0.7, 0.1)),
                1 => (c(0.6, 0.57, 0.52), c(0.45, 0.43, 0.4)),
                _ => (c(0.5, 0.37, 0.25), c(0.4, 0.3, 0.2)),
            };
            for s in [-1.0f32, 1.0] {
                boxr(&mut p.paint, v(s * (DOOR_WIDTH / 2.0 + 0.05), 0.0, -0.35), v(s * (DOOR_WIDTH / 2.0 + 0.45), 3.6, 0.35), col);
                boxr(&mut p.paint, v(s * (DOOR_WIDTH / 2.0), 3.6, -0.4), v(s * (DOOR_WIDTH / 2.0 + 0.5), 3.75, 0.4), cap);
                self.collide_local(d.pos, yaw, v(s * (DOOR_WIDTH / 2.0 + 0.25), 1.8, 0.0), v(0.4, 3.6, 0.7));
            }
            boxr(&mut p.paint, v(-DOOR_WIDTH / 2.0 - 0.5, 3.3, -0.25), v(DOOR_WIDTH / 2.0 + 0.5, 3.6, 0.25), shade(col, 0.8));
            for s in [-1.0f32, 1.0] {
                p.metal.cyl(v(0.0, 3.75, s * 0.2), 0.1, 0.12, Quat::IDENTITY, c(0.2, 0.2, 0.2));
            }
            self.place(p, d.pos, yaw);
        }
        self.place(a, Vec3::ZERO, 0.0);
    }

    /// A big building filling a corner between two areas. Style: 0 metal
    /// warehouse, 1 brick apartments, 2 corner shops.
    fn corner_block(&mut self, x: f32, z: f32, style: u8, seed: f32) {
        let size = OUTER - INNER - 1.0;
        let hs = size / 2.0;
        let mut a = Art::default();
        let (wall, trim, height) = match style {
            0 => (c(0.48 + hash(seed, 1.0) * 0.15, 0.5, 0.52), c(0.75, 0.55, 0.1), 9.0),
            1 => (c(0.55 + hash(seed, 2.0) * 0.15, 0.3, 0.24), c(0.82, 0.8, 0.75), 16.0 + hash(seed, 3.0) * 8.0),
            _ => (c(0.75, 0.68 - hash(seed, 4.0) * 0.15, 0.55), c(0.3, 0.32, 0.36), 7.0),
        };
        boxr(&mut a.paint, v(-hs, 0.0, -hs), v(hs, height, hs), wall);
        boxr(&mut a.paint, v(-hs - 0.15, height, -hs - 0.15), v(hs + 0.15, height + 0.4, hs + 0.15), trim);
        boxr(&mut a.paint, v(-hs - 0.05, 0.0, -hs - 0.05), v(hs + 0.05, 0.6, hs + 0.05), shade(wall, 0.7));
        // The two faces that look into the play areas get the detail.
        let faces = [(x < 0.0, v(1.0, 0.0, 0.0)), (x > 0.0, v(-1.0, 0.0, 0.0)), (z < 0.0, v(0.0, 0.0, 1.0)), (z > 0.0, v(0.0, 0.0, -1.0))];
        for (show, n) in faces {
            if !show {
                continue;
            }
            let along = v(n.z.abs(), 0.0, n.x.abs());
            let face = n * (hs + 0.02);
            match style {
                0 => {
                    // Corrugated cladding, roller doors and a loading dock.
                    let mut t = -hs + 0.3;
                    while t < hs {
                        a.paint.cuboid(face + along * t + v(0.0, height / 2.0, 0.0), along * 0.12 + n * 0.06 + v(0.0, height - 0.8, 0.0), shade(wall, 0.85));
                        t += 0.6;
                    }
                    for k in [-4.5f32, 0.0, 4.5] {
                        let at = face + n * 0.05 + along * k;
                        a.metal.cuboid(at + v(0.0, 2.0, 0.0), along * 3.4 + n * 0.08 + v(0.0, 4.0, 0.0), c(0.6, 0.6, 0.6));
                        for y in 0..12 {
                            a.metal.cuboid(at + n * 0.05 + v(0.0, 0.2 + y as f32 * 0.32, 0.0), along * 3.4 + n * 0.02 + v(0.0, 0.04, 0.0), c(0.45, 0.45, 0.45));
                        }
                        a.paint.cuboid(at + v(0.0, 4.15, 0.0), along * 3.8 + n * 0.3 + v(0.0, 0.3, 0.0), trim);
                        a.glow.cuboid(at + n * 0.2 + v(0.0, 4.6, 0.0), along * 0.6 + n * 0.1 + v(0.0, 0.25, 0.0), c(1.0, 0.9, 0.6));
                    }
                    a.glass.cuboid(face + v(0.0, 6.8, 0.0), along * (size - 2.0) + n * 0.06 + v(0.0, 1.0, 0.0), c(0.35, 0.45, 0.55));
                }
                1 => {
                    // Rows of windows with sills, balconies and a doorway.
                    let mut y = 3.2;
                    let mut row = 0;
                    while y < height - 1.5 {
                        let mut t = -hs + 1.6;
                        let mut col = 0;
                        while t < hs - 1.0 {
                            let at = face + along * t + v(0.0, y, 0.0);
                            a.glass.cuboid(at, along * 1.1 + n * 0.06 + v(0.0, 1.4, 0.0), c(0.3, 0.38, 0.46));
                            a.paint.cuboid(at - v(0.0, 0.8, 0.0) + n * 0.08, along * 1.3 + n * 0.18 + v(0.0, 0.1, 0.0), trim);
                            if (row + col) % 3 == 0 {
                                a.glow.cuboid(at + n * 0.035, along * 1.0 + n * 0.02 + v(0.0, 1.3, 0.0), c(1.0, 0.85, 0.55));
                            }
                            if row % 2 == 1 && col % 3 == 1 {
                                a.metal.cuboid(at - v(0.0, 0.75, 0.0) + n * 0.6, along * 2.6 + n * 1.2 + v(0.0, 0.1, 0.0), c(0.3, 0.3, 0.32));
                                a.metal.cuboid(at - v(0.0, 0.3, 0.0) + n * 1.18, along * 2.6 + n * 0.04 + v(0.0, 0.9, 0.0), c(0.25, 0.25, 0.27));
                            }
                            t += 2.6;
                            col += 1;
                        }
                        y += 3.0;
                        row += 1;
                    }
                    a.paint.cuboid(face + v(0.0, 1.3, 0.0) + n * 0.05, along * 2.4 + n * 0.1 + v(0.0, 2.6, 0.0), c(0.25, 0.18, 0.12));
                    a.paint.cuboid(face + v(0.0, 2.9, 0.0) + n * 0.6, along * 3.2 + n * 1.2 + v(0.0, 0.15, 0.0), trim);
                    a.glow.cuboid(face + v(0.0, 2.75, 0.0) + n * 0.9, along * 0.5 + n * 0.1 + v(0.0, 0.08, 0.0), c(1.0, 0.9, 0.6));
                }
                _ => {
                    // Shop fronts with awnings and glowing signs.
                    for (i, k) in [-5.5f32, 0.0, 5.5].into_iter().enumerate() {
                        let at = face + along * k;
                        a.glass.cuboid(at + v(0.0, 1.5, 0.0), along * 4.0 + n * 0.06 + v(0.0, 2.2, 0.0), c(0.35, 0.45, 0.5));
                        a.paint.cuboid(at + v(0.0, 1.5, 0.0) + n * 0.04, along * 0.12 + n * 0.08 + v(0.0, 2.2, 0.0), trim);
                        let awn = [c(0.8, 0.15, 0.12), c(0.15, 0.45, 0.2), c(0.2, 0.3, 0.7)][(i + seed as usize) % 3];
                        a.paint.cuboid(at + v(0.0, 3.0, 0.0) + n * 0.6, along * 4.4 + n * 1.2 + v(0.0, 0.12, 0.0), awn);
                        a.glow.cuboid(at + v(0.0, 3.7, 0.0) + n * 0.06, along * 3.0 + n * 0.06 + v(0.0, 0.6, 0.0), [c(1.0, 0.85, 0.3), c(0.4, 1.0, 0.6), c(1.0, 0.45, 0.4)][(i + 1) % 3]);
                        a.glass.cuboid(at + v(0.0, 5.2, 0.0), along * 3.0 + n * 0.06 + v(0.0, 1.4, 0.0), c(0.3, 0.38, 0.46));
                    }
                }
            }
        }
        // Roof clutter.
        for i in 0..4 {
            let px = (hash(seed, i as f32) - 0.5) * (size - 4.0);
            let pz = (hash(i as f32, seed) - 0.5) * (size - 4.0);
            boxr(&mut a.metal, v(px - 0.8, height + 0.4, pz - 0.6), v(px + 0.8, height + 1.4, pz + 0.6), c(0.6, 0.6, 0.62));
            a.metal.cyl(v(px, height + 1.45, pz), 0.4, 0.1, Quat::IDENTITY, c(0.3, 0.3, 0.3));
        }
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, height / 2.0, z), v(size, height, size));
    }

    // -----------------------------------------------------------------------
    // Shared props for the new areas
    // -----------------------------------------------------------------------

    /// Railway track along X from `x0` to `x1`.
    fn track(&mut self, x0: f32, x1: f32, z: f32) {
        let mut a = Art::default();
        let mut x = x0;
        while x <= x1 {
            boxr(&mut a.paint, v(x - 0.12, 0.0, z - 1.3), v(x + 0.12, 0.1, z + 1.3), c(0.32, 0.24, 0.17));
            x += 0.7;
        }
        for s in [-0.72f32, 0.72] {
            boxr(&mut a.metal, v(x0, 0.1, z + s - 0.04), v(x1, 0.22, z + s + 0.04), c(0.42, 0.4, 0.38));
        }
        a.paint.cuboid(v((x0 + x1) / 2.0, 0.02, z), v(x1 - x0, 0.04, 3.2), c(0.36, 0.34, 0.32));
        self.place(a, Vec3::ZERO, 0.0);
    }

    /// Railway boxcar standing on a track along X.
    fn boxcar(&mut self, x: f32, z: f32, color: Color, seed: f32) {
        let mut a = Art::default();
        let (l, w, h) = (12.0, 3.0, 3.6);
        boxr(&mut a.paint, v(-l / 2.0, 1.1, -w / 2.0), v(l / 2.0, 1.1 + h, w / 2.0), color);
        for i in 0..17 {
            let px = -l / 2.0 + 0.4 + i as f32 * 0.7;
            for s in [-1.0f32, 1.0] {
                boxr(&mut a.paint, v(px - 0.05, 1.15, s * (w / 2.0) - 0.02), v(px + 0.05, 1.05 + h, s * (w / 2.0) + 0.03), shade(color, 0.75));
            }
        }
        boxr(&mut a.paint, v(-l / 2.0 - 0.1, 1.05 + h, -w / 2.0 - 0.1), v(l / 2.0 + 0.1, 1.25 + h, w / 2.0 + 0.1), shade(color, 0.6));
        for s in [-1.0f32, 1.0] {
            boxr(&mut a.paint, v(-1.4, 1.2, s * (w / 2.0 + 0.04)), v(1.4, 0.9 + h, s * (w / 2.0 + 0.07)), shade(color, 0.85));
            a.metal.cuboid(v(0.0, 2.6, s * (w / 2.0 + 0.1)), v(0.1, 0.5, 0.05), c(0.3, 0.3, 0.3));
            boxr(&mut a.paint, v(-3.5, 3.2, s * (w / 2.0 + 0.04)), v(-2.0, 3.8, s * (w / 2.0 + 0.05)), c(0.92, 0.92, 0.9));
        }
        boxr(&mut a.metal, v(-l / 2.0 + 0.2, 0.75, -w / 2.0 + 0.2), v(l / 2.0 - 0.2, 1.1, w / 2.0 - 0.2), c(0.15, 0.15, 0.15));
        for bx in [-l / 2.0 + 1.8, l / 2.0 - 1.8] {
            boxr(&mut a.metal, v(bx - 1.2, 0.3, -1.0), v(bx + 1.2, 0.75, 1.0), c(0.2, 0.2, 0.2));
            for wx in [-0.7f32, 0.7] {
                for s in [-0.72f32, 0.72] {
                    a.metal.cyl(v(bx + wx, 0.42, s), 0.42, 0.1, Quat::from_rotation_x(FRAC_PI_2), c(0.25, 0.25, 0.25));
                }
            }
        }
        let _ = seed;
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 2.4, z), v(l, 4.8, w));
    }

    fn bollard(&mut self, x: f32, z: f32) {
        let mut a = Art::default();
        let iron = c(0.12, 0.12, 0.13);
        a.metal.cyl(v(0.0, 0.35, 0.0), 0.22, 0.7, Quat::IDENTITY, iron);
        a.metal.cyl(v(0.0, 0.75, 0.0), 0.3, 0.12, Quat::IDENTITY, iron);
        a.metal.cyl(v(0.0, 0.85, 0.0), 0.2, 0.1, Quat::IDENTITY, c(0.85, 0.75, 0.15));
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 0.4, z), v(0.6, 0.8, 0.6));
    }

    /// Wooden crates, stacked.
    fn crates(&mut self, x: f32, z: f32, seed: f32) {
        let mut a = Art::default();
        let wood = c(0.6, 0.45, 0.27);
        let n = 2 + (hash(seed, 1.0) * 3.0) as usize;
        for i in 0..n {
            let (px, py, pz) = match i {
                0 => (0.0, 0.0, 0.0),
                1 => (1.15, 0.0, 0.1),
                2 => (0.5, 1.1, 0.05),
                _ => (0.0, 0.0, 1.15),
            };
            let col = shade(wood, 0.85 + hash(seed, i as f32) * 0.3);
            boxr(&mut a.paint, v(px - 0.55, py, pz - 0.55), v(px + 0.55, py + 1.1, pz + 0.55), col);
            for s in [-1.0f32, 1.0] {
                a.paint.beam(v(px - 0.5, py + 0.05, pz + s * 0.56), v(px + 0.5, py + 1.05, pz + s * 0.56), Vec2::new(0.1, 0.02), shade(col, 0.7));
                a.paint.beam(v(px + s * 0.56, py + 0.05, pz - 0.5), v(px + s * 0.56, py + 1.05, pz + 0.5), Vec2::new(0.1, 0.02), shade(col, 0.7));
            }
        }
        let tall = if n > 2 { 2.2 } else { 1.1 };
        self.place(a, v(x, 0.0, z), hash(seed, 9.0) * 0.4);
        self.collide(v(x + 0.55, tall / 2.0, z + 0.4), v(2.4, tall, 2.4));
    }

    /// Harbour or yard lamp post with an overhanging lamp.
    fn lamp_post(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        let grey = c(0.3, 0.31, 0.33);
        a.metal.cyl(v(0.0, 3.5, 0.0), 0.08, 7.0, Quat::IDENTITY, grey);
        a.metal.beam(v(0.0, 6.9, 0.0), v(0.0, 6.9, 1.2), Vec2::splat(0.08), grey);
        boxr(&mut a.metal, v(-0.25, 6.7, 0.9), v(0.25, 6.95, 1.5), grey);
        a.glow.cuboid(v(0.0, 6.68, 1.2), v(0.4, 0.04, 0.5), color);
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 2.0, z), v(0.3, 4.0, 0.3));
        self.light(v(x, 6.4, z) + Quat::from_rotation_y(yaw) * v(0.0, 0.0, 1.2), color, 180_000.0);
    }

    /// Semi truck (cab facing +Z) with a box trailer behind it.
    fn semi(&mut self, x: f32, z: f32, yaw: f32, color: Color, trailer: Color) {
        let mut a = Art::default();
        let dark = c(0.08, 0.08, 0.08);
        let chrome = c(0.8, 0.8, 0.82);
        // Cab.
        boxr(&mut a.metal, v(-1.25, 1.0, 2.6), v(1.25, 3.4, 4.6), color);
        boxr(&mut a.metal, v(-1.2, 1.0, 4.6), v(1.2, 2.2, 6.0), color);
        boxr(&mut a.glass, v(-1.1, 2.5, 4.55), v(1.1, 3.25, 4.62), c(0.12, 0.16, 0.2));
        for s in [-1.0f32, 1.0] {
            boxr(&mut a.glass, v(s * 1.26 - 0.02, 2.4, 3.6), v(s * 1.26 + 0.02, 3.2, 4.4), c(0.12, 0.16, 0.2));
            a.metal.cyl(v(s * 1.0, 3.6, 2.7), 0.1, 2.2, Quat::IDENTITY, chrome);
            a.metal.cyl(v(s * 1.3, 0.9, 3.2), 0.3, 1.0, Quat::from_rotation_x(FRAC_PI_2), chrome);
            a.glow.cuboid(v(s * 0.85, 1.6, 6.02), v(0.4, 0.2, 0.04), c(1.0, 0.97, 0.85));
        }
        boxr(&mut a.metal, v(-1.0, 1.2, 6.0), v(1.0, 2.0, 6.08), chrome);
        boxr(&mut a.metal, v(-1.3, 0.5, 5.9), v(1.3, 0.8, 6.15), chrome);
        boxr(&mut a.metal, v(-1.1, 0.6, -6.5), v(1.1, 1.0, 2.6), dark);
        // Trailer.
        boxr(&mut a.paint, v(-1.3, 1.2, -9.5), v(1.3, 4.0, 2.2), trailer);
        for i in 0..20 {
            let pz = -9.3 + i as f32 * 0.58;
            for s in [-1.0f32, 1.0] {
                boxr(&mut a.paint, v(s * 1.3 - 0.02, 1.3, pz - 0.03), v(s * 1.3 + 0.02, 3.9, pz + 0.03), shade(trailer, 0.8));
            }
        }
        for wz in [5.0f32, 2.0, -7.0, -8.3] {
            for s in [-1.0f32, 1.0] {
                a.paint.cyl(v(s * 1.1, 0.5, wz), 0.5, 0.4, Quat::from_rotation_z(FRAC_PI_2), dark);
                a.metal.cyl(v(s * 1.31, 0.5, wz), 0.25, 0.02, Quat::from_rotation_z(FRAC_PI_2), chrome);
            }
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 2.0, -1.75), v(2.7, 4.0, 15.6));
    }

    /// Fuel pump island under a canopy (canopy optional).
    fn fuel_pumps(&mut self, x: f32, z: f32, yaw: f32, canopy: bool) {
        let mut a = Art::default();
        let white = c(0.92, 0.92, 0.9);
        let red = c(0.8, 0.12, 0.1);
        boxr(&mut a.paint, v(-4.0, 0.0, -0.8), v(4.0, 0.2, 0.8), c(0.6, 0.6, 0.58));
        for px in [-2.2f32, 2.2] {
            boxr(&mut a.metal, v(px - 0.45, 0.2, -0.3), v(px + 0.45, 1.9, 0.3), white);
            boxr(&mut a.metal, v(px - 0.47, 1.9, -0.32), v(px + 0.47, 2.2, 0.32), red);
            for s in [-1.0f32, 1.0] {
                a.glow.cuboid(v(px, 1.4, s * 0.31), v(0.5, 0.3, 0.02), c(0.5, 0.9, 1.0));
                a.metal.beam(v(px + 0.3, 1.1, s * 0.33), v(px + 0.5, 0.5, s * 0.5), Vec2::splat(0.04), c(0.1, 0.1, 0.1));
            }
        }
        if canopy {
            for (px, pz) in [(-4.5f32, 0.0f32), (4.5, 0.0)] {
                a.metal.cuboid(v(px, 2.5, pz), v(0.4, 5.0, 0.4), white);
                self.collide_local(v(x, 0.0, z), yaw, v(px, 2.5, pz), v(0.4, 5.0, 0.4));
            }
            boxr(&mut a.metal, v(-7.0, 5.0, -4.0), v(7.0, 5.8, 4.0), white);
            boxr(&mut a.paint, v(-7.05, 5.1, -4.05), v(7.05, 5.5, 4.05), red);
            a.glow.cuboid(v(0.0, 4.98, 0.0), v(12.0, 0.04, 6.0), c(1.0, 0.97, 0.9));
            let rot = Quat::from_rotation_y(yaw);
            for px in [-3.5f32, 3.5] {
                self.light(v(x, 4.6, z) + rot * v(px, 0.0, 0.0), c(1.0, 0.97, 0.9), 150_000.0);
            }
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.0, 0.0), v(6.0, 2.0, 1.4));
    }

    /// Open-sided steel shed roof on columns over a rectangle.
    fn canopy(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, height: f32, color: Color) {
        let mut a = Art::default();
        let grey = c(0.5, 0.5, 0.52);
        let mut x = x0;
        while x <= x1 + 0.01 {
            for zz in [z0, z1] {
                a.metal.cuboid(v(x, height / 2.0, zz), v(0.35, height, 0.35), grey);
                self.collide(v(x, height / 2.0, zz), v(0.4, height, 0.4));
            }
            a.metal.beam(v(x, height, z0), v(x, height, z1), Vec2::new(0.25, 0.4), grey);
            x += (x1 - x0) / 4.0;
        }
        boxr(&mut a.metal, v(x0 - 0.5, height + 0.2, z0 - 0.5), v(x1 + 0.5, height + 0.45, z1 + 0.5), color);
        let mut x = x0 + 2.0;
        while x < x1 - 1.0 {
            a.glow.cuboid(v(x, height - 0.05, (z0 + z1) / 2.0), v(0.3, 0.06, (z1 - z0) * 0.7), c(0.9, 0.95, 1.0));
            self.light(v(x, height - 0.6, (z0 + z1) / 2.0), c(0.9, 0.95, 1.0), 120_000.0);
            x += (x1 - x0) / 3.0;
        }
        self.place(a, Vec3::ZERO, 0.0);
    }

    /// Storage rack with boxes on three shelves.
    fn shelving(&mut self, x: f32, z: f32, yaw: f32, seed: f32) {
        let mut a = Art::default();
        let blue = c(0.15, 0.3, 0.65);
        let orange = c(0.9, 0.45, 0.1);
        let l = 6.0;
        for px in [-l / 2.0, 0.0, l / 2.0] {
            for s in [-0.5f32, 0.5] {
                a.metal.cuboid(v(px, 2.0, s), v(0.1, 4.0, 0.1), blue);
            }
        }
        for y in [0.3f32, 1.6, 2.9] {
            for s in [-0.5f32, 0.5] {
                a.metal.cuboid(v(0.0, y, s), v(l, 0.12, 0.08), orange);
            }
            for i in 0..5 {
                let px = -l / 2.0 + 0.7 + i as f32 * 1.15;
                if hash(seed + y, i as f32) > 0.25 {
                    let hgt = 0.5 + hash(seed, i as f32 + y) * 0.6;
                    let col = if hash(i as f32, seed + y) > 0.5 { c(0.62, 0.48, 0.3) } else { c(0.55, 0.56, 0.5) };
                    boxr(&mut a.paint, v(px - 0.45, y + 0.06, -0.45), v(px + 0.45, y + 0.06 + hgt, 0.45), col);
                }
            }
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 2.0, 0.0), v(l + 0.2, 4.0, 1.2));
    }

    /// Small booth (guard hut, ticket booth).
    fn booth(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        boxr(&mut a.paint, v(-1.2, 0.0, -1.2), v(1.2, 2.6, 1.2), color);
        boxr(&mut a.paint, v(-1.5, 2.6, -1.5), v(1.5, 2.8, 1.5), shade(color, 0.6));
        for s in [-1.0f32, 1.0] {
            boxr(&mut a.glass, v(-0.9, 1.1, s * 1.21 - 0.02), v(0.9, 2.2, s * 1.21 + 0.02), c(0.3, 0.4, 0.5));
            boxr(&mut a.glass, v(s * 1.21 - 0.02, 1.1, -0.9), v(s * 1.21 + 0.02, 2.2, 0.9), c(0.3, 0.4, 0.5));
        }
        a.glow.cuboid(v(0.0, 2.58, 0.0), v(1.6, 0.03, 1.6), c(1.0, 0.95, 0.8));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 1.4, z), v(2.6, 2.8, 2.6));
    }

    // -----------------------------------------------------------------------
    // Shipping Yard
    // -----------------------------------------------------------------------

    fn expand_yard(&mut self) {
        self.door(10.0, 0.0, 1, 1000, "Rail Yard");
        self.door(-6.0, 0.0, 2, 750, "Dockside");
        self.door(0.0, 15.0, 3, 1000, "Warehouse");
        self.door(0.0, -18.0, 4, 1250, "Truck Depot");
        self.perk_spots[4] = v(20.0, 0.0, -47.0);

        // North: rail yard with boxcars.
        self.wall_buy(v(-4.0, 0.0, 41.3), 0.0, 3);
        let cars = [c(0.55, 0.2, 0.12), c(0.2, 0.3, 0.45), c(0.35, 0.36, 0.3), c(0.6, 0.45, 0.15)];
        for (i, z) in [47.0f32, 53.0].into_iter().enumerate() {
            self.track(-40.0, 40.0, z);
            let xs: &[f32] = if i == 0 { &[-28.0, 22.0] } else { &[-8.0, 6.0, 30.0] };
            for (j, x) in xs.iter().enumerate() {
                self.boxcar(*x, z, cars[(i * 2 + j) % 4], *x);
            }
        }
        self.booth(-36.0, 43.5, 0.0, c(0.7, 0.6, 0.3));
        self.crates(4.0, 43.0, 1.0);
        self.crates(-16.0, 50.0, 2.0);
        for x in [-24.0f32, 0.0, 24.0] {
            self.lamp_post(x, 50.0, 0.0, c(1.0, 0.85, 0.6));
        }

        // South: the quay, with bollards along the water and the ship beyond.
        let mut a = Art::default();
        a.paint.cuboid(v(0.0, 0.02, -57.0), v(80.0, 0.04, 2.0), c(0.85, 0.75, 0.15));
        self.place(a, Vec3::ZERO, 0.0);
        let mut x = -36.0;
        while x <= 36.0 {
            self.bollard(x, -56.0);
            x += 8.0;
        }
        for (i, (x, z)) in [(-30.0f32, -45.0f32), (-14.0, -50.0), (8.0, -45.0), (30.0, -50.0), (-2.0, -52.0)].into_iter().enumerate() {
            self.crates(x, z, i as f32 + 10.0);
        }
        self.container(-22.0, -48.0, 0.0, true, c(0.12, 0.3, 0.6), 300.0);
        self.container(34.0, -46.0, 0.0, false, c(0.6, 0.18, 0.12), 301.0);
        self.container(34.0, -46.0, 2.6, false, c(0.15, 0.45, 0.25), 302.0);
        for x in [-26.0f32, 0.0, 26.0] {
            self.lamp_post(x, -54.5, PI, c(1.0, 0.8, 0.5));
        }

        // East: warehouse floor under a canopy, with racks and a forklift.
        self.wall_buy(v(41.3, 0.0, -8.0), FRAC_PI_2, 10);
        self.canopy(44.0, -32.0, 56.0, 32.0, 7.0, c(0.35, 0.37, 0.4));
        for (i, z) in [-26.0f32, -16.0, 24.0].into_iter().enumerate() {
            self.shelving(50.0, z, 0.0, i as f32);
        }
        for z in [-26.0f32, -16.0, 24.0] {
            self.shelving(50.0, z + 3.0, 0.0, z);
        }
        self.forklift(46.0, 6.0, 0.4, c(0.95, 0.7, 0.1));
        self.pallet_stack(53.0, 4.0, 0.0, 11.0);
        self.pallet_stack(53.0, 8.0, 0.2, 12.0);
        self.pallet_stack(46.0, -6.0, 0.1, 13.0);

        // West: truck depot.
        self.semi(-50.0, 30.0, PI * 0.95, c(0.75, 0.12, 0.1), c(0.9, 0.9, 0.88));
        self.semi(-50.0, 4.0, PI * 1.05, c(0.15, 0.3, 0.65), c(0.8, 0.8, 0.78));
        self.semi(-46.0, -30.0, 0.1, c(0.95, 0.75, 0.1), c(0.6, 0.18, 0.12));
        self.fuel_pumps(-48.0, -10.0, FRAC_PI_2, true);
        self.tires(-55.0, 16.0, 4);
        self.tires(-44.0, 20.0, 3);
        self.drums(-55.0, -22.0, 3, 21.0);
        self.booth(-44.0, -36.0, 0.0, c(0.85, 0.85, 0.82));
    }

    // -----------------------------------------------------------------------
    // Central Park
    // -----------------------------------------------------------------------

    fn greenhouse(&mut self, x: f32, z: f32) {
        let mut a = Art::default();
        let (l, w, h) = (14.0, 7.0, 3.2);
        let white = c(0.92, 0.93, 0.9);
        boxr(&mut a.paint, v(-l / 2.0, 0.0, -w / 2.0), v(l / 2.0, 0.6, w / 2.0), c(0.6, 0.55, 0.5));
        boxr(&mut a.glass, v(-l / 2.0, 0.6, -w / 2.0), v(l / 2.0, h, w / 2.0), c(0.7, 0.85, 0.8));
        a.glass.wedge(v(0.0, h + 1.0, 0.0), v(w, 2.0, l), Quat::from_rotation_y(FRAC_PI_2), c(0.7, 0.85, 0.8));
        let mut t = -l / 2.0;
        while t <= l / 2.0 + 0.01 {
            for s in [-1.0f32, 1.0] {
                a.metal.cuboid(v(t, h / 2.0, s * w / 2.0), v(0.08, h, 0.08), white);
                a.metal.beam(v(t, h, s * w / 2.0), v(t, h + 2.0, 0.0), Vec2::splat(0.08), white);
            }
            t += 1.75;
        }
        a.metal.beam(v(-l / 2.0, h + 2.0, 0.0), v(l / 2.0, h + 2.0, 0.0), Vec2::splat(0.1), white);
        // Plants inside.
        for i in 0..10 {
            let px = -l / 2.0 + 1.0 + i as f32 * 1.3;
            for s in [-1.0f32, 1.0] {
                a.paint.blob(v(px, 1.0, s * 2.0), v(0.5, 0.6 + hash(px, s) * 0.5, 0.5), c(0.2, 0.5 + hash(s, px) * 0.2, 0.18));
            }
        }
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 2.5, z), v(l, 5.0, w));
    }

    fn flower_bed(&mut self, x: f32, z: f32, l: f32, seed: f32) {
        let mut a = Art::default();
        boxr(&mut a.paint, v(-l / 2.0, 0.0, -0.8), v(l / 2.0, 0.45, 0.8), c(0.5, 0.36, 0.25));
        boxr(&mut a.paint, v(-l / 2.0 + 0.1, 0.45, -0.7), v(l / 2.0 - 0.1, 0.5, 0.7), c(0.3, 0.2, 0.12));
        let cols = [c(0.95, 0.3, 0.4), c(0.95, 0.85, 0.25), c(0.65, 0.4, 0.95), c(1.0, 1.0, 1.0), c(1.0, 0.55, 0.2)];
        let n = (l * 2.0) as i32;
        for i in 0..n {
            let px = -l / 2.0 + 0.3 + i as f32 * (l - 0.6) / n as f32;
            for s in [-0.35f32, 0.35] {
                a.paint.cyl(v(px, 0.65, s), 0.02, 0.3, Quat::IDENTITY, c(0.2, 0.45, 0.15));
                a.paint.sphere(v(px, 0.82, s), 0.11, cols[((hash(px, seed + s) * 5.0) as usize).min(4)]);
            }
        }
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 0.25, z), v(l, 0.5, 1.6));
    }

    fn tennis_court(&mut self, x: f32, z: f32, color: Color) {
        let mut a = Art::default();
        let (l, w) = (24.0, 11.0);
        a.paint.cuboid(v(0.0, 0.015, 0.0), v(w + 3.0, 0.02, l + 4.0), shade(color, 0.7));
        a.paint.cuboid(v(0.0, 0.02, 0.0), v(w, 0.02, l), color);
        let white = c(0.95, 0.95, 0.95);
        for s in [-1.0f32, 1.0] {
            a.paint.cuboid(v(s * w / 2.0, 0.03, 0.0), v(0.08, 0.02, l), white);
            a.paint.cuboid(v(0.0, 0.03, s * l / 2.0), v(w, 0.02, 0.08), white);
            a.paint.cuboid(v(s * (w / 2.0 - 1.4), 0.03, 0.0), v(0.06, 0.02, l), white);
            a.paint.cuboid(v(0.0, 0.03, s * 6.4), v(w - 2.8, 0.02, 0.06), white);
            a.metal.cyl(v(s * (w / 2.0 + 0.5), 0.55, 0.0), 0.05, 1.1, Quat::IDENTITY, c(0.2, 0.25, 0.2));
        }
        a.paint.cuboid(v(0.0, 0.03, 0.0), v(0.06, 0.02, 12.8), white);
        a.glass.cuboid(v(0.0, 0.5, 0.0), v(w + 1.0, 0.9, 0.03), c(0.1, 0.1, 0.1));
        a.paint.cuboid(v(0.0, 0.97, 0.0), v(w + 1.0, 0.06, 0.05), white);
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 0.5, z), v(w + 1.0, 1.0, 0.2));
    }

    fn rowboat(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        a.paint.blob(v(0.0, 0.2, 0.0), v(0.7, 0.3, 1.9), color);
        a.paint.blob(v(0.0, 0.32, 0.0), v(0.58, 0.2, 1.75), c(0.45, 0.32, 0.2));
        for zz in [-0.6f32, 0.6] {
            a.paint.cuboid(v(0.0, 0.42, zz), v(1.1, 0.06, 0.25), c(0.55, 0.4, 0.25));
        }
        a.paint.cyl(v(0.4, 0.45, 0.0), 0.03, 2.2, Quat::from_rotation_x(1.3), c(0.6, 0.45, 0.3));
        self.place(a, v(x, 0.0, z), yaw);
    }

    fn boathouse(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let wood = c(0.5, 0.3, 0.2);
        boxr(&mut a.paint, v(-4.0, 0.0, -3.0), v(4.0, 3.2, 3.0), wood);
        for i in 0..14 {
            let y = 0.2 + i as f32 * 0.22;
            boxr(&mut a.paint, v(-4.05, y, -3.05), v(4.05, y + 0.04, 3.05), shade(wood, 0.8));
        }
        a.paint.wedge(v(0.0, 4.2, 0.0), v(8.6, 2.0, 6.6), Quat::IDENTITY, c(0.25, 0.3, 0.28));
        boxr(&mut a.paint, v(-1.0, 0.0, 3.0), v(1.0, 2.3, 3.06), c(0.85, 0.85, 0.8));
        for s in [-1.0f32, 1.0] {
            boxr(&mut a.glass, v(s * 2.6 - 0.6, 1.3, 3.0), v(s * 2.6 + 0.6, 2.3, 3.06), c(0.35, 0.45, 0.55));
        }
        a.glow.cuboid(v(0.0, 2.6, 3.2), v(0.3, 0.2, 0.1), c(1.0, 0.85, 0.55));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 2.0, 0.0), v(8.0, 4.0, 6.0));
        self.light(v(x, 2.7, z) + Quat::from_rotation_y(yaw) * v(0.0, 0.0, 3.6), c(1.0, 0.85, 0.55), 50_000.0);
    }

    fn expand_park(&mut self) {
        self.door(0.0, 0.0, 1, 1000, "Greenhouse Garden");
        self.door(0.0, 0.0, 2, 750, "Parking Lot");
        self.door(0.0, 0.0, 3, 1000, "Tennis Courts");
        self.door(0.0, 0.0, 4, 1250, "Boathouse Lake");
        self.perk_spots[4] = v(-46.0, 0.0, 12.0);
        let path = c(0.7, 0.64, 0.52);

        // North: greenhouse and flower beds.
        self.wall_buy(v(-7.0, 0.0, 41.3), 0.0, 2);
        self.ground_paint(v(0.0, 0.0, 49.0), Vec2::new(4.0, 17.0), 0.0, path, 0.008);
        self.ground_paint(v(0.0, 0.0, 46.0), Vec2::new(70.0, 3.0), 0.0, path, 0.008);
        self.greenhouse(-20.0, 52.0);
        self.greenhouse(20.0, 52.0);
        for x in [-30.0f32, -10.0, 10.0, 30.0] {
            self.flower_bed(x, 43.6, 6.0, x);
        }
        for x in [-6.0f32, 6.0] {
            self.flower_bed(x, 52.0, 4.0, x + 1.0);
        }
        self.park_lamp(-6.0, 47.6);
        self.park_lamp(6.0, 47.6);
        self.bench(-14.0, 47.8, PI);
        self.bench(14.0, 47.8, PI);

        // South: car park.
        let mut a = Art::default();
        a.paint.cuboid(v(0.0, 0.01, -49.5), v(80.0, 0.02, 17.0), c(0.22, 0.22, 0.24));
        for i in 0..20 {
            let x = -38.0 + i as f32 * 4.0;
            for z in [-45.5f32, -53.5] {
                a.paint.cuboid(v(x, 0.025, z), v(0.12, 0.02, 4.5), c(0.9, 0.9, 0.88));
            }
        }
        self.place(a, Vec3::ZERO, 0.0);
        let car_colors = [c(0.7, 0.1, 0.1), c(0.1, 0.2, 0.6), c(0.85, 0.85, 0.85), c(0.15, 0.15, 0.15), c(0.2, 0.45, 0.3), c(0.9, 0.7, 0.2)];
        for (i, (x, z)) in [(-32.0f32, -45.5f32), (-24.0, -53.5), (-12.0, -45.5), (-4.0, -53.5), (10.0, -45.5), (18.0, -45.5), (26.0, -53.5), (34.0, -45.5)].into_iter().enumerate() {
            self.car(x + 2.0, z, FRAC_PI_2, car_colors[i % car_colors.len()]);
        }
        self.booth(-36.0, -42.5, 0.0, c(0.85, 0.85, 0.82));
        for x in [-20.0f32, 0.0, 20.0] {
            self.lamp_post(x, -49.5, 0.0, c(1.0, 0.85, 0.6));
        }

        // East: two tennis courts and benches.
        self.wall_buy(v(41.3, 0.0, -12.0), FRAC_PI_2, 7);
        self.tennis_court(50.0, -18.0, c(0.2, 0.45, 0.6));
        self.tennis_court(50.0, 18.0, c(0.25, 0.5, 0.3));
        for z in [-2.0f32, 2.0] {
            self.bench(50.0, z, if z < 0.0 { PI } else { 0.0 });
        }
        self.park_lamp(44.0, 0.0);
        self.park_lamp(56.0, 0.0);

        // West: the lake, a boardwalk, boats and the boathouse.
        let mut a = Art::default();
        a.glass.cuboid(v(-52.0, 0.04, 0.0), v(9.0, 0.04, 50.0), c(0.2, 0.42, 0.62));
        a.paint.cuboid(v(-52.0, 0.01, 0.0), v(9.4, 0.02, 50.4), c(0.18, 0.25, 0.2));
        let wood = c(0.55, 0.4, 0.27);
        let mut zz = -24.0;
        while zz < 24.0 {
            a.paint.cuboid(v(-46.8, 0.25, zz), v(2.2, 0.08, 0.28), shade(wood, 0.9 + hash(zz, 1.0) * 0.2));
            zz += 0.32;
        }
        for zz in [-24.0f32, -12.0, 0.0, 12.0, 24.0] {
            for s in [-1.0f32, 1.0] {
                a.paint.cyl(v(-46.8 + s * 1.0, 0.5, zz), 0.07, 1.0, Quat::IDENTITY, shade(wood, 0.7));
            }
        }
        self.place(a, Vec3::ZERO, 0.0);
        self.rowboat(-52.0, -8.0, 0.3, c(0.85, 0.85, 0.82));
        self.rowboat(-53.5, 6.0, -0.4, c(0.7, 0.15, 0.12));
        self.rowboat(-51.0, 20.0, 0.1, c(0.2, 0.35, 0.6));
        self.boathouse(-50.0, -32.0, 0.0);
        self.park_lamp(-45.0, -14.0);
        self.park_lamp(-45.0, 26.0);
        for i in 0..6 {
            let zz = -20.0 + i as f32 * 8.0;
            self.bush(-56.5, zz, 1.2, zz, None);
        }
    }

    // -----------------------------------------------------------------------
    // The Neighborhood
    // -----------------------------------------------------------------------

    fn garage(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        boxr(&mut a.paint, v(-3.0, 0.0, -3.0), v(3.0, 3.0, 3.0), color);
        boxr(&mut a.paint, v(-3.2, 3.0, -3.2), v(3.2, 3.25, 3.2), shade(color, 0.6));
        boxr(&mut a.metal, v(-2.3, 0.0, 3.0), v(2.3, 2.4, 3.06), c(0.82, 0.82, 0.8));
        for i in 0..6 {
            let y = 0.35 + i as f32 * 0.38;
            boxr(&mut a.metal, v(-2.3, y, 3.06), v(2.3, y + 0.04, 3.09), c(0.6, 0.6, 0.6));
        }
        a.glow.cuboid(v(0.0, 2.75, 3.15), v(0.35, 0.18, 0.1), c(1.0, 0.85, 0.55));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.6, 0.0), v(6.0, 3.2, 6.0));
    }

    fn dumpster(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        boxr(&mut a.metal, v(-1.0, 0.15, -0.8), v(1.0, 1.3, 0.8), color);
        a.metal.cuboid_rot(v(0.0, 1.38, 0.0), v(2.05, 0.08, 1.7), Quat::from_rotation_x(0.08), shade(color, 0.7));
        for (px, pz) in [(-0.8f32, -0.6f32), (0.8, -0.6), (-0.8, 0.6), (0.8, 0.6)] {
            a.paint.cyl(v(px, 0.08, pz), 0.08, 0.08, Quat::from_rotation_z(FRAC_PI_2), c(0.05, 0.05, 0.05));
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.7, 0.0), v(2.0, 1.4, 1.6));
    }

    /// Wooden power poles along X with sagging wires between them.
    fn power_line(&mut self, x0: f32, x1: f32, z: f32) {
        let mut a = Art::default();
        let wood = c(0.42, 0.3, 0.2);
        let mut x = x0;
        let mut prev: Option<f32> = None;
        while x <= x1 {
            a.paint.cyl(v(x, 4.5, z), 0.13, 9.0, Quat::IDENTITY, wood);
            a.paint.cuboid(v(x, 8.4, z), v(0.15, 0.15, 2.2), wood);
            for s in [-0.9f32, 0.0, 0.9] {
                a.metal.cyl(v(x, 8.6, z + s), 0.05, 0.2, Quat::IDENTITY, c(0.4, 0.55, 0.45));
            }
            a.metal.cyl(v(x, 7.4, z + 0.3), 0.25, 0.8, Quat::IDENTITY, c(0.55, 0.56, 0.58));
            if let Some(p) = prev {
                for s in [-0.9f32, 0.0, 0.9] {
                    let n = 8;
                    for i in 0..n {
                        let t0 = i as f32 / n as f32;
                        let t1 = (i + 1) as f32 / n as f32;
                        let sag = |t: f32| 8.7 - 0.8 * (1.0 - (2.0 * t - 1.0).powi(2));
                        a.metal.beam(v(p + (x - p) * t0, sag(t0), z + s), v(p + (x - p) * t1, sag(t1), z + s), Vec2::splat(0.02), c(0.05, 0.05, 0.05));
                    }
                }
            }
            self.collide(v(x, 2.0, z), v(0.3, 4.0, 0.3));
            prev = Some(x);
            x += 20.0;
        }
        self.place(a, Vec3::ZERO, 0.0);
    }

    fn bleachers(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let alu = c(0.75, 0.76, 0.78);
        for row in 0..4 {
            let y = 0.4 + row as f32 * 0.4;
            let zz = row as f32 * 0.6;
            boxr(&mut a.metal, v(-5.0, y, zz - 0.25), v(5.0, y + 0.06, zz + 0.25), alu);
            boxr(&mut a.metal, v(-5.0, y - 0.4, zz - 0.3), v(5.0, y, zz - 0.26), shade(alu, 0.8));
        }
        for px in [-4.8f32, 0.0, 4.8] {
            a.metal.beam(v(px, 0.0, -0.3), v(px, 2.0, 2.1), Vec2::splat(0.08), c(0.4, 0.4, 0.42));
            a.metal.cuboid(v(px, 1.0, 2.1), v(0.08, 2.0, 0.08), c(0.4, 0.4, 0.42));
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.0, 0.9), v(10.0, 2.0, 2.4));
    }

    fn school_bus(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let yellow = c(0.95, 0.72, 0.1);
        let dark = c(0.06, 0.06, 0.06);
        boxr(&mut a.metal, v(-1.25, 0.6, -5.0), v(1.25, 3.0, 4.2), yellow);
        boxr(&mut a.metal, v(-1.2, 0.6, 4.2), v(1.2, 1.9, 5.6), yellow);
        for s in [-1.0f32, 1.0] {
            boxr(&mut a.paint, v(s * 1.26 - 0.02, 1.25, -5.0), v(s * 1.26 + 0.02, 1.35, 4.2), dark);
            let mut zz = -4.4;
            while zz < 3.8 {
                boxr(&mut a.glass, v(s * 1.26 - 0.02, 1.8, zz), v(s * 1.26 + 0.02, 2.6, zz + 0.9), c(0.12, 0.16, 0.2));
                zz += 1.1;
            }
            for wz in [-3.2f32, 3.6] {
                a.paint.cyl(v(s * 1.1, 0.5, wz), 0.5, 0.35, Quat::from_rotation_z(FRAC_PI_2), dark);
            }
        }
        boxr(&mut a.glass, v(-1.1, 1.9, 4.18), v(1.1, 2.8, 4.24), c(0.12, 0.16, 0.2));
        a.glow.cuboid(v(0.0, 1.4, 5.62), v(1.8, 0.2, 0.04), c(1.0, 0.97, 0.85));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.5, 0.3), v(2.6, 3.0, 10.6));
    }

    fn store(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let wall = c(0.85, 0.82, 0.75);
        boxr(&mut a.paint, v(-6.0, 0.0, -4.0), v(6.0, 4.0, 4.0), wall);
        boxr(&mut a.paint, v(-6.2, 4.0, -4.2), v(6.2, 4.6, 4.2), c(0.75, 0.15, 0.12));
        boxr(&mut a.glass, v(-5.0, 0.3, 4.0), v(3.0, 3.0, 4.06), c(0.35, 0.45, 0.5));
        boxr(&mut a.paint, v(3.6, 0.0, 4.0), v(5.2, 2.6, 4.06), c(0.3, 0.33, 0.38));
        a.glow.cuboid(v(0.0, 4.3, 4.25), v(7.0, 0.5, 0.06), c(1.0, 0.95, 0.6));
        a.glow.cuboid(v(-1.0, 1.6, 4.02), v(7.6, 2.4, 0.01), c(0.55, 0.6, 0.55));
        // Shelves seen through the window.
        for px in [-3.5f32, -0.5, 2.0] {
            boxr(&mut a.paint, v(px - 0.8, 0.0, 1.5), v(px + 0.8, 1.6, 2.2), c(0.6, 0.3, 0.2));
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 2.3, 0.0), v(12.0, 4.6, 8.0));
    }

    fn scaffold(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let pipe = c(0.55, 0.57, 0.6);
        let plank = c(0.65, 0.5, 0.3);
        let (l, w) = (10.0, 8.0);
        // House frame: studs and a floor slab.
        boxr(&mut a.paint, v(-l / 2.0, 0.0, -w / 2.0), v(l / 2.0, 0.3, w / 2.0), c(0.65, 0.65, 0.63));
        let mut t = -l / 2.0;
        while t <= l / 2.0 + 0.01 {
            for s in [-1.0f32, 1.0] {
                a.paint.cuboid(v(t, 1.8, s * w / 2.0), v(0.1, 3.0, 0.15), plank);
            }
            t += 0.8;
        }
        let mut t = -w / 2.0;
        while t <= w / 2.0 + 0.01 {
            a.paint.cuboid(v(-l / 2.0, 1.8, t), v(0.15, 3.0, 0.1), plank);
            t += 0.8;
        }
        boxr(&mut a.paint, v(-l / 2.0, 3.3, -w / 2.0), v(l / 2.0, 3.45, w / 2.0), plank);
        // Scaffold along the front.
        for px in [-l / 2.0, 0.0, l / 2.0] {
            for zz in [w / 2.0 + 0.6, w / 2.0 + 1.8] {
                a.metal.cyl(v(px, 3.0, zz), 0.04, 6.0, Quat::IDENTITY, pipe);
            }
        }
        for y in [1.5f32, 3.4, 5.2] {
            boxr(&mut a.paint, v(-l / 2.0, y, w / 2.0 + 0.5), v(l / 2.0, y + 0.06, w / 2.0 + 1.9), plank);
            a.metal.cyl(v(0.0, y + 1.0, w / 2.0 + 1.85), 0.03, l, Quat::from_rotation_z(FRAC_PI_2), pipe);
        }
        self.place(a, v(x, 0.0, z), yaw);
        // Walls you can't pass, but the open back lets you through.
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.8, w / 2.0), v(l, 3.6, 0.3));
        self.collide_local(v(x, 0.0, z), yaw, v(-l / 2.0, 1.8, 0.0), v(0.3, 3.6, w));
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.8, -w / 2.0), v(l * 0.5, 3.6, 0.3));
    }

    fn dirt_pile(&mut self, x: f32, z: f32, size: f32) {
        let mut a = Art::default();
        a.paint.blob(v(0.0, 0.0, 0.0), v(2.2, 1.3, 1.8) * size, c(0.42, 0.3, 0.2));
        a.paint.blob(v(0.8, 0.0, 0.6) * size, v(1.4, 0.9, 1.2) * size, c(0.38, 0.27, 0.18));
        self.place(a, v(x, 0.0, z), x);
        self.collide(v(x, 0.5 * size, z), v(3.6, 1.0, 3.0) * size);
    }

    fn pipes(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        for (i, (px, py)) in [(-0.5f32, 0.35f32), (0.25, 0.35), (1.0, 0.35), (-0.12, 0.98), (0.62, 0.98)].into_iter().enumerate() {
            a.metal.cyl(v(px, py, 0.0), 0.33, 4.0, Quat::from_rotation_x(FRAC_PI_2), c(0.55 + i as f32 * 0.02, 0.56, 0.6));
            a.paint.cyl(v(px, py, 2.01), 0.25, 0.02, Quat::from_rotation_x(FRAC_PI_2), c(0.1, 0.1, 0.1));
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.25, 0.65, 0.0), v(2.2, 1.3, 4.0));
    }

    fn cement_mixer(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let orange = c(0.9, 0.45, 0.1);
        boxr(&mut a.metal, v(-0.6, 0.4, -0.8), v(0.6, 0.6, 0.8), c(0.3, 0.3, 0.3));
        a.metal.frustum(v(0.0, 1.3, 0.1), 0.35, 0.6, 1.1, Quat::from_rotation_x(-0.6), orange);
        a.metal.cyl(v(0.0, 1.1, -0.3), 0.55, 0.3, Quat::from_rotation_x(-0.6), orange);
        for s in [-1.0f32, 1.0] {
            a.paint.cyl(v(s * 0.65, 0.3, -0.5), 0.3, 0.15, Quat::from_rotation_z(FRAC_PI_2), c(0.06, 0.06, 0.06));
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 0.8, z), v(1.6, 1.6, 1.8));
    }

    fn porta_potty(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let blue = c(0.15, 0.35, 0.75);
        boxr(&mut a.paint, v(-0.6, 0.0, -0.6), v(0.6, 2.3, 0.6), blue);
        a.paint.cyl(v(0.0, 2.35, 0.0), 0.62, 0.1, Quat::IDENTITY, c(0.9, 0.9, 0.9));
        boxr(&mut a.paint, v(-0.45, 0.1, 0.6), v(0.45, 2.1, 0.63), shade(blue, 0.8));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 1.15, z), v(1.2, 2.3, 1.2));
    }

    fn expand_neighborhood(&mut self) {
        self.door(10.0, 0.0, 1, 1000, "Back Alley");
        self.door(-10.0, 0.0, 2, 750, "School Yard");
        self.door(0.0, 0.0, 3, 1000, "Gas Station");
        self.door(0.0, 0.0, 4, 1250, "Construction Site");
        self.perk_spots[4] = v(16.0, 0.0, -47.0);
        let asphalt = c(0.17, 0.17, 0.19);

        // North: back alley with garages, dumpsters and power lines.
        self.wall_buy(v(-6.0, 0.0, 41.3), 0.0, 12);
        self.ground_paint(v(0.0, 0.0, 48.0), Vec2::new(80.0, 6.0), 0.0, asphalt, 0.006);
        let cols = [c(0.75, 0.7, 0.6), c(0.6, 0.65, 0.7), c(0.7, 0.55, 0.45), c(0.65, 0.7, 0.6)];
        for (i, x) in [-30.0f32, -20.0, -4.0, 6.0, 22.0, 32.0].into_iter().enumerate() {
            self.garage(x, 54.0, PI, cols[i % 4]);
        }
        self.dumpster(-12.0, 51.8, 0.0, c(0.2, 0.4, 0.25));
        self.dumpster(14.0, 51.8, 0.0, c(0.25, 0.3, 0.55));
        self.trash_cans(-25.0, 43.5);
        self.trash_cans(27.0, 43.5);
        self.power_line(-38.0, 38.0, 44.2);
        self.street_light(0.0, 51.5, 0.0);

        // South: school yard with a court, bleachers and a bus.
        let mut a = Art::default();
        a.paint.cuboid(v(0.0, 0.01, -49.0), v(30.0, 0.02, 15.0), c(0.55, 0.25, 0.2));
        let white = c(0.95, 0.95, 0.95);
        for s in [-1.0f32, 1.0] {
            a.paint.cuboid(v(s * 14.0, 0.025, -49.0), v(0.1, 0.02, 14.0), white);
            a.paint.cuboid(v(0.0, 0.025, -49.0 + s * 7.0), v(28.0, 0.02, 0.1), white);
            a.paint.torus(v(s * 11.0, 0.025, -49.0), 0.05, 3.0, Quat::IDENTITY, white);
        }
        a.paint.torus(v(0.0, 0.025, -49.0), 0.05, 1.8, Quat::IDENTITY, white);
        a.paint.cuboid(v(0.0, 0.025, -49.0), v(0.1, 0.02, 14.0), white);
        self.place(a, Vec3::ZERO, 0.0);
        self.hoop(-14.5, -49.0, -FRAC_PI_2);
        self.hoop(14.5, -49.0, FRAC_PI_2);
        self.bleachers(0.0, -56.0, 0.0);
        self.school_bus(-30.0, -49.0, 0.0);
        let mut a = Art::default();
        a.metal.cyl(v(0.0, 5.0, 0.0), 0.06, 10.0, Quat::IDENTITY, c(0.75, 0.75, 0.78));
        boxr(&mut a.paint, v(0.06, 8.6, -0.02), v(1.9, 9.8, 0.02), c(0.2, 0.3, 0.7));
        self.place(a, v(28.0, 0.0, -44.0), 0.0);
        self.collide(v(28.0, 2.0, -44.0), v(0.3, 4.0, 0.3));
        self.street_light(-20.0, -42.5, PI);
        self.street_light(22.0, -55.0, 0.0);

        // East: the street carries on to a gas station and a corner store.
        self.ground_paint(v(49.0, 0.0, 0.0), Vec2::new(18.0, 8.0), 0.0, asphalt, 0.006);
        self.ground_paint(v(50.0, 0.0, -14.0), Vec2::new(16.0, 20.0), 0.0, c(0.3, 0.3, 0.31), 0.005);
        self.wall_buy(v(41.3, 0.0, 14.0), FRAC_PI_2, 9);
        self.fuel_pumps(50.0, -11.0, 0.0, true);
        self.store(50.0, -27.0, 0.0);
        self.car(46.0, -5.6, 0.0, c(0.7, 0.1, 0.1));
        self.car(52.0, 14.0, FRAC_PI_2, c(0.15, 0.15, 0.15));
        self.trash_cans(44.0, -20.0);
        self.street_light(48.0, 6.6, 0.0);
        self.street_light(56.0, -6.6, PI);
        for z in [20.0f32, 28.0, 34.0] {
            self.bush(56.0, z, 1.1, z, None);
        }

        // West: a house going up, with diggings and site clutter.
        self.ground_paint(v(-49.0, 0.0, 0.0), Vec2::new(18.0, 8.0), 0.0, asphalt, 0.006);
        self.ground_paint(v(-50.0, 0.0, 22.0), Vec2::new(16.0, 26.0), 0.0, c(0.45, 0.35, 0.25), 0.005);
        self.scaffold(-50.0, 24.0, PI);
        self.dirt_pile(-46.0, -16.0, 1.0);
        self.dirt_pile(-53.0, -26.0, 1.3);
        self.pipes(-45.0, -32.0, FRAC_PI_2);
        self.cement_mixer(-54.0, -12.0, 0.6);
        self.porta_potty(-56.0, 9.0, FRAC_PI_2);
        self.barriers(-44.0, -6.5, 0.0, 2, c(0.9, 0.5, 0.1));
        self.barriers(-44.0, 6.5, 0.0, 2, c(0.9, 0.5, 0.1));
        for (x, z) in [(-48.0f32, -4.0f32), (-50.0, 4.0), (-46.0, 10.0)] {
            self.traffic_cone(x, z);
        }
        self.street_light(-48.0, -6.6, PI);
        self.light_tower(-55.0, 35.0, PI * 0.75);
    }
}

// ---------------------------------------------------------------------------
// Doors and wall guns in the world
// ---------------------------------------------------------------------------

/// A buyable door; it rolls up when its area is opened.
#[derive(Component)]
pub struct Door {
    zone: u8,
    lift: f32,
}

/// The rolling shutter of a door.
#[derive(Component)]
pub struct DoorPanel;

/// The lamp over a door: red while locked, green once open.
#[derive(Component)]
pub struct DoorLamp;

#[derive(Resource)]
pub struct DoorMats {
    locked: Handle<StandardMaterial>,
    open: Handle<StandardMaterial>,
}

const DOOR_HEIGHT: f32 = 3.25;

/// Spawns the doors (a shutter, a collider and a lamp each) and the guns on
/// the wall-buy boards.
pub fn spawn_doors(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    guns: &crate::gunmodels::GunAssets,
    layout: &MapLayout,
) {
    let shutter = {
        let mut k = crate::kit::Kit::new();
        let w = DOOR_WIDTH + 0.1;
        let steel = c(0.52, 0.54, 0.56);
        k.cuboid(v(0.0, DOOR_HEIGHT / 2.0, 0.0), v(w, DOOR_HEIGHT, 0.08), steel);
        // Ribs on both faces.
        let mut y = 0.2;
        while y < DOOR_HEIGHT {
            for s in [-1.0f32, 1.0] {
                k.cuboid(v(0.0, y, s * 0.05), v(w, 0.05, 0.03), shade(steel, 0.75));
            }
            y += 0.22;
        }
        // Hazard stripes along the bottom, and a handle.
        for i in 0..10 {
            let x = -w / 2.0 + (i as f32 + 0.5) * w / 10.0;
            let col = if i % 2 == 0 { c(0.95, 0.75, 0.1) } else { c(0.1, 0.1, 0.1) };
            for s in [-1.0f32, 1.0] {
                k.cuboid(v(x, 0.17, s * 0.06), v(w / 10.0, 0.3, 0.02), col);
            }
        }
        for s in [-1.0f32, 1.0] {
            k.cuboid(v(0.0, 0.45, s * 0.09), v(0.6, 0.06, 0.05), c(0.2, 0.2, 0.2));
        }
        meshes.add(k.build_or_empty())
    };
    let steel = materials.add(crate::kit::vertex_material(0.5, 0.6));
    let lamp = |color: Color, materials: &mut Assets<StandardMaterial>| {
        materials.add(StandardMaterial {
            base_color: color,
            emissive: LinearRgba::from(color) * 6.0,
            ..default()
        })
    };
    let mats = DoorMats {
        locked: lamp(Color::srgb(1.0, 0.15, 0.1), materials),
        open: lamp(Color::srgb(0.2, 1.0, 0.3), materials),
    };
    let bulb = meshes.add(Sphere::new(0.13));
    for d in &layout.doors {
        let yaw = if d.along_x { 0.0 } else { FRAC_PI_2 };
        let half = if d.along_x { v(DOOR_WIDTH / 2.0, DOOR_HEIGHT / 2.0, 0.2) } else { v(0.2, DOOR_HEIGHT / 2.0, DOOR_WIDTH / 2.0) };
        commands
            .spawn((
                crate::InGameEntity,
                Door { zone: d.zone, lift: 0.0 },
                Transform::from_translation(d.pos + Vec3::Y * DOOR_HEIGHT / 2.0),
                Visibility::default(),
                crate::Collider { half },
            ))
            .with_children(|p| {
                p.spawn((
                    DoorPanel,
                    Mesh3d(shutter.clone()),
                    MeshMaterial3d(steel.clone()),
                    Transform::from_xyz(0.0, -DOOR_HEIGHT / 2.0, 0.0).with_rotation(Quat::from_rotation_y(yaw)),
                ));
                for s in [-1.0f32, 1.0] {
                    p.spawn((
                        DoorLamp,
                        Mesh3d(bulb.clone()),
                        MeshMaterial3d(mats.locked.clone()),
                        Transform::from_translation(Quat::from_rotation_y(yaw) * v(0.0, 3.75 - DOOR_HEIGHT / 2.0 + 0.1, s * 0.2)),
                    ));
                }
                p.spawn((
                    PointLight {
                        intensity: 25_000.0,
                        color: Color::srgb(1.0, 0.3, 0.2),
                        range: 7.0,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 2.6 - DOOR_HEIGHT / 2.0 + 0.6, 0.0),
                ));
            });
    }
    commands.insert_resource(mats);

    // Chalk-white guns on the wall boards.
    let chalk = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.88),
        emissive: LinearRgba::rgb(0.5, 0.5, 0.42),
        ..default()
    });
    for w in &layout.wall_buys {
        let rot = Quat::from_rotation_y(w.yaw);
        commands
            .spawn((
                crate::InGameEntity,
                Transform::from_translation(w.pos + rot * v(0.0, 1.5, 0.12))
                    .with_rotation(rot * Quat::from_rotation_y(-FRAC_PI_2))
                    .with_scale(Vec3::splat(2.4)),
                Visibility::default(),
            ))
            .with_children(|g| crate::gunmodels::spawn_gun(g, guns, w.gun, crate::data::Attach::NONE, chalk.clone(), false));
    }
}

/// Opens doors when their area is bought: the collider goes, the shutter
/// rolls up and the lamps turn green.
#[allow(clippy::type_complexity)]
pub fn door_visuals(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<crate::MatchState>,
    mats: Option<Res<DoorMats>>,
    mut doors: Query<(Entity, &mut Door, &Children, Has<crate::Collider>)>,
    mut panels: Query<&mut Transform, With<DoorPanel>>,
    mut lamps: Query<&mut MeshMaterial3d<StandardMaterial>, With<DoorLamp>>,
    mut lights: Query<&mut PointLight>,
) {
    let Some(mats) = mats else { return };
    for (e, mut door, children, solid) in &mut doors {
        let open = MapLayout::zone_open(state.doors, door.zone);
        if open && solid {
            commands.entity(e).remove::<crate::Collider>();
            for &child in children {
                if let Ok(mut m) = lamps.get_mut(child) {
                    m.0 = mats.open.clone();
                }
                if let Ok(mut l) = lights.get_mut(child) {
                    l.color = Color::srgb(0.3, 1.0, 0.4);
                }
            }
        }
        let target = if open { 1.0 } else { 0.0 };
        if door.lift == target {
            continue;
        }
        door.lift = (door.lift + time.delta_secs() * 0.8).min(target);
        for &child in children {
            if let Ok(mut tf) = panels.get_mut(child) {
                // Rolls up into the lintel.
                let k = door.lift;
                tf.scale.y = (1.0 - k).max(0.02);
                tf.translation.y = -DOOR_HEIGHT / 2.0 + k * (DOOR_HEIGHT - 0.1);
            }
        }
    }
}
