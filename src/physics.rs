//! Tiny hand-rolled collision and ray-cast helpers. The level is made only of
//! axis-aligned boxes, players are vertical circles and enemies are capsules.

use bevy::prelude::*;

use crate::Collider;

pub type Boxes = Vec<(Vec3, Vec3)>;

pub fn collect_boxes<'a>(iter: impl Iterator<Item = (&'a Transform, &'a Collider)>) -> Boxes {
    iter.map(|(t, c)| (t.translation, c.half)).collect()
}

/// Pushes a circle (in XZ) out of every box it overlaps, ignoring boxes whose
/// top is below `feet_y` (so you can stand on crates) and boxes overhead
/// (crane beams, roofs).
pub fn resolve_collisions(pos: &mut Vec3, radius: f32, feet_y: f32, colliders: &Boxes) {
    for (center, half) in colliders {
        let top = center.y + half.y;
        let bottom = center.y - half.y;
        if feet_y >= top - 0.05 || bottom > feet_y + 2.2 {
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

/// Enemies are person-shaped with their origin at the feet. Returns the hit
/// distance and whether it was a headshot.
pub fn ray_enemy(origin: Vec3, dir: Vec3, feet: Vec3, scale: f32) -> Option<(f32, bool)> {
    let head = ray_sphere(origin, dir, feet + Vec3::Y * 1.78 * scale, 0.24 * scale);
    let body = [(1.3, 0.36), (0.95, 0.34), (0.45, 0.3)]
        .iter()
        .filter_map(|(y, r)| ray_sphere(origin, dir, feet + Vec3::Y * y * scale, r * scale))
        .min_by(|a, b| a.total_cmp(b));
    match (head, body) {
        (Some(h), Some(b)) if b < h => Some((b, false)),
        (Some(h), _) => Some((h, true)),
        (None, Some(b)) => Some((b, false)),
        (None, None) => None,
    }
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

/// Clear line between two points?
pub fn line_of_sight(a: Vec3, b: Vec3, colliders: &Boxes) -> bool {
    let d = b - a;
    let len = d.length();
    if len < 1e-3 {
        return true;
    }
    ray_world(a, d / len, len, colliders) >= len - 0.05
}

pub struct ShotHit {
    pub dist: f32,
    pub enemy: Option<(Entity, bool)>,
}

/// Finds the closest enemy hit before any wall.
pub fn trace_shot(
    origin: Vec3,
    dir: Vec3,
    max: f32,
    colliders: &Boxes,
    enemies: impl Iterator<Item = (Entity, Vec3, f32)>,
) -> ShotHit {
    let wall = ray_world(origin, dir, max, colliders);
    let mut best = ShotHit {
        dist: wall,
        enemy: None,
    };
    for (e, feet, scale) in enemies {
        if let Some((t, head)) = ray_enemy(origin, dir, feet, scale) {
            if t < best.dist {
                best = ShotHit {
                    dist: t,
                    enemy: Some((e, head)),
                };
            }
        }
    }
    best
}
