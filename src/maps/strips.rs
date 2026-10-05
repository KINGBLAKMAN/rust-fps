//! Props shared by the maps (rail cars, trucks, sheds, school and building
//! site clutter), and the wall guns in the world.

use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

use crate::kit::c;
use crate::maps::MapLayout;
use crate::props::{boxr, hash, shade, v, Art};


impl MapLayout {
    // -----------------------------------------------------------------------
    // Shared props for the new areas
    // -----------------------------------------------------------------------

    /// Railway track along X from `x0` to `x1`.
    pub(crate) fn track(&mut self, x0: f32, x1: f32, z: f32) {
        let mut a = Art::default();
        let mut x = x0;
        while x <= x1 {
            boxr(
                &mut a.paint,
                v(x - 0.12, 0.0, z - 1.3),
                v(x + 0.12, 0.1, z + 1.3),
                c(0.32, 0.24, 0.17),
            );
            x += 0.7;
        }
        for s in [-0.72f32, 0.72] {
            boxr(
                &mut a.metal,
                v(x0, 0.1, z + s - 0.04),
                v(x1, 0.22, z + s + 0.04),
                c(0.42, 0.4, 0.38),
            );
        }
        a.paint.cuboid(
            v((x0 + x1) / 2.0, 0.02, z),
            v(x1 - x0, 0.04, 3.2),
            c(0.36, 0.34, 0.32),
        );
        self.place(a, Vec3::ZERO, 0.0);
    }

