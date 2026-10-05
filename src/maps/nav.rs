//! Enemy pathfinding: a 1m grid over the map and a flow field that points
//! from every cell towards the nearest living player, rebuilt a few times a
//! second. Enemies just follow the arrows, which gets them around containers,
//! houses and fences.

use bevy::prelude::*;
use std::collections::VecDeque;

use crate::physics::Boxes;

const CELL: f32 = 1.0;
const UNREACHED: u16 = u16::MAX;

#[derive(Resource)]
pub struct NavGrid {
    half: f32,
    n: i32,
    blocked: Vec<bool>,
    dist: Vec<u16>,
    timer: f32,
}

impl NavGrid {
    pub fn new(half: f32) -> Self {
        let n = (half * 2.0 / CELL).ceil() as i32;
        Self {
            half,
            n,
            blocked: vec![false; (n * n) as usize],
            dist: vec![UNREACHED; (n * n) as usize],
            timer: 0.0,
        }
    }

    fn cell(&self, p: Vec3) -> Option<(i32, i32)> {
        let x = ((p.x + self.half) / CELL).floor() as i32;
        let z = ((p.z + self.half) / CELL).floor() as i32;
        (x >= 0 && z >= 0 && x < self.n && z < self.n).then_some((x, z))
    }

    fn idx(&self, x: i32, z: i32) -> usize {
        (z * self.n + x) as usize
    }

    fn center(&self, x: i32, z: i32) -> Vec3 {
        Vec3::new(
            (x as f32 + 0.5) * CELL - self.half,
            0.0,
            (z as f32 + 0.5) * CELL - self.half,
        )
    }

    fn free(&self, x: i32, z: i32) -> bool {
        x >= 0 && z >= 0 && x < self.n && z < self.n && !self.blocked[self.idx(x, z)]
    }

    /// Marks cells covered by ground-level obstacles (inflated a little so
    /// enemies don't scrape corners).
    fn rebuild_obstacles(&mut self, boxes: &Boxes) {
        self.blocked.iter_mut().for_each(|b| *b = false);
        let pad = 0.45;
        for (c, h) in boxes {
            let bottom = c.y - h.y;
            let top = c.y + h.y;
            if bottom > 1.6 || top < 0.3 {
                continue;
            }
            let x0 = ((c.x - h.x - pad + self.half) / CELL).floor() as i32;
            let x1 = ((c.x + h.x + pad + self.half) / CELL).floor() as i32;
            let z0 = ((c.z - h.z - pad + self.half) / CELL).floor() as i32;
            let z1 = ((c.z + h.z + pad + self.half) / CELL).floor() as i32;
            for z in z0.max(0)..=z1.min(self.n - 1) {
                for x in x0.max(0)..=x1.min(self.n - 1) {
                    let i = self.idx(x, z);
                    self.blocked[i] = true;
                }
            }
        }
    }

