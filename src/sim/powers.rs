//! What every ability does on the host, plus the things abilities leave
//! behind: projectiles, grenades, mines, turrets and drones, delayed strikes
//! and the pull/push areas zombies feel.

use bevy::prelude::*;
use rand::Rng;

use super::{
    enemy_scale, explode, level_multiplier, DamageEvent, DamageQueue, EnemyBrain, Strikes, Zone,
    Zones,
};
use crate::data::{elements_in, Ability, Element};
use crate::fx::{emit, rgb, Fx, FxOutbox, FxQueue};
use crate::physics::{collect_boxes, line_of_sight, ray_world, Boxes};
use crate::{Collider, MatchState, NetKind, Replicated, Roster};

/// Projectile looks (`NetKind::Missile`): crescent cuts in three sizes, then
/// the rest.
pub mod look {
    pub const CRESCENT: u8 = 0;
    pub const KUNAI: u8 = 3;
    pub const ROCKET: u8 = 4;
    pub const FIREBALL: u8 = 5;
    pub const CRYO: u8 = 6;
    pub const BOMBLET: u8 = 7;
    pub const SMOKE: u8 = 8;
    pub const GRAV: u8 = 9;
}

/// Things falling from the sky (`Fx::Falling`).
pub mod falling {
    pub const BOMB: u8 = 0;
    pub const SHELL: u8 = 1;
    pub const METEOR: u8 = 2;
    pub const SPEAR: u8 = 3;
}

/// Zone kinds (`Fx::Zone`): 0 blade storm, 1 tesla, 2 fire pool, 3 inferno,
/// 4 Ragnarok, then these.
pub mod zone {
    pub const DOME: u8 = 5;
    pub const BLIZZARD: u8 = 6;
    pub const SMOKE: u8 = 7;
    pub const SINGULARITY: u8 = 8;
}

/// A hit that lands after a delay.
pub struct Strike {
    pub owner: u8,
    pub pos: Vec3,
    pub delay: f32,
    pub radius: f32,
    pub damage: f32,
    pub elements: u8,
    pub stun: f32,
    /// Hits this one enemy wherever it has got to (Thousand Cuts).
    pub target: Option<Entity>,
    /// Shown when it lands (for a target, at the target).
    pub fx: Option<Fx>,
    /// Leaves a pool of fire this wide.
    pub pool: f32,
}

impl Strike {
    fn new(owner: u8, pos: Vec3, delay: f32, radius: f32, damage: f32, elements: u8) -> Self {
        Self {
            owner,
            pos,
            delay,
            radius,
            damage,
            elements,
            stun: 0.0,
            target: None,
            fx: None,
            pool: 0.0,
        }
    }
}

/// An area that drags zombies in (strength > 0) or keeps them out (< 0).
pub struct Force {
    pub pos: Vec3,
    pub radius: f32,
    pub strength: f32,
    pub life: f32,
}

#[derive(Resource, Default)]
pub struct Forces(pub Vec<Force>);

/// What a thrown grenade does when it goes off.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Nade {
    Frag,
    Fire,
    Cluster,
    Smoke,
    Grav,
    Mine,
}

#[derive(Component)]
pub struct GrenadeBrain {
    pub owner: u8,
    pub velocity: Vec3,
    pub fuse: f32,
    pub damage: f32,
    pub radius: f32,
    pub elements: u8,
    pub kind: Nade,
}

/// Tinker's sentry turret, or the combat drone when it follows a player.
#[derive(Component)]
pub struct TurretBrain {
    pub owner: u8,
    pub life: f32,
    pub cooldown: f32,
    pub damage: f32,
    pub elements: u8,
    pub follow: Option<u8>,
}

#[derive(Component)]
pub struct MineBrain {
    owner: u8,
    arm: f32,
    life: f32,
    damage: f32,
    radius: f32,
    elements: u8,
}

/// What a projectile does when it stops.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum End {
    Fizzle,
    FirePool,
    Shatter,
}

#[derive(Component)]
pub struct Missile {
    owner: u8,
    vel: Vec3,
    gravity: f32,
    life: f32,
    hit_radius: f32,
    damage: f32,
    elements: u8,
    stun: f32,
    /// Flies through enemies (hitting each once) instead of stopping.
    pierce: bool,
    hits: Vec<Entity>,
    /// Explodes this wide at the end (0: no blast).
    blast: f32,
    blast_damage: f32,
    /// Turns toward enemies ahead.
    homing: f32,
    /// Chills enemies this close as it passes (Cryo Orb).
    chill: f32,
    chill_timer: f32,
    end: End,
    color: [f32; 3],
}

impl Missile {
    fn new(owner: u8, vel: Vec3, life: f32, damage: f32, elements: u8) -> Self {
        Self {
            owner,
            vel,
            gravity: 0.0,
            life,
            hit_radius: 0.5,
            damage,
            elements,
            stun: 0.0,
            pierce: false,
            hits: Vec::new(),
            blast: 0.0,
            blast_damage: 0.0,
            homing: 0.0,
            chill: 0.0,
            chill_timer: 0.0,
            end: End::Fizzle,
            color: [1.0, 0.6, 0.2],
        }
    }
}

/// Everything a cast can touch.
pub(super) struct World<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub state: &'a mut MatchState,
    pub damage: &'a mut DamageQueue,
    pub strikes: &'a mut Strikes,
    pub zones: &'a mut Zones,
    pub forces: &'a mut Forces,
    pub fx: &'a mut FxQueue,
    pub out: &'a mut FxOutbox,
    pub enemies: &'a [(Entity, Vec3)],
    pub boxes: &'a Boxes,
}

impl World<'_, '_, '_> {
    fn emit(&mut self, f: Fx) {
        emit(self.fx, self.out, f);
    }

    /// Shown only to everyone else (the caster already played it).
    fn send(&mut self, f: Fx) {
        self.out.0.push(f);
    }

    fn hit(&mut self, target: Entity, amount: f32, from: u8, elements: u8, stun: f32) {
        self.damage.0.push(DamageEvent {
            target,
            amount,
            from: Some(from),
            headshot: false,
            legs: false,
            elements,
            chained: false,
            stun,
        });
    }

    fn spawn(&mut self, kind: NetKind, tf: Transform, brain: impl Bundle) -> Entity {
        let id = self.state.next_net_id;
        self.state.next_net_id += 1;
        self.commands
            .spawn((crate::InGameEntity, Replicated { id, kind }, tf, brain))
            .id()
    }

    fn missile(&mut self, look: u8, from: Vec3, m: Missile) {
        let tf = Transform::from_translation(from).looking_to(m.vel, Vec3::Y);
        self.spawn(NetKind::Missile(look), tf, m);
    }

    fn nade(&mut self, kind: NetKind, from: Vec3, g: GrenadeBrain) {
        self.spawn(kind, Transform::from_translation(from), g);
    }

    fn strike(&mut self, s: Strike) {
        self.strikes.0.push(s);
    }

    fn zone(&mut self, z: Zone) {
        self.emit(Fx::Zone {
            pos: z.pos.to_array(),
            radius: z.radius,
            life: z.life,
            follow: z.follow.unwrap_or(255),
            kind: z.kind,
        });
        self.zones.0.push(z);
    }

    fn spell(&mut self, ability: Ability, pos: Vec3, dir: Vec3, size: f32) {
        self.emit(Fx::Spell {
            ability,
            pos: pos.to_array(),
            dir: dir.to_array(),
            size,
        });
    }

    /// Where on the floor the player is aiming, out to `range`.
    fn aim_ground(&self, origin: Vec3, dir: Vec3, range: f32) -> Vec3 {
        aim_ground(origin, dir, range, self.boxes)
    }

    /// Enemies within `radius` of `pos` (on the ground plane).
    fn near(&self, pos: Vec3, radius: f32) -> Vec<(Entity, Vec3)> {
        self.enemies
            .iter()
            .filter(|(_, p)| p.with_y(0.0).distance(pos.with_y(0.0)) < radius)
            .copied()
            .collect()
    }
}

