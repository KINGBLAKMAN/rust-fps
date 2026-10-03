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