    /// Railway boxcar standing on a track along X.
    pub(crate) fn boxcar(&mut self, x: f32, z: f32, color: Color, seed: f32) {
        let mut a = Art::default();
        let (l, w, h) = (12.0, 3.0, 3.6);
        boxr(
            &mut a.paint,
            v(-l / 2.0, 1.1, -w / 2.0),
            v(l / 2.0, 1.1 + h, w / 2.0),
            color,
        );
        for i in 0..17 {
            let px = -l / 2.0 + 0.4 + i as f32 * 0.7;
            for s in [-1.0f32, 1.0] {
                boxr(
                    &mut a.paint,
                    v(px - 0.05, 1.15, s * (w / 2.0) - 0.02),
                    v(px + 0.05, 1.05 + h, s * (w / 2.0) + 0.03),
                    shade(color, 0.75),
                );
            }
        }
        boxr(
            &mut a.paint,
            v(-l / 2.0 - 0.1, 1.05 + h, -w / 2.0 - 0.1),
            v(l / 2.0 + 0.1, 1.25 + h, w / 2.0 + 0.1),
            shade(color, 0.6),
        );
        for s in [-1.0f32, 1.0] {
            boxr(
                &mut a.paint,
                v(-1.4, 1.2, s * (w / 2.0 + 0.04)),
                v(1.4, 0.9 + h, s * (w / 2.0 + 0.07)),
                shade(color, 0.85),
            );
            a.metal.cuboid(
                v(0.0, 2.6, s * (w / 2.0 + 0.1)),
                v(0.1, 0.5, 0.05),
                c(0.3, 0.3, 0.3),
            );
            boxr(
                &mut a.paint,
                v(-3.5, 3.2, s * (w / 2.0 + 0.04)),
                v(-2.0, 3.8, s * (w / 2.0 + 0.05)),
                c(0.92, 0.92, 0.9),
            );
        }
        boxr(
            &mut a.metal,
            v(-l / 2.0 + 0.2, 0.75, -w / 2.0 + 0.2),
            v(l / 2.0 - 0.2, 1.1, w / 2.0 - 0.2),
            c(0.15, 0.15, 0.15),
        );
        for bx in [-l / 2.0 + 1.8, l / 2.0 - 1.8] {
            boxr(
                &mut a.metal,
                v(bx - 1.2, 0.3, -1.0),
                v(bx + 1.2, 0.75, 1.0),
                c(0.2, 0.2, 0.2),
            );
            for wx in [-0.7f32, 0.7] {
                for s in [-0.72f32, 0.72] {
                    a.metal.cyl(
                        v(bx + wx, 0.42, s),
                        0.42,
                        0.1,
                        Quat::from_rotation_x(FRAC_PI_2),
                        c(0.25, 0.25, 0.25),
                    );
                }
            }
        }
        let _ = seed;
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 2.4, z), v(l, 4.8, w));
    }

    pub(crate) fn bollard(&mut self, x: f32, z: f32) {
        let mut a = Art::default();
        let iron = c(0.12, 0.12, 0.13);
        a.metal
            .cyl(v(0.0, 0.35, 0.0), 0.22, 0.7, Quat::IDENTITY, iron);
        a.metal
            .cyl(v(0.0, 0.75, 0.0), 0.3, 0.12, Quat::IDENTITY, iron);
        a.metal.cyl(
            v(0.0, 0.85, 0.0),
            0.2,
            0.1,
            Quat::IDENTITY,
            c(0.85, 0.75, 0.15),
        );
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 0.4, z), v(0.6, 0.8, 0.6));
    }

    /// Wooden crates, stacked.
    pub(crate) fn crates(&mut self, x: f32, z: f32, seed: f32) {
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
            boxr(
                &mut a.paint,
                v(px - 0.55, py, pz - 0.55),
                v(px + 0.55, py + 1.1, pz + 0.55),
                col,
            );
            for s in [-1.0f32, 1.0] {
                a.paint.beam(
                    v(px - 0.5, py + 0.05, pz + s * 0.56),
                    v(px + 0.5, py + 1.05, pz + s * 0.56),
                    Vec2::new(0.1, 0.02),
                    shade(col, 0.7),
                );
                a.paint.beam(
                    v(px + s * 0.56, py + 0.05, pz - 0.5),
                    v(px + s * 0.56, py + 1.05, pz + 0.5),
                    Vec2::new(0.1, 0.02),
                    shade(col, 0.7),
                );
            }
        }
        let tall = if n > 2 { 2.2 } else { 1.1 };
        self.place(a, v(x, 0.0, z), hash(seed, 9.0) * 0.4);
        self.collide(v(x + 0.55, tall / 2.0, z + 0.4), v(2.4, tall, 2.4));
    }

    /// Harbour or yard lamp post with an overhanging lamp.
    pub(crate) fn lamp_post(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        let grey = c(0.3, 0.31, 0.33);
        a.metal
            .cyl(v(0.0, 3.5, 0.0), 0.08, 7.0, Quat::IDENTITY, grey);
        a.metal
            .beam(v(0.0, 6.9, 0.0), v(0.0, 6.9, 1.2), Vec2::splat(0.08), grey);
        boxr(&mut a.metal, v(-0.25, 6.7, 0.9), v(0.25, 6.95, 1.5), grey);
        a.glow.cuboid(v(0.0, 6.68, 1.2), v(0.4, 0.04, 0.5), color);
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 2.0, z), v(0.3, 4.0, 0.3));
        self.light(
            v(x, 6.4, z) + Quat::from_rotation_y(yaw) * v(0.0, 0.0, 1.2),
            color,
            180_000.0,
        );
    }

    /// Semi truck (cab facing +Z) with a box trailer behind it.
    pub(crate) fn semi(&mut self, x: f32, z: f32, yaw: f32, color: Color, trailer: Color) {
        let mut a = Art::default();
        let dark = c(0.08, 0.08, 0.08);
        let chrome = c(0.8, 0.8, 0.82);
        // Cab.
        boxr(&mut a.metal, v(-1.25, 1.0, 2.6), v(1.25, 3.4, 4.6), color);
        boxr(&mut a.metal, v(-1.2, 1.0, 4.6), v(1.2, 2.2, 6.0), color);
        boxr(
            &mut a.glass,
            v(-1.1, 2.5, 4.55),
            v(1.1, 3.25, 4.62),
            c(0.12, 0.16, 0.2),
        );
        for s in [-1.0f32, 1.0] {
            boxr(
                &mut a.glass,
                v(s * 1.26 - 0.02, 2.4, 3.6),
                v(s * 1.26 + 0.02, 3.2, 4.4),
                c(0.12, 0.16, 0.2),
            );
            a.metal
                .cyl(v(s * 1.0, 3.6, 2.7), 0.1, 2.2, Quat::IDENTITY, chrome);
            a.metal.cyl(
                v(s * 1.3, 0.9, 3.2),
                0.3,
                1.0,
                Quat::from_rotation_x(FRAC_PI_2),
                chrome,
            );
            a.glow.cuboid(
                v(s * 0.85, 1.6, 6.02),
                v(0.4, 0.2, 0.04),
                c(1.0, 0.97, 0.85),
            );
        }
        boxr(&mut a.metal, v(-1.0, 1.2, 6.0), v(1.0, 2.0, 6.08), chrome);
        boxr(&mut a.metal, v(-1.3, 0.5, 5.9), v(1.3, 0.8, 6.15), chrome);
        boxr(&mut a.metal, v(-1.1, 0.6, -6.5), v(1.1, 1.0, 2.6), dark);
        // Trailer.
        boxr(&mut a.paint, v(-1.3, 1.2, -9.5), v(1.3, 4.0, 2.2), trailer);
        for i in 0..20 {
            let pz = -9.3 + i as f32 * 0.58;
            for s in [-1.0f32, 1.0] {
                boxr(
                    &mut a.paint,
                    v(s * 1.3 - 0.02, 1.3, pz - 0.03),
                    v(s * 1.3 + 0.02, 3.9, pz + 0.03),
                    shade(trailer, 0.8),
                );
            }
        }
        for wz in [5.0f32, 2.0, -7.0, -8.3] {
            for s in [-1.0f32, 1.0] {
                a.paint.cyl(
                    v(s * 1.1, 0.5, wz),
                    0.5,
                    0.4,
                    Quat::from_rotation_z(FRAC_PI_2),
                    dark,
                );
                a.metal.cyl(
                    v(s * 1.31, 0.5, wz),
                    0.25,
                    0.02,
                    Quat::from_rotation_z(FRAC_PI_2),
                    chrome,
                );
            }
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 2.0, -1.75), v(2.7, 4.0, 15.6));
    }

    /// Fuel pump island under a canopy (canopy optional).
    pub(crate) fn fuel_pumps(&mut self, x: f32, z: f32, yaw: f32, canopy: bool) {
        let mut a = Art::default();
        let white = c(0.92, 0.92, 0.9);
        let red = c(0.8, 0.12, 0.1);
        boxr(
            &mut a.paint,
            v(-4.0, 0.0, -0.8),
            v(4.0, 0.2, 0.8),
            c(0.6, 0.6, 0.58),
        );
        for px in [-2.2f32, 2.2] {
            boxr(
                &mut a.metal,
                v(px - 0.45, 0.2, -0.3),
                v(px + 0.45, 1.9, 0.3),
                white,
            );
            boxr(
                &mut a.metal,
                v(px - 0.47, 1.9, -0.32),
                v(px + 0.47, 2.2, 0.32),
                red,
            );
            for s in [-1.0f32, 1.0] {
                a.glow
                    .cuboid(v(px, 1.4, s * 0.31), v(0.5, 0.3, 0.02), c(0.5, 0.9, 1.0));
                a.metal.beam(
                    v(px + 0.3, 1.1, s * 0.33),
                    v(px + 0.5, 0.5, s * 0.5),
                    Vec2::splat(0.04),
                    c(0.1, 0.1, 0.1),
                );
            }
        }
        if canopy {
            for (px, pz) in [(-4.5f32, 0.0f32), (4.5, 0.0)] {
                a.metal.cuboid(v(px, 2.5, pz), v(0.4, 5.0, 0.4), white);
                self.collide_local(v(x, 0.0, z), yaw, v(px, 2.5, pz), v(0.4, 5.0, 0.4));
            }
            boxr(&mut a.metal, v(-7.0, 5.0, -4.0), v(7.0, 5.8, 4.0), white);
            boxr(&mut a.paint, v(-7.05, 5.1, -4.05), v(7.05, 5.5, 4.05), red);
            a.glow
                .cuboid(v(0.0, 4.98, 0.0), v(12.0, 0.04, 6.0), c(1.0, 0.97, 0.9));
            let rot = Quat::from_rotation_y(yaw);
            for px in [-3.5f32, 3.5] {
                self.light(
                    v(x, 4.6, z) + rot * v(px, 0.0, 0.0),
                    c(1.0, 0.97, 0.9),
                    150_000.0,
                );
            }
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.0, 0.0), v(6.0, 2.0, 1.4));
    }

    /// Storage rack with boxes on three shelves.
    pub(crate) fn shelving(&mut self, x: f32, z: f32, yaw: f32, seed: f32) {
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
                    let col = if hash(i as f32, seed + y) > 0.5 {
                        c(0.62, 0.48, 0.3)
                    } else {
                        c(0.55, 0.56, 0.5)
                    };
                    boxr(
                        &mut a.paint,
                        v(px - 0.45, y + 0.06, -0.45),
                        v(px + 0.45, y + 0.06 + hgt, 0.45),
                        col,
                    );
                }
            }
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 2.0, 0.0), v(l + 0.2, 4.0, 1.2));
    }

    /// Small booth (guard hut, ticket booth).
    pub(crate) fn booth(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        boxr(&mut a.paint, v(-1.2, 0.0, -1.2), v(1.2, 2.6, 1.2), color);
        boxr(
            &mut a.paint,
            v(-1.5, 2.6, -1.5),
            v(1.5, 2.8, 1.5),
            shade(color, 0.6),
        );
        for s in [-1.0f32, 1.0] {
            boxr(
                &mut a.glass,
                v(-0.9, 1.1, s * 1.21 - 0.02),
                v(0.9, 2.2, s * 1.21 + 0.02),
                c(0.3, 0.4, 0.5),
            );
            boxr(
                &mut a.glass,
                v(s * 1.21 - 0.02, 1.1, -0.9),
                v(s * 1.21 + 0.02, 2.2, 0.9),
                c(0.3, 0.4, 0.5),
            );
        }
        a.glow
            .cuboid(v(0.0, 2.58, 0.0), v(1.6, 0.03, 1.6), c(1.0, 0.95, 0.8));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 1.4, z), v(2.6, 2.8, 2.6));
    }

    // -----------------------------------------------------------------------
    // Central Park
    // -----------------------------------------------------------------------

    pub(crate) fn flower_bed(&mut self, x: f32, z: f32, l: f32, seed: f32) {
        let mut a = Art::default();
        boxr(
            &mut a.paint,
            v(-l / 2.0, 0.0, -0.8),
            v(l / 2.0, 0.45, 0.8),
            c(0.5, 0.36, 0.25),
        );
        boxr(
            &mut a.paint,
            v(-l / 2.0 + 0.1, 0.45, -0.7),
            v(l / 2.0 - 0.1, 0.5, 0.7),
            c(0.3, 0.2, 0.12),
        );
        let cols = [
            c(0.95, 0.3, 0.4),
            c(0.95, 0.85, 0.25),
            c(0.65, 0.4, 0.95),
            c(1.0, 1.0, 1.0),
            c(1.0, 0.55, 0.2),
        ];
        let n = (l * 2.0) as i32;
        for i in 0..n {
            let px = -l / 2.0 + 0.3 + i as f32 * (l - 0.6) / n as f32;
            for s in [-0.35f32, 0.35] {
                a.paint.cyl(
                    v(px, 0.65, s),
                    0.02,
                    0.3,
                    Quat::IDENTITY,
                    c(0.2, 0.45, 0.15),
                );
                a.paint.sphere(
                    v(px, 0.82, s),
                    0.11,
                    cols[((hash(px, seed + s) * 5.0) as usize).min(4)],
                );
            }
        }
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 0.25, z), v(l, 0.5, 1.6));
    }

    pub(crate) fn tennis_court(&mut self, x: f32, z: f32, color: Color) {
        let mut a = Art::default();
        let (l, w) = (24.0, 11.0);
        a.paint.cuboid(
            v(0.0, 0.015, 0.0),
            v(w + 3.0, 0.02, l + 4.0),
            shade(color, 0.7),
        );
        a.paint.cuboid(v(0.0, 0.02, 0.0), v(w, 0.02, l), color);
        let white = c(0.95, 0.95, 0.95);
        for s in [-1.0f32, 1.0] {
            a.paint
                .cuboid(v(s * w / 2.0, 0.03, 0.0), v(0.08, 0.02, l), white);
            a.paint
                .cuboid(v(0.0, 0.03, s * l / 2.0), v(w, 0.02, 0.08), white);
            a.paint
                .cuboid(v(s * (w / 2.0 - 1.4), 0.03, 0.0), v(0.06, 0.02, l), white);
            a.paint
                .cuboid(v(0.0, 0.03, s * 6.4), v(w - 2.8, 0.02, 0.06), white);
            a.metal.cyl(
                v(s * (w / 2.0 + 0.5), 0.55, 0.0),
                0.05,
                1.1,
                Quat::IDENTITY,
                c(0.2, 0.25, 0.2),
            );
        }
        a.paint
            .cuboid(v(0.0, 0.03, 0.0), v(0.06, 0.02, 12.8), white);
        a.glass
            .cuboid(v(0.0, 0.5, 0.0), v(w + 1.0, 0.9, 0.03), c(0.1, 0.1, 0.1));
        a.paint
            .cuboid(v(0.0, 0.97, 0.0), v(w + 1.0, 0.06, 0.05), white);
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 0.5, z), v(w + 1.0, 1.0, 0.2));
    }

    pub(crate) fn rowboat(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        a.paint.blob(v(0.0, 0.2, 0.0), v(0.7, 0.3, 1.9), color);
        a.paint
            .blob(v(0.0, 0.32, 0.0), v(0.58, 0.2, 1.75), c(0.45, 0.32, 0.2));
        for zz in [-0.6f32, 0.6] {
            a.paint
                .cuboid(v(0.0, 0.42, zz), v(1.1, 0.06, 0.25), c(0.55, 0.4, 0.25));
        }
        a.paint.cyl(
            v(0.4, 0.45, 0.0),
            0.03,
            2.2,
            Quat::from_rotation_x(1.3),
            c(0.6, 0.45, 0.3),
        );
        self.place(a, v(x, 0.0, z), yaw);
    }

    // -----------------------------------------------------------------------
    // The Neighborhood
    // -----------------------------------------------------------------------

    pub(crate) fn garage(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        boxr(&mut a.paint, v(-3.0, 0.0, -3.0), v(3.0, 3.0, 3.0), color);
        boxr(
            &mut a.paint,
            v(-3.2, 3.0, -3.2),
            v(3.2, 3.25, 3.2),
            shade(color, 0.6),
        );
        boxr(
            &mut a.metal,
            v(-2.3, 0.0, 3.0),
            v(2.3, 2.4, 3.06),
            c(0.82, 0.82, 0.8),
        );
        for i in 0..6 {
            let y = 0.35 + i as f32 * 0.38;
            boxr(
                &mut a.metal,
                v(-2.3, y, 3.06),
                v(2.3, y + 0.04, 3.09),
                c(0.6, 0.6, 0.6),
            );
        }
        a.glow
            .cuboid(v(0.0, 2.75, 3.15), v(0.35, 0.18, 0.1), c(1.0, 0.85, 0.55));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.6, 0.0), v(6.0, 3.2, 6.0));
    }

    pub(crate) fn dumpster(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        boxr(&mut a.metal, v(-1.0, 0.15, -0.8), v(1.0, 1.3, 0.8), color);
        a.metal.cuboid_rot(
            v(0.0, 1.38, 0.0),
            v(2.05, 0.08, 1.7),
            Quat::from_rotation_x(0.08),
            shade(color, 0.7),
        );
        for (px, pz) in [(-0.8f32, -0.6f32), (0.8, -0.6), (-0.8, 0.6), (0.8, 0.6)] {
            a.paint.cyl(
                v(px, 0.08, pz),
                0.08,
                0.08,
                Quat::from_rotation_z(FRAC_PI_2),
                c(0.05, 0.05, 0.05),
            );
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.7, 0.0), v(2.0, 1.4, 1.6));
    }

    /// Wooden power poles along X with sagging wires between them.
    pub(crate) fn power_line(&mut self, x0: f32, x1: f32, z: f32) {
        let mut a = Art::default();
        let wood = c(0.42, 0.3, 0.2);
        let mut x = x0;
        let mut prev: Option<f32> = None;
        while x <= x1 {
            a.paint.cyl(v(x, 4.5, z), 0.13, 9.0, Quat::IDENTITY, wood);
            a.paint.cuboid(v(x, 8.4, z), v(0.15, 0.15, 2.2), wood);
            for s in [-0.9f32, 0.0, 0.9] {
                a.metal.cyl(
                    v(x, 8.6, z + s),
                    0.05,
                    0.2,
                    Quat::IDENTITY,
                    c(0.4, 0.55, 0.45),
                );
            }
            a.metal.cyl(
                v(x, 7.4, z + 0.3),
                0.25,
                0.8,
                Quat::IDENTITY,
                c(0.55, 0.56, 0.58),
            );
            if let Some(p) = prev {
                for s in [-0.9f32, 0.0, 0.9] {
                    let n = 8;
                    for i in 0..n {
                        let t0 = i as f32 / n as f32;
                        let t1 = (i + 1) as f32 / n as f32;
                        let sag = |t: f32| 8.7 - 0.8 * (1.0 - (2.0 * t - 1.0).powi(2));
                        a.metal.beam(
                            v(p + (x - p) * t0, sag(t0), z + s),
                            v(p + (x - p) * t1, sag(t1), z + s),
                            Vec2::splat(0.02),
                            c(0.05, 0.05, 0.05),
                        );
                    }
                }
            }
            self.collide(v(x, 2.0, z), v(0.3, 4.0, 0.3));
            prev = Some(x);
            x += 20.0;
        }
        self.place(a, Vec3::ZERO, 0.0);
    }

    pub(crate) fn bleachers(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let alu = c(0.75, 0.76, 0.78);
        for row in 0..4 {
            let y = 0.4 + row as f32 * 0.4;
            let zz = row as f32 * 0.6;
            boxr(
                &mut a.metal,
                v(-5.0, y, zz - 0.25),
                v(5.0, y + 0.06, zz + 0.25),
                alu,
            );
            boxr(
                &mut a.metal,
                v(-5.0, y - 0.4, zz - 0.3),
                v(5.0, y, zz - 0.26),
                shade(alu, 0.8),
            );
        }
        for px in [-4.8f32, 0.0, 4.8] {
            a.metal.beam(
                v(px, 0.0, -0.3),
                v(px, 2.0, 2.1),
                Vec2::splat(0.08),
                c(0.4, 0.4, 0.42),
            );
            a.metal
                .cuboid(v(px, 1.0, 2.1), v(0.08, 2.0, 0.08), c(0.4, 0.4, 0.42));
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.0, 0.9), v(10.0, 2.0, 2.4));
    }

    pub(crate) fn school_bus(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let yellow = c(0.95, 0.72, 0.1);
        let dark = c(0.06, 0.06, 0.06);
        boxr(&mut a.metal, v(-1.25, 0.6, -5.0), v(1.25, 3.0, 4.2), yellow);
        boxr(&mut a.metal, v(-1.2, 0.6, 4.2), v(1.2, 1.9, 5.6), yellow);
        for s in [-1.0f32, 1.0] {
            boxr(
                &mut a.paint,
                v(s * 1.26 - 0.02, 1.25, -5.0),
                v(s * 1.26 + 0.02, 1.35, 4.2),
                dark,
            );
            let mut zz = -4.4;
            while zz < 3.8 {
                boxr(
                    &mut a.glass,
                    v(s * 1.26 - 0.02, 1.8, zz),
                    v(s * 1.26 + 0.02, 2.6, zz + 0.9),
                    c(0.12, 0.16, 0.2),
                );
                zz += 1.1;
            }
            for wz in [-3.2f32, 3.6] {
                a.paint.cyl(
                    v(s * 1.1, 0.5, wz),
                    0.5,
                    0.35,
                    Quat::from_rotation_z(FRAC_PI_2),
                    dark,
                );
            }
        }
        boxr(
            &mut a.glass,
            v(-1.1, 1.9, 4.18),
            v(1.1, 2.8, 4.24),
            c(0.12, 0.16, 0.2),
        );
        a.glow
            .cuboid(v(0.0, 1.4, 5.62), v(1.8, 0.2, 0.04), c(1.0, 0.97, 0.85));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.5, 0.3), v(2.6, 3.0, 10.6));
    }

    pub(crate) fn pipes(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        for (i, (px, py)) in [
            (-0.5f32, 0.35f32),
            (0.25, 0.35),
            (1.0, 0.35),
            (-0.12, 0.98),
            (0.62, 0.98),
        ]
        .into_iter()
        .enumerate()
        {
            a.metal.cyl(
                v(px, py, 0.0),
                0.33,
                4.0,
                Quat::from_rotation_x(FRAC_PI_2),
                c(0.55 + i as f32 * 0.02, 0.56, 0.6),
            );
            a.paint.cyl(
                v(px, py, 2.01),
                0.25,
                0.02,
                Quat::from_rotation_x(FRAC_PI_2),
                c(0.1, 0.1, 0.1),
            );
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.25, 0.65, 0.0), v(2.2, 1.3, 4.0));
    }

}

// ---------------------------------------------------------------------------
// Wall guns in the world
// ---------------------------------------------------------------------------

/// Spawns the guns on the wall-buy boards.
pub fn spawn_wall_guns(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    guns: &crate::gunmodels::GunAssets,
    layout: &MapLayout,
) {    // Chalk-white guns on the wall boards.
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
            .with_children(|g| {
                crate::gunmodels::spawn_gun(g, guns, w.gun, w.attach, chalk.clone(), false, None)
            });
    }
}