/// Where on the floor a look from `origin` along `dir` lands, out to `range`
/// (walls stop it short).
pub fn aim_ground(origin: Vec3, dir: Vec3, range: f32, boxes: &Boxes) -> Vec3 {
    let mut dist = ray_world(origin, dir, range, boxes);
    if dir.y < -0.01 {
        dist = dist.min(origin.y / -dir.y);
    }
    let hit = origin + dir * (dist - 0.4).max(0.5);
    hit.with_y(0.0)
}

fn blast(at: Vec3, radius: f32, color: [f32; 3]) -> Fx {
    Fx::Explosion {
        pos: (at + Vec3::Y * 0.5).to_array(),
        radius,
        color,
    }
}

fn zone(owner: u8, pos: Vec3, radius: f32, damage: f32, interval: f32, life: f32) -> Zone {
    Zone {
        owner,
        pos,
        follow: None,
        radius,
        damage,
        interval,
        timer: 0.0,
        life,
        elements: 0,
        kind: 0,
        model: None,
    }
}

/// A fan of directions spread across `angle` radians around `dir` (flat).
fn fan(dir: Vec3, count: usize, angle: f32) -> Vec<Vec3> {
    (0..count)
        .map(|i| {
            let f = if count == 1 {
                0.0
            } else {
                i as f32 / (count - 1) as f32 - 0.5
            };
            Quat::from_rotation_y(f * angle) * dir
        })
        .collect()
}

