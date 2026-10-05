//! The floor-plan kit every map is drawn with: walls with doorways and
//! windows, roofs, floors and ceiling lamps, plus the furniture that gives
//! each room its look (offices, lockers, kitchens, pews, labs and so on).
//!
//! A map lists its walls in a `Plan`, puts its bought doors on the wall
//! lines, then `build` cuts the doorways and windows and adds the art and
//! the boxes to collide with.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::kit::c;
use crate::maps::{DoorDef, MapLayout, DOOR_WIDTH};
use crate::props::{boxr, hash, shade, v, Art};

/// What a wall is made of.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// Painted concrete block.
    Block,
    Brick,
    /// Plaster with a darker lower band.
    Plaster,
    /// Clapboard siding.
    Siding,
    /// Corrugated sheet metal.
    Metal,
    Stone,
    /// Clipped hedge.
    Hedge,
    /// Concrete footing with chain-link above (you can see through it).
    Fence,
    /// Wooden plank fence.
    Wood,
    /// Glasshouse panes in a white frame.
    Glass,
}

impl Finish {
    fn thick(self) -> f32 {
        match self {
            Finish::Hedge => 1.0,
            Finish::Fence | Finish::Wood | Finish::Glass => 0.3,
            _ => 0.4,
        }
    }

    /// Solid walls that get lintels over doorways and windows in them.
    fn building(self) -> bool {
        !matches!(
            self,
            Finish::Hedge | Finish::Fence | Finish::Wood | Finish::Glass
        )
    }
}

#[derive(Clone, Copy)]
pub struct Look {
    pub finish: Finish,
    pub color: Color,
    pub trim: Color,
}

