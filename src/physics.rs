//! Tiny hand-rolled collision and ray-cast helpers. The level is made only of
//! axis-aligned boxes, players are vertical circles and enemies are capsules.

use bevy::prelude::*;

use crate::Collider;

pub type Boxes = Vec<(Vec3, Vec3)>;

pub fn collect_boxes<'a>(iter: impl Iterator<Item = (&'a Transform, &'a Collider)>) -> Boxes {
    iter.map(|(t, c)| (t.translation, c.half)).collect()
}

/// Pushes a circle (in XZ) out of every box it overlaps, ignoring boxes whose
/// top is below `feet_y` (so you can stand on crates).
pub fn resolve_collisions(pos: &mut Vec3, radius: f32, feet_y: f32, colliders: &Boxes) {
    for (center, half) in colliders {
        let top = center.y + half.y;
        if feet_y >= top - 0.05 {
            continue;
        }
        let closest = Vec2::new(
            pos.x.clamp(center.x - half.x, center.x + half.x),
            pos.z.clamp(center.z - half.z, center.z + half.z),
        );
        let p = Vec2::new(pos.x, pos.z);
        let diff = p - closest;
        let dist = diff.length();
        if dist < radius {
            let push = if dist > 1e-4 {
                diff / dist * (radius - dist)
            } else {
                // Center is inside the box: push out along the shallowest axis.
                let dx = half.x - (pos.x - center.x).abs();
                let dz = half.z - (pos.z - center.z).abs();
                if dx < dz {
                    Vec2::new((dx + radius) * (pos.x - center.x).signum(), 0.0)
                } else {
                    Vec2::new(0.0, (dz + radius) * (pos.z - center.z).signum())
                }
            };
            pos.x += push.x;
            pos.z += push.y;
        }
    }
}

/// Highest box top under the circle that the feet are above (0 = floor).
pub fn ground_height(pos: Vec3, radius: f32, feet_y: f32, colliders: &Boxes) -> f32 {
    let mut ground: f32 = 0.0;
    for (center, half) in colliders {
        let top = center.y + half.y;
        let inside_x = (pos.x - center.x).abs() < half.x + radius * 0.7;
        let inside_z = (pos.z - center.z).abs() < half.z + radius * 0.7;
        if inside_x && inside_z && feet_y >= top - 0.3 {
            ground = ground.max(top);
        }
    }
    ground
}

/// Ray vs axis-aligned box (slab method). Returns hit distance.
pub fn ray_aabb(origin: Vec3, dir: Vec3, center: Vec3, half: Vec3) -> Option<f32> {
    let min = center - half;
    let max = center + half;
    let inv = dir.recip();
    let t1 = (min - origin) * inv;
    let t2 = (max - origin) * inv;
    let tmin = t1.min(t2).max_element();
    let tmax = t1.max(t2).min_element();
    (tmax >= tmin.max(0.0)).then_some(tmin.max(0.0))
}

/// Ray vs sphere. Returns hit distance.
pub fn ray_sphere(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = origin - center;
    let b = oc.dot(dir);
    let c = oc.length_squared() - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    (t >= 0.0).then_some(t)
}

/// Enemies are capsules centered 1m above the floor; approximate with spheres.
pub fn ray_enemy(origin: Vec3, dir: Vec3, enemy_center: Vec3) -> Option<f32> {
    [0.55, 1.0, 1.45]
        .iter()
        .filter_map(|y| ray_sphere(origin, dir, enemy_center + Vec3::Y * (y - 1.0), 0.52))
        .min_by(|a, b| a.total_cmp(b))
}

/// Distance to the first wall or floor hit along the ray.
pub fn ray_world(origin: Vec3, dir: Vec3, max: f32, colliders: &Boxes) -> f32 {
    let mut nearest = max;
    for (center, half) in colliders {
        if let Some(t) = ray_aabb(origin, dir, *center, *half) {
            nearest = nearest.min(t);
        }
    }
    if dir.y < 0.0 {
        nearest = nearest.min(-origin.y / dir.y);
    }
    nearest
}

/// Finds the closest enemy hit before any wall. Returns (distance, enemy).
pub fn trace_shot(
    origin: Vec3,
    dir: Vec3,
    max: f32,
    colliders: &Boxes,
    enemies: impl Iterator<Item = (Entity, Vec3)>,
) -> (f32, Option<Entity>) {
    let wall = ray_world(origin, dir, max, colliders);
    let mut best: (f32, Option<Entity>) = (wall, None);
    for (e, center) in enemies {
        if let Some(t) = ray_enemy(origin, dir, center) {
            if t < best.0 {
                best = (t, Some(e));
            }
        }
    }
    best
}