/// Does the ability: player `id` casts their ability in `slot`, looking from
/// `origin` along `dir`, charged up to `charge` (0 to 1).
#[allow(clippy::too_many_arguments)]
pub(super) fn cast(
    w: &mut World,
    roster: &mut Roster,
    id: u8,
    slot: u8,
    origin: Vec3,
    dir: Vec3,
    charge: f32,
) {
    let Some(p) = roster.0.get(&id) else { return };
    let ability = p.kit[slot as usize];
    let tier = p.tiers[slot as usize] as f32;
    let mult = level_multiplier(p);
    let el = p.ability_elements;
    let feet = p.feet();
    let flat = dir.with_y(0.0).normalize_or(Vec3::NEG_Z);
    let right = Vec3::new(-flat.z, 0.0, flat.x);
    let hand = origin + dir * 0.6 - Vec3::Y * 0.2;
    let mut rng = rand::thread_rng();
    let fire = el | Element::Fire.bit();
    let ice = el | Element::Ice.bit();
    let shock = el | Element::Shock.bit();
    let throw = |speed: f32| dir * speed + Vec3::Y * crate::abilities::GRENADE_LIFT;
    use Ability as A;
    match ability {
        // --- Striker ---------------------------------------------------------
        A::Dash => {
            // The dash itself is done locally; everyone else sees the streak.
            let b = feet + flat * 22.0 * (0.18 + 0.03 * tier);
            w.send(Fx::Dash {
                player: id,
                a: feet.to_array(),
                b: b.to_array(),
            });
            w.spell(ability, feet, flat, 1.0);
        }
        A::FragGrenade | A::ClusterGrenade => {
            let cluster = ability == A::ClusterGrenade;
            w.nade(
                NetKind::Grenade,
                origin + dir * 0.6,
                GrenadeBrain {
                    owner: id,
                    velocity: throw(crate::abilities::GRENADE_SPEED),
                    fuse: crate::abilities::GRENADE_FUSE,
                    damage: if cluster { 90.0 + 35.0 * tier } else { 150.0 + 60.0 * tier } * mult,
                    radius: if cluster { 3.5 } else { 4.0 + 0.5 * tier },
                    elements: el,
                    kind: if cluster { Nade::Cluster } else { Nade::Frag },
                },
            );
        }
        A::Overdrive => {
            if let Some(p) = roster.0.get_mut(&id) {
                p.overdrive = 8.0 + 2.0 * tier;
            }
            w.spell(ability, feet, flat, 1.0);
        }
        A::RocketBarrage => {
            for (i, d) in fan(dir, 6, 0.7).into_iter().enumerate() {
                let lift = Vec3::Y * (0.08 + 0.05 * (i % 2) as f32);
                let mut m = Missile::new(id, (d + lift) * 30.0, 1.3, 0.0, el);
                m.blast = 2.8;
                m.blast_damage = (70.0 + 28.0 * tier) * mult;
                m.homing = 3.0;
                m.color = [1.0, 0.55, 0.2];
                w.missile(look::ROCKET, hand + right * 0.1, m);
            }
            w.spell(ability, hand, dir, 1.0);
        }
        A::Airstrike => {
            // Jets come in over your shoulder and walk bombs along the line.
            let start = w.aim_ground(origin, dir, 18.0);
            let len = 26.0;
            w.spell(ability, start, flat, len);
            for i in 0..12 {
                let at = start + flat * (i as f32 * len / 11.0)
                    + right * rng.gen_range(-2.0..2.0);
                let delay = 1.1 + i as f32 * 0.09;
                w.emit(Fx::Falling {
                    kind: falling::BOMB,
                    from: (at - flat * 10.0 + Vec3::Y * 22.0).to_array(),
                    to: at.to_array(),
                    time: delay,
                });
                let mut s = Strike::new(id, at, delay, 4.0, 320.0 * mult, el);
                s.fx = Some(blast(at, 4.0, [1.0, 0.55, 0.15]));
                w.strike(s);
            }
        }
        A::CombatStim => {
            if let Some(p) = roster.0.get_mut(&id) {
                p.stim = 6.0 + 2.0 * tier;
                p.health = (p.health + p.max_health() * 0.5).min(p.max_health());
            }
            w.spell(ability, feet, flat, 1.0);
        }

        // --- Warden ----------------------------------------------------------
        A::HealPulse => {
            let heal = 40.0 + 20.0 * tier;
            for other in roster.0.values_mut() {
                if other.alive && other.feet().distance(feet) < 8.0 {
                    other.health = (other.health + heal).min(other.max_health());
                }
            }
            w.emit(Fx::Heal {
                pos: feet.to_array(),
                radius: 8.0,
            });
        }
        A::FrostNova => {
            let dmg = (40.0 + 30.0 * tier) * mult;
            for (e, _) in w.near(feet, 7.0) {
                w.hit(e, dmg, id, ice, 0.8);
            }
            w.emit(Fx::Nova {
                pos: feet.to_array(),
                radius: 7.0,
            });
        }
        A::OrbitalStrike => {
            let target = w.aim_ground(origin, dir, 80.0);
            let radius = 9.0 + tier;
            w.strike(Strike::new(id, target, 1.2, radius, 3000.0 * mult, el));
            w.emit(Fx::Beam {
                pos: target.to_array(),
                radius,
                delay: 1.2,
            });
        }
        A::GlacierSpike => {
            // A wave of ice tearing along the ground, freezing what it hits.
            let len = 16.0 + 2.0 * tier;
            let dist = ray_world(feet + Vec3::Y * 0.5, flat, len, w.boxes);
            let dmg = (110.0 + 45.0 * tier) * mult;
            let steps = (dist / 1.6).ceil().max(1.0) as usize;
            for i in 1..=steps {
                let d = i as f32 * dist / steps as f32;
                let mut s = Strike::new(id, feet + flat * d, d / 40.0, 1.9, dmg, ice);
                s.stun = 1.6 + 0.3 * tier;
                w.strike(s);
            }
            // One hit per enemy: the strikes overlap, so share the damage.
            for s in w.strikes.0.iter_mut().rev().take(steps) {
                s.damage *= 0.55;
            }
            w.spell(ability, feet, flat, dist);
        }
        A::BarrierDome => {
            let life = 8.0 + 1.5 * tier;
            let radius = 5.0;
            let mut z = zone(id, feet, radius, 10.0 + 4.0 * tier, 0.5, life);
            z.kind = zone::DOME;
            w.zone(z);
            w.forces.0.push(Force {
                pos: feet,
                radius: radius + 0.6,
                strength: -9.0,
                life,
            });
        }
        A::Blizzard => {
            let at = w.aim_ground(origin, dir, 60.0);
            let mut z = zone(id, at, 9.0, 55.0 * mult, 0.35, 8.0 + tier);
            z.kind = zone::BLIZZARD;
            z.elements = ice;
            w.zone(z);
        }
        A::CryoOrb => {
            let mut m = Missile::new(id, dir * 9.0, 2.4, (60.0 + 20.0 * tier) * mult, ice);
            m.pierce = true;
            m.hit_radius = 1.0;
            m.stun = 0.8;
            m.chill = 3.5;
            m.blast = 5.5;
            m.blast_damage = (140.0 + 50.0 * tier) * mult;
            m.end = End::Shatter;
            m.color = [0.5, 0.85, 1.0];
            w.missile(look::CRYO, hand, m);
        }

        // --- Ronin -----------------------------------------------------------
        A::Iaido => {
            // A flying crescent cut: further, wider and harder the longer
            // the draw was held.
            let speed = 38.0;
            let range = 12.0 + 28.0 * charge + 2.0 * tier;
            let size = (charge * 2.99) as u8;
            let mut m = Missile::new(
                id,
                dir * speed,
                range / speed,
                (150.0 + 60.0 * tier) * (0.6 + 1.4 * charge) * mult,
                el,
            );
            m.pierce = true;
            m.hit_radius = 1.2 + 1.2 * charge;
            m.color = [1.0, 0.85, 0.4];
            w.missile(look::CRESCENT + size, origin + dir * 0.8 - Vec3::Y * 0.25, m);
            // A point-blank cut too, so it never whiffs up close.
            let dmg = (100.0 + 40.0 * tier) * mult;
            for (e, pos) in w.near(feet, 2.6) {
                if (pos - feet).with_y(0.0).normalize_or_zero().dot(flat) > 0.3 {
                    w.hit(e, dmg, id, el, 0.0);
                }
            }
            w.spell(ability, origin - Vec3::Y * 0.3, dir, charge);
        }
        A::ShadowStep => {
            // The blink is done locally; cut everything along the path.
            let len = 22.0 * (0.2 + 0.03 * tier);
            let end = feet + flat * len;
            let dmg = (120.0 + 50.0 * tier) * mult;
            for (e, pos) in w.enemies.to_vec() {
                let (a, b, q) = (feet.with_y(0.0), end.with_y(0.0), pos.with_y(0.0));
                let ab = b - a;
                let t = ((q - a).dot(ab) / ab.length_squared().max(1e-4)).clamp(0.0, 1.0);
                if q.distance(a + ab * t) < 1.8 {
                    w.hit(e, dmg, id, el, 0.0);
                }
            }
            w.send(Fx::Dash {
                player: id,
                a: feet.to_array(),
                b: end.to_array(),
            });
            w.spell(ability, feet, flat, len);
        }
        A::BladeStorm => {
            let life = 6.0 + tier;
            let mut z = zone(id, feet, 5.0, 70.0 * mult, 0.25, life);
            z.follow = Some(id);
            z.elements = el;
            w.zone(z);
        }
        A::KunaiFan => {
            for d in fan(flat.with_y(dir.y).normalize(), 7, 0.75) {
                let mut m = Missile::new(id, d * 45.0, 0.65, (70.0 + 25.0 * tier) * mult, el);
                m.pierce = true;
                m.hit_radius = 0.6;
                m.color = [1.0, 0.3, 0.3];
                w.missile(look::KUNAI, hand, m);
            }
        }
        A::SmokeBomb => {
            w.nade(
                NetKind::Missile(look::SMOKE),
                origin + dir * 0.6,
                GrenadeBrain {
                    owner: id,
                    velocity: throw(crate::abilities::GRENADE_SPEED),
                    fuse: 1.2,
                    damage: (12.0 + 5.0 * tier) * mult,
                    radius: 5.0 + 0.5 * tier,
                    elements: el,
                    kind: Nade::Smoke,
                },
            );
        }
        A::ThousandCuts => {
            // Vanish, then cuts land on everything around, one after another.
            if let Some(p) = roster.0.get_mut(&id) {
                p.vanish = 2.2;
            }
            let mut targets = w.near(feet, 15.0);
            targets.sort_by(|a, b| a.1.distance(feet).total_cmp(&b.1.distance(feet)));
            let count = targets.len().min(14 + 2 * tier as usize);
            let dmg = 420.0 * mult;
            for (i, (e, _)) in targets.into_iter().take(count).enumerate() {
                for k in 0..2 {
                    let mut s = Strike::new(id, feet, 0.25 + i as f32 * 0.11 + k as f32 * 0.9, 1.0, dmg * 0.5, el);
                    s.target = Some(e);
                    s.stun = 1.0;
                    s.fx = Some(Fx::Spell {
                        ability,
                        pos: [0.0; 3],
                        dir: (Quat::from_rotation_y(rng.gen_range(0.0..6.3)) * Vec3::X).to_array(),
                        size: 1.0,
                    });
                    w.strike(s);
                }
            }
            // The finishing flourish where you reappear.
            let mut s = Strike::new(id, feet, 2.1, 6.0, 150.0 * mult, el);
            s.fx = Some(Fx::Slash {
                pos: (feet + Vec3::Y * 1.1).to_array(),
                dir: flat.to_array(),
                radius: 6.0,
            });
            w.strike(s);
            w.spell(ability, feet, flat, 0.0);
        }
        A::RisingDragon => {
            // The jump is done locally; launch everything in front.
            let dmg = (200.0 + 70.0 * tier) * mult;
            for (e, pos) in w.near(feet, 4.8) {
                let to = (pos - feet).with_y(0.0);
                if to.length() < 1.0 || to.normalize().dot(flat) > 0.2 {
                    w.hit(e, dmg, id, fire, 1.4);
                }
            }
            w.spell(ability, feet, flat, 1.0);
        }

        // --- Tinker ----------------------------------------------------------
        A::Sentry => {
            let dist = ray_world(feet + Vec3::Y * 0.5, flat, 3.0, w.boxes);
            let at = feet + flat * (dist - 0.6).max(0.3);
            w.spawn(
                NetKind::Turret,
                Transform::from_translation(at).with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, flat)),
                TurretBrain {
                    owner: id,
                    life: 15.0 + 5.0 * tier,
                    cooldown: 0.5,
                    damage: (24.0 + 8.0 * tier) * mult,
                    elements: el,
                    follow: None,
                },
            );
            w.emit(Fx::Ring {
                pos: at.to_array(),
                radius: 1.5,
                color: [0.3, 0.9, 1.0],
            });
        }
        A::SupplyDrop => {
            let heal = 35.0 + 15.0 * tier;
            for other in roster.0.values_mut() {
                if other.alive && other.feet().distance(feet) < 8.0 {
                    other.health = (other.health + heal).min(other.max_health());
                    other.supply_seq = other.supply_seq.wrapping_add(1);
                }
            }
            w.spell(ability, feet, flat, 8.0);
        }
        A::TeslaCoil => {
            let dist = ray_world(origin, dir, 40.0, w.boxes);
            let at = origin + dir * (dist - 0.5).max(0.5);
            let at = if at.y < feet.y + 0.3 { at } else { at.with_y(feet.y) };
            let at = at.with_y(at.y.max(0.0));
            let model = w.spawn(NetKind::Coil, Transform::from_translation(at), ());
            let mut z = zone(id, at, 8.0, 90.0 * mult, 0.4, 10.0 + tier);
            z.timer = 0.3;
            z.elements = shock;
            z.kind = 1;
            z.model = Some(model);
            w.zone(z);
        }
        A::ProximityMines => {
            for d in fan(flat, 3, 0.6) {
                w.nade(
                    NetKind::Mine,
                    origin + dir * 0.6,
                    GrenadeBrain {
                        owner: id,
                        velocity: d * 11.0 + Vec3::Y * 4.0,
                        fuse: 30.0,
                        damage: (200.0 + 80.0 * tier) * mult,
                        radius: 4.0,
                        elements: el,
                        kind: Nade::Mine,
                    },
                );
            }
        }
        A::CombatDrone => {
            w.spawn(
                NetKind::Drone,
                Transform::from_translation(feet + Vec3::Y * 2.2 + right * 0.8),
                TurretBrain {
                    owner: id,
                    life: 15.0 + 5.0 * tier,
                    cooldown: 0.3,
                    damage: (20.0 + 7.0 * tier) * mult,
                    elements: el,
                    follow: Some(id),
                },
            );
            w.emit(Fx::Ring {
                pos: (feet + Vec3::Y * 2.0).to_array(),
                radius: 1.2,
                color: [0.3, 1.0, 0.8],
            });
        }
        A::MortarBattery => {
            let center = w.aim_ground(origin, dir, 60.0);
            let radius = 9.0;
            w.spell(ability, center, flat, radius);
            for i in 0..18 {
                let ang = rng.gen_range(0.0..std::f32::consts::TAU);
                let r = radius * rng.gen_range(0.0f32..1.0).sqrt();
                let at = center + Vec3::new(ang.cos(), 0.0, ang.sin()) * r;
                let delay = 1.0 + i as f32 * 0.26 + rng.gen_range(0.0..0.15);
                w.emit(Fx::Falling {
                    kind: falling::SHELL,
                    from: (at + Vec3::Y * 30.0).to_array(),
                    to: at.to_array(),
                    time: delay,
                });
                let mut s = Strike::new(id, at, delay, 3.2, 200.0 * mult, el);
                s.fx = Some(blast(at, 3.2, [1.0, 0.7, 0.3]));
                w.strike(s);
            }
        }
        A::GravGrenade => {
            w.nade(
                NetKind::Missile(look::GRAV),
                origin + dir * 0.6,
                GrenadeBrain {
                    owner: id,
                    velocity: throw(crate::abilities::GRENADE_SPEED),
                    fuse: 1.0,
                    damage: (240.0 + 90.0 * tier) * mult,
                    radius: 6.5,
                    elements: el,
                    kind: Nade::Grav,
                },
            );
        }

        // --- Blaze -----------------------------------------------------------
        A::Firebomb => {
            w.nade(
                NetKind::Firebomb,
                origin + dir * 0.6,
                GrenadeBrain {
                    owner: id,
                    velocity: throw(crate::abilities::GRENADE_SPEED),
                    fuse: crate::abilities::GRENADE_FUSE,
                    damage: (60.0 + 20.0 * tier) * mult,
                    radius: 4.0 + 0.5 * tier,
                    elements: fire,
                    kind: Nade::Fire,
                },
            );
        }
        A::FlameWave => {
            let range = 8.0 + tier;
            let dmg = (70.0 + 30.0 * tier) * mult;
            for (e, pos) in w.enemies.to_vec() {
                let to = pos + Vec3::Y - origin;
                let d = to.length();
                if d < range && (d < 1.0 || to.normalize().dot(dir) > 0.8) {
                    w.hit(e, dmg, id, fire, 0.0);
                }
            }
            w.emit(Fx::Cone {
                pos: (origin + dir * 0.5 - Vec3::Y * 0.2).to_array(),
                dir: dir.to_array(),
                range,
            });
        }
        A::Inferno => {
            let mut z = zone(id, feet, 6.0, 50.0 * mult, 0.3, 8.0 + tier);
            z.follow = Some(id);
            z.elements = fire;
            z.kind = 3;
            w.zone(z);
        }
        A::Fireball => {
            let mut m = Missile::new(id, dir * 26.0, 1.6, 0.0, fire);
            m.hit_radius = 0.8;
            m.blast = 4.5;
            m.blast_damage = (160.0 + 60.0 * tier) * mult;
            m.end = End::FirePool;
            m.color = [1.0, 0.5, 0.12];
            w.missile(look::FIREBALL, hand, m);
            w.spell(ability, hand, dir, 1.0);
        }
        A::FlameDash => {
            // The dash is done locally; the path is left burning.
            let len = 22.0 * (0.22 + 0.03 * tier);
            let end = feet + flat * len;
            let dmg = (60.0 + 25.0 * tier) * mult;
            for (e, pos) in w.enemies.to_vec() {
                let (a, b, q) = (feet.with_y(0.0), end.with_y(0.0), pos.with_y(0.0));
                let ab = b - a;
                let t = ((q - a).dot(ab) / ab.length_squared().max(1e-4)).clamp(0.0, 1.0);
                if q.distance(a + ab * t) < 1.8 {
                    w.hit(e, dmg, id, fire, 0.0);
                }
            }
            for i in 0..4 {
                let at = feet.lerp(end, (i as f32 + 0.5) / 4.0).with_y(0.0);
                let mut z = zone(id, at, 1.8, 18.0 * mult, 0.3, 4.0 + tier);
                z.elements = fire;
                z.kind = 2;
                w.zone(z);
            }
            w.send(Fx::Dash {
                player: id,
                a: feet.to_array(),
                b: end.to_array(),
            });
            w.spell(ability, feet, flat, len);
        }
        A::MeteorShower => {
            let center = w.aim_ground(origin, dir, 60.0);
            let radius = 10.0;
            for i in 0..12 {
                let ang = rng.gen_range(0.0..std::f32::consts::TAU);
                let r = radius * rng.gen_range(0.0f32..1.0).sqrt();
                let at = center + Vec3::new(ang.cos(), 0.0, ang.sin()) * r;
                let delay = 0.8 + i as f32 * 0.33 + rng.gen_range(0.0..0.2);
                let side = Vec3::new(rng.gen_range(-1.0..1.0), 0.0, rng.gen_range(-1.0..1.0));
                w.emit(Fx::Falling {
                    kind: falling::METEOR,
                    from: (at + side * 14.0 + Vec3::Y * 34.0).to_array(),
                    to: at.to_array(),
                    time: delay,
                });
                let mut s = Strike::new(id, at, delay, 3.8, 280.0 * mult, fire);
                s.pool = if i % 3 == 0 { 2.5 } else { 0.0 };
                s.fx = Some(blast(at, 3.8, [1.0, 0.4, 0.05]));
                w.strike(s);
            }
            w.spell(ability, center, flat, radius);
        }
        A::MagmaGeyser => {
            let at = w.aim_ground(origin, dir, 30.0);
            let mut s = Strike::new(id, at, 0.75, 3.6, (240.0 + 90.0 * tier) * mult, fire);
            s.stun = 1.2;
            s.pool = 3.0;
            // Size 0 is the eruption; the cast shows the ground cracking.
            s.fx = Some(Fx::Spell {
                ability,
                pos: at.to_array(),
                dir: flat.to_array(),
                size: 0.0,
            });
            w.strike(s);
            w.spell(ability, at, flat, 3.6);
        }

        // --- Valkyrie --------------------------------------------------------
        A::ArcSpear => {
            use crate::abilities::{SPEAR_RANGE, SPEAR_WIDTH};
            let dist = ray_world(origin, dir, SPEAR_RANGE, w.boxes);
            let end = origin + dir * dist;
            let dmg = (140.0 + 55.0 * tier) * mult;
            for (e, pos) in w.enemies.to_vec() {
                let chest = pos + Vec3::Y;
                let t = (chest - origin).dot(dir).clamp(0.0, dist);
                if chest.distance(origin + dir * t) < SPEAR_WIDTH {
                    w.hit(e, dmg, id, shock, 0.3);
                }
            }
            w.emit(Fx::Spear {
                a: (origin + dir * 0.6 - Vec3::Y * 0.15).to_array(),
                b: end.to_array(),
            });
        }
        A::StormLeap => {
            use crate::abilities::{leap_length, LEAP_RADIUS};
            let land = feet + flat * leap_length(tier);
            let dmg = (110.0 + 45.0 * tier) * mult;
            // Lands as you come down.
            let mut s = Strike::new(id, land, 0.45, LEAP_RADIUS, dmg, shock);
            s.stun = 0.6;
            s.fx = Some(Fx::Slam {
                pos: land.to_array(),
                radius: LEAP_RADIUS,
            });
            w.strike(s);
            w.send(Fx::Dash {
                player: id,
                a: feet.to_array(),
                b: land.to_array(),
            });
        }
        A::Ragnarok => {
            let radius = crate::abilities::RAGNAROK_RADIUS;
            let mut z = zone(id, feet, radius, 160.0 * mult, 0.3, 8.0 + tier);
            z.timer = 0.2;
            z.follow = Some(id);
            z.elements = shock;
            z.kind = 4;
            w.zone(z);
        }
        A::ThunderClap => {
            let radius = 6.5 + 0.5 * tier;
            let dmg = (90.0 + 35.0 * tier) * mult;
            for (e, _) in w.near(feet, radius) {
                w.hit(e, dmg, id, shock, 1.4 + 0.2 * tier);
            }
            w.spell(ability, feet, flat, radius);
        }
        A::ChainLightning => {
            // The first bolt goes to the enemy closest to where you aim.
            let first = w
                .enemies
                .iter()
                .map(|(e, p)| (*e, *p + Vec3::Y))
                .filter(|(_, c)| {
                    let to = *c - origin;
                    to.length() < 28.0
                        && to.normalize_or_zero().dot(dir) > 0.9
                        && line_of_sight(origin, *c, w.boxes)
                })
                .min_by(|a, b| {
                    let off = |c: Vec3| (c - origin).normalize_or_zero().dot(dir);
                    off(b.1).total_cmp(&off(a.1))
                });
            let mut from = hand;
            let mut dmg = (130.0 + 45.0 * tier) * mult;
            let Some(mut next) = first else {
                let end = origin + dir * ray_world(origin, dir, 20.0, w.boxes);
                w.emit(Fx::Lightning {
                    a: hand.to_array(),
                    b: end.to_array(),
                });
                return;
            };
            let mut hit: Vec<Entity> = Vec::new();
            for _ in 0..(7 + tier as usize) {
                let (e, at) = next;
                hit.push(e);
                w.hit(e, dmg, id, el, 0.5);
                w.emit(Fx::Lightning {
                    a: from.to_array(),
                    b: at.to_array(),
                });
                from = at;
                dmg *= 0.9;
                let Some(n) = w
                    .enemies
                    .iter()
                    .map(|(e, p)| (*e, *p + Vec3::Y))
                    .filter(|(e, p)| !hit.contains(e) && p.distance(at) < 9.0)
                    .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)))
                else {
                    break;
                };
                next = n;
            }
        }
        A::Bifrost => {
            // A beam from the sky sweeping out along where you look.
            let start = feet + flat * 2.0;
            let len = ray_world(start + Vec3::Y, flat, 36.0, w.boxes).max(6.0);
            let sweep = 2.2;
            let steps = 16;
            for i in 0..=steps {
                let f = i as f32 / steps as f32;
                let mut s = Strike::new(id, start + flat * len * f, 0.5 + f * sweep, 3.0, 150.0 * mult, shock);
                s.stun = 0.4;
                w.strike(s);
            }
            w.spell(ability, start, flat, len);
        }
        A::SpearRain => {
            let center = w.aim_ground(origin, dir, 50.0);
            let radius = 6.0 + 0.5 * tier;
            for i in 0..12 {
                let ang = rng.gen_range(0.0..std::f32::consts::TAU);
                let r = radius * rng.gen_range(0.0f32..1.0).sqrt();
                let at = center + Vec3::new(ang.cos(), 0.0, ang.sin()) * r;
                let delay = 0.35 + i as f32 * 0.09;
                w.emit(Fx::Falling {
                    kind: falling::SPEAR,
                    from: (at + Vec3::new(rng.gen_range(-2.0..2.0), 26.0, rng.gen_range(-2.0..2.0)))
                        .to_array(),
                    to: at.to_array(),
                    time: delay,
                });
                let mut s = Strike::new(id, at, delay, 2.4, (120.0 + 45.0 * tier) * mult, shock);
                s.stun = 0.5;
                s.fx = Some(Fx::Slam {
                    pos: at.to_array(),
                    radius: 2.4,
                });
                w.strike(s);
            }
            w.spell(ability, center, flat, radius);
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Delayed strikes land.
#[allow(clippy::too_many_arguments)]
pub(super) fn strikes(
    time: Res<Time>,
    mut strikes: ResMut<Strikes>,
    mut zones: ResMut<Zones>,
    mut damage: ResMut<DamageQueue>,
    mut fx: ResMut<FxQueue>,
    mut out: ResMut<FxOutbox>,
    enemies: Query<(Entity, &Transform), With<EnemyBrain>>,
) {
    let dt = time.delta_secs();
    for s in strikes.0.iter_mut() {
        s.delay -= dt;
    }
    let (ready, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut strikes.0)
        .into_iter()
        .partition(|s| s.delay <= 0.0);
    strikes.0 = waiting;
    for s in ready {
        let hit = |e: Entity, damage: &mut DamageQueue| {
            damage.0.push(DamageEvent {
                target: e,
                amount: s.damage,
                from: Some(s.owner),
                headshot: false,
                legs: false,
                elements: s.elements,
                chained: false,
                stun: s.stun,
            });
        };
        if let Some(target) = s.target {
            let Ok((e, tf)) = enemies.get(target) else {
                continue;
            };
            hit(e, &mut damage);
            if let Some(Fx::Spell { ability, dir, size, .. }) = s.fx.clone() {
                emit(
                    &mut fx,
                    &mut out,
                    Fx::Spell {
                        ability,
                        pos: (tf.translation + Vec3::Y * 1.1).to_array(),
                        dir,
                        size,
                    },
                );
            }
            continue;
        }
        for (e, t) in &enemies {
            if t.translation.with_y(0.0).distance(s.pos.with_y(0.0)) < s.radius {
                hit(e, &mut damage);
            }
        }
        if let Some(f) = s.fx.clone() {
            emit(&mut fx, &mut out, f);
        }
        if s.pool > 0.0 {
            let life = 4.0;
            let mut z = zone(s.owner, s.pos.with_y(0.0), s.pool, s.damage * 0.08, 0.3, life);
            z.elements = s.elements;
            z.kind = 2;
            emit(
                &mut fx,
                &mut out,
                Fx::Zone {
                    pos: z.pos.to_array(),
                    radius: z.radius,
                    life,
                    follow: 255,
                    kind: 2,
                },
            );
            zones.0.push(z);
        }
    }
}

/// Pull and push areas wear off; vanish and stim timers run down.
pub fn timers(time: Res<Time>, mut forces: ResMut<Forces>, mut roster: ResMut<Roster>) {
    let dt = time.delta_secs();
    for f in forces.0.iter_mut() {
        f.life -= dt;
    }
    forces.0.retain(|f| f.life > 0.0);
    for p in roster.0.values_mut() {
        p.stim = (p.stim - dt).max(0.0);
        p.vanish = (p.vanish - dt).max(0.0);
    }
}

/// Ability projectiles fly, hit, and burst.
#[allow(clippy::too_many_arguments)]
pub(super) fn missiles(
    mut commands: Commands,
    time: Res<Time>,
    mut zones: ResMut<Zones>,
    mut damage: ResMut<DamageQueue>,
    mut fx: ResMut<FxQueue>,
    mut out: ResMut<FxOutbox>,
    mut shots: Query<(Entity, &mut Transform, &mut Missile), Without<EnemyBrain>>,
    enemies: Query<(Entity, &Transform, &EnemyBrain)>,
    colliders: Query<(&Transform, &Collider), (Without<Missile>, Without<EnemyBrain>)>,
) {
    if shots.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    let boxes = collect_boxes(colliders.iter());
    let list: Vec<(Entity, Vec3)> = enemies
        .iter()
        .map(|(e, t, b)| (e, t.translation + Vec3::Y * enemy_scale(b.kind)))
        .collect();
    let positions: Vec<(Entity, Vec3)> = enemies.iter().map(|(e, t, _)| (e, t.translation)).collect();
    for (e, mut tf, mut m) in &mut shots {
        m.life -= dt;
        m.vel.y -= m.gravity * dt;
        if m.homing > 0.0 {
            let pos = tf.translation;
            let speed = m.vel.length();
            let ahead = m.vel / speed.max(1e-3);
            if let Some((_, target)) = list
                .iter()
                .filter(|(_, p)| {
                    let to = *p - pos;
                    to.length() < 18.0 && to.normalize_or_zero().dot(ahead) > 0.6
                })
                .min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)))
            {
                let want = (*target - pos).normalize_or_zero() * speed;
                let turn = (m.homing * dt).min(1.0);
                m.vel = m.vel.lerp(want, turn).normalize_or_zero() * speed;
            }
        }
        let from = tf.translation;
        let to = from + m.vel * dt;
        let mut stop = m.life <= 0.0;
        if to.y < 0.05 || !line_of_sight(from, to, &boxes) {
            stop = true;
        }
        // Hits along this step.
        let seg = to - from;
        let len2 = seg.length_squared().max(1e-6);
        for (enemy, chest) in &list {
            let t = ((*chest - from).dot(seg) / len2).clamp(0.0, 1.0);
            if chest.distance(from + seg * t) > m.hit_radius + 0.45 {
                continue;
            }
            if m.pierce {
                if m.hits.contains(enemy) {
                    continue;
                }
                m.hits.push(*enemy);
            } else {
                stop = true;
            }
            if m.damage > 0.0 {
                damage.0.push(DamageEvent {
                    target: *enemy,
                    amount: m.damage,
                    from: Some(m.owner),
                    headshot: false,
                    legs: false,
                    elements: m.elements,
                    chained: false,
                    stun: m.stun,
                });
            }
            if !m.pierce {
                break;
            }
        }
        if m.chill > 0.0 {
            m.chill_timer -= dt;
            if m.chill_timer <= 0.0 {
                m.chill_timer = 0.25;
                for (enemy, p) in &positions {
                    if p.distance(from.with_y(p.y)) < m.chill {
                        damage.0.push(DamageEvent {
                            target: *enemy,
                            amount: m.damage * 0.2,
                            from: Some(m.owner),
                            headshot: false,
                            legs: false,
                            elements: m.elements,
                            chained: true,
                            stun: 0.6,
                        });
                    }
                }
            }
        }
        if !stop {
            tf.translation = to;
            if m.vel.length_squared() > 1e-4 {
                tf.look_to(m.vel, Vec3::Y);
            }
            continue;
        }
        // Burst.
        let at = from;
        let color = elements_in(m.elements)
            .next()
            .map_or(Color::srgb(m.color[0], m.color[1], m.color[2]), |el| el.color());
        if m.blast > 0.0 {
            explode(
                at,
                m.blast,
                m.blast_damage,
                Some(m.owner),
                m.elements,
                &positions,
                &mut damage,
                &mut fx,
                &mut out,
                if m.end == End::Fizzle { color } else { Color::srgb(m.color[0], m.color[1], m.color[2]) },
            );
        } else {
            emit(
                &mut fx,
                &mut out,
                Fx::Ring {
                    pos: at.to_array(),
                    radius: m.hit_radius.max(1.0),
                    color: rgb(color),
                },
            );
        }
        match m.end {
            End::Fizzle => {}
            End::FirePool => {
                let life = 4.0;
                let mut z = zone(m.owner, at.with_y(0.0), 3.5, m.blast_damage * 0.1, 0.3, life);
                z.elements = m.elements;
                z.kind = 2;
                emit(
                    &mut fx,
                    &mut out,
                    Fx::Zone {
                        pos: z.pos.to_array(),
                        radius: z.radius,
                        life,
                        follow: 255,
                        kind: 2,
                    },
                );
                zones.0.push(z);
            }
            End::Shatter => {
                for (enemy, p) in &positions {
                    if p.distance(at.with_y(p.y)) < m.blast {
                        damage.0.push(DamageEvent {
                            target: *enemy,
                            amount: 0.0,
                            from: Some(m.owner),
                            headshot: false,
                            legs: false,
                            elements: m.elements,
                            chained: true,
                            stun: 1.8,
                        });
                    }
                }
                emit(
                    &mut fx,
                    &mut out,
                    Fx::Nova {
                        pos: at.with_y(0.0).to_array(),
                        radius: m.blast,
                    },
                );
            }
        }
        commands.entity(e).despawn();
    }
}