pub const fn look(finish: Finish, color: Color, trim: Color) -> Look {
    Look {
        finish,
        color,
        trim,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GapKind {
    Door,
    Open,
    Window,
}

struct Wall {
    along_x: bool,
    line: f32,
    a: f32,
    b: f32,
    h: f32,
    look: Look,
}

struct Gap {
    along_x: bool,
    line: f32,
    at: f32,
    w: f32,
    kind: GapKind,
}

/// A map's walls and the openings in them.
#[derive(Default)]
pub struct Plan {
    walls: Vec<Wall>,
    gaps: Vec<Gap>,
}

impl Plan {
    /// A wall along X at `z`, from `x0` to `x1`.
    pub fn wx(&mut self, z: f32, x0: f32, x1: f32, h: f32, look: Look) {
        self.walls.push(Wall {
            along_x: true,
            line: z,
            a: x0.min(x1),
            b: x0.max(x1),
            h,
            look,
        });
    }

    /// A wall along Z at `x`, from `z0` to `z1`.
    pub fn wz(&mut self, x: f32, z0: f32, z1: f32, h: f32, look: Look) {
        self.walls.push(Wall {
            along_x: false,
            line: x,
            a: z0.min(z1),
            b: z0.max(z1),
            h,
            look,
        });
    }

    /// An open doorway `w` wide in the wall along X at `z`, centred on `x`.
    pub fn open_x(&mut self, x: f32, z: f32, w: f32) {
        self.gap(true, z, x, w, GapKind::Open);
    }

    /// An open doorway in the wall along Z at `x`, centred on `z`.
    pub fn open_z(&mut self, x: f32, z: f32, w: f32) {
        self.gap(false, x, z, w, GapKind::Open);
    }

    /// A window (you can see through but not climb through).
    pub fn win_x(&mut self, x: f32, z: f32, w: f32) {
        self.gap(true, z, x, w, GapKind::Window);
    }

    pub fn win_z(&mut self, x: f32, z: f32, w: f32) {
        self.gap(false, x, z, w, GapKind::Window);
    }

    fn gap(&mut self, along_x: bool, line: f32, at: f32, w: f32, kind: GapKind) {
        self.gaps.push(Gap {
            along_x,
            line,
            at,
            w,
            kind,
        });
    }
}

/// Floor coverings.
#[derive(Clone, Copy)]
pub enum Floor {
    /// Bare concrete with joints.
    Concrete(Color),
    /// Square tiles with grout lines.
    Tiles(Color),
    /// Black and white (or any two colours) checks.
    Checker(Color, Color),
    /// Boards running along X.
    Boards(Color),
    /// Carpet with a border.
    Carpet(Color),
}

/// Lintel height over open doorways and windows.
const OPEN_H: f32 = 3.5;
/// Lintel height over bought doors (clear of their lamps).
const DOOR_TOP: f32 = 4.0;
const SILL: f32 = 1.0;
const WIN_TOP: f32 = 2.4;

impl MapLayout {
    /// A wide doorway in a wall along X (at `z`).
    pub(crate) fn door_x(&mut self, x: f32, z: f32) {
        self.doors.push(DoorDef {
            pos: v(x, 0.0, z),
            along_x: true,
        });
    }

    /// A wide doorway in a wall along Z (at `x`).
    pub(crate) fn door_z(&mut self, x: f32, z: f32) {
        self.doors.push(DoorDef {
            pos: v(x, 0.0, z),
            along_x: false,
        });
    }

    /// Builds every wall in the plan, cutting doorways for the doors that
    /// sit on it and for its openings and windows.
    pub(crate) fn build(&mut self, plan: &Plan) {
        let mut a = Art::default();
        for w in &plan.walls {
            let on = |along_x: bool, line: f32, at: f32| {
                along_x == w.along_x
                    && (line - w.line).abs() < 0.35
                    && at > w.a
                    && at < w.b
            };
            let mut cuts: Vec<(f32, f32, GapKind)> = self
                .doors
                .iter()
                .filter(|d| {
                    let (line, at) = if d.along_x {
                        (d.pos.z, d.pos.x)
                    } else {
                        (d.pos.x, d.pos.z)
                    };
                    on(d.along_x, line, at)
                })
                .map(|d| {
                    let at = if d.along_x { d.pos.x } else { d.pos.z };
                    (at, DOOR_WIDTH, GapKind::Door)
                })
                .collect();
            cuts.extend(
                plan.gaps
                    .iter()
                    .filter(|g| on(g.along_x, g.line, g.at))
                    .map(|g| (g.at, g.w, g.kind)),
            );
            cuts.sort_by(|p, q| p.0.total_cmp(&q.0));
            let t = w.look.finish.thick();
            // Collide in runs broken only by ways through.
            let mut run = w.a;
            let mut pos = w.a;
            for (at, width, kind) in cuts {
                let (s, e) = (at - width / 2.0, at + width / 2.0);
                if s > pos + 0.02 {
                    wall_art(&mut a, w, pos, s, 0.0, w.h);
                }
                match kind {
                    GapKind::Window => {
                        wall_art(&mut a, w, s, e, 0.0, SILL.min(w.h));
                        if w.h > WIN_TOP + 0.05 {
                            wall_art(&mut a, w, s, e, WIN_TOP, w.h);
                        }
                        window_art(&mut a, w, s, e);
                    }
                    GapKind::Open | GapKind::Door => {
                        if s > run + 0.02 {
                            self.wall_box(w, run, s, t);
                        }
                        run = e;
                        let top = if kind == GapKind::Door {
                            DOOR_TOP
                        } else {
                            OPEN_H
                        };
                        if w.look.finish.building() && w.h > top + 0.05 {
                            wall_art(&mut a, w, s, e, top, w.h);
                            frame_art(&mut a, w, s, e, top);
                        }
                    }
                }
                pos = e;
            }
            if w.b > pos + 0.02 {
                wall_art(&mut a, w, pos, w.b, 0.0, w.h);
            }
            if w.b > run + 0.02 {
                self.wall_box(w, run, w.b, t);
            }
        }
        self.place(a, Vec3::ZERO, 0.0);
    }

    fn wall_box(&mut self, w: &Wall, s: f32, e: f32, t: f32) {
        // Tall enough that nobody hops over from a crate.
        let h = w.h.max(5.0);
        let (mid, len) = ((s + e) / 2.0, e - s);
        if w.along_x {
            self.collide(v(mid, h / 2.0, w.line), v(len, h, t));
        } else {
            self.collide(v(w.line, h / 2.0, mid), v(t, h, len));
        }
    }

    /// A flat roof over a rectangle, with a ceiling under it.
    pub(crate) fn roof(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, h: f32, color: Color) {
        let mut a = Art::default();
        boxr(
            &mut a.paint,
            v(x0 - 0.35, h, z0 - 0.35),
            v(x1 + 0.35, h + 0.35, z1 + 0.35),
            color,
        );
        boxr(
            &mut a.paint,
            v(x0 - 0.45, h + 0.35, z0 - 0.45),
            v(x1 + 0.45, h + 0.5, z1 + 0.45),
            shade(color, 0.8),
        );
        boxr(
            &mut a.paint,
            v(x0, h - 0.06, z0),
            v(x1, h, z1),
            c(0.82, 0.82, 0.8),
        );
        self.place(a, Vec3::ZERO, 0.0);
    }

    /// A pitched roof (ridge along X) with gables, over a ceiling.
    pub(crate) fn gable_roof(
        &mut self,
        x0: f32,
        z0: f32,
        x1: f32,
        z1: f32,
        h: f32,
        wall: Color,
        roof: Color,
    ) {
        let mut a = Art::default();
        let (cx, cz) = ((x0 + x1) / 2.0, (z0 + z1) / 2.0);
        let (l, w) = (x1 - x0, z1 - z0);
        let rh = (w * 0.22).min(3.5);
        boxr(&mut a.paint, v(x0, h - 0.06, z0), v(x1, h, z1), c(0.85, 0.84, 0.8));
        a.paint.wedge(
            v(cx, h + rh / 2.0, cz),
            v(w, rh, l),
            Quat::from_rotation_y(FRAC_PI_2),
            wall,
        );
        let slope = (rh / (w / 2.0)).atan();
        let slab = ((w / 2.0).powi(2) + rh * rh).sqrt() + 0.7;
        for s in [-1.0f32, 1.0] {
            let at = v(cx, h + rh / 2.0 + 0.1, cz + s * w / 4.0 + s * 0.15);
            let rot = Quat::from_rotation_x(s * slope);
            a.paint.cuboid_rot(at, v(l + 0.8, 0.18, slab), rot, roof);
            for i in 0..6 {
                let along = (i as f32 / 5.0 - 0.5) * (slab - 0.4);
                a.paint.cuboid_rot(at + rot * v(0.0, 0.11, along), v(l + 0.82, 0.04, 0.08), rot, shade(roof, 0.8));
            }
        }
        self.place(a, Vec3::ZERO, 0.0);
    }

    /// A glasshouse roof: panes on a white frame, ridged along X.
    pub(crate) fn glass_roof(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, h: f32) {
        let mut a = Art::default();
        let white = c(0.92, 0.93, 0.9);
        let (mid, half) = ((z0 + z1) / 2.0, (z1 - z0) / 2.0);
        let rise = 1.6;
        let slope = (rise / half).atan();
        let len = (half * half + rise * rise).sqrt();
        for s in [-1.0f32, 1.0] {
            let rot = Quat::from_rotation_x(s * slope);
            let at = v((x0 + x1) / 2.0, h + rise / 2.0, mid + s * half / 2.0);
            a.glass.cuboid_rot(at, v(x1 - x0, 0.05, len), rot, c(0.75, 0.88, 0.85));
            let mut x = x0;
            while x <= x1 + 0.01 {
                a.metal.cuboid_rot(v(x, at.y, at.z), v(0.1, 0.1, len), rot, white);
                x += 2.0;
            }
        }
        a.metal.cuboid(v((x0 + x1) / 2.0, h + rise, mid), v(x1 - x0, 0.15, 0.15), white);
        boxr(&mut a.metal, v(x0, h - 0.1, z0), v(x1, h, z0 + 0.15), white);
        boxr(&mut a.metal, v(x0, h - 0.1, z1 - 0.15), v(x1, h, z1), white);
        self.place(a, Vec3::ZERO, 0.0);
    }

    /// A floor covering over a rectangle.
    pub(crate) fn floor(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, floor: Floor) {
        let mut a = Art::default();
        let k = &mut a.paint;
        let (y0, y1) = (0.004, 0.02);
        match floor {
            Floor::Concrete(col) => {
                boxr(k, v(x0, y0, z0), v(x1, y1, z1), col);
                let mut x = x0 + 4.0;
                while x < x1 - 0.5 {
                    boxr(k, v(x - 0.03, y1, z0), v(x + 0.03, y1 + 0.004, z1), shade(col, 0.75));
                    x += 4.0;
                }
                let mut z = z0 + 4.0;
                while z < z1 - 0.5 {
                    boxr(k, v(x0, y1, z - 0.03), v(x1, y1 + 0.004, z + 0.03), shade(col, 0.75));
                    z += 4.0;
                }
            }
            Floor::Tiles(col) => {
                boxr(k, v(x0, y0, z0), v(x1, y1, z1), col);
                let mut x = x0 + 1.0;
                while x < x1 - 0.2 {
                    boxr(k, v(x - 0.02, y1, z0), v(x + 0.02, y1 + 0.004, z1), shade(col, 0.8));
                    x += 1.0;
                }
                let mut z = z0 + 1.0;
                while z < z1 - 0.2 {
                    boxr(k, v(x0, y1, z - 0.02), v(x1, y1 + 0.004, z + 0.02), shade(col, 0.8));
                    z += 1.0;
                }
            }
            Floor::Checker(c1, c2) => {
                boxr(k, v(x0, y0, z0), v(x1, y1, z1), c1);
                let s = 1.0;
                let mut i = 0;
                let mut x = x0;
                while x < x1 - 0.05 {
                    let mut j = 0;
                    let mut z = z0;
                    while z < z1 - 0.05 {
                        if (i + j) % 2 == 0 {
                            boxr(
                                k,
                                v(x, y1, z),
                                v((x + s).min(x1), y1 + 0.004, (z + s).min(z1)),
                                c2,
                            );
                        }
                        z += s;
                        j += 1;
                    }
                    x += s;
                    i += 1;
                }
            }
            Floor::Boards(col) => {
                boxr(k, v(x0, y0, z0), v(x1, y1, z1), col);
                let mut z = z0 + 0.3;
                let mut i = 0;
                while z < z1 - 0.1 {
                    boxr(
                        k,
                        v(x0, y1, z - 0.012),
                        v(x1, y1 + 0.004, z + 0.012),
                        shade(col, 0.7 + 0.1 * (i % 2) as f32),
                    );
                    z += 0.3;
                    i += 1;
                }
            }
            Floor::Carpet(col) => {
                boxr(k, v(x0, y0, z0), v(x1, y1, z1), shade(col, 0.6));
                boxr(
                    k,
                    v(x0 + 0.5, y1, z0 + 0.5),
                    v(x1 - 0.5, y1 + 0.006, z1 - 0.5),
                    col,
                );
            }
        }
        self.place(a, Vec3::ZERO, 0.0);
    }

    /// A ceiling lamp (a lit panel) with its light.
    pub(crate) fn lamp(&mut self, x: f32, z: f32, h: f32, color: Color, power: f32) {
        let mut a = Art::default();
        boxr(
            &mut a.metal,
            v(-0.7, -0.12, -0.25),
            v(0.7, 0.0, 0.25),
            c(0.3, 0.3, 0.32),
        );
        a.glow.cuboid(v(0.0, -0.13, 0.0), v(1.3, 0.02, 0.4), color);
        self.place(a, v(x, h - 0.06, z), 0.0);
        self.light(v(x, h - 0.7, z), color, power);
    }

    /// A gun to buy, drawn in chalk on a board on a wall. (`x`, `z`) is on
    /// the wall's face and the board faces `yaw`.
    pub(crate) fn wall_gun(&mut self, x: f32, z: f32, yaw: f32, gun: u8) {
        let rot = Quat::from_rotation_y(yaw);
        let pos = v(x, 0.0, z) + rot * v(0.0, 0.0, 0.22);
        let mut a = Art::default();
        boxr(&mut a.paint, v(-1.4, 0.75, -0.04), v(1.4, 2.25, 0.0), c(0.16, 0.2, 0.17));
        boxr(&mut a.paint, v(-1.5, 0.65, -0.05), v(1.5, 0.75, 0.04), c(0.45, 0.32, 0.2));
        boxr(&mut a.paint, v(-1.5, 2.25, -0.05), v(1.5, 2.35, 0.04), c(0.45, 0.32, 0.2));
        let chalk = c(0.95, 0.95, 0.85);
        for (p0, p1) in [
            (v(-1.25, 0.9, 0.01), v(1.25, 0.9, 0.01)),
            (v(-1.25, 2.1, 0.01), v(1.25, 2.1, 0.01)),
            (v(-1.25, 0.9, 0.01), v(-1.25, 2.1, 0.01)),
            (v(1.25, 0.9, 0.01), v(1.25, 2.1, 0.01)),
        ] {
            a.glow.beam(p0, p1, Vec2::new(0.04, 0.01), chalk);
        }
        a.metal.beam(
            v(0.0, 2.6, -0.04),
            v(0.0, 2.7, 0.35),
            Vec2::splat(0.05),
            c(0.2, 0.2, 0.2),
        );
        a.metal.cone(v(0.0, 2.65, 0.42), 0.18, 0.15, Quat::IDENTITY, c(0.2, 0.2, 0.2));
        a.glow.cyl(v(0.0, 2.57, 0.42), 0.12, 0.02, Quat::IDENTITY, c(1.0, 0.95, 0.8));
        self.place(a, pos, yaw);
        self.light(pos + rot * v(0.0, 2.4, 0.8), c(1.0, 0.92, 0.75), 30_000.0);
        self.wall_buys.push(crate::maps::WallBuy {
            pos,
            yaw,
            gun,
            attach: crate::data::Attach::NONE,
            owner: None,
        });
    }

    /// Puts perk machine `i` at (`x`, `z`), facing `yaw`.
    pub(crate) fn perk(&mut self, i: usize, x: f32, z: f32, yaw: f32) {
        self.perk_spots[i] = v(x, 0.0, z);
        self.perk_yaws[i] = yaw;
    }
}

/// One stretch of wall from `s` to `e` along it, between heights `y0` and
/// `y1`, with the finish's detail.
fn wall_art(a: &mut Art, w: &Wall, s: f32, e: f32, y0: f32, y1: f32) {
    let t = w.look.finish.thick();
    let line = w.line;
    let along_x = w.along_x;
    let p = |u: f32, y: f32, d: f32| {
        if along_x {
            v(u, y, line + d)
        } else {
            v(line + d, y, u)
        }
    };
    let col = w.look.color;
    let trim = w.look.trim;
    let k = &mut a.paint;
    // Horizontal lines (courses, boards) between y0 and y1, every `step`.
    let lines = |k: &mut crate::kit::Kit, step: f32, col: Color| {
        let mut y = step;
        while y < y1 - 0.1 {
            if y > y0 + 0.05 {
                boxr(k, p(s, y - 0.015, -t / 2.0 - 0.012), p(e, y + 0.015, t / 2.0 + 0.012), col);
            }
            y += step;
        }
    };
    match w.look.finish {
        Finish::Hedge => {
            boxr(k, p(s, y0, -t / 2.0), p(e, y1, t / 2.0), col);
            if y1 >= w.h - 0.01 {
                let n = ((e - s) / 1.1).ceil().max(1.0) as i32;
                for i in 0..n {
                    let u = s + (i as f32 + 0.5) * (e - s) / n as f32;
                    k.blob(
                        p(u, y1, 0.0),
                        v(0.7, 0.35, 0.7),
                        shade(col, 0.9 + hash(u, line) * 0.25),
                    );
                }
            }
            boxr(k, p(s, y0, -t / 2.0 - 0.02), p(e, y0 + 0.25, t / 2.0 + 0.02), shade(col, 0.6));
        }
        Finish::Fence => {
            let base = 1.0f32.min(y1);
            if y0 < base {
                boxr(k, p(s, y0, -0.2), p(e, base, 0.2), c(0.55, 0.55, 0.53));
            }
            if y1 > base {
                boxr(&mut a.glass, p(s, base.max(y0), -0.02), p(e, y1, 0.02), c(0.5, 0.52, 0.55));
                let mut u = s;
                while u <= e + 0.01 {
                    a.metal.cyl(
                        p(u, (base + y1) / 2.0, 0.0),
                        0.05,
                        y1 - base,
                        Quat::IDENTITY,
                        c(0.6, 0.6, 0.62),
                    );
                    u += 3.0;
                }
                a.metal.beam(p(s, y1 - 0.1, 0.0), p(e, y1 - 0.1, 0.0), Vec2::splat(0.06), c(0.6, 0.6, 0.62));
            }
        }
        Finish::Wood => {
            let mut u = s;
            let mut i = 0;
            while u < e - 0.01 {
                let u1 = (u + 0.2).min(e);
                let hh = y1 + (hash(u, line) - 0.5) * 0.08;
                boxr(
                    k,
                    p(u, y0, -0.03),
                    p(u1 - 0.02, hh, 0.03),
                    shade(col, 0.85 + 0.2 * hash(u, 1.0) + 0.05 * (i % 2) as f32),
                );
                u = u1;
                i += 1;
            }
            for y in [0.4f32, y1 - 0.4] {
                if y > y0 && y < y1 {
                    boxr(k, p(s, y - 0.06, -0.1), p(e, y + 0.06, 0.1), shade(col, 0.7));
                }
            }
        }
        Finish::Glass => {
            let base = 0.6f32.min(y1);
            if y0 < base {
                boxr(k, p(s, y0, -0.15), p(e, base, 0.15), c(0.6, 0.55, 0.5));
            }
            if y1 > base {
                boxr(&mut a.glass, p(s, base.max(y0), -0.03), p(e, y1, 0.03), c(0.7, 0.85, 0.8));
                let mut u = s;
                while u <= e + 0.01 {
                    boxr(&mut a.metal, p(u - 0.05, base, -0.06), p(u + 0.05, y1, 0.06), trim);
                    u += 1.5;
                }
                boxr(&mut a.metal, p(s, y1 - 0.1, -0.08), p(e, y1, 0.08), trim);
            }
        }
        _ => {
            boxr(k, p(s, y0, -t / 2.0), p(e, y1, t / 2.0), col);
            match w.look.finish {
                Finish::Brick => lines(k, 0.42, shade(col, 0.72)),
                Finish::Block => lines(k, 0.8, shade(col, 0.82)),
                Finish::Siding => lines(k, 0.32, shade(col, 0.82)),
                Finish::Stone => {
                    lines(k, 0.6, shade(col, 0.7));
                    let mut y = 0.0;
                    let mut row = 0;
                    while y < y1 - 0.3 {
                        if y >= y0 {
                            let mut u = s + 0.5 + 0.6 * (row % 2) as f32;
                            while u < e - 0.2 {
                                boxr(
                                    k,
                                    p(u - 0.015, y, -t / 2.0 - 0.012),
                                    p(u + 0.015, y + 0.6, t / 2.0 + 0.012),
                                    shade(col, 0.7),
                                );
                                u += 1.2;
                            }
                        }
                        y += 0.6;
                        row += 1;
                    }
                }
                Finish::Metal => {
                    let mut u = s + 0.2;
                    while u < e - 0.1 {
                        boxr(k, p(u - 0.06, y0, -t / 2.0 - 0.04), p(u + 0.06, y1, t / 2.0 + 0.04), shade(col, 0.8));
                        u += 0.7;
                    }
                }
                Finish::Plaster => {
                    if y0 < 1.1 {
                        boxr(k, p(s, y0, -t / 2.0 - 0.015), p(e, 1.1f32.min(y1), t / 2.0 + 0.015), trim);
                        if y1 > 1.15 {
                            boxr(k, p(s, 1.08, -t / 2.0 - 0.03), p(e, 1.16, t / 2.0 + 0.03), shade(trim, 0.7));
                        }
                    }
                }
                _ => {}
            }
            // Skirting and a cap along the top.
            if y0 < 0.01 {
                boxr(k, p(s, 0.0, -t / 2.0 - 0.03), p(e, 0.22, t / 2.0 + 0.03), shade(col, 0.55));
            }
            if y1 >= w.h - 0.01 {
                boxr(k, p(s, y1 - 0.15, -t / 2.0 - 0.06), p(e, y1, t / 2.0 + 0.06), trim);
            }
        }
    }
}

/// Glass, a frame and a sill in a window from `s` to `e`.
fn window_art(a: &mut Art, w: &Wall, s: f32, e: f32) {
    let (line, along_x) = (w.line, w.along_x);
    let p = |u: f32, y: f32, d: f32| {
        if along_x {
            v(u, y, line + d)
        } else {
            v(line + d, y, u)
        }
    };
    let t = w.look.finish.thick();
    let frame = shade(w.look.trim, 0.9);
    boxr(&mut a.glass, p(s, SILL, -0.02), p(e, WIN_TOP, 0.02), c(0.45, 0.55, 0.62));
    boxr(&mut a.paint, p(s - 0.05, SILL - 0.08, -t / 2.0 - 0.1), p(e + 0.05, SILL, t / 2.0 + 0.1), frame);
    for u in [s, e] {
        boxr(&mut a.paint, p(u - 0.06, SILL, -t / 2.0 - 0.02), p(u + 0.06, WIN_TOP, t / 2.0 + 0.02), frame);
    }
    boxr(&mut a.paint, p(s - 0.06, WIN_TOP - 0.06, -t / 2.0 - 0.02), p(e + 0.06, WIN_TOP, t / 2.0 + 0.02), frame);
    // Bars or glazing bars.
    let n = ((e - s) / 0.5) as i32;
    for i in 1..n {
        let u = s + i as f32 * (e - s) / n as f32;
        a.metal.cyl(
            p(u, (SILL + WIN_TOP) / 2.0, 0.0),
            0.025,
            WIN_TOP - SILL,
            Quat::IDENTITY,
            c(0.2, 0.2, 0.22),
        );
    }
}

/// Posts and a header round a doorway from `s` to `e`, `top` high.
fn frame_art(a: &mut Art, w: &Wall, s: f32, e: f32, top: f32) {
    let (line, along_x) = (w.line, w.along_x);
    let p = |u: f32, y: f32, d: f32| {
        if along_x {
            v(u, y, line + d)
        } else {
            v(line + d, y, u)
        }
    };
    let t = w.look.finish.thick();
    let frame = shade(w.look.trim, 0.8);
    for u in [s, e] {
        boxr(&mut a.paint, p(u - 0.12, 0.0, -t / 2.0 - 0.06), p(u + 0.12, top, t / 2.0 + 0.06), frame);
    }
    boxr(&mut a.paint, p(s - 0.12, top - 0.2, -t / 2.0 - 0.06), p(e + 0.12, top, t / 2.0 + 0.06), frame);
}

/// A room's rectangle and the spots to keep clear in it (doorways).
pub(crate) struct Room {
    pub x0: f32,
    pub x1: f32,
    pub z0: f32,
    pub z1: f32,
    /// Ceiling height.
    pub h: f32,
    pub keep_clear: Vec<Vec3>,
}

impl Room {
    pub fn new(x0: f32, z0: f32, x1: f32, z1: f32, h: f32, keep_clear: &[(f32, f32)]) -> Self {
        Self {
            x0,
            x1,
            z0,
            z1,
            h,
            keep_clear: keep_clear.iter().map(|(x, z)| v(*x, 0.0, *z)).collect(),
        }
    }

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

// ---------------------------------------------------------------------------
// Furnished rooms
// ---------------------------------------------------------------------------

impl MapLayout {

    pub(crate) fn library(&mut self, room: &Room, seed: f32) {
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

    pub(crate) fn museum(&mut self, room: &Room, seed: f32) {
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

    pub(crate) fn machine_shop(&mut self, room: &Room, seed: f32) {
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
                p + v(0.0, room.h, 0.0),
                p + v(0.0, room.h - 1.2, 0.0),
                0.01,
                c(0.1, 0.1, 0.1),
            );
            a.metal.cone(
                p + v(0.0, room.h - 1.3, 0.0),
                0.35,
                0.25,
                Quat::IDENTITY,
                green,
            );
            a.glow
                .sphere(p + v(0.0, room.h - 1.45, 0.0), 0.1, c(1.0, 0.95, 0.8));
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

    pub(crate) fn diner(&mut self, room: &Room, seed: f32) {
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
                v(0.0, room.h - 1.2, 0.0),
                v(0.06, 0.4, 2.5),
                c(1.0, 0.3, 0.55),
            );
            a.glow.cuboid(
                v(0.0, room.h - 1.65, 0.0),
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


// ---------------------------------------------------------------------------
// Furniture (modelled facing +Z, base on the ground)
// ---------------------------------------------------------------------------

impl MapLayout {
    /// Office desk with a monitor, papers and a chair.
    pub(crate) fn desk(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let wood = c(0.5, 0.36, 0.22);
        let grey = c(0.25, 0.26, 0.28);
        boxr(&mut a.paint, v(-0.8, 0.72, -0.4), v(0.8, 0.78, 0.4), wood);
        boxr(&mut a.paint, v(-0.78, 0.0, -0.38), v(-0.3, 0.72, 0.38), shade(wood, 0.8));
        boxr(&mut a.paint, v(0.74, 0.0, -0.38), v(0.78, 0.72, 0.38), shade(wood, 0.8));
        boxr(&mut a.metal, v(-0.3, 0.8, -0.3), v(0.3, 1.18, -0.26), grey);
        a.glow.cuboid(v(0.0, 0.99, -0.255), v(0.52, 0.32, 0.01), c(0.45, 0.75, 0.9));
        a.metal.cuboid(v(0.0, 0.79, -0.3), v(0.06, 0.04, 0.12), grey);
        a.paint.cuboid(v(0.0, 0.79, 0.05), v(0.45, 0.02, 0.15), c(0.15, 0.15, 0.15));
        a.paint.cuboid_rot(v(0.5, 0.79, 0.1), v(0.3, 0.02, 0.22), Quat::from_rotation_y(0.3), c(0.95, 0.95, 0.92));
        // Chair.
        boxr(&mut a.paint, v(-0.25, 0.45, 0.55), v(0.25, 0.52, 1.0), grey);
        boxr(&mut a.paint, v(-0.25, 0.52, 0.95), v(0.25, 1.1, 1.02), grey);
        a.metal.cyl(v(0.0, 0.22, 0.78), 0.04, 0.45, Quat::IDENTITY, c(0.6, 0.6, 0.62));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.4, 0.2), v(1.6, 0.8, 1.4));
    }

    /// A row of `n` steel lockers with a bench in front.
    pub(crate) fn lockers(&mut self, x: f32, z: f32, yaw: f32, n: usize, color: Color) {
        let mut a = Art::default();
        let w = 0.55;
        let len = n as f32 * w;
        for i in 0..n {
            let px = -len / 2.0 + (i as f32 + 0.5) * w;
            boxr(&mut a.metal, v(px - w / 2.0 + 0.01, 0.1, -0.25), v(px + w / 2.0 - 0.01, 2.0, 0.25), color);
            for y in [1.65f32, 1.75, 1.85] {
                a.paint.cuboid(v(px, y, 0.255), v(0.3, 0.02, 0.01), shade(color, 0.6));
            }
            a.metal.cuboid(v(px + 0.18, 1.1, 0.27), v(0.03, 0.15, 0.03), c(0.7, 0.7, 0.72));
            if hash(px, x + z) > 0.8 {
                // One left hanging open.
                a.metal.cuboid_rot(
                    v(px - w / 2.0 + 0.02, 1.05, 0.5),
                    v(0.02, 1.85, w - 0.04),
                    Quat::from_rotation_y(0.3),
                    shade(color, 1.1),
                );
            }
        }
        boxr(&mut a.paint, v(-len / 2.0, 0.0, -0.27), v(len / 2.0, 0.1, 0.27), c(0.2, 0.2, 0.2));
        boxr(&mut a.paint, v(-len / 2.0 + 0.2, 0.42, 0.75), v(len / 2.0 - 0.2, 0.48, 1.05), c(0.55, 0.4, 0.25));
        for px in [-len / 2.0 + 0.4, len / 2.0 - 0.4] {
            a.metal.cuboid(v(px, 0.21, 0.9), v(0.06, 0.42, 0.2), c(0.3, 0.3, 0.32));
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.0, 0.0), v(len, 2.0, 0.55));
    }

    /// Kitchen counter `len` long with a sink, a stove and cupboards over it.
    pub(crate) fn kitchen(&mut self, x: f32, z: f32, yaw: f32, len: f32, color: Color) {
        let mut a = Art::default();
        let top = c(0.85, 0.84, 0.8);
        let steel = c(0.72, 0.73, 0.75);
        boxr(&mut a.paint, v(-len / 2.0, 0.08, -0.3), v(len / 2.0, 0.88, 0.3), color);
        boxr(&mut a.paint, v(-len / 2.0, 0.0, -0.26), v(len / 2.0, 0.08, 0.26), c(0.15, 0.15, 0.15));
        boxr(&mut a.paint, v(-len / 2.0 - 0.02, 0.88, -0.32), v(len / 2.0 + 0.02, 0.93, 0.33), top);
        let mut u = -len / 2.0 + 0.3;
        while u < len / 2.0 - 0.1 {
            a.paint.cuboid(v(u, 0.48, 0.305), v(0.02, 0.7, 0.01), shade(color, 0.7));
            a.metal.cuboid(v(u + 0.12, 0.75, 0.32), v(0.12, 0.02, 0.03), steel);
            u += 0.6;
        }
        // Sink and stove.
        boxr(&mut a.metal, v(-len / 4.0 - 0.3, 0.86, -0.2), v(-len / 4.0 + 0.3, 0.94, 0.2), steel);
        a.metal.capsule_between(v(-len / 4.0, 0.93, -0.25), v(-len / 4.0, 1.2, -0.2), 0.02, steel);
        boxr(&mut a.metal, v(len / 4.0 - 0.35, 0.93, -0.25), v(len / 4.0 + 0.35, 0.95, 0.25), c(0.1, 0.1, 0.1));
        for (dx, dz) in [(-0.17f32, -0.1f32), (0.17, -0.1), (-0.17, 0.12), (0.17, 0.12)] {
            a.metal.torus(v(len / 4.0 + dx, 0.96, dz), 0.012, 0.08, Quat::IDENTITY, c(0.3, 0.3, 0.3));
        }
        // Wall cupboards.
        boxr(&mut a.paint, v(-len / 2.0, 1.5, -0.3), v(len / 2.0, 2.2, 0.02), color);
        boxr(&mut a.paint, v(-len / 2.0, 0.93, -0.32), v(len / 2.0, 1.5, -0.29), c(0.88, 0.88, 0.85));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.47, 0.0), v(len, 0.94, 0.66));
    }

    /// Tall fridge (or a drinks cooler when `glass`).
    pub(crate) fn fridge(&mut self, x: f32, z: f32, yaw: f32, glass: bool) {
        let mut a = Art::default();
        let white = c(0.9, 0.9, 0.88);
        boxr(&mut a.metal, v(-0.45, 0.0, -0.35), v(0.45, 2.0, 0.35), white);
        if glass {
            a.glow.cuboid(v(0.0, 1.05, 0.34), v(0.75, 1.6, 0.02), c(0.75, 0.9, 1.0));
            for y in [0.5f32, 0.9, 1.3, 1.7] {
                for i in 0..5 {
                    let col = [c(0.85, 0.1, 0.1), c(0.1, 0.5, 0.85), c(0.95, 0.75, 0.1)][(i + y as usize) % 3];
                    a.paint.cyl(v(-0.28 + i as f32 * 0.14, y + 0.12, 0.2), 0.04, 0.24, Quat::IDENTITY, col);
                }
            }
            a.glass.cuboid(v(0.0, 1.05, 0.36), v(0.78, 1.65, 0.02), c(0.7, 0.85, 0.9));
        } else {
            a.paint.cuboid(v(0.0, 1.35, 0.355), v(0.88, 0.02, 0.01), c(0.6, 0.6, 0.6));
            a.metal.cuboid(v(0.35, 1.6, 0.38), v(0.04, 0.4, 0.04), c(0.7, 0.7, 0.72));
            a.metal.cuboid(v(0.35, 0.9, 0.38), v(0.04, 0.4, 0.04), c(0.7, 0.7, 0.72));
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 1.0, z), v(0.9, 2.0, 0.9));
    }

    /// Table with four chairs.
    pub(crate) fn table_set(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        boxr(&mut a.paint, v(-0.8, 0.72, -0.55), v(0.8, 0.78, 0.55), color);
        for (lx, lz) in [(-0.7f32, -0.45f32), (0.7, -0.45), (-0.7, 0.45), (0.7, 0.45)] {
            a.metal.cuboid(v(lx, 0.36, lz), v(0.05, 0.72, 0.05), c(0.3, 0.3, 0.32));
        }
        let seat = c(0.6, 0.2, 0.15);
        for (cx, cz, r) in [(-0.4f32, -0.8f32, 0.0f32), (0.4, -0.8, 0.0), (-0.4, 0.8, PI), (0.4, 0.8, PI)] {
            let rot = Quat::from_rotation_y(r + (hash(cx, cz + x) - 0.5) * 0.5);
            let at = v(cx, 0.0, cz);
            a.paint.cuboid_rot(at + v(0.0, 0.45, 0.0), v(0.42, 0.05, 0.42), rot, seat);
            a.paint.cuboid_rot(at + rot * v(0.0, 0.75, -0.2), v(0.42, 0.55, 0.04), rot, seat);
            for (lx, lz) in [(-0.18f32, -0.18f32), (0.18, -0.18), (-0.18, 0.18), (0.18, 0.18)] {
                a.metal.cuboid_rot(at + rot * v(lx, 0.22, lz), v(0.03, 0.45, 0.03), rot, c(0.3, 0.3, 0.32));
            }
        }
        a.paint.cyl(v(0.2, 0.83, 0.1), 0.05, 0.1, Quat::IDENTITY, c(0.9, 0.9, 0.88));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.4, 0.0), v(1.6, 0.8, 1.1));
    }

    /// Sofa facing +Z.
    pub(crate) fn sofa(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        boxr(&mut a.paint, v(-1.1, 0.1, -0.45), v(1.1, 0.45, 0.45), color);
        boxr(&mut a.paint, v(-1.1, 0.45, -0.45), v(1.1, 0.95, -0.2), color);
        for s in [-1.0f32, 1.0] {
            boxr(&mut a.paint, v(s * 0.9, 0.1, -0.45), v(s * 1.1, 0.7, 0.45), shade(color, 0.9));
            a.paint.blob(v(s * 0.5, 0.55, 0.0), v(0.5, 0.12, 0.38), shade(color, 1.1));
        }
        boxr(&mut a.paint, v(-1.1, 0.0, -0.4), v(1.1, 0.1, 0.4), c(0.2, 0.15, 0.1));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.45, 0.0), v(2.2, 0.9, 0.9));
    }

    /// Low TV stand with a set on it, facing +Z.
    pub(crate) fn tv(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        boxr(&mut a.paint, v(-0.8, 0.0, -0.25), v(0.8, 0.5, 0.25), c(0.3, 0.2, 0.12));
        boxr(&mut a.metal, v(-0.6, 0.55, -0.05), v(0.6, 1.25, 0.02), c(0.08, 0.08, 0.09));
        a.glow.cuboid(v(0.0, 0.9, 0.025), v(1.1, 0.6, 0.01), c(0.35, 0.5, 0.6));
        a.metal.cuboid(v(0.0, 0.52, 0.0), v(0.3, 0.04, 0.15), c(0.08, 0.08, 0.09));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.5, 0.0), v(1.6, 1.0, 0.5));
    }

    /// Bed with a pillow and a blanket, head towards -Z.
    pub(crate) fn bed(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        let wood = c(0.42, 0.28, 0.16);
        boxr(&mut a.paint, v(-0.8, 0.0, -1.0), v(0.8, 0.45, 1.0), wood);
        boxr(&mut a.paint, v(-0.78, 0.45, -0.98), v(0.78, 0.6, 0.98), c(0.92, 0.92, 0.9));
        boxr(&mut a.paint, v(-0.8, 0.0, -1.06), v(0.8, 1.1, -0.98), wood);
        a.paint.blob(v(0.0, 0.66, -0.75), v(0.55, 0.1, 0.2), c(0.95, 0.95, 0.95));
        boxr(&mut a.paint, v(-0.82, 0.4, -0.3), v(0.82, 0.64, 1.0), color);
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.35, 0.0), v(1.6, 0.7, 2.1));
    }

    /// A church pew facing +Z.
    pub(crate) fn pew(&mut self, x: f32, z: f32, yaw: f32, len: f32) {
        let mut a = Art::default();
        let wood = c(0.38, 0.22, 0.12);
        boxr(&mut a.paint, v(-len / 2.0, 0.42, -0.25), v(len / 2.0, 0.48, 0.2), wood);
        boxr(&mut a.paint, v(-len / 2.0, 0.48, -0.3), v(len / 2.0, 1.0, -0.24), wood);
        for s in [-1.0f32, 1.0] {
            boxr(&mut a.paint, v(s * len / 2.0 - 0.05, 0.0, -0.3), v(s * len / 2.0 + 0.05, 1.05, 0.25), shade(wood, 0.8));
        }
        boxr(&mut a.paint, v(-len / 2.0, 0.6, -0.44), v(len / 2.0, 0.66, -0.3), shade(wood, 0.9));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.5, -0.05), v(len, 1.0, 0.6));
    }

    /// Altar table with candles, a cross and a rug, facing +Z.
    pub(crate) fn altar(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let stone = c(0.85, 0.83, 0.78);
        boxr(&mut a.paint, v(-2.5, 0.0, -1.5), v(2.5, 0.3, 1.0), c(0.55, 0.12, 0.12));
        boxr(&mut a.paint, v(-1.2, 0.3, -0.5), v(1.2, 1.1, 0.3), stone);
        boxr(&mut a.paint, v(-1.3, 1.1, -0.55), v(1.3, 1.18, 0.35), stone);
        a.paint.cuboid(v(0.0, 0.75, 0.31), v(0.8, 0.6, 0.02), c(0.75, 0.6, 0.2));
        for s in [-0.9f32, -0.6, 0.6, 0.9] {
            a.paint.cyl(v(s, 1.33, -0.1), 0.04, 0.3, Quat::IDENTITY, c(0.95, 0.93, 0.85));
            a.glow.sphere(v(s, 1.52, -0.1), 0.035, c(1.0, 0.75, 0.3));
        }
        a.metal.cuboid(v(0.0, 3.2, -1.3), v(0.15, 2.4, 0.12), c(0.8, 0.65, 0.25));
        a.metal.cuboid(v(0.0, 3.8, -1.3), v(1.2, 0.15, 0.12), c(0.8, 0.65, 0.25));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.6, -0.1), v(2.6, 1.2, 0.9));
        self.light(v(x, 1.8, z), c(1.0, 0.75, 0.4), 30_000.0);
    }

    /// Lab bench with sinks, glassware and a fume hood.
    pub(crate) fn lab_bench(&mut self, x: f32, z: f32, yaw: f32, seed: f32) {
        let mut a = Art::default();
        let white = c(0.88, 0.88, 0.86);
        boxr(&mut a.paint, v(-1.5, 0.0, -0.45), v(1.5, 0.88, 0.45), white);
        boxr(&mut a.paint, v(-1.55, 0.88, -0.5), v(1.55, 0.94, 0.5), c(0.12, 0.12, 0.13));
        let cols = [c(0.3, 1.0, 0.4), c(0.3, 0.7, 1.0), c(1.0, 0.4, 0.8), c(1.0, 0.8, 0.2)];
        for i in 0..6 {
            let px = -1.2 + i as f32 * 0.48;
            let pz = (hash(seed, i as f32) - 0.5) * 0.5;
            let col = cols[(hash(i as f32, seed) * 4.0) as usize % 4];
            a.glass.cyl(v(px, 1.06, pz), 0.07, 0.24, Quat::IDENTITY, c(0.8, 0.9, 0.95));
            a.glow.cyl(v(px, 1.0, pz), 0.06, 0.1, Quat::IDENTITY, col);
        }
        a.metal.cyl(v(0.0, 1.3, -0.35), 0.02, 0.7, Quat::IDENTITY, c(0.6, 0.6, 0.62));
        boxr(&mut a.metal, v(-0.3, 1.55, -0.45), v(0.3, 1.6, -0.25), c(0.6, 0.6, 0.62));
        for d in [-0.4f32, 0.4] {
            a.paint.cuboid(v(d, 0.45, 0.455), v(0.6, 0.8, 0.01), shade(white, 0.85));
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.47, 0.0), v(3.1, 0.94, 1.0));
    }

    /// Radio or control desk with glowing screens and dials.
    pub(crate) fn console(&mut self, x: f32, z: f32, yaw: f32, len: f32) {
        let mut a = Art::default();
        let grey = c(0.35, 0.38, 0.4);
        boxr(&mut a.metal, v(-len / 2.0, 0.0, -0.4), v(len / 2.0, 0.9, 0.35), grey);
        a.metal.cuboid_rot(v(0.0, 0.95, 0.15), v(len, 0.08, 0.5), Quat::from_rotation_x(0.25), shade(grey, 0.8));
        boxr(&mut a.metal, v(-len / 2.0, 0.9, -0.4), v(len / 2.0, 1.7, -0.2), grey);
        let mut u = -len / 2.0 + 0.4;
        let mut i = 0;
        while u < len / 2.0 - 0.2 {
            let col = [c(0.4, 1.0, 0.5), c(1.0, 0.7, 0.2), c(0.4, 0.8, 1.0)][i % 3];
            a.glow.cuboid(v(u, 1.35, -0.19), v(0.5, 0.35, 0.01), shade(col, 0.7));
            for k in 0..3 {
                a.glow.sphere(v(u - 0.15 + k as f32 * 0.15, 0.98, 0.25), 0.025, if (i + k) % 2 == 0 { c(1.0, 0.2, 0.2) } else { col });
            }
            u += 0.7;
            i += 1;
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.85, -0.02), v(len, 1.7, 0.8));
    }

    /// Shop or reception counter `len` long, its customer side facing +Z.
    pub(crate) fn counter(&mut self, x: f32, z: f32, yaw: f32, len: f32, color: Color) {
        let mut a = Art::default();
        boxr(&mut a.paint, v(-len / 2.0, 0.0, -0.35), v(len / 2.0, 1.05, 0.35), color);
        boxr(&mut a.paint, v(-len / 2.0 - 0.05, 1.05, -0.4), v(len / 2.0 + 0.05, 1.12, 0.42), c(0.8, 0.78, 0.72));
        boxr(&mut a.paint, v(-len / 2.0, 0.0, 0.33), v(len / 2.0, 0.12, 0.38), shade(color, 0.5));
        let mut u = -len / 2.0 + 0.5;
        while u < len / 2.0 {
            a.paint.cuboid(v(u, 0.6, 0.36), v(0.04, 0.8, 0.02), shade(color, 0.8));
            u += 1.0;
        }
        boxr(&mut a.metal, v(len / 2.0 - 0.8, 1.12, -0.2), v(len / 2.0 - 0.3, 1.35, 0.1), c(0.15, 0.15, 0.16));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.55, 0.0), v(len, 1.1, 0.8));
    }

    /// Vending machine, front facing +Z.
    pub(crate) fn vending(&mut self, x: f32, z: f32, yaw: f32, color: Color) {
        let mut a = Art::default();
        boxr(&mut a.metal, v(-0.5, 0.0, -0.4), v(0.5, 1.9, 0.4), color);
        a.glow.cuboid(v(-0.1, 1.15, 0.405), v(0.65, 1.2, 0.01), c(0.95, 0.95, 0.85));
        for row in 0..4 {
            for i in 0..4 {
                let col = [c(0.9, 0.2, 0.2), c(0.2, 0.5, 0.9), c(0.95, 0.75, 0.15), c(0.3, 0.8, 0.3)][(row + i) % 4];
                a.paint.cuboid(v(-0.35 + i as f32 * 0.17, 0.7 + row as f32 * 0.28, 0.41), v(0.1, 0.16, 0.02), col);
            }
        }
        a.glow.cuboid(v(0.36, 1.3, 0.405), v(0.12, 0.3, 0.01), c(0.4, 1.0, 0.5));
        boxr(&mut a.paint, v(-0.4, 0.15, 0.4), v(0.2, 0.4, 0.43), c(0.1, 0.1, 0.1));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide(v(x, 0.95, z), v(1.0, 1.9, 1.0));
    }

    /// Roller conveyor `len` long with parcels on it.
    pub(crate) fn conveyor(&mut self, x: f32, z: f32, yaw: f32, len: f32) {
        let mut a = Art::default();
        let frame = c(0.25, 0.4, 0.6);
        for s in [-0.45f32, 0.45] {
            boxr(&mut a.metal, v(-len / 2.0, 0.75, s - 0.05), v(len / 2.0, 0.9, s + 0.05), frame);
        }
        let mut u = -len / 2.0 + 0.1;
        while u < len / 2.0 {
            a.metal.cyl(v(u, 0.86, 0.0), 0.04, 0.85, Quat::from_rotation_x(FRAC_PI_2), c(0.7, 0.7, 0.72));
            u += 0.2;
        }
        let mut u = -len / 2.0 + 0.3;
        while u < len / 2.0 {
            a.metal.cuboid(v(u, 0.4, 0.0), v(0.08, 0.8, 0.9), frame);
            u += 1.5;
        }
        let mut u = -len / 2.0 + 0.6;
        while u < len / 2.0 - 0.4 {
            if hash(u, x + z) > 0.4 {
                let s = 0.35 + hash(z, u) * 0.25;
                a.paint.cuboid_rot(
                    v(u, 0.92 + s / 2.0, 0.0),
                    v(s * 1.2, s, s),
                    Quat::from_rotation_y(hash(u, 3.0) - 0.5),
                    c(0.62, 0.48, 0.3),
                );
            }
            u += 1.1;
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.5, 0.0), v(len, 1.0, 1.0));
    }

    /// Filing cabinets against a wall.
    pub(crate) fn cabinets(&mut self, x: f32, z: f32, yaw: f32, n: usize) {
        let mut a = Art::default();
        let grey = c(0.55, 0.58, 0.55);
        let len = n as f32 * 0.5;
        for i in 0..n {
            let px = -len / 2.0 + 0.25 + i as f32 * 0.5;
            boxr(&mut a.metal, v(px - 0.24, 0.0, -0.3), v(px + 0.24, 1.35, 0.3), grey);
            for d in 0..4 {
                let y = 0.18 + d as f32 * 0.32;
                a.metal.cuboid(v(px, y, 0.305), v(0.4, 0.26, 0.01), shade(grey, 0.9));
                a.metal.cuboid(v(px, y + 0.06, 0.32), v(0.12, 0.03, 0.03), c(0.75, 0.75, 0.75));
            }
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.7, 0.0), v(len, 1.4, 0.6));
    }

    /// Big diesel generator with a glowing panel and pipes.
    pub(crate) fn generator(&mut self, x: f32, z: f32, yaw: f32) {
        let mut a = Art::default();
        let green = c(0.25, 0.4, 0.28);
        boxr(&mut a.metal, v(-2.0, 0.0, -0.9), v(2.0, 0.3, 0.9), c(0.2, 0.2, 0.22));
        boxr(&mut a.metal, v(-1.8, 0.3, -0.8), v(1.0, 2.0, 0.8), green);
        a.metal.cyl(v(1.5, 1.1, 0.0), 0.7, 1.0, Quat::from_rotation_z(FRAC_PI_2), c(0.5, 0.5, 0.52));
        a.metal.cyl(v(-1.2, 2.6, 0.3), 0.15, 1.2, Quat::IDENTITY, c(0.3, 0.3, 0.3));
        let mut u = -1.6;
        while u < 0.9 {
            a.metal.cuboid(v(u, 1.2, 0.81), v(0.05, 1.3, 0.02), shade(green, 0.7));
            u += 0.2;
        }
        a.glow.cuboid(v(0.6, 1.5, 0.82), v(0.4, 0.3, 0.01), c(1.0, 0.6, 0.15));
        a.paint.cuboid(v(-0.5, 2.05, 0.0), v(1.2, 0.1, 1.0), c(0.85, 0.65, 0.1));
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 1.0, 0.0), v(4.0, 2.0, 1.8));
    }

    /// Gravestones in a little plot.
    pub(crate) fn graves(&mut self, x: f32, z: f32, yaw: f32, seed: f32) {
        let mut a = Art::default();
        let stone = c(0.55, 0.56, 0.55);
        for i in 0..3 {
            let px = -1.6 + i as f32 * 1.6;
            let tilt = Quat::from_rotation_z((hash(seed, i as f32) - 0.5) * 0.25);
            let s = shade(stone, 0.8 + hash(i as f32, seed) * 0.3);
            match (hash(seed + i as f32, 2.0) * 3.0) as i32 {
                0 => {
                    a.paint.cuboid_rot(v(px, 0.5, 0.0), v(0.6, 1.0, 0.15), tilt, s);
                    a.paint.cyl(v(px, 1.0, 0.0), 0.3, 0.15, Quat::from_rotation_x(FRAC_PI_2), s);
                }
                1 => {
                    a.paint.cuboid_rot(v(px, 0.7, 0.0), v(0.15, 1.4, 0.15), tilt, s);
                    a.paint.cuboid_rot(v(px, 1.05, 0.0), v(0.7, 0.15, 0.15), tilt, s);
                }
                _ => a.paint.cuboid_rot(v(px, 0.4, 0.0), v(0.8, 0.8, 0.2), tilt, s),
            }
            a.paint.cuboid(v(px, 0.03, 0.9), v(0.8, 0.06, 1.6), c(0.3, 0.25, 0.18));
        }
        self.place(a, v(x, 0.0, z), yaw);
        self.collide_local(v(x, 0.0, z), yaw, v(0.0, 0.6, 0.0), v(4.0, 1.2, 0.4));
    }

    /// Iron-barred cage (an old zoo enclosure) `w` by `d`, open at the back.
    pub(crate) fn cage(&mut self, x: f32, z: f32, w: f32, d: f32) {
        let mut a = Art::default();
        let iron = c(0.12, 0.12, 0.12);
        boxr(&mut a.paint, v(-w / 2.0, 0.0, -d / 2.0), v(w / 2.0, 0.3, d / 2.0), c(0.5, 0.48, 0.45));
        boxr(&mut a.paint, v(-w / 2.0, 0.3, -d / 2.0), v(w / 2.0, 0.32, d / 2.0), c(0.45, 0.4, 0.25));
        let mut u = -w / 2.0;
        while u <= w / 2.0 + 0.01 {
            a.metal.cyl(v(u, 1.8, d / 2.0), 0.03, 3.0, Quat::IDENTITY, iron);
            u += 0.25;
        }
        let mut u = -d / 2.0;
        while u <= d / 2.0 + 0.01 {
            for s in [-1.0f32, 1.0] {
                a.metal.cyl(v(s * w / 2.0, 1.8, u), 0.03, 3.0, Quat::IDENTITY, iron);
            }
            u += 0.25;
        }
        for y in [0.4f32, 3.25] {
            a.metal.beam(v(-w / 2.0, y, d / 2.0), v(w / 2.0, y, d / 2.0), Vec2::splat(0.06), iron);
        }
        a.paint.blob(v(w * 0.2, 0.5, -d * 0.1), v(0.8, 0.4, 0.6), c(0.5, 0.45, 0.35));
        boxr(&mut a.paint, v(-w / 2.0, 0.0, -d / 2.0), v(w / 2.0, 3.3, -d / 2.0 + 0.2), c(0.55, 0.5, 0.45));
        self.place(a, v(x, 0.0, z), 0.0);
        self.collide(v(x, 1.65, z), v(w, 3.3, d));
    }
}