    /// Breadth-first search outwards from every target at once.
    fn rebuild_field(&mut self, targets: &[Vec3]) {
        self.dist.iter_mut().for_each(|d| *d = UNREACHED);
        let mut queue = VecDeque::new();
        for t in targets {
            if let Some((x, z)) = self.cell(*t) {
                let i = self.idx(x, z);
                self.dist[i] = 0;
                queue.push_back((x, z));
            }
        }
        while let Some((x, z)) = queue.pop_front() {
            let d = self.dist[self.idx(x, z)];
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, nz) = (x + dx, z + dz);
                if !self.free(nx, nz) {
                    continue;
                }
                let ni = self.idx(nx, nz);
                if self.dist[ni] == UNREACHED {
                    self.dist[ni] = d + 1;
                    queue.push_back((nx, nz));
                }
            }
        }
    }

    pub fn update(&mut self, dt: f32, boxes: &Boxes, targets: &[Vec3]) {
        self.timer -= dt;
        if self.timer > 0.0 {
            return;
        }
        self.timer = 0.25;
        self.rebuild_obstacles(boxes);
        self.rebuild_field(targets);
    }

    /// Which way to walk from `pos` to get closer to a player. None when
    /// there's no path (then walk straight at the target).
    pub fn direction(&self, pos: Vec3) -> Option<Vec3> {
        let (x, z) = self.cell(pos)?;
        let here = self.dist[self.idx(x, z)];
        let mut best: Option<((i32, i32), u16)> = None;
        for dz in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let (nx, nz) = (x + dx, z + dz);
                if !self.free(nx, nz) {
                    continue;
                }
                // No cutting diagonally past a corner.
                if dx != 0 && dz != 0 && (!self.free(x + dx, z) || !self.free(x, z + dz)) {
                    continue;
                }
                let d = self.dist[self.idx(nx, nz)];
                if d == UNREACHED {
                    continue;
                }
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some(((nx, nz), d));
                }
            }
        }
        let ((bx, bz), bd) = best?;
        if here != UNREACHED && bd >= here {
            return None;
        }
        let to = self.center(bx, bz) - pos.with_y(0.0);
        Some(to.normalize_or_zero())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::{layout, MapLayout, BOX_HALF};

    /// The map's colliders.
    fn boxes(m: &MapLayout) -> Boxes {
        let mut b: Boxes = m.solids.iter().map(|s| (s.pos, s.size / 2.0)).collect();
        for p in &m.perk_spots {
            b.push((*p + Vec3::Y * 1.2, Vec3::new(0.6, 1.2, 0.6)));
        }
        for p in &m.box_spots {
            b.push((*p + Vec3::Y * BOX_HALF.y, BOX_HALF));
        }
        b
    }

    fn field(m: &MapLayout) -> NavGrid {
        let mut g = NavGrid::new(m.half);
        g.rebuild_obstacles(&boxes(m));
        g.rebuild_field(&[m.player_spawns[0]]);
        g
    }

    /// Can you walk from the first player spawn to near `p`?
    fn reach(g: &NavGrid, p: Vec3, r: i32) -> bool {
        let Some((x, z)) = g.cell(p) else {
            return false;
        };
        (-r..=r).any(|dz| {
            (-r..=r).any(|dx| g.free(x + dx, z + dz) && g.dist[g.idx(x + dx, z + dz)] != UNREACHED)
        })
    }

    /// Prints each map's walkable cells (north at the top). Run with
    /// `cargo test map_plans -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn map_plans() {
        for map in 0..3u8 {
            let m = layout(map);
            let g = field(&m);
            let mut rows: Vec<Vec<char>> = (0..g.n)
                .map(|z| {
                    (0..g.n)
                        .map(|x| {
                            let i = g.idx(x, z);
                            if g.blocked[i] {
                                '#'
                            } else if g.dist[i] != UNREACHED {
                                '.'
                            } else {
                                ' '
                            }
                        })
                        .collect()
                })
                .collect();
            let mut mark = |p: Vec3, ch: char| {
                if let Some((x, z)) = g.cell(p) {
                    rows[z as usize][x as usize] = ch;
                }
            };
            for d in &m.doors {
                mark(d.pos, 'D');
            }
            for (s, zone) in &m.enemy_spawns {
                mark(*s, char::from_digit(*zone as u32, 10).unwrap_or('?'));
            }
            for p in &m.perk_spots {
                mark(*p, 'P');
            }
            for p in &m.box_spots {
                mark(*p, 'B');
            }
            for w in &m.wall_buys {
                mark(w.stand(), 'W');
            }
            mark(m.player_spawns[0], 'S');
            mark(m.extraction, 'E');
            println!("map {map}:");
            for row in rows.iter().rev() {
                println!("{}", row.iter().collect::<String>());
            }
        }
    }

    #[test]
    fn maps_connect() {
        let mut errors = Vec::new();
        let mut check = |ok: bool, what: String| {
            if !ok {
                errors.push(what);
            }
        };
        for map in 0..3u8 {
            let m = layout(map);
            println!(
                "map {map}: {} solids, {} lights, {} doorways, {} wall buys, {} spawns",
                m.solids.len(),
                m.lights.len(),
                m.doors.len(),
                m.wall_buys.len(),
                m.enemy_spawns.len(),
            );
            assert!(m.wall_buys.len() >= 2);
            let g = field(&m);
            for (i, s) in m.player_spawns.iter().enumerate() {
                check(reach(&g, *s, 0), format!("map {map} player spawn {i} blocked"));
            }
            for (s, zone) in &m.enemy_spawns {
                check(reach(&g, *s, 0), format!("map {map} zone {zone} spawn {s} unreachable"));
            }
            for (i, p) in m.perk_spots.iter().enumerate() {
                check(reach(&g, *p, 2), format!("map {map} perk {i} {p} unreachable"));
            }
            for (i, p) in m.box_spots.iter().enumerate() {
                check(reach(&g, *p, 2), format!("map {map} box {i} {p} unreachable"));
            }
            for (i, w) in m.wall_buys.iter().enumerate() {
                check(reach(&g, w.stand(), 1), format!("map {map} wall buy {i} {} unreachable", w.pos));
            }
            check(reach(&g, m.extraction, 1), format!("map {map} extraction unreachable"));
            for d in &m.doors {
                let side = if d.along_x { Vec3::Z } else { Vec3::X };
                for s in [-2.0f32, 2.0] {
                    check(reach(&g, d.pos + side * s, 0), format!("map {map} doorway {} side {s} blocked", d.pos));
                }
            }
        }
        assert!(errors.is_empty(), "{}", errors.join("\n"));
    }
}