/// Thrown grenades bounce and go off.
#[allow(clippy::too_many_arguments)]
pub(super) fn grenades(
    mut commands: Commands,
    time: Res<Time>,
    mut state: ResMut<MatchState>,
    mut zones: ResMut<Zones>,
    mut strikes: ResMut<Strikes>,
    mut forces: ResMut<Forces>,
    mut damage: ResMut<DamageQueue>,
    mut fx: ResMut<FxQueue>,
    mut out: ResMut<FxOutbox>,
    mut nades: Query<(Entity, &mut Transform, &mut GrenadeBrain), Without<EnemyBrain>>,
    enemies: Query<(Entity, &Transform), With<EnemyBrain>>,
    colliders: Query<(&Transform, &Collider), (Without<GrenadeBrain>, Without<EnemyBrain>)>,
) {
    let dt = time.delta_secs();
    if nades.is_empty() {
        return;
    }
    let boxes = collect_boxes(colliders.iter());
    let enemy_list: Vec<(Entity, Vec3)> = enemies.iter().map(|(e, t)| (e, t.translation)).collect();
    let mut rng = rand::thread_rng();
    for (e, mut tf, mut g) in &mut nades {
        g.fuse -= dt;
        g.velocity.y -= crate::abilities::GRENADE_GRAVITY * dt;
        let next = tf.translation + g.velocity * dt;
        // Firebombs and grav grenades burst on landing; mines stick and arm;
        // the rest bounce off the floor and stop at walls.
        let sticks = matches!(g.kind, Nade::Fire | Nade::Grav | Nade::Mine);
        let mut landed = false;
        if next.y < 0.1 && sticks {
            landed = true;
            tf.translation.y = 0.1;
        } else if next.y < 0.1 {
            g.velocity.y = -g.velocity.y * 0.35;
            g.velocity *= 0.6;
            tf.translation.y = 0.1;
        } else if !line_of_sight(tf.translation, next, &boxes) {
            g.velocity = -g.velocity * 0.3;
        } else {
            tf.translation = next;
        }
        if g.kind == Nade::Mine {
            if landed {
                let id = state.next_net_id;
                state.next_net_id += 1;
                commands.spawn((
                    crate::InGameEntity,
                    Replicated {
                        id,
                        kind: NetKind::Mine,
                    },
                    Transform::from_translation(tf.translation.with_y(0.02)),
                    MineBrain {
                        owner: g.owner,
                        arm: 0.8,
                        life: g.fuse,
                        damage: g.damage,
                        radius: g.radius,
                        elements: g.elements,
                    },
                ));
                commands.entity(e).despawn();
            }
            continue;
        }
        let touching = g.kind != Nade::Smoke
            && enemy_list
                .iter()
                .any(|(_, p)| (*p + Vec3::Y).distance(tf.translation) < 1.0);
        if !(g.fuse <= 0.0 || touching || landed) {
            continue;
        }
        let at = tf.translation;
        let ground = at.with_y(0.0);
        let mut color = Color::srgb(1.0, 0.55, 0.15);
        if let Some(el) = elements_in(g.elements).next() {
            color = el.color();
        }
        let mut add_zone = |z: Zone, fx: &mut FxQueue, out: &mut FxOutbox| {
            emit(
                fx,
                out,
                Fx::Zone {
                    pos: z.pos.to_array(),
                    radius: z.radius,
                    life: z.life,
                    follow: 255,
                    kind: z.kind,
                },
            );
            zones.0.push(z);
        };
        match g.kind {
            Nade::Frag | Nade::Mine => {}
            Nade::Fire => {
                let mut z = zone(g.owner, ground, g.radius, g.damage * 0.4, 0.3, 6.0);
                z.elements = g.elements;
                z.kind = 2;
                add_zone(z, &mut fx, &mut out);
                color = Color::srgb(1.0, 0.45, 0.1);
            }
            Nade::Cluster => {
                // Six bomblets scatter and go off one after another.
                for i in 0..6 {
                    let ang = i as f32 / 6.0 * std::f32::consts::TAU + rng.gen_range(-0.3..0.3);
                    let out_dir = Vec3::new(ang.cos(), 0.0, ang.sin());
                    let mut m = Missile::new(
                        g.owner,
                        out_dir * rng.gen_range(4.0..7.0) + Vec3::Y * rng.gen_range(4.0..6.5),
                        rng.gen_range(0.55..0.95),
                        0.0,
                        g.elements,
                    );
                    m.gravity = 16.0;
                    m.blast = 2.8;
                    m.blast_damage = g.damage * 0.8;
                    let id = state.next_net_id;
                    state.next_net_id += 1;
                    commands.spawn((
                        crate::InGameEntity,
                        Replicated {
                            id,
                            kind: NetKind::Missile(look::BOMBLET),
                        },
                        Transform::from_translation(at + Vec3::Y * 0.3),
                        m,
                    ));
                }
            }
            Nade::Smoke => {
                let mut z = zone(g.owner, ground, g.radius, g.damage, 0.5, 6.5);
                z.elements = g.elements;
                z.kind = zone::SMOKE;
                add_zone(z, &mut fx, &mut out);
                commands.entity(e).despawn();
                continue;
            }
            Nade::Grav => {
                let life = 2.6;
                let mut z = zone(g.owner, ground, g.radius, g.damage * 0.05, 0.25, life);
                z.elements = g.elements;
                z.kind = zone::SINGULARITY;
                add_zone(z, &mut fx, &mut out);
                forces.0.push(Force {
                    pos: ground,
                    radius: g.radius + 2.0,
                    strength: 9.0,
                    life,
                });
                let mut s = Strike::new(g.owner, ground, life, g.radius, g.damage, g.elements);
                s.fx = Some(Fx::Explosion {
                    pos: (ground + Vec3::Y).to_array(),
                    radius: g.radius,
                    color: [0.75, 0.4, 1.0],
                });
                strikes.0.push(s);
                commands.entity(e).despawn();
                continue;
            }
        }
        explode(
            at + Vec3::Y * 0.3,
            g.radius,
            g.damage,
            Some(g.owner),
            g.elements,
            &enemy_list,
            &mut damage,
            &mut fx,
            &mut out,
            color,
        );
        commands.entity(e).despawn();
    }
}

