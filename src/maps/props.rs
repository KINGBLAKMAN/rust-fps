//! Modelled props for the maps. Each prop adds its look to the map's art
//! kits (merged into a few big meshes) and simple boxes to collide with.
//!
//! Props are modelled facing +Z with their base on the ground; `place`
//! rotates and moves them into the world.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

use crate::kit::{c, Kit};
use crate::maps::{MapLayout, Solid};

/// The map's look, split by material.
#[derive(Default)]
pub struct Art {
    /// Matte painted / natural surfaces.
    pub paint: Kit,
    /// Shiny metal and car paint.
    pub metal: Kit,
    /// Lights, lit windows and signs.
    pub glow: Kit,
    /// Glass and water.
    pub glass: Kit,
}

impl Art {
    pub fn append(&mut self, other: Art, tf: Transform) {
        self.paint.append(other.paint, tf);
        self.metal.append(other.metal, tf);
        self.glow.append(other.glow, tf);
        self.glass.append(other.glass, tf);
    }
}

pub fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// Box from two corners.
pub fn boxr(k: &mut Kit, a: Vec3, b: Vec3, col: Color) {
    let min = a.min(b);
    let max = a.max(b);
    k.cuboid((min + max) / 2.0, max - min, col);
}

pub fn shade(col: Color, f: f32) -> Color {
    let s = col.to_srgba();
    Color::srgb(
        (s.red * f).min(1.0),
        (s.green * f).min(1.0),
        (s.blue * f).min(1.0),
    )
}

/// Cheap deterministic pseudo-random number in 0..1.
pub fn hash(a: f32, b: f32) -> f32 {
    let x = (a * 127.1 + b * 311.7).sin() * 43_758.547;
    x - x.floor()
}

