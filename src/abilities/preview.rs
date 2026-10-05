//! Shows where an ability will land while you aim it.

use bevy::prelude::*;
use std::f32::consts::TAU;

use super::{move_time, CastState, DASH_SPEED, GRAPPLE_RANGE, GRENADE_GRAVITY, GRENADE_LIFT, GRENADE_SPEED};
use crate::data::Ability;
use crate::markers::{Marker, Markers, Shape, PREVIEW};
use crate::physics::{collect_boxes, line_of_sight, ray_world, Boxes};
use crate::player::LocalPlayer;
use crate::sim::powers::{aim_ground, PLAGUE_MAX, PLAGUE_START};
use crate::{Collider, Enemy, Roster, Session};

fn flat_circle(markers: &mut Markers, center: Vec3, radius: f32, color: Color) {
    markers.push(Marker::new(center, Shape::Circle { radius }, color));
}

fn ring(markers: &mut Markers, center: Vec3, inner: f32, outer: f32, color: Color) {
    markers.push(Marker::new(center, Shape::Donut { inner, outer }, color));
}

/// A strip along the ground.
fn arrow(markers: &mut Markers, from: Vec3, dir: Vec3, len: f32, width: f32, color: Color) {
    markers.push(
        Marker::new(
            from,
            Shape::Line {
                length: len,
                half_width: width,
            },
            color,
        )
        .facing(dir),
    );
}

/// A wedge on the ground in front of `feet`, as wide as a host cone test
/// with this `cos`.
fn cone(markers: &mut Markers, feet: Vec3, flat: Vec3, r: f32, cos: f32, color: Color) {
    markers.push(
        Marker::new(
            feet,
            Shape::Cone {
                radius: r,
                half_angle: cos.acos(),
            },
            color,
        )
        .facing(flat),
    );
}

/// A target on the ground with a line up to the sky.
fn sky_target(markers: &mut Markers, gizmos: &mut Gizmos, at: Vec3, r: f32, color: Color) {
    flat_circle(markers, at, r, color);
    ring(markers, at, r * 0.42, r * 0.5, color);
    gizmos.line(at, at + Vec3::Y * 25.0, color.with_alpha(0.5));
}