/// Mines wait for a zombie to come close.
#[allow(clippy::too_many_arguments)]
pub(super) fn mines(
    mut commands: Commands,
    time: Res<Time>,
    mut damage: ResMut<DamageQueue>,
    mut fx: ResMut<FxQueue>,
    mut out: ResMut<FxOutbox>,
    mut mines: Query<(Entity, &Transform, &mut MineBrain), Without<EnemyBrain>>,
    enemies: Query<(Entity, &Transform), With<EnemyBrain>>,
) {
    if mines.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    let list: Vec<(Entity, Vec3)> = enemies.iter().map(|(e, t)| (e, t.translation)).collect();
    for (e, tf, mut m) in &mut mines {
        m.arm -= dt;
        m.life -= dt;
        let pos = tf.translation;
        let tripped = m.arm <= 0.0 && list.iter().any(|(_, p)| p.with_y(0.0).distance(pos.with_y(0.0)) < 2.4);
        if tripped || m.life <= 0.0 {
            explode(
                pos + Vec3::Y * 0.4,
                m.radius,
                m.damage,
                Some(m.owner),
                m.elements,
                &list,
                &mut damage,
                &mut fx,
                &mut out,
                Color::srgb(1.0, 0.4, 0.2),
            );
            commands.entity(e).despawn();
        }
    }
}