impl MapLayout {
    /// Adds a prop's art at `pos` turned by `yaw`.
    pub fn place(&mut self, art: Art, pos: Vec3, yaw: f32) {
        self.art.append(
            art,
            Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw)),
        );
    }

    /// An invisible box to collide with (a prop's art draws it).
    pub fn collide(&mut self, center: Vec3, size: Vec3) {
        self.solids.push(Solid {
            pos: center,
            size,
            color: Color::WHITE,
            show: false,
        });
    }

    /// Collision box given in a prop's own space (rotated to the nearest
    /// right angle).
    pub fn collide_local(&mut self, pos: Vec3, yaw: f32, center: Vec3, size: Vec3) {
        let rot = Quat::from_rotation_y(yaw);
        let c = pos + rot * center;
        let quarter = ((yaw / FRAC_PI_2).round() as i32).rem_euclid(2);
        let size = if quarter == 1 {
            v(size.z, size.y, size.x)
        } else {
            size
        };
        self.collide(c, size);
    }

    pub fn light(&mut self, pos: Vec3, color: Color, intensity: f32) {
        self.lights.push((pos, color, intensity));
    }

    // -----------------------------------------------------------------------
    // Shipping yard
    // -----------------------------------------------------------------------

    /// Shipping container (6.1 x 2.6 x 2.44) with corrugated walls, corner
    /// castings and locking bars on the doors. `base_y` stacks them.
    pub fn container(
        &mut self,
        x: f32,
        z: f32,
        base_y: f32,
        along_z: bool,
        color: Color,
        seed: f32,
    ) {
        let yaw = if along_z { FRAC_PI_2 } else { 0.0 };
        let mut a = Art::default();
        let (l, h, w) = (6.06, 2.59, 2.44);
        let dark = shade(color, 0.72);
        let frame = shade(color, 0.55);
        let k = &mut a.paint;
        boxr(
            k,
            v(-l / 2.0 + 0.04, 0.06, -w / 2.0 + 0.03),
            v(l / 2.0 - 0.04, h - 0.06, w / 2.0 - 0.03),
            color,
        );
        // Corrugation on both long sides.
        let n = 22;
        for i in 0..n {
            let px = -l / 2.0 + 0.3 + i as f32 * (l - 0.6) / (n - 1) as f32;
            for s in [-1.0, 1.0] {
                boxr(
                    k,
                    v(px - 0.07, 0.12, s * (w / 2.0 - 0.01)),
                    v(px + 0.07, h - 0.12, s * (w / 2.0 + 0.025)),
                    dark,
                );
            }
        }
        // Roof ribs.
        for i in 0..10 {
            let px = -l / 2.0 + 0.35 + i as f32 * 0.6;
            boxr(
                k,
                v(px - 0.12, h - 0.06, -w / 2.0 + 0.1),
                v(px + 0.12, h - 0.02, w / 2.0 - 0.1),
                dark,
            );
        }
        // Frame rails and corner posts.
        for (y0, y1) in [(0.0, 0.16), (h - 0.14, h)] {
            for s in [-1.0, 1.0] {
                boxr(
                    k,
                    v(-l / 2.0, y0, s * w / 2.0 - 0.06),
                    v(l / 2.0, y1, s * w / 2.0 + 0.04),
                    frame,
                );
            }
            for s in [-1.0, 1.0] {
                boxr(
                    k,
                    v(s * l / 2.0 - 0.06, y0, -w / 2.0),
                    v(s * l / 2.0 + 0.04, y1, w / 2.0),
                    frame,
                );
            }
        }
        for sx in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                boxr(
                    k,
                    v(sx * l / 2.0 - 0.1, 0.0, sz * w / 2.0 - 0.1),
                    v(sx * l / 2.0 + 0.03, h, sz * w / 2.0 + 0.03),
                    frame,
                );
                for y in [0.08, h - 0.08] {
                    a.metal.cuboid(
                        v(sx * (l / 2.0 - 0.06), y, sz * (w / 2.0 - 0.06)),
                        Vec3::splat(0.18),
                        c(0.3, 0.3, 0.3),
                    );
                }
            }
        }
        // Doors on the +X end: two leaves, four locking bars, handles.
        let ex = l / 2.0 + 0.02;
        boxr(
            &mut a.paint,
            v(ex - 0.02, 0.18, -w / 2.0 + 0.08),
            v(ex + 0.01, h - 0.16, -0.01),
            shade(color, 0.9),
        );
        boxr(
            &mut a.paint,
            v(ex - 0.02, 0.18, 0.01),
            v(ex + 0.01, h - 0.16, w / 2.0 - 0.08),
            shade(color, 0.9),
        );
        for zz in [-0.9, -0.35, 0.35, 0.9] {
            a.metal.cyl(
                v(ex + 0.05, h / 2.0, zz),
                0.025,
                h - 0.3,
                Quat::IDENTITY,
                c(0.45, 0.45, 0.45),
            );
            a.metal.cuboid(
                v(ex + 0.08, 1.15, zz + 0.08),
                v(0.04, 0.05, 0.25),
                c(0.4, 0.4, 0.4),
            );
        }
        // A painted logo band and some rust streaks.
        let logo = if hash(seed, 1.0) > 0.5 {
            c(0.92, 0.92, 0.9)
        } else {
            c(0.1, 0.1, 0.1)
        };
        for s in [-1.0, 1.0] {
            boxr(
                &mut a.paint,
                v(-1.6, 1.55, s * (w / 2.0 + 0.03)),
                v(1.6, 1.95, s * (w / 2.0 + 0.035)),
                logo,
            );
            for i in 0..3 {
                let rx = (hash(seed, i as f32 + s) - 0.5) * 5.0;
                let rh = 0.3 + hash(seed + 3.0, i as f32) * 0.9;
                boxr(
                    &mut a.paint,
                    v(rx - 0.08, h - 0.2 - rh, s * (w / 2.0 + 0.03)),
                    v(rx + 0.08, h - 0.2, s * (w / 2.0 + 0.037)),
                    c(0.42, 0.22, 0.1),
                );
            }
        }
        self.place(a, v(x, base_y, z), yaw);
        let size = if along_z { v(w, h, l) } else { v(l, h, w) };
        self.collide(v(x, base_y + h / 2.0, z), size);
    }

    /// Lattice gantry crane spanning `span` metres along Z, with a cab,
    /// hoist cables and spreader. Only the legs collide.
    pub fn gantry(&mut self, x: f32, z: f32, span: f32, height: f32) {
        let mut a = Art::default();
        let yel = c(0.92, 0.7, 0.1);
        let ydark = c(0.75, 0.55, 0.08);
        let t = Vec2::splat(0.22);
        let k = &mut a.metal;
        let hs = span / 2.0;
        for sz in [-hs, hs] {
            for sx in [-1.2f32, 1.2] {
                k.beam(
                    v(sx, 0.0, sz),
                    v(sx * 0.7, height, sz),
                    Vec2::splat(0.5),
                    yel,
                );
                k.cuboid(v(sx, 0.25, sz), v(0.9, 0.5, 1.6), c(0.2, 0.2, 0.2));
                // Wheels on rails.
                for wz in [-0.5, 0.5] {
                    k.cyl(
                        v(sx, 0.25, sz + wz),
                        0.22,
                        0.2,
                        Quat::from_rotation_z(FRAC_PI_2),
                        c(0.1, 0.1, 0.1),
                    );
                }
            }
            // Cross bracing between leg pairs.
            let mut y = 0.8;
            while y < height - 1.5 {
                k.beam(v(-1.15, y, sz), v(1.15, y + 1.8, sz), t, ydark);
                k.beam(v(1.15, y, sz), v(-1.15, y + 1.8, sz), t, ydark);
                y += 2.2;
            }
        }
        // Box girders with lattice web along the span.
        for sx in [-0.84f32, 0.84] {
            boxr(
                k,
                v(sx - 0.3, height, -hs - 2.0),
                v(sx + 0.3, height + 0.5, hs + 2.0),
                yel,
            );
            boxr(
                k,
                v(sx - 0.3, height + 2.0, -hs - 2.0),
                v(sx + 0.3, height + 2.4, hs + 2.0),
                yel,
            );
            let mut zz = -hs - 2.0;
            while zz < hs + 2.0 {
                k.beam(
                    v(sx, height + 0.5, zz),
                    v(sx, height + 2.0, zz + 1.5),
                    t,
                    ydark,
                );
                k.beam(
                    v(sx, height + 0.5, zz + 1.5),
                    v(sx, height + 2.0, zz + 1.5),
                    t,
                    ydark,
                );
                zz += 1.5;
            }
        }
        // Warning stripes at the leg feet.
        for sz in [-hs, hs] {
            for sx in [-1.2f32, 1.2] {
                for i in 0..4 {
                    let col = if i % 2 == 0 { c(0.08, 0.08, 0.08) } else { yel };
                    a.paint
                        .cuboid(v(sx, 0.7 + i as f32 * 0.3, sz), v(0.56, 0.3, 0.56), col);
                }
            }
        }
        // Trolley, cab and hoist.
        let tz = hs * 0.3;
        boxr(
            &mut a.metal,
            v(-1.3, height + 2.4, tz - 1.2),
            v(1.3, height + 3.4, tz + 1.2),
            ydark,
        );
        boxr(
            &mut a.paint,
            v(0.9, height - 1.6, tz - 0.9),
            v(2.2, height + 0.2, tz + 0.9),
            c(0.85, 0.85, 0.82),
        );
        boxr(
            &mut a.glass,
            v(0.88, height - 1.2, tz - 0.8),
            v(2.22, height - 0.3, tz + 0.8),
            c(0.35, 0.5, 0.6),
        );
        for sx in [-0.6f32, 0.6] {
            a.metal.cyl(
                v(sx, height - 3.0, tz),
                0.03,
                6.0,
                Quat::IDENTITY,
                c(0.15, 0.15, 0.15),
            );
        }
        boxr(
            &mut a.metal,
            v(-1.5, height - 6.2, tz - 0.4),
            v(1.5, height - 5.9, tz + 0.4),
            yel,
        );
        self.place(a, v(x, 0.0, z), 0.0);
        for sz in [-hs, hs] {
            for sx in [-1.2f32, 1.2] {
                self.collide(v(x + sx, 3.0, z + sz), v(0.7, 6.0, 1.6));
            }
        }
    }

    pub fn forklift(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        let k = &mut a.metal;
        let dark = c(0.12, 0.12, 0.13);
        boxr(k, v(-0.6, 0.3, -1.0), v(0.6, 1.1, 1.0), color);
        boxr(k, v(-0.62, 0.3, 0.7), v(0.62, 1.3, 1.25), shade(color, 0.7));
        // Overhead guard.
        for sx in [-0.55f32, 0.55] {
            for sz in [-0.5f32, 0.6] {
                k.beam(v(sx, 1.1, sz), v(sx, 2.2, sz), Vec2::splat(0.07), dark);
            }
        }
        boxr(k, v(-0.6, 2.15, -0.55), v(0.6, 2.22, 0.65), dark);
        boxr(
            &mut a.paint,
            v(-0.35, 1.1, -0.1),
            v(0.35, 1.25, 0.45),
            c(0.1, 0.1, 0.1),
        );
        boxr(
            &mut a.paint,
            v(-0.3, 1.25, 0.35),
            v(0.3, 1.75, 0.45),
            c(0.1, 0.1, 0.1),
        );
        k.cyl(
            v(0.0, 1.5, -0.25),
            0.16,
            0.04,
            Quat::from_rotation_x(1.0),
            dark,
        );
        // Mast and forks.
        for sx in [-0.45f32, 0.45] {
            boxr(k, v(sx - 0.06, 0.2, -1.25), v(sx + 0.06, 2.6, -1.1), dark);
        }
        boxr(k, v(-0.5, 0.5, -1.3), v(0.5, 1.0, -1.22), dark);
        for sx in [-0.3f32, 0.3] {
            boxr(
                k,
                v(sx - 0.06, 0.12, -2.4),
                v(sx + 0.06, 0.18, -1.25),
                c(0.25, 0.25, 0.25),
            );
        }
        for (sz, r) in [(-0.7f32, 0.33), (0.75, 0.28)] {
            for sx in [-0.62f32, 0.62] {
                k.cyl(
                    v(sx, r, sz),
                    r,
                    0.25,
                    Quat::from_rotation_z(FRAC_PI_2),
                    c(0.05, 0.05, 0.05),
                );
                k.cyl(
                    v(sx * 1.03, r, sz),
                    r * 0.5,
                    0.26,
                    Quat::from_rotation_z(FRAC_PI_2),
                    c(0.6, 0.6, 0.6),
                );
            }
        }
        a.glow
            .cuboid(v(0.4, 2.0, -0.6), v(0.12, 0.1, 0.05), c(1.0, 0.95, 0.8));
        a.glow
            .cuboid(v(0.0, 2.3, 0.0), v(0.12, 0.1, 0.12), c(1.0, 0.6, 0.1));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.1, -0.4), v(1.3, 2.2, 2.6));
    }

    /// Pallet with stacked crates or sacks.
    pub fn pallet_stack(&mut self, x: f32, z: f32, yaw: f32, seed: f32) {
        let mut a = Art::default();
        let wood = c(0.6, 0.45, 0.28);
        let k = &mut a.paint;
        for i in 0..5 {
            let px = -0.5 + i as f32 * 0.25;
            boxr(k, v(px - 0.05, 0.12, -0.6), v(px + 0.05, 0.15, 0.6), wood);
        }
        for sz in [-0.5f32, 0.0, 0.5] {
            boxr(
                k,
                v(-0.55, 0.0, sz - 0.05),
                v(0.55, 0.12, sz + 0.05),
                shade(wood, 0.8),
            );
        }
        let layers = 1 + (hash(seed, 2.0) * 2.5) as i32;
        let crate_c = c(0.55, 0.4, 0.22);
        for l in 0..layers {
            let y = 0.15 + l as f32 * 0.5;
            boxr(k, v(-0.52, y, -0.58), v(0.52, y + 0.48, 0.58), crate_c);
            for s in [-1.0f32, 1.0] {
                boxr(
                    k,
                    v(-0.53, y + 0.02, s * 0.585 - 0.01),
                    v(0.53, y + 0.08, s * 0.585 + 0.01),
                    shade(crate_c, 0.7),
                );
                boxr(
                    k,
                    v(-0.53, y + 0.4, s * 0.585 - 0.01),
                    v(0.53, y + 0.46, s * 0.585 + 0.01),
                    shade(crate_c, 0.7),
                );
                k.beam(
                    v(-0.5, y + 0.06, s * 0.59),
                    v(0.5, y + 0.42, s * 0.59),
                    Vec2::new(0.06, 0.015),
                    shade(crate_c, 0.7),
                );
            }
        }
        self.place(a, v(x, 0.0, z), yaw);
        let h = 0.15 + layers as f32 * 0.5;
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, h / 2.0, 0.0), v(1.1, h, 1.2));
    }

    /// Oil drum (optionally on its side).
    pub fn drums(&mut self, x: f32, z: f32, count: usize, seed: f32) {
        let cols = [
            c(0.15, 0.3, 0.6),
            c(0.65, 0.12, 0.1),
            c(0.2, 0.4, 0.2),
            c(0.75, 0.6, 0.1),
        ];
        let mut a = Art::default();
        for i in 0..count {
            let ang = i as f32 * 2.3 + seed;
            let p = if i == 0 {
                Vec3::ZERO
            } else {
                v(ang.cos() * 0.62, 0.0, ang.sin() * 0.62)
            };
            let col = cols[((hash(seed, i as f32) * 4.0) as usize).min(3)];
            a.metal
                .cyl(p + v(0.0, 0.45, 0.0), 0.29, 0.88, Quat::IDENTITY, col);
            for y in [0.05, 0.3, 0.6, 0.86] {
                a.metal.torus(
                    p + v(0.0, y, 0.0),
                    0.015,
                    0.295,
                    Quat::IDENTITY,
                    shade(col, 0.7),
                );
            }
            a.metal.cyl(
                p + v(0.1, 0.9, 0.1),
                0.04,
                0.02,
                Quat::IDENTITY,
                c(0.3, 0.3, 0.3),
            );
        }
        self.place(a, v(x, 0.0, z), 0.0);
        let r = if count > 1 { 1.1 } else { 0.32 };
        self.collide(v(x, 0.45, z), v(r * 2.0, 0.9, r * 2.0));
    }

    /// Concrete jersey barrier run of `n` sections.
    pub fn barriers(&mut self, x: f32, z: f32, yaw: f32, n: usize, color: Color) {
        let mut a = Art::default();
        for i in 0..n {
            let px = (i as f32 - (n as f32 - 1.0) / 2.0) * 2.05;
            let k = &mut a.paint;
            boxr(k, v(px - 1.0, 0.0, -0.3), v(px + 1.0, 0.25, 0.3), color);
            k.cuboid_rot(
                v(px, 0.35, -0.18),
                v(2.0, 0.3, 0.1),
                Quat::from_rotation_x(-0.5),
                color,
            );
            k.cuboid_rot(
                v(px, 0.35, 0.18),
                v(2.0, 0.3, 0.1),
                Quat::from_rotation_x(0.5),
                color,
            );
            boxr(k, v(px - 1.0, 0.2, -0.1), v(px + 1.0, 0.85, 0.1), color);
            for s in [-1.0f32, 1.0] {
                boxr(
                    k,
                    v(px - 0.5, 0.55, s * 0.105),
                    v(px + 0.5, 0.7, s * 0.11),
                    c(0.9, 0.75, 0.1),
                );
            }
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(
            v(x, 0.0, z),
            yaw,
            v(0.0, 0.43, 0.0),
            v(n as f32 * 2.05, 0.86, 0.6),
        );
    }

    pub fn traffic_cone(&mut self, x: f32, z: f32) {
        let mut a = Art::default();
        a.paint
            .cuboid(v(0.0, 0.02, 0.0), v(0.36, 0.04, 0.36), c(0.1, 0.1, 0.1));
        a.paint.cone(
            v(0.0, 0.33, 0.0),
            0.15,
            0.6,
            Quat::IDENTITY,
            c(1.0, 0.4, 0.05),
        );
        a.paint.frustum(
            v(0.0, 0.33, 0.0),
            0.087,
            0.1,
            0.12,
            Quat::IDENTITY,
            c(0.95, 0.95, 0.95),
        );
        self.place(a, v(x, 0.0, z), 0.0);
    }

    pub fn tires(&mut self, x: f32, z: f32, n: usize) {
        let mut a = Art::default();
        for i in 0..n {
            a.paint.torus(
                v((i % 2) as f32 * 0.05, 0.13 + i as f32 * 0.26, 0.0),
                0.12,
                0.38,
                Quat::IDENTITY,
                c(0.06, 0.06, 0.06),
            );
        }
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, n as f32 * 0.13, z), v(1.0, n as f32 * 0.26, 1.0));
    }

    /// Floodlight tower with a lamp bank.
    pub fn light_tower(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let grey = c(0.55, 0.56, 0.58);
        a.metal
            .frustum(v(0.0, 5.0, 0.0), 0.12, 0.22, 10.0, Quat::IDENTITY, grey);
        a.paint
            .cuboid(v(0.0, 0.2, 0.0), v(1.0, 0.4, 1.0), c(0.5, 0.5, 0.48));
        boxr(&mut a.metal, v(-1.1, 9.9, -0.3), v(1.1, 10.1, 0.3), grey);
        for i in 0..4 {
            let px = -0.8 + i as f32 * 0.53;
            boxr(
                &mut a.metal,
                v(px - 0.22, 10.1, -0.25),
                v(px + 0.22, 10.5, 0.15),
                c(0.2, 0.2, 0.2),
            );
            a.glow
                .cuboid(v(px, 10.3, -0.26), v(0.38, 0.3, 0.02), c(1.0, 0.97, 0.85));
        }
        // Ladder.
        for s in [-1.0f32, 1.0] {
            a.metal
                .cyl(v(s * 0.18, 4.5, 0.3), 0.02, 9.0, Quat::IDENTITY, grey);
        }
        let mut y = 0.6;
        while y < 9.0 {
            a.metal.cyl(
                v(0.0, y, 0.3),
                0.015,
                0.36,
                Quat::from_rotation_z(FRAC_PI_2),
                grey,
            );
            y += 0.35;
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 2.0, z), v(0.5, 4.0, 0.5));
        let fwd = Quat::from_rotation_y(yaw) * v(0.0, 0.0, -1.5);
        self.light(v(x, 9.0, z) + fwd, c(1.0, 0.95, 0.85), 400_000.0);
    }

    /// Portable site office with windows, door, steps and an AC unit.
    pub fn site_office(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let wall = c(0.86, 0.86, 0.8);
        let trim = c(0.25, 0.35, 0.55);
        let (l, h, w) = (8.0, 3.0, 4.0);
        boxr(
            &mut a.paint,
            v(-l / 2.0, 0.3, -w / 2.0),
            v(l / 2.0, h, w / 2.0),
            wall,
        );
        boxr(
            &mut a.paint,
            v(-l / 2.0 - 0.1, h, -w / 2.0 - 0.1),
            v(l / 2.0 + 0.1, h + 0.15, w / 2.0 + 0.1),
            trim,
        );
        boxr(
            &mut a.paint,
            v(-l / 2.0 + 0.2, 0.0, -w / 2.0 + 0.2),
            v(l / 2.0 - 0.2, 0.3, w / 2.0 - 0.2),
            c(0.2, 0.2, 0.2),
        );
        // Panel seams.
        for i in 1..8 {
            let px = -l / 2.0 + i as f32;
            for s in [-1.0f32, 1.0] {
                boxr(
                    &mut a.paint,
                    v(px - 0.02, 0.3, s * (w / 2.0 + 0.005)),
                    v(px + 0.02, h, s * (w / 2.0 + 0.012)),
                    shade(wall, 0.85),
                );
            }
        }
        // Windows on the front (+Z) and a door.
        for px in [-2.8f32, -1.0, 2.6] {
            boxr(
                &mut a.paint,
                v(px - 0.65, 1.15, w / 2.0),
                v(px + 0.65, 2.35, w / 2.0 + 0.06),
                trim,
            );
            boxr(
                &mut a.glow,
                v(px - 0.55, 1.25, w / 2.0 + 0.05),
                v(px + 0.55, 2.25, w / 2.0 + 0.07),
                c(0.55, 0.65, 0.7),
            );
            boxr(
                &mut a.paint,
                v(px - 0.02, 1.25, w / 2.0 + 0.07),
                v(px + 0.02, 2.25, w / 2.0 + 0.09),
                trim,
            );
        }
        boxr(
            &mut a.paint,
            v(0.4, 0.3, w / 2.0),
            v(1.4, 2.4, w / 2.0 + 0.06),
            c(0.3, 0.33, 0.38),
        );
        a.metal
            .sphere(v(1.25, 1.3, w / 2.0 + 0.09), 0.04, c(0.7, 0.7, 0.7));
        for i in 0..3 {
            let y = 0.1 + i as f32 * 0.1;
            boxr(
                &mut a.metal,
                v(0.3, 0.0, w / 2.0 + 0.2 + (2 - i) as f32 * 0.25),
                v(1.5, y + 0.1, w / 2.0 + 0.45 + (2 - i) as f32 * 0.25),
                c(0.45, 0.45, 0.45),
            );
        }
        boxr(
            &mut a.metal,
            v(-3.6, 2.0, -w / 2.0 - 0.5),
            v(-2.6, 2.7, -w / 2.0),
            c(0.75, 0.75, 0.72),
        );
        a.metal.cyl(
            v(-3.1, 2.35, -w / 2.0 - 0.51),
            0.25,
            0.02,
            Quat::from_rotation_x(FRAC_PI_2),
            c(0.2, 0.2, 0.2),
        );
        // A sign over the door.
        boxr(
            &mut a.paint,
            v(-0.3, 2.5, w / 2.0),
            v(2.1, 2.85, w / 2.0 + 0.04),
            c(0.95, 0.75, 0.1),
        );
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, h / 2.0, 0.0), v(l, h, w));
    }

    /// Cargo ship moored outside the yard (scenery, no collision).
    pub fn ship(&mut self, x: f32, z: f32, length: f32) {
        let mut a = Art::default();
        let hull = c(0.15, 0.22, 0.35);
        let red = c(0.55, 0.12, 0.1);
        let hw = 9.0;
        boxr(
            &mut a.paint,
            v(-length / 2.0, -2.0, -hw),
            v(length / 2.0, 6.0, hw),
            hull,
        );
        boxr(
            &mut a.paint,
            v(-length / 2.0, -3.0, -hw + 0.5),
            v(length / 2.0, -2.0, hw - 0.5),
            red,
        );
        a.paint.wedge(
            v(-length / 2.0 - 6.0, 2.0, 0.0),
            v(hw * 2.0, 8.0, 12.0),
            Quat::from_rotation_y(FRAC_PI_2) * Quat::from_rotation_z(FRAC_PI_2),
            hull,
        );
        // Superstructure.
        let sx = length / 2.0 - 12.0;
        boxr(
            &mut a.paint,
            v(sx - 5.0, 6.0, -7.0),
            v(sx + 5.0, 20.0, 7.0),
            c(0.92, 0.92, 0.9),
        );
        for row in 0..5 {
            let y = 8.0 + row as f32 * 2.6;
            boxr(
                &mut a.glow,
                v(sx - 5.05, y, -6.0),
                v(sx - 5.0, y + 1.0, 6.0),
                c(0.35, 0.45, 0.55),
            );
        }
        a.paint
            .cyl(v(sx + 2.0, 24.0, 0.0), 1.6, 8.0, Quat::IDENTITY, red);
        // Containers stacked on deck.
        let cols = [
            c(0.6, 0.18, 0.12),
            c(0.12, 0.3, 0.6),
            c(0.15, 0.45, 0.25),
            c(0.85, 0.45, 0.1),
            c(0.5, 0.5, 0.52),
        ];
        let mut i = 0;
        let mut px = -length / 2.0 + 6.0;
        while px < sx - 9.0 {
            for row in -2..=2 {
                let hgt = 1 + (hash(px, row as f32) * 3.0) as i32;
                for l in 0..hgt {
                    let col = cols[(i * 7 + l as usize) % cols.len()];
                    i += 1;
                    boxr(
                        &mut a.paint,
                        v(px, 6.0 + l as f32 * 2.6, row as f32 * 2.5 - 1.2),
                        v(px + 6.0, 8.55 + l as f32 * 2.6, row as f32 * 2.5 + 1.2),
                        col,
                    );
                }
            }
            px += 6.3;
        }
        self.place(a, v(x, 0.0, z), 0.0);
    }

    // -----------------------------------------------------------------------
    // Park
    // -----------------------------------------------------------------------

    /// Broadleaf tree with branches and a clumpy crown.
    pub fn oak(&mut self, x: f32, z: f32, size: f32, seed: f32) {
        let mut a = Art::default();
        let bark = c(0.33, 0.23, 0.15);
        let s = size;
        a.paint.frustum(
            v(0.0, 1.6 * s, 0.0),
            0.18 * s,
            0.32 * s,
            3.2 * s,
            Quat::IDENTITY,
            bark,
        );
        // Root flare.
        for i in 0..4 {
            let ang = i as f32 * FRAC_PI_2 + seed;
            a.paint.beam(
                v(0.0, 0.35 * s, 0.0),
                v(ang.cos() * 0.6 * s, 0.0, ang.sin() * 0.6 * s),
                Vec2::splat(0.18 * s),
                bark,
            );
        }
        let greens = [
            c(0.2, 0.42, 0.15),
            c(0.24, 0.48, 0.17),
            c(0.17, 0.36, 0.13),
            c(0.28, 0.5, 0.2),
        ];
        for i in 0..7 {
            let ang = i as f32 / 7.0 * TAU + seed;
            let r = (0.9 + hash(seed, i as f32) * 0.8) * s;
            let y = (3.4 + hash(seed + 1.0, i as f32) * 1.6) * s;
            let tip = v(ang.cos() * r, y, ang.sin() * r);
            a.paint
                .beam(v(0.0, 2.6 * s, 0.0), tip, Vec2::splat(0.12 * s), bark);
            let rad = (1.0 + hash(seed + 2.0, i as f32) * 0.6) * s;
            a.paint.blob(
                tip + v(0.0, 0.3 * s, 0.0),
                v(rad, rad * 0.8, rad),
                greens[i % 4],
            );
        }
        a.paint.blob(
            v(0.0, 4.9 * s, 0.0),
            v(1.7 * s, 1.3 * s, 1.7 * s),
            greens[1],
        );
        self.place(a, v(x, 0.0, z), seed);
        self.collide(v(x, 1.5, z), v(0.5 * s, 3.0, 0.5 * s));
    }

    pub fn pine(&mut self, x: f32, z: f32, size: f32, seed: f32) {
        let mut a = Art::default();
        let s = size;
        a.paint.cyl(
            v(0.0, 1.0 * s, 0.0),
            0.2 * s,
            2.0 * s,
            Quat::IDENTITY,
            c(0.35, 0.22, 0.13),
        );
        let g = [
            c(0.1, 0.3, 0.16),
            c(0.12, 0.34, 0.18),
            c(0.09, 0.27, 0.14),
            c(0.13, 0.37, 0.2),
        ];
        for i in 0..4 {
            let y = (1.6 + i as f32 * 1.15) * s;
            let r = (2.0 - i as f32 * 0.4) * s;
            a.paint.cone(
                v(0.0, y + 0.9 * s, 0.0),
                r,
                2.0 * s,
                Quat::from_rotation_y(seed + i as f32),
                g[i],
            );
        }
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 1.5, z), v(0.45 * s, 3.0, 0.45 * s));
    }

    pub fn bush(&mut self, x: f32, z: f32, size: f32, seed: f32, flowers: Option<Color>) {
        let mut a = Art::default();
        let g = c(0.18, 0.4, 0.14);
        for i in 0..4 {
            let ang = i as f32 * 1.7 + seed;
            let p = v(
                ang.cos() * 0.35 * size,
                0.45 * size,
                ang.sin() * 0.35 * size,
            );
            let r = (0.45 + hash(seed, i as f32) * 0.2) * size;
            a.paint.blob(
                p,
                v(r, r * 0.85, r),
                shade(g, 0.85 + hash(i as f32, seed) * 0.3),
            );
        }
        if let Some(fc) = flowers {
            for i in 0..10 {
                let ang = i as f32 * 2.4 + seed;
                let r = 0.55 * size;
                a.paint.sphere(
                    v(
                        ang.cos() * r,
                        (0.55 + hash(seed, i as f32) * 0.35) * size,
                        ang.sin() * r,
                    ),
                    0.07,
                    fc,
                );
            }
        }
        self.place(a, v(x, 0.0, z), 0.0);
    }

    /// Park bench: cast iron ends, wooden slats.
    pub fn bench(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let iron = c(0.12, 0.13, 0.12);
        let wood = c(0.55, 0.36, 0.2);
        for sx in [-0.85f32, 0.85] {
            a.metal.beam(
                v(sx, 0.0, -0.25),
                v(sx, 0.45, -0.2),
                Vec2::splat(0.05),
                iron,
            );
            a.metal
                .beam(v(sx, 0.0, 0.2), v(sx, 0.45, 0.15), Vec2::splat(0.05), iron);
            a.metal.beam(
                v(sx, 0.45, -0.25),
                v(sx, 0.45, 0.25),
                Vec2::splat(0.05),
                iron,
            );
            a.metal
                .beam(v(sx, 0.45, 0.22), v(sx, 0.9, 0.32), Vec2::splat(0.05), iron);
            a.metal.torus(
                v(sx, 0.25, 0.0),
                0.012,
                0.12,
                Quat::from_rotation_z(FRAC_PI_2),
                iron,
            );
        }
        for i in 0..4 {
            let zz = -0.2 + i as f32 * 0.13;
            boxr(
                &mut a.paint,
                v(-1.0, 0.45, zz - 0.05),
                v(1.0, 0.5, zz + 0.05),
                wood,
            );
        }
        for i in 0..3 {
            let y = 0.58 + i as f32 * 0.12;
            let zz = 0.25 + i as f32 * 0.025;
            a.paint.cuboid_rot(
                v(0.0, y, zz),
                v(2.0, 0.08, 0.04),
                Quat::from_rotation_x(-0.2),
                wood,
            );
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.25, 0.0), v(2.0, 0.5, 0.6));
    }

    /// Old-fashioned park lamp with a glowing lantern.
    pub fn park_lamp(&mut self, x: f32, z: f32) {
        let mut a = Art::default();
        let iron = c(0.1, 0.11, 0.1);
        a.metal
            .frustum(v(0.0, 0.3, 0.0), 0.1, 0.2, 0.6, Quat::IDENTITY, iron);
        a.metal
            .cyl(v(0.0, 2.0, 0.0), 0.06, 3.0, Quat::IDENTITY, iron);
        a.metal
            .torus(v(0.0, 1.2, 0.0), 0.02, 0.08, Quat::IDENTITY, iron);
        a.metal
            .cyl(v(0.0, 3.55, 0.0), 0.12, 0.1, Quat::IDENTITY, iron);
        a.glow.cyl(
            v(0.0, 3.85, 0.0),
            0.15,
            0.5,
            Quat::IDENTITY,
            c(1.0, 0.88, 0.6),
        );
        a.metal
            .cone(v(0.0, 4.25, 0.0), 0.26, 0.3, Quat::IDENTITY, iron);
        a.metal.sphere(v(0.0, 4.43, 0.0), 0.05, iron);
        for i in 0..4 {
            let ang = i as f32 * FRAC_PI_2;
            a.metal.cyl(
                v(ang.cos() * 0.15, 3.85, ang.sin() * 0.15),
                0.012,
                0.5,
                Quat::IDENTITY,
                iron,
            );
        }
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 1.5, z), v(0.25, 3.0, 0.25));
        self.light(v(x, 3.6, z), c(1.0, 0.85, 0.6), 60_000.0);
    }

    /// Two-tier stone fountain with water and jets.
    pub fn fountain(&mut self, x: f32, z: f32) {
        let mut a = Art::default();
        let stone = c(0.66, 0.64, 0.6);
        let dark = c(0.5, 0.49, 0.46);
        let r = 3.6;
        let n = 16;
        for i in 0..n {
            let ang = i as f32 / n as f32 * TAU;
            let p = v(ang.cos() * r, 0.35, ang.sin() * r);
            a.paint
                .cuboid_rot(p, v(0.55, 0.7, 1.5), Quat::from_rotation_y(-ang), stone);
            a.paint.cuboid_rot(
                p + v(0.0, 0.38, 0.0),
                v(0.7, 0.08, 1.52),
                Quat::from_rotation_y(-ang),
                dark,
            );
        }
        a.glass.cyl(
            v(0.0, 0.5, 0.0),
            r - 0.2,
            0.06,
            Quat::IDENTITY,
            c(0.25, 0.5, 0.75),
        );
        a.paint.cyl(
            v(0.0, 0.02, 0.0),
            r - 0.2,
            0.04,
            Quat::IDENTITY,
            c(0.25, 0.3, 0.32),
        );
        // Pedestal and bowls.
        a.paint
            .frustum(v(0.0, 1.0, 0.0), 0.35, 0.6, 2.0, Quat::IDENTITY, stone);
        a.paint
            .frustum(v(0.0, 2.1, 0.0), 1.6, 0.4, 0.4, Quat::IDENTITY, stone);
        a.glass.cyl(
            v(0.0, 2.28, 0.0),
            1.45,
            0.04,
            Quat::IDENTITY,
            c(0.3, 0.55, 0.8),
        );
        a.paint
            .frustum(v(0.0, 2.9, 0.0), 0.15, 0.25, 1.2, Quat::IDENTITY, stone);
        a.paint
            .frustum(v(0.0, 3.55, 0.0), 0.8, 0.2, 0.25, Quat::IDENTITY, stone);
        a.paint.sphere(v(0.0, 3.85, 0.0), 0.22, stone);
        // Water falling from the top bowl and jets.
        for i in 0..8 {
            let ang = i as f32 / 8.0 * TAU;
            a.glass.cyl(
                v(ang.cos() * 0.8, 2.95, ang.sin() * 0.8),
                0.05,
                1.2,
                Quat::IDENTITY,
                c(0.6, 0.8, 1.0),
            );
            a.glass.cyl(
                v(ang.cos() * 1.6, 1.45, ang.sin() * 1.6),
                0.05,
                1.7,
                Quat::IDENTITY,
                c(0.6, 0.8, 1.0),
            );
        }
        a.glass.cyl(
            v(0.0, 4.3, 0.0),
            0.05,
            0.9,
            Quat::IDENTITY,
            c(0.7, 0.85, 1.0),
        );
        self.place(a, v(x, 0.0, z), 0.0);
        // Collide as four straight walls (gaps at the corners) and the middle.
        for (cx, cz, sx, sz) in [
            (0.0, -r, 6.0, 0.6),
            (0.0, r, 6.0, 0.6),
            (-r, 0.0, 0.6, 6.0),
            (r, 0.0, 0.6, 6.0),
        ] {
            self.collide(v(x + cx, 0.35, z + cz), v(sx, 0.7, sz));
        }
        self.collide(v(x, 1.3, z), v(1.0, 2.6, 1.0));
    }

    /// Octagonal bandstand with columns, railings and a pointed roof.
    pub fn bandstand(&mut self, x: f32, z: f32) {
        let mut a = Art::default();
        let white = c(0.92, 0.91, 0.86);
        let roof = c(0.45, 0.18, 0.14);
        let r = 4.0;
        a.paint.cyl(
            v(0.0, 0.08, 0.0),
            r + 0.3,
            0.16,
            Quat::IDENTITY,
            c(0.55, 0.42, 0.3),
        );
        for i in 0..8 {
            let ang = i as f32 / 8.0 * TAU + PI / 8.0;
            let p = v(ang.cos() * r, 0.0, ang.sin() * r);
            a.paint
                .cyl(p + v(0.0, 1.7, 0.0), 0.14, 3.4, Quat::IDENTITY, white);
            a.paint
                .cuboid(p + v(0.0, 0.25, 0.0), v(0.4, 0.2, 0.4), white);
            // Railing to the next column, except the entrance on +Z.
            let next = (i + 1) as f32 / 8.0 * TAU + PI / 8.0;
            let q = v(next.cos() * r, 0.0, next.sin() * r);
            let mid = (p + q) / 2.0;
            if !(mid.z > 2.0 && mid.x.abs() < 2.0) {
                a.paint.beam(
                    p + v(0.0, 0.95, 0.0),
                    q + v(0.0, 0.95, 0.0),
                    Vec2::splat(0.08),
                    white,
                );
                for k in 1..8 {
                    let t = k as f32 / 8.0;
                    let b = p.lerp(q, t);
                    a.paint
                        .cyl(b + v(0.0, 0.55, 0.0), 0.025, 0.8, Quat::IDENTITY, white);
                }
            }
            // Decorative brackets under the roof.
            a.paint.beam(
                p + v(0.0, 3.0, 0.0),
                p * 0.8 + v(0.0, 3.4, 0.0),
                Vec2::splat(0.08),
                white,
            );
        }
        a.paint
            .cyl(v(0.0, 3.45, 0.0), r + 0.4, 0.15, Quat::IDENTITY, white);
        a.paint.cone(
            v(0.0, 4.5, 0.0),
            r + 0.7,
            2.0,
            Quat::from_rotation_y(PI / 8.0),
            roof,
        );
        a.paint
            .cyl(v(0.0, 5.7, 0.0), 0.08, 0.6, Quat::IDENTITY, white);
        a.paint.sphere(v(0.0, 6.05, 0.0), 0.15, c(0.85, 0.7, 0.2));
        self.place(a, v(x, 0.0, z), 0.0);
        for i in 0..8 {
            let ang = i as f32 / 8.0 * TAU + PI / 8.0;
            self.collide(
                v(x + ang.cos() * r, 1.7, z + ang.sin() * r),
                v(0.35, 3.4, 0.35),
            );
        }
    }

    /// Bronze statue of a figure on a stone plinth.
    pub fn statue(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let stone = c(0.62, 0.6, 0.56);
        let bronze = c(0.25, 0.42, 0.36);
        boxr(
            &mut a.paint,
            v(-1.1, 0.0, -1.1),
            v(1.1, 0.4, 1.1),
            shade(stone, 0.85),
        );
        boxr(&mut a.paint, v(-0.8, 0.4, -0.8), v(0.8, 2.0, 0.8), stone);
        boxr(
            &mut a.paint,
            v(-0.95, 2.0, -0.95),
            v(0.95, 2.2, 0.95),
            shade(stone, 0.85),
        );
        boxr(
            &mut a.metal,
            v(-0.4, 1.0, 0.8),
            v(0.4, 1.4, 0.82),
            c(0.7, 0.55, 0.25),
        );
        let b = &mut a.metal;
        let base = 2.2;
        b.capsule_between(v(-0.15, base, 0.0), v(-0.15, base + 0.9, 0.0), 0.11, bronze);
        b.capsule_between(v(0.15, base, 0.1), v(0.15, base + 0.9, 0.0), 0.11, bronze);
        b.capsule_between(
            v(0.0, base + 1.0, 0.0),
            v(0.0, base + 1.6, 0.0),
            0.25,
            bronze,
        );
        b.sphere(v(0.0, base + 2.0, 0.0), 0.17, bronze);
        b.capsule_between(
            v(-0.3, base + 1.55, 0.0),
            v(-0.45, base + 1.0, 0.1),
            0.08,
            bronze,
        );
        b.capsule_between(
            v(0.3, base + 1.55, 0.0),
            v(0.5, base + 2.1, -0.2),
            0.08,
            bronze,
        );
        b.beam(
            v(0.5, base + 2.05, -0.2),
            v(0.6, base + 2.9, -0.5),
            Vec2::splat(0.04),
            c(0.3, 0.45, 0.4),
        );
        b.cone(v(0.0, base + 2.2, 0.0), 0.22, 0.2, Quat::IDENTITY, bronze);
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 1.1, z), v(2.2, 2.2, 2.2));
    }

    /// Slide and swing set.
    pub fn playground(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let red = c(0.85, 0.2, 0.15);
        let blue = c(0.15, 0.35, 0.8);
        let yel = c(0.95, 0.75, 0.15);
        a.paint
            .cuboid(v(0.0, 0.02, 0.0), v(10.0, 0.04, 6.0), c(0.75, 0.62, 0.42));
        // Slide tower.
        for (sx, sz) in [(-3.5f32, -1.5f32), (-2.3, -1.5), (-3.5, -0.3), (-2.3, -0.3)] {
            a.metal.cyl(v(sx, 1.2, sz), 0.07, 2.4, Quat::IDENTITY, blue);
        }
        boxr(&mut a.paint, v(-3.6, 1.4, -1.6), v(-2.2, 1.5, -0.2), yel);
        a.paint.cone(
            v(-2.9, 2.75, -0.9),
            1.1,
            0.7,
            Quat::from_rotation_y(FRAC_PI_2 / 2.0),
            red,
        );
        a.metal.cuboid_rot(
            v(-1.0, 0.75, -0.9),
            v(3.2, 0.06, 0.7),
            Quat::from_rotation_z(-0.5),
            red,
        );
        for s in [-1.0f32, 1.0] {
            a.metal.cuboid_rot(
                v(-1.0, 0.85, -0.9 + s * 0.35),
                v(3.2, 0.2, 0.04),
                Quat::from_rotation_z(-0.5),
                red,
            );
        }
        for i in 0..5 {
            a.metal.cyl(
                v(-3.0, 0.2 + i as f32 * 0.28, -0.1),
                0.025,
                0.6,
                Quat::from_rotation_z(FRAC_PI_2),
                blue,
            );
        }
        // Swings.
        for sx in [1.5f32, 4.0] {
            a.metal
                .beam(v(sx, 0.0, -1.0), v(sx, 2.4, 0.0), Vec2::splat(0.09), blue);
            a.metal
                .beam(v(sx, 0.0, 1.0), v(sx, 2.4, 0.0), Vec2::splat(0.09), blue);
        }
        a.metal.cyl(
            v(2.75, 2.4, 0.0),
            0.07,
            2.7,
            Quat::from_rotation_z(FRAC_PI_2),
            blue,
        );
        for sx in [2.2f32, 3.3] {
            for s in [-0.2f32, 0.2] {
                a.metal.cyl(
                    v(sx + s, 1.6, 0.0),
                    0.012,
                    1.6,
                    Quat::IDENTITY,
                    c(0.6, 0.6, 0.6),
                );
            }
            boxr(
                &mut a.paint,
                v(sx - 0.25, 0.75, -0.12),
                v(sx + 0.25, 0.8, 0.12),
                c(0.1, 0.1, 0.1),
            );
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(-2.9, 1.2, -0.9), v(1.4, 2.4, 1.4));
        self.collide_local(v(x, 0.0, z), yaw, v(1.5, 1.2, 0.0), v(0.3, 2.4, 2.0));
        self.collide_local(v(x, 0.0, z), yaw, v(4.0, 1.2, 0.0), v(0.3, 2.4, 2.0));
    }

    pub fn rock(&mut self, x: f32, z: f32, size: f32, seed: f32) {
        let mut a = Art::default();
        let g = c(0.46, 0.46, 0.47);
        for i in 0..3 {
            let ang = seed + i as f32 * 2.1;
            let p = v(ang.cos() * 0.4 * size, 0.3 * size, ang.sin() * 0.4 * size);
            a.paint.blob(
                p,
                v(
                    0.9 * size,
                    (0.55 + hash(seed, i as f32) * 0.3) * size,
                    0.75 * size,
                ),
                shade(g, 0.85 + hash(i as f32, seed) * 0.3),
            );
        }
        self.place(a, v(x, 0.0, z), seed);
        self.collide(v(x, 0.5 * size, z), v(1.8 * size, size, 1.6 * size));
    }

    pub fn bin(&mut self, x: f32, z: f32) {
        let mut a = Art::default();
        a.metal.cyl(
            v(0.0, 0.45, 0.0),
            0.28,
            0.9,
            Quat::IDENTITY,
            c(0.15, 0.3, 0.18),
        );
        a.metal.torus(
            v(0.0, 0.9, 0.0),
            0.03,
            0.29,
            Quat::IDENTITY,
            c(0.1, 0.1, 0.1),
        );
        a.metal.cyl(
            v(0.0, 0.92, 0.0),
            0.24,
            0.04,
            Quat::IDENTITY,
            c(0.05, 0.05, 0.05),
        );
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 0.45, z), v(0.6, 0.9, 0.6));
    }

    /// A city skyline outside the play area (scenery).
    pub fn skyline(&mut self, half: f32, seed: f32) {
        let mut a = Art::default();
        let cols = [
            c(0.55, 0.58, 0.62),
            c(0.65, 0.6, 0.55),
            c(0.4, 0.45, 0.52),
            c(0.75, 0.74, 0.7),
        ];
        for side in 0..4 {
            let mut t = -half - 10.0;
            let mut i = 0;
            while t < half + 10.0 {
                let w = 8.0 + hash(seed + side as f32, i as f32) * 10.0;
                let h = 15.0 + hash(seed, i as f32 + side as f32 * 13.0) * 35.0;
                let d = half + 18.0 + hash(i as f32, seed) * 15.0;
                let col = cols[(i + side) % 4];
                let center = t + w / 2.0;
                let (p, size) = match side {
                    0 => (v(center, h / 2.0, -d), v(w, h, 10.0)),
                    1 => (v(center, h / 2.0, d), v(w, h, 10.0)),
                    2 => (v(-d, h / 2.0, center), v(10.0, h, w)),
                    _ => (v(d, h / 2.0, center), v(10.0, h, w)),
                };
                a.paint.cuboid(p, size, col);
                // Window bands on the side facing the park.
                let face = match side {
                    0 => v(0.0, 0.0, 5.02),
                    1 => v(0.0, 0.0, -5.02),
                    2 => v(5.02, 0.0, 0.0),
                    _ => v(-5.02, 0.0, 0.0),
                };
                let mut y = 3.0;
                while y < h - 2.0 {
                    let band = if face.x == 0.0 {
                        v(w * 0.85, 1.0, 0.05)
                    } else {
                        v(0.05, 1.0, w * 0.85)
                    };
                    a.glass
                        .cuboid(p + face + v(0.0, y - h / 2.0, 0.0), band, c(0.3, 0.4, 0.5));
                    y += 3.0;
                }
                t += w + 2.0;
                i += 1;
            }
        }
        self.place(a, Vec3::ZERO, 0.0);
    }

    // -----------------------------------------------------------------------
    // Neighborhood
    // -----------------------------------------------------------------------

    /// Two-storey-ish house facing +Z: siding, pitched roof with gables,
    /// framed windows with shutters, a porch with columns, chimney, garage.
    pub fn house(&mut self, x: f32, z: f32, yaw: f32, wall: Color, roof: Color, seed: f32) {
        let mut a = Art::default();
        let (l, h, w) = (9.0, 4.0, 8.0);
        let trim = c(0.95, 0.95, 0.92);
        let k = &mut a.paint;
        boxr(k, v(-l / 2.0, 0.0, -w / 2.0), v(l / 2.0, h, w / 2.0), wall);
        boxr(
            k,
            v(-l / 2.0 - 0.05, 0.0, -w / 2.0 - 0.05),
            v(l / 2.0 + 0.05, 0.45, w / 2.0 + 0.05),
            c(0.5, 0.48, 0.45),
        );
        // Siding lines.
        let mut y = 0.7;
        while y < h {
            for s in [-1.0f32, 1.0] {
                boxr(
                    k,
                    v(-l / 2.0, y, s * (w / 2.0 + 0.005) - 0.01),
                    v(l / 2.0, y + 0.03, s * (w / 2.0 + 0.012)),
                    shade(wall, 0.88),
                );
            }
            for s in [-1.0f32, 1.0] {
                boxr(
                    k,
                    v(s * (l / 2.0 + 0.005) - 0.01, y, -w / 2.0),
                    v(s * (l / 2.0 + 0.012), y + 0.03, w / 2.0),
                    shade(wall, 0.88),
                );
            }
            y += 0.3;
        }
        // Corner boards.
        for sx in [-1.0f32, 1.0] {
            for sz in [-1.0f32, 1.0] {
                boxr(
                    k,
                    v(sx * l / 2.0 - 0.1, 0.45, sz * w / 2.0 - 0.1),
                    v(sx * l / 2.0 + 0.1, h, sz * w / 2.0 + 0.1),
                    trim,
                );
            }
        }
        // Gables and roof (ridge along X).
        let rh = 2.6;
        a.paint.wedge(
            v(0.0, h + rh / 2.0, 0.0),
            v(w, rh, l),
            Quat::from_rotation_y(FRAC_PI_2),
            wall,
        );
        let slope = (rh / (w / 2.0)).atan();
        let slab_len = ((w / 2.0).powi(2) + rh * rh).sqrt() + 0.6;
        for s in [-1.0f32, 1.0] {
            let center = v(0.0, h + rh / 2.0 + 0.1, s * w / 4.0 + s * 0.15);
            let rot = Quat::from_rotation_x(s * slope);
            a.paint
                .cuboid_rot(center, v(l + 0.8, 0.16, slab_len), rot, roof);
            // Shingle rows.
            for i in 0..6 {
                let along = (i as f32 / 5.0 - 0.5) * (slab_len - 0.4);
                let p = center + rot * v(0.0, 0.1, along);
                a.paint
                    .cuboid_rot(p, v(l + 0.82, 0.04, 0.08), rot, shade(roof, 0.8));
            }
            // Gutter.
            let eave = v(0.0, h - 0.05, s * (w / 2.0 + 0.45));
            a.metal.cuboid(eave, v(l + 0.8, 0.12, 0.12), trim);
        }
        // Chimney.
        let cx = l / 2.0 - 1.4;
        boxr(
            &mut a.paint,
            v(cx - 0.4, h, -1.5),
            v(cx + 0.4, h + rh + 1.0, -0.7),
            c(0.55, 0.25, 0.18),
        );
        boxr(
            &mut a.paint,
            v(cx - 0.48, h + rh + 1.0, -1.58),
            v(cx + 0.48, h + rh + 1.15, -0.62),
            c(0.4, 0.4, 0.4),
        );
        // Front: door, windows with shutters, porch.
        let fz = w / 2.0;
        let door_x = -1.0;
        boxr(
            &mut a.paint,
            v(door_x - 0.65, 0.45, fz),
            v(door_x + 0.65, 2.65, fz + 0.06),
            trim,
        );
        let door = [c(0.55, 0.15, 0.12), c(0.15, 0.3, 0.2), c(0.2, 0.25, 0.45)]
            [(hash(seed, 5.0) * 3.0) as usize % 3];
        boxr(
            &mut a.paint,
            v(door_x - 0.5, 0.45, fz + 0.04),
            v(door_x + 0.5, 2.5, fz + 0.09),
            door,
        );
        a.metal
            .sphere(v(door_x + 0.35, 1.45, fz + 0.12), 0.05, c(0.85, 0.7, 0.3));
        let win = |a: &mut Art, cx: f32, cy: f32, front: bool| {
            let (zz, sgn) = if front { (fz, 1.0) } else { (-fz, -1.0) };
            boxr(
                &mut a.paint,
                v(cx - 0.75, cy - 0.65, zz),
                v(cx + 0.75, cy + 0.65, zz + sgn * 0.06),
                trim,
            );
            boxr(
                &mut a.glow,
                v(cx - 0.62, cy - 0.52, zz + sgn * 0.05),
                v(cx + 0.62, cy + 0.52, zz + sgn * 0.07),
                c(0.95, 0.85, 0.55),
            );
            boxr(
                &mut a.paint,
                v(cx - 0.03, cy - 0.52, zz + sgn * 0.07),
                v(cx + 0.03, cy + 0.52, zz + sgn * 0.09),
                trim,
            );
            boxr(
                &mut a.paint,
                v(cx - 0.62, cy - 0.03, zz + sgn * 0.07),
                v(cx + 0.62, cy + 0.03, zz + sgn * 0.09),
                trim,
            );
            for s in [-1.0f32, 1.0] {
                boxr(
                    &mut a.paint,
                    v(cx + s * 0.8 - 0.22, cy - 0.65, zz),
                    v(cx + s * 0.8 + 0.22, cy + 0.65, zz + sgn * 0.08),
                    c(0.2, 0.22, 0.25),
                );
            }
            boxr(
                &mut a.paint,
                v(cx - 0.85, cy - 0.75, zz),
                v(cx + 0.85, cy - 0.68, zz + sgn * 0.15),
                trim,
            );
        };
        win(&mut a, 1.6, 2.0, true);
        win(&mut a, -3.2, 2.0, true);
        win(&mut a, 2.5, 2.0, false);
        win(&mut a, -2.5, 2.0, false);
        // Porch deck, steps, columns and roof.
        boxr(
            &mut a.paint,
            v(-3.0, 0.0, fz),
            v(1.0, 0.35, fz + 1.8),
            c(0.6, 0.45, 0.32),
        );
        for i in 0..2 {
            boxr(
                &mut a.paint,
                v(door_x - 0.8, 0.0, fz + 1.8 + i as f32 * 0.3),
                v(
                    door_x + 0.8,
                    0.25 - i as f32 * 0.12,
                    fz + 2.1 + i as f32 * 0.3,
                ),
                c(0.6, 0.6, 0.58),
            );
        }
        for px in [-2.85f32, 0.85] {
            a.paint
                .cyl(v(px, 1.6, fz + 1.65), 0.1, 2.5, Quat::IDENTITY, trim);
        }
        a.paint.cuboid_rot(
            v(-1.0, 2.95, fz + 1.0),
            v(4.4, 0.12, 2.3),
            Quat::from_rotation_x(0.2),
            roof,
        );
        // Railing.
        boxr(
            &mut a.paint,
            v(-2.9, 0.9, fz + 1.7),
            v(-1.9, 0.95, fz + 1.75),
            trim,
        );
        for i in 0..6 {
            let px = -2.85 + i as f32 * 0.18;
            boxr(
                &mut a.paint,
                v(px - 0.02, 0.35, fz + 1.7),
                v(px + 0.02, 0.92, fz + 1.74),
                trim,
            );
        }
        // Garage door on the side facing +X.
        let gx = l / 2.0;
        boxr(&mut a.paint, v(gx, 0.1, -1.6), v(gx + 0.06, 2.6, 1.6), trim);
        boxr(
            &mut a.paint,
            v(gx + 0.03, 0.12, -1.5),
            v(gx + 0.09, 2.5, 1.5),
            c(0.85, 0.85, 0.82),
        );
        for i in 1..5 {
            let y = 0.12 + i as f32 * 0.48;
            boxr(
                &mut a.paint,
                v(gx + 0.08, y - 0.02, -1.5),
                v(gx + 0.1, y + 0.02, 1.5),
                c(0.7, 0.7, 0.68),
            );
        }
        // Garden flower bed along the front.
        boxr(
            &mut a.paint,
            v(1.0, 0.0, fz),
            v(4.3, 0.2, fz + 0.7),
            c(0.3, 0.2, 0.12),
        );
        for i in 0..8 {
            let px = 1.2 + i as f32 * 0.4;
            let fc = [c(0.9, 0.2, 0.3), c(0.95, 0.85, 0.2), c(0.6, 0.3, 0.9)][i % 3];
            a.paint
                .sphere(v(px, 0.35, fz + 0.35), 0.15, c(0.2, 0.45, 0.15));
            a.paint.sphere(v(px, 0.45, fz + 0.38), 0.07, fc);
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(
            v(x, 0.0, z),
            yaw,
            v(0.0, h / 2.0 + 1.0, 0.0),
            v(l, h + 2.0, w),
        );
        for px in [-2.85f32, 0.85] {
            self.collide_local(v(x, 0.0, z), yaw, v(px, 1.3, fz + 1.65), v(0.25, 2.6, 0.25));
        }
    }

    /// White picket fence from `a` to `b` (axis aligned), with an optional
    /// gate gap in the middle.
    pub fn picket_fence(&mut self, a: Vec3, b: Vec3, gap: f32) {
        let mut art = Art::default();
        let white = c(0.95, 0.95, 0.92);
        let d = b - a;
        let len = d.length();
        let dir = d / len;
        let mid = len / 2.0;
        let mut t = 0.05;
        while t < len {
            if (t - mid).abs() > gap / 2.0 {
                let p = a + dir * t;
                art.paint
                    .cuboid(p + v(0.0, 0.42, 0.0), v(0.09, 0.84, 0.09).abs(), white);
                art.paint.cuboid_rot(
                    p + v(0.0, 0.88, 0.0),
                    v(0.064, 0.064, 0.02),
                    Quat::from_rotation_arc(Vec3::X, dir) * Quat::from_rotation_z(PI / 4.0),
                    white,
                );
            }
            t += 0.17;
        }
        for (s0, s1) in [
            (0.0, (mid - gap / 2.0).max(0.0)),
            ((mid + gap / 2.0).min(len), len),
        ] {
            if s1 - s0 < 0.1 {
                continue;
            }
            for y in [0.25, 0.65] {
                art.paint.beam(
                    a + dir * s0 + v(0.0, y, 0.0),
                    a + dir * s1 + v(0.0, y, 0.0),
                    Vec2::new(0.1, 0.03),
                    white,
                );
            }
            let p0 = a + dir * s0;
            let p1 = a + dir * s1;
            let center = (p0 + p1) / 2.0 + v(0.0, 0.45, 0.0);
            let size = (p1 - p0).abs() + v(0.15, 0.9, 0.15);
            self.collide(center, size);
        }
        self.place(art, Vec3::ZERO, 0.0);
    }

    /// Sedan with windows, wheels and lights; faces +X before `yaw`.
    pub fn car(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        let k = &mut a.metal;
        let dark = c(0.05, 0.05, 0.05);
        boxr(k, v(-2.1, 0.35, -0.9), v(2.1, 0.95, 0.9), color);
        k.cuboid_rot(
            v(1.65, 0.95, 0.0),
            v(0.9, 0.12, 1.78),
            Quat::from_rotation_z(-0.08),
            color,
        );
        boxr(k, v(-1.3, 0.95, -0.82), v(0.8, 1.5, 0.82), color);
        boxr(
            &mut a.glass,
            v(-1.25, 1.0, -0.83),
            v(0.75, 1.42, 0.83),
            c(0.12, 0.16, 0.2),
        );
        a.glass.cuboid_rot(
            v(1.0, 1.2, 0.0),
            v(0.05, 0.62, 1.55),
            Quat::from_rotation_z(0.75),
            c(0.12, 0.16, 0.2),
        );
        a.glass.cuboid_rot(
            v(-1.5, 1.2, 0.0),
            v(0.05, 0.58, 1.55),
            Quat::from_rotation_z(-0.7),
            c(0.12, 0.16, 0.2),
        );
        a.metal.cuboid_rot(
            v(1.0, 1.25, 0.0),
            v(0.5, 0.06, 1.62),
            Quat::from_rotation_z(-0.8),
            color,
        );
        a.metal.cuboid_rot(
            v(-1.5, 1.24, 0.0),
            v(0.45, 0.06, 1.62),
            Quat::from_rotation_z(0.75),
            color,
        );
        boxr(
            &mut a.paint,
            v(-2.18, 0.3, -0.92),
            v(-2.05, 0.55, 0.92),
            dark,
        );
        boxr(&mut a.paint, v(2.05, 0.3, -0.92), v(2.18, 0.55, 0.92), dark);
        for s in [-1.0f32, 1.0] {
            a.glow.cuboid(
                v(2.11, 0.78, s * 0.62),
                v(0.04, 0.16, 0.36),
                c(1.0, 0.97, 0.85),
            );
            a.glow.cuboid(
                v(-2.11, 0.8, s * 0.65),
                v(0.04, 0.14, 0.3),
                c(0.9, 0.08, 0.08),
            );
            a.metal
                .cuboid(v(0.55, 1.05, s * 0.95), v(0.12, 0.1, 0.12), color);
            // Door seams and handles.
            boxr(
                &mut a.paint,
                v(-0.25, 0.42, s * 0.905 - 0.003),
                v(-0.22, 0.95, s * 0.905 + 0.003),
                shade(color, 0.6),
            );
            a.metal.cuboid(
                v(0.2, 0.85, s * 0.91),
                v(0.18, 0.04, 0.02),
                c(0.7, 0.7, 0.7),
            );
            a.metal.cuboid(
                v(-0.7, 0.85, s * 0.91),
                v(0.18, 0.04, 0.02),
                c(0.7, 0.7, 0.7),
            );
            for wx in [-1.35f32, 1.35] {
                a.paint.cyl(
                    v(wx, 0.34, s * 0.82),
                    0.34,
                    0.24,
                    Quat::from_rotation_x(FRAC_PI_2),
                    dark,
                );
                a.metal.cyl(
                    v(wx, 0.34, s * 0.95),
                    0.2,
                    0.02,
                    Quat::from_rotation_x(FRAC_PI_2),
                    c(0.75, 0.75, 0.75),
                );
            }
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.75, 0.0), v(4.3, 1.5, 1.9));
    }

    pub fn mailbox(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        a.paint
            .cuboid(v(0.0, 0.5, 0.0), v(0.1, 1.0, 0.1), c(0.4, 0.3, 0.2));
        a.metal.cuboid(v(0.0, 1.1, 0.0), v(0.25, 0.2, 0.45), color);
        a.metal.cyl(
            v(0.0, 1.2, 0.0),
            0.125,
            0.45,
            Quat::from_rotation_x(FRAC_PI_2),
            color,
        );
        a.metal
            .cuboid(v(0.14, 1.25, 0.1), v(0.02, 0.2, 0.05), c(0.9, 0.1, 0.1));
        self.place(a, v(x, 0.0, z), yaw);
    }

    pub fn hydrant(&mut self, x: f32, z: f32) {
        let mut a = Art::default();
        let red = c(0.8, 0.1, 0.08);
        a.metal
            .cyl(v(0.0, 0.35, 0.0), 0.13, 0.7, Quat::IDENTITY, red);
        a.metal
            .cyl(v(0.0, 0.05, 0.0), 0.18, 0.1, Quat::IDENTITY, red);
        a.metal.sphere(v(0.0, 0.72, 0.0), 0.14, red);
        a.metal
            .cyl(v(0.0, 0.88, 0.0), 0.04, 0.08, Quat::IDENTITY, red);
        for s in [-1.0f32, 1.0] {
            a.metal.cyl(
                v(s * 0.16, 0.5, 0.0),
                0.06,
                0.1,
                Quat::from_rotation_z(FRAC_PI_2),
                c(0.85, 0.85, 0.85),
            );
        }
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 0.4, z), v(0.35, 0.8, 0.35));
    }

    /// Street light: pole with a curved arm out over the road (-Z).
    pub fn street_light(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let grey = c(0.35, 0.36, 0.38);
        a.metal
            .frustum(v(0.0, 2.75, 0.0), 0.07, 0.12, 5.5, Quat::IDENTITY, grey);
        a.metal
            .cyl(v(0.0, 0.2, 0.0), 0.18, 0.4, Quat::IDENTITY, grey);
        a.metal
            .beam(v(0.0, 5.4, 0.0), v(0.0, 5.9, -0.8), Vec2::splat(0.08), grey);
        a.metal.beam(
            v(0.0, 5.9, -0.8),
            v(0.0, 5.95, -1.8),
            Vec2::splat(0.08),
            grey,
        );
        boxr(&mut a.metal, v(-0.22, 5.8, -2.4), v(0.22, 6.0, -1.6), grey);
        a.glow
            .cuboid(v(0.0, 5.78, -2.0), v(0.36, 0.04, 0.7), c(1.0, 0.9, 0.65));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 2.0, z), v(0.3, 4.0, 0.3));
        let p = v(x, 5.4, z) + Quat::from_rotation_y(yaw) * v(0.0, 0.0, -2.0);
        self.light(p, c(1.0, 0.85, 0.6), 150_000.0);
    }

    pub fn trash_cans(&mut self, x: f32, z: f32) {
        let mut a = Art::default();
        for (i, col) in [c(0.15, 0.3, 0.18), c(0.2, 0.25, 0.5)].iter().enumerate() {
            let px = i as f32 * 0.75;
            boxr(
                &mut a.paint,
                v(px - 0.3, 0.0, -0.35),
                v(px + 0.3, 1.0, 0.35),
                *col,
            );
            boxr(
                &mut a.paint,
                v(px - 0.33, 1.0, -0.38),
                v(px + 0.33, 1.08, 0.38),
                shade(*col, 0.7),
            );
            a.paint.cyl(
                v(px, 0.08, 0.38),
                0.08,
                0.06,
                Quat::from_rotation_z(FRAC_PI_2),
                c(0.05, 0.05, 0.05),
            );
        }
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x + 0.37, 0.5, z), v(1.4, 1.0, 0.75));
    }

    pub fn shed(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let wood = c(0.55, 0.4, 0.28);
        boxr(&mut a.paint, v(-1.5, 0.0, -1.5), v(1.5, 2.2, 1.5), wood);
        for i in 0..10 {
            let px = -1.5 + i as f32 * 0.3;
            boxr(
                &mut a.paint,
                v(px - 0.015, 0.0, 1.5),
                v(px + 0.015, 2.2, 1.52),
                shade(wood, 0.7),
            );
        }
        a.paint
            .wedge(v(0.0, 2.7, 0.0), v(3.0, 1.0, 3.0), Quat::IDENTITY, wood);
        for s in [-1.0f32, 1.0] {
            a.paint.cuboid_rot(
                v(s * 0.8, 2.75, 0.0),
                v(1.9, 0.08, 3.4),
                Quat::from_rotation_z(-s * 0.58),
                c(0.25, 0.25, 0.28),
            );
        }
        boxr(
            &mut a.paint,
            v(-0.5, 0.0, 1.51),
            v(0.5, 1.9, 1.56),
            shade(wood, 0.8),
        );
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 1.2, z), v(3.0, 2.4, 3.0));
    }

    pub fn hoop(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        a.metal.cyl(
            v(0.0, 1.6, 0.0),
            0.07,
            3.2,
            Quat::IDENTITY,
            c(0.25, 0.25, 0.27),
        );
        a.metal.beam(
            v(0.0, 3.1, 0.0),
            v(0.0, 3.1, -0.6),
            Vec2::splat(0.06),
            c(0.25, 0.25, 0.27),
        );
        boxr(
            &mut a.paint,
            v(-0.9, 2.8, -0.65),
            v(0.9, 3.85, -0.6),
            c(0.95, 0.95, 0.95),
        );
        boxr(
            &mut a.paint,
            v(-0.3, 3.05, -0.66),
            v(0.3, 3.45, -0.65),
            c(0.85, 0.2, 0.1),
        );
        a.metal.torus(
            v(0.0, 3.05, -0.9),
            0.015,
            0.23,
            Quat::IDENTITY,
            c(0.95, 0.4, 0.1),
        );
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 1.5, z), v(0.2, 3.0, 0.2));
    }

    // -----------------------------------------------------------------------
    // Boundaries
    // -----------------------------------------------------------------------

    /// Visual edge around the play area plus invisible walls. Style:
    /// 0 concrete + chain-link, 1 stone + iron railing, 2 wooden fence.
    pub fn boundary(&mut self, style: u8, height: f32) {
        let h = self.half;
        let mut a = Art::default();
        let sides = [
            (v(-h, 0.0, -h - 0.5), v(h, 0.0, -h - 0.5)),
            (v(-h, 0.0, h + 0.5), v(h, 0.0, h + 0.5)),
            (v(-h - 0.5, 0.0, -h), v(-h - 0.5, 0.0, h)),
            (v(h + 0.5, 0.0, -h), v(h + 0.5, 0.0, h)),
        ];
        for (p0, p1) in sides {
            self.fence_run(&mut a, p0, p1, style, height);
        }
        self.place(a, Vec3::ZERO, 0.0);
    }

    /// One straight run of fence from `p0` to `p1` (along X or Z) with an
    /// invisible wall. Style: 0 concrete + chain-link, 1 stone + iron
    /// railing, 2 wooden fence.
    pub fn fence_run(&mut self, a: &mut Art, p0: Vec3, p1: Vec3, style: u8, height: f32) {
        let d = p1 - p0;
        let len = d.length() + 1.0;
        let dir = d.normalize();
        let center = (p0 + p1) / 2.0;
        let across = v(dir.z.abs(), 0.0, dir.x.abs());
        let size = |t: f32, hh: f32| dir.abs() * len + across * t + v(0.0, hh, 0.0);
        match style {
            0 => {
                a.paint.cuboid(
                    center + v(0.0, 0.6, 0.0),
                    size(0.5, 1.2),
                    c(0.55, 0.55, 0.53),
                );
                a.glass.cuboid(
                    center + v(0.0, 1.2 + (height - 1.2) / 2.0, 0.0),
                    size(0.02, height - 1.2),
                    c(0.5, 0.52, 0.55),
                );
                let mut t = 0.0;
                while t <= len {
                    let p = p0 - dir * 0.5 + dir * t;
                    a.metal.cyl(
                        p + v(0.0, height / 2.0 + 0.3, 0.0),
                        0.05,
                        height - 0.6,
                        Quat::IDENTITY,
                        c(0.6, 0.6, 0.62),
                    );
                    t += 3.0;
                }
                a.metal.beam(
                    p0 - dir * 0.5 + v(0.0, height - 0.1, 0.0),
                    p1 + dir * 0.5 + v(0.0, height - 0.1, 0.0),
                    Vec2::splat(0.06),
                    c(0.6, 0.6, 0.62),
                );
            }
            1 => {
                a.paint.cuboid(
                    center + v(0.0, 0.5, 0.0),
                    size(0.6, 1.0),
                    c(0.55, 0.52, 0.48),
                );
                a.paint.cuboid(
                    center + v(0.0, 1.05, 0.0),
                    size(0.7, 0.1),
                    c(0.45, 0.43, 0.4),
                );
                let mut t = 0.0;
                while t <= len {
                    let p = p0 - dir * 0.5 + dir * t;
                    a.metal.cyl(
                        p + v(0.0, 1.1 + (height - 1.1) / 2.0, 0.0),
                        0.025,
                        height - 1.1,
                        Quat::IDENTITY,
                        c(0.08, 0.08, 0.08),
                    );
                    a.metal.cone(
                        p + v(0.0, height + 0.07, 0.0),
                        0.05,
                        0.14,
                        Quat::IDENTITY,
                        c(0.08, 0.08, 0.08),
                    );
                    t += 0.25;
                }
                a.metal.beam(
                    p0 - dir * 0.5 + v(0.0, height - 0.25, 0.0),
                    p1 + dir * 0.5 + v(0.0, height - 0.25, 0.0),
                    Vec2::splat(0.04),
                    c(0.08, 0.08, 0.08),
                );
            }
            _ => {
                let wood = c(0.55, 0.4, 0.27);
                let mut t = 0.0;
                while t <= len {
                    let p = p0 - dir * 0.5 + dir * t;
                    let hh = height + (hash(t, len) - 0.5) * 0.06;
                    a.paint.cuboid(
                        p + v(0.0, hh / 2.0, 0.0),
                        dir.abs() * 0.14 + across * 0.04 + v(0.0, hh, 0.0),
                        shade(wood, 0.85 + hash(t, 1.0) * 0.25),
                    );
                    t += 0.15;
                }
                for y in [0.4, height - 0.4] {
                    a.paint.cuboid(
                        center + v(0.0, y, 0.0)
                            - across * 0.06 * if center.x + center.z > 0.0 { 1.0 } else { -1.0 },
                        size(0.05, 0.12),
                        shade(wood, 0.7),
                    );
                }
            }
        }
        self.collide(center + v(0.0, 2.5, 0.0), size(1.0, 5.0));
    }
}