/// The arc a thrown gadget flies, and where it lands.
fn throw_arc(gizmos: &mut Gizmos, origin: Vec3, forward: Vec3, speed: f32, boxes: &Boxes, color: Color) -> Vec3 {
    let mut pos = origin + forward * 0.6;
    let mut vel = forward * speed + Vec3::Y * GRENADE_LIFT;
    let mut pts = vec![pos];
    let step = 0.025;
    for _ in 0..200 {
        vel.y -= GRENADE_GRAVITY * step;
        let next = pos + vel * step;
        if next.y < 0.1 || !line_of_sight(pos, next, boxes) {
            break;
        }
        pos = next;
        pts.push(pos);
    }
    gizmos.linestrip(pts, color);
    pos.with_y(0.0)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_previews(
    time: Res<Time>,
    cast: Res<CastState>,
    session: Res<Session>,
    roster: Res<Roster>,
    player: Single<(&Transform, &LocalPlayer)>,
    colliders: Query<(&Transform, &Collider)>,
    enemies: Query<&Transform, With<Enemy>>,
    mut markers: ResMut<Markers>,
    mut gizmos: Gizmos,
) {
    let Some(slot) = cast.aiming else { return };
    let Some(me) = roster.me(&session) else {
        return;
    };
    let ability = me.kit[slot as usize];
    let (cam, p) = player.into_inner();
    let t = time.elapsed_secs();
    let tier = me.tiers[slot as usize] as f32;
    let feet = p.feet;
    let origin = cam.translation;
    let forward = cam.forward().as_vec3();
    let flat = forward.with_y(0.0).normalize_or(Vec3::NEG_Z);
    let hand = origin + forward * 0.6 - Vec3::Y * 0.2;
    let color = PREVIEW;
    // Lines in the air (paths and beams).
    let line_col = PREVIEW.with_alpha(0.6);
    let boxes = collect_boxes(colliders.iter());
    let ground = |range: f32| aim_ground(origin, forward, range, &boxes);
    // Where a dash stops (as the host works it out).
    let dash_len = |tier: f32| {
        let len = DASH_SPEED * move_time(ability, tier);
        (ray_world(feet + Vec3::Y * 0.5, flat, len, &boxes) - 0.4).max(0.0)
    };
    use Ability as A;
    match ability {
        // Bulwark
        A::ShieldCharge => {
            let len = dash_len(tier);
            arrow(&mut markers, feet, flat, len, 1.5, color);
        }
        A::GroundPound => flat_circle(&mut markers, feet, 6.0 + 0.4 * tier, color),
        A::Fortress => {
            flat_circle(&mut markers, feet, 5.0, color);
            // The dome's outline.
            for a in [0.0, TAU / 4.0] {
                let iso = Isometry3d::new(feet, Quat::from_rotation_y(a));
                gizmos.circle(iso, 5.0, line_col).resolution(40);
            }
        }
        A::RallyCry => ring(&mut markers, feet, 9.4, 10.0, color),
        A::Earthshaker => {
            for r in [6.0, 10.0, 14.0] {
                ring(&mut markers, feet, r - 0.5, r, color);
            }
        }

        // Medic
        A::HealingGrenade => {
            let at = throw_arc(&mut gizmos, origin, forward, GRENADE_SPEED, &boxes, line_col);
            flat_circle(&mut markers, at, 5.0 + 0.3 * tier, color);
        }
        A::NeurotoxinDart => {
            let dist = ray_world(origin, forward, 44.0, &boxes);
            let b = origin + forward * dist;
            gizmos.line(hand, b, line_col);
            flat_circle(&mut markers, b.with_y(feet.y), 0.6, color);
        }
        A::Resurrection => {
            ring(&mut markers, feet, 1.0, 1.3 + 0.1 * (t * 6.0).sin(), color);
            gizmos.line(feet, feet + Vec3::Y * 25.0, line_col);
        }
        A::MedDrone => {
            let at = feet + flat * 0.6;
            flat_circle(&mut markers, at, 0.5, color);
            gizmos.line(at, at + Vec3::Y * 2.2, line_col);
        }
        A::Sterilize => flat_circle(&mut markers, feet, 16.0, color),

        // Revenant
        A::ScytheSweep => ring(&mut markers, feet, 0.8, 4.5 + 0.3 * tier, color),
        A::SoulChains => cone(&mut markers, feet, flat, 15.0, 0.85, color),
        A::Reaper => ring(&mut markers, feet, 1.0, 5.5, color),
        A::WraithStep => {
            let len = dash_len(tier);
            arrow(&mut markers, feet, flat, len, 1.6, color);
        }
        A::ArmyOfTheDead => {
            for i in 0..4 {
                let ang = i as f32 / 4.0 * TAU;
                let at = feet + Vec3::new(ang.cos(), 0.0, ang.sin()) * 2.0;
                flat_circle(&mut markers, at, 0.6, color);
            }
        }

        // Demolisher
        A::StickyBomb => {
            let at = throw_arc(&mut gizmos, origin, forward, GRENADE_SPEED * 1.2, &boxes, line_col);
            flat_circle(&mut markers, at, 4.5, color);
        }
        A::BlastJump => {
            flat_circle(&mut markers, feet, 4.5, color);
            let len = dash_len(tier);
            let land = feet + flat * len;
            gizmos.line(feet + Vec3::Y * 0.1, land + Vec3::Y * 0.1, line_col);
            ring(&mut markers, land, 0.6, 0.9, color);
        }
        A::Payload => sky_target(&mut markers, &mut gizmos, ground(70.0), 11.0, color),
        A::Claymore => {
            let dist = ray_world(feet + Vec3::Y * 0.5, flat, 2.5, &boxes);
            let at = feet + flat * (dist - 0.5).max(0.4);
            flat_circle(&mut markers, at, 0.4, color);
            cone(&mut markers, at, flat, 8.0, 0.45, color.with_alpha(0.4));
        }
        A::ChainReaction => ring(&mut markers, feet, 1.0, 1.3 + 0.1 * (t * 8.0).sin(), color),

        // Chemist
        A::AcidFlask => {
            let at = throw_arc(&mut gizmos, origin, forward, GRENADE_SPEED, &boxes, line_col);
            flat_circle(&mut markers, at, 3.5 + 0.3 * tier, color);
        }
        A::ToxicCloud => {
            // The three puffs the spray leaves.
            for (d, r) in [(2.5, 2.4), (5.5, 3.0), (9.0, 3.8)] {
                let reach = ray_world(feet + Vec3::Y, flat, d + 1.0, &boxes) - 1.0;
                flat_circle(&mut markers, feet + flat * reach.min(d).max(0.5), r, color);
            }
        }
        A::PlagueBloom => {
            // Starts small and spreads out to the outer ring.
            let at = ground(50.0);
            sky_target(&mut markers, &mut gizmos, at, PLAGUE_START, color);
            ring(&mut markers, at, PLAGUE_MAX - 0.4, PLAGUE_MAX, color.with_alpha(0.5));
        }
        A::Catalyst => ring(&mut markers, feet, 1.0, 1.3 + 0.1 * (t * 10.0).sin(), color),
        A::Petrify => cone(&mut markers, feet, flat, 18.0, 0.7, color),

        // Ranger
        A::Grapple => {
            let dist = ray_world(origin, forward, GRAPPLE_RANGE, &boxes);
            let b = origin + forward * dist;
            gizmos.line(hand, b, line_col);
            let hit = dist < GRAPPLE_RANGE - 0.1;
            let col = if hit { color } else { color.with_alpha(0.3) };
            ring(&mut markers, b.with_y(feet.y.max(b.y - 0.1)), 0.4, 0.7, col);
        }
        A::HuntersMark => cone(&mut markers, feet, flat, 28.0, 0.82, color),
        A::Deadeye => {
            // The ten zombies nearest your aim get locked on.
            let off = |p: Vec3| (p + Vec3::Y - origin).normalize_or_zero().dot(forward);
            let mut targets: Vec<Vec3> = enemies
                .iter()
                .map(|tf| tf.translation)
                .filter(|p| {
                    let to = *p + Vec3::Y - origin;
                    let d = to.length();
                    d < 60.0 && (d < 2.0 || off(*p) > 0.45) && line_of_sight(origin, *p + Vec3::Y, &boxes)
                })
                .collect();
            targets.sort_by(|a, b| off(*b).total_cmp(&off(*a)));
            let pulse = 0.8 + 0.1 * (t * 8.0).sin();
            for pos in targets.into_iter().take(10) {
                ring(&mut markers, pos, pulse - 0.15, pulse, color);
                gizmos.line(pos + Vec3::Y * 2.4, pos + Vec3::Y * 2.0, line_col);
            }
            cone(&mut markers, feet, flat, 12.0, 0.45, color.with_alpha(0.3));
        }
        A::BearTrap => {
            let at = throw_arc(&mut gizmos, origin, forward, GRENADE_SPEED * 0.85, &boxes, line_col);
            flat_circle(&mut markers, at, 1.8, color);
        }
        A::ArrowStorm => sky_target(&mut markers, &mut gizmos, ground(60.0), 8.0, color),
    }
}