/// Sentries and drones pick the nearest enemy they can see and shoot it.
#[allow(clippy::too_many_arguments)]
pub(super) fn turrets(
    mut commands: Commands,
    time: Res<Time>,
    roster: Res<Roster>,
    mut damage: ResMut<DamageQueue>,
    mut fx: ResMut<FxQueue>,
    mut out: ResMut<FxOutbox>,
    mut turrets: Query<(Entity, &mut Transform, &mut TurretBrain), Without<EnemyBrain>>,
    enemies: Query<(Entity, &Transform, &EnemyBrain)>,
    colliders: Query<(&Transform, &Collider), (Without<TurretBrain>, Without<EnemyBrain>)>,
) {
    if turrets.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    let t_now = time.elapsed_secs();
    let boxes = collect_boxes(colliders.iter());
    for (e, mut tf, mut t) in &mut turrets {
        t.life -= dt;
        let owner = t.follow.and_then(|id| roster.0.get(&id));
        if t.life <= 0.0 || (t.follow.is_some() && owner.is_none_or(|p| !p.alive)) {
            emit(
                &mut fx,
                &mut out,
                Fx::Explosion {
                    pos: (tf.translation + Vec3::Y * 0.6).to_array(),
                    radius: 1.2,
                    color: [0.3, 0.9, 1.0],
                },
            );
            commands.entity(e).despawn();
            continue;
        }
        // The drone hovers over your right shoulder, bobbing.
        if let Some(p) = owner {
            let fwd = Vec3::new(-p.yaw.sin(), 0.0, -p.yaw.cos());
            let right = Vec3::new(-fwd.z, 0.0, fwd.x);
            let want = p.feet() + Vec3::Y * (2.3 + 0.15 * (t_now * 2.5).sin()) + right * 1.0 - fwd * 0.3;
            let k = (dt * 5.0).min(1.0);
            tf.translation = tf.translation.lerp(want, k);
        }
        t.cooldown -= dt;
        let (gun, range) = if t.follow.is_some() {
            (tf.translation, 22.0)
        } else {
            (tf.translation + Vec3::Y * 0.85, 24.0)
        };
        let target = enemies
            .iter()
            .map(|(e, et, b)| (e, et.translation + Vec3::Y * 1.1 * enemy_scale(b.kind)))
            .filter(|(_, p)| p.distance(gun) < range && line_of_sight(gun, *p, &boxes))
            .min_by(|a, b| a.1.distance(gun).total_cmp(&b.1.distance(gun)));
        let Some((enemy, at)) = target else { continue };
        let flat = (at - gun).with_y(0.0).normalize_or(Vec3::NEG_Z);
        tf.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, flat);
        if t.cooldown <= 0.0 {
            t.cooldown = if t.follow.is_some() { 0.2 } else { 0.15 };
            damage.0.push(DamageEvent {
                target: enemy,
                amount: t.damage,
                from: Some(t.owner),
                headshot: false,
                legs: false,
                elements: t.elements,
                chained: false,
                stun: 0.0,
            });
            emit(
                &mut fx,
                &mut out,
                Fx::Tracer {
                    shooter: 255,
                    a: (gun + (at - gun).normalize_or_zero() * 0.5).to_array(),
                    b: at.to_array(),
                },
            );
        }
    }
}
