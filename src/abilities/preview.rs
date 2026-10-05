//! Shows where an ability will land while you aim it.

use bevy::prelude::*;
use std::f32::consts::TAU;

use super::{
    dash_direction, leap_length, CastState, FULL_CHARGE, GRENADE_GRAVITY, GRENADE_LIFT,
    GRENADE_SPEED, LEAP_RADIUS, RAGNAROK_RADIUS, SPEAR_RANGE, SPEAR_WIDTH,
};
use crate::config::Settings;
use crate::markers::{Marker, Markers, Shape, PREVIEW};
use crate::data::Ability;
use crate::physics::{collect_boxes, line_of_sight, ray_world};
use crate::player::LocalPlayer;
use crate::sim::powers::aim_ground;
use crate::{Collider, Roster, Session};

fn flat_circle(markers: &mut Markers, center: Vec3, radius: f32, color: Color) {
    markers.push(Marker::new(center, Shape::Circle { radius }, color));
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

/// A wedge on the ground in front of `feet`, `spread` radians wide.
fn arc(markers: &mut Markers, feet: Vec3, flat: Vec3, r: f32, spread: f32, color: Color) {
    markers.push(
        Marker::new(
            feet,
            Shape::Cone {
                radius: r,
                half_angle: spread / 2.0,
            },
            color,
        )
        .facing(flat),
    );
}

/// A target on the ground with a line up to the sky.
fn sky_target(markers: &mut Markers, gizmos: &mut Gizmos, at: Vec3, r: f32, color: Color) {
    flat_circle(markers, at, r, color);
    markers.push(Marker::new(
        at,
        Shape::Donut {
            inner: r * 0.42,
            outer: r * 0.5,
        },
        color,
    ));
    gizmos.line(at, at + Vec3::Y * 25.0, color.with_alpha(0.5));
}

#[allow(clippy::too_many_arguments)]
pub fn draw_previews(
    time: Res<Time>,
    cast: Res<CastState>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    session: Res<Session>,
    roster: Res<Roster>,
    player: Single<(&Transform, &LocalPlayer)>,
    colliders: Query<(&Transform, &Collider)>,
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
    let color = PREVIEW;
    // Lines in the air (paths and beams).
    let line_col = PREVIEW.with_alpha(0.6);
    let boxes = collect_boxes(colliders.iter());
    let ground = |range: f32| aim_ground(origin, forward, range, &boxes);
    use Ability as A;
    match ability {
        A::Dash => {
            let dir = dash_direction(&keys, &settings, p.yaw);
            arrow(&mut markers, feet, dir, 22.0 * (0.18 + 0.03 * tier), 0.3, color);
        }
        A::ShadowStep | A::FlameDash => {
            let base = if ability == A::FlameDash { 0.22 } else { 0.2 };
            arrow(&mut markers, feet, flat, 22.0 * (base + 0.03 * tier), 0.9, color);
        }
        A::StormLeap => {
            let land = feet + flat * leap_length(tier);
            gizmos.line(feet + Vec3::Y * 0.1, land + Vec3::Y * 0.1, line_col);
            flat_circle(&mut markers, land, LEAP_RADIUS, color);
        }
        A::FragGrenade
        | A::ClusterGrenade
        | A::Firebomb
        | A::SmokeBomb
        | A::GravGrenade
        | A::ProximityMines => {
            // The arc it will fly and where it lands.
            let (speed, lift) = if ability == A::ProximityMines {
                (11.0, 4.0)
            } else {
                (GRENADE_SPEED, GRENADE_LIFT)
            };
            let dir = if ability == A::ProximityMines {
                flat
            } else {
                forward
            };
            let mut pos = origin + forward * 0.6;
            let mut vel = dir * speed + Vec3::Y * lift;
            let mut pts = vec![pos];
            let step = 0.025;
            for _ in 0..160 {
                vel.y -= GRENADE_GRAVITY * step;
                let next = pos + vel * step;
                if next.y < 0.1 || !line_of_sight(pos, next, &boxes) {
                    break;
                }
                pos = next;
                pts.push(pos);
            }
            gizmos.linestrip(pts, line_col);
            let r = match ability {
                A::SmokeBomb => 5.0 + 0.5 * tier,
                A::GravGrenade => 6.5,
                A::ClusterGrenade => 5.5,
                A::ProximityMines => 2.4,
                _ => 4.0 + 0.5 * tier,
            };
            flat_circle(&mut markers, pos.with_y(0.0), r, color);
        }
        A::Overdrive | A::CombatStim => flat_circle(&mut markers, feet, 1.2, color),
        A::RocketBarrage | A::KunaiFan => {
            let (n, spread, range) = if ability == A::KunaiFan {
                (7, 0.75, 29.0)
            } else {
                (6, 0.7, 30.0)
            };
            for i in 0..n {
                let f = i as f32 / (n - 1) as f32 - 0.5;
                let d = Quat::from_rotation_y(f * spread) * flat;
                let len = ray_world(feet + Vec3::Y, d, range, &boxes);
                gizmos.line(feet + Vec3::Y * 0.1, feet + Vec3::Y * 0.1 + d * len, line_col);
            }
        }
        A::Airstrike => {
            let start = ground(18.0);
            arrow(&mut markers, start, flat, 26.0, 3.0, color);
        }
        A::HealPulse | A::SupplyDrop => flat_circle(&mut markers, feet, 8.0, color),
        A::FrostNova => flat_circle(&mut markers, feet, 7.0, color),
        A::BarrierDome => {
            flat_circle(&mut markers, feet, 5.0, color);
            let iso = Isometry3d::new(feet, Quat::IDENTITY);
            gizmos.circle(iso, 5.0, line_col).resolution(40);
        }
        A::GlacierSpike => {
            let len = ray_world(feet + Vec3::Y * 0.5, flat, 16.0 + 2.0 * tier, &boxes);
            arrow(&mut markers, feet, flat, len, 1.0, color);
        }
        A::OrbitalStrike => sky_target(&mut markers, &mut gizmos, ground(80.0), 9.0 + tier, color),
        A::Blizzard => sky_target(&mut markers, &mut gizmos, ground(60.0), 9.0, color),
        A::MortarBattery => sky_target(&mut markers, &mut gizmos, ground(60.0), 9.0, color),
        A::MeteorShower => sky_target(&mut markers, &mut gizmos, ground(60.0), 10.0, color),
        A::SpearRain => sky_target(&mut markers, &mut gizmos, ground(50.0), 6.0 + 0.5 * tier, color),
        A::MagmaGeyser => {
            let at = ground(30.0);
            flat_circle(&mut markers, at, 3.6, color);
            gizmos.line(at, at + Vec3::Y * 6.0, line_col);
        }
        A::CryoOrb | A::Fireball | A::ChainLightning => {
            let range = match ability {
                A::CryoOrb => 21.0,
                A::Fireball => 40.0,
                _ => 28.0,
            };
            let dist = ray_world(origin, forward, range, &boxes);
            let b = origin + forward * dist;
            gizmos.line(origin + forward * 0.8 - Vec3::Y * 0.3, b, line_col);
            let r = match ability {
                A::CryoOrb => 5.5,
                A::Fireball => 4.5,
                _ => 1.0,
            };
            flat_circle(&mut markers, b.with_y(feet.y), r, color);
        }
        A::Iaido => {
            // The crescent's reach grows as you hold the draw.
            let charge = (cast.held / FULL_CHARGE).min(1.0);
            let range = 12.0 + 28.0 * charge + 2.0 * tier;
            let dist = ray_world(origin, forward, range, &boxes);
            let width = 1.2 + 1.2 * charge;
            let end = (origin + forward * dist).with_y(feet.y);
            arrow(&mut markers, feet, flat, end.distance(feet).max(1.0), width, color);
            if charge >= 1.0 {
                flat_circle(&mut markers, feet, 1.0 + 0.2 * (t * 12.0).sin(), color);
            }
        }
        A::RisingDragon => arc(&mut markers, feet, flat, 4.8, 2.5, color),
        A::BladeStorm => flat_circle(&mut markers, feet, 5.0, color),
        A::ThousandCuts => flat_circle(&mut markers, feet, 15.0, color),
        A::Sentry => {
            let dist = ray_world(feet + Vec3::Y * 0.5, flat, 3.0, &boxes);
            let at = feet + flat * (dist - 0.6).max(0.3);
            flat_circle(&mut markers, at, 0.6, color);
            flat_circle(&mut markers, at, 24.0, color.with_alpha(0.4));
        }
        A::CombatDrone => flat_circle(&mut markers, feet, 22.0, color.with_alpha(0.4)),
        A::TeslaCoil => {
            let dist = ray_world(origin, forward, 40.0, &boxes);
            let target = (origin + forward * (dist - 0.5).max(0.5)).with_y(feet.y);
            flat_circle(&mut markers, target, 8.0, color);
            gizmos.line(target, target + Vec3::Y * 2.6, line_col);
        }
        A::FlameWave => {
            let range = 8.0 + tier;
            let right = forward.cross(Vec3::Y).normalize_or(Vec3::X);
            let up = right.cross(forward);
            let r = range * 0.6;
            let end = origin + forward * range;
            let ring: Vec<Vec3> = (0..=16)
                .map(|i| {
                    let a = i as f32 / 16.0 * TAU;
                    end + (right * a.cos() + up * a.sin()) * r
                })
                .collect();
            for i in 0..4 {
                gizmos.line(origin + forward * 0.5, ring[i * 4], line_col);
            }
            gizmos.linestrip(ring, line_col);
        }
        A::Inferno => flat_circle(&mut markers, feet, 6.0, color),
        A::ArcSpear => {
            let dist = ray_world(origin, forward, SPEAR_RANGE, &boxes);
            let a = origin + forward * 0.8 - Vec3::Y * 0.3;
            let b = origin + forward * dist;
            gizmos.line(a, b, line_col);
            flat_circle(&mut markers, b.with_y(feet.y), SPEAR_WIDTH, color);
        }
        A::Ragnarok => flat_circle(&mut markers, feet, RAGNAROK_RADIUS, color),
        A::ThunderClap => flat_circle(&mut markers, feet, 6.5 + 0.5 * tier, color),
        A::Bifrost => {
            let start = feet + flat * 5.0;
            let len = ray_world(start + Vec3::Y, flat, 34.0, &boxes).max(6.0);
            arrow(&mut markers, start, flat, len, 3.0, color);
        }
    }
}
