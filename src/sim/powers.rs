//! What every ability does on the host, plus the things abilities leave
//! behind: darts, chains and bombs, thrown canisters, claymores and traps,
//! the med drone and spectral warriors, delayed strikes and the pull/push
//! areas zombies feel.

use bevy::prelude::*;
use rand::Rng;

use super::{
    enemy_scale, explode, level_multiplier, DamageEvent, DamageQueue, EnemyBrain, Strikes, Zone,
    Zones,
};
use crate::abilities::{move_time, DASH_SPEED, GRAPPLE_RANGE};
use crate::data::{elements_in, Ability, Element};
use crate::fx::{emit, rgb, Fx, FxOutbox, FxQueue};
use crate::physics::{collect_boxes, line_of_sight, ray_world, Boxes};
use crate::{Collider, MatchState, NetKind, Replicated, Roster};

/// Projectile and gadget looks (`NetKind::Missile`).
pub mod look {
    /// Medic's neurotoxin dart.
    pub const DART: u8 = 0;
    /// Demolisher's sticky bomb.
    pub const STICKY: u8 = 1;
    /// Medic's healing canister.
    pub const MEDKIT: u8 = 2;
    /// Chemist's acid flask.
    pub const FLASK: u8 = 3;
    /// Ranger's bear trap (thrown, then lying open).
    pub const TRAP: u8 = 4;
    /// Demolisher's claymore.
    pub const CLAYMORE: u8 = 5;
    pub const LAST: u8 = CLAYMORE;
}

/// Things falling from the sky (`Fx::Falling`).
pub mod falling {
    /// Payload's huge bomb.
    pub const BOMB: u8 = 0;
    /// Arrow Storm.
    pub const ARROW: u8 = 1;
}

/// Zone kinds (`Fx::Zone`).
pub mod zone {
    /// Bulwark's Fortress: a golden aura that follows them.
    pub const FORTRESS: u8 = 0;
    /// Medic's healing mist.
    pub const HEAL: u8 = 1;
    /// A pool of fire (Dragon's Breath and the like).
    pub const FIRE: u8 = 2;
    /// Revenant's Reaper: spectral scythes whirling round them.
    pub const REAPER: u8 = 3;
    /// Chemist's acid pool.
    pub const ACID: u8 = 4;
    /// Chemist's toxic cloud.
    pub const TOXIC: u8 = 5;
    /// Chemist's Plague Bloom: spreads at `PLAGUE_GROW` up to `PLAGUE_MAX`.
    pub const PLAGUE: u8 = 6;
}

pub const PLAGUE_START: f32 = 3.0;
pub const PLAGUE_GROW: f32 = 1.0;
pub const PLAGUE_MAX: f32 = 12.0;

/// Extra bits in a damage event's element mask (above the real elements)
/// for what abilities do to zombies.
pub mod effect {
    /// Poisons: damage over time and a short slow.
    pub const POISON: u8 = 1 << 5;
    /// Hunter's Mark.
    pub const MARK: u8 = 1 << 6;
    /// Heals whoever dealt it (`DRAIN_FRACTION` of the damage).
    pub const DRAIN: u8 = 1 << 7;
}

/// Hunter's Mark: how long it lasts and the extra damage taken.
pub const MARK_TIME: f32 = 8.0;
pub const MARK_BONUS: f32 = 1.5;
/// Share of scythe damage the Revenant gets back as health.
pub const DRAIN_FRACTION: f32 = 0.12;

/// A hit that lands after a delay.
pub struct Strike {
    pub owner: u8,
    pub pos: Vec3,
    pub delay: f32,
    pub radius: f32,
    pub damage: f32,
    pub elements: u8,
    pub stun: f32,
    /// Hits this one enemy wherever it has got to (Deadeye, Petrify).
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
    /// Only this zombie feels it (Soul Chains).
    pub only: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct Forces(pub Vec<Force>);

/// What a thrown thing does when it lands.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Nade {
    /// Sticks to the first zombie or surface and blows when the fuse runs out.
    Sticky,
    /// Bursts into a healing mist.
    Heal,
    /// Shatters into an acid pool.
    Acid,
    /// Lands and lies open as a bear trap.
    Trap,
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
    /// What a sticky bomb is stuck to, and where on it.
    pub stuck: Option<(Entity, Vec3)>,
    pub landed: bool,
    /// How long a bear trap holds its zombie.
    pub stun: f32,
}

impl GrenadeBrain {
    fn new(owner: u8, velocity: Vec3, fuse: f32, damage: f32, radius: f32, elements: u8, kind: Nade) -> Self {
        Self {
            owner,
            velocity,
            fuse,
            damage,
            radius,
            elements,
            kind,
            stuck: None,
            landed: false,
            stun: 0.0,
        }
    }
}

/// The Medic's drone (follows its owner, heals and zaps), or one of the
/// Revenant's spectral warriors (`wraith`: chases zombies and cuts them).
#[derive(Component)]
pub struct TurretBrain {
    pub owner: u8,
    pub life: f32,
    pub cooldown: f32,
    pub damage: f32,
    pub elements: u8,
    pub follow: Option<u8>,
    /// Heals players near the owner by this much a second.
    pub heal: f32,
    pub heal_timer: f32,
    pub wraith: bool,
    /// Where a warrior waits beside its owner (0-3).
    pub slot: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MineKind {
    /// Blasts a cone toward `facing`.
    Claymore,
    /// Snaps on one zombie and holds it.
    Trap,
}

#[derive(Component)]
pub struct MineBrain {
    owner: u8,
    arm: f32,
    life: f32,
    damage: f32,
    radius: f32,
    elements: u8,
    kind: MineKind,
    facing: Vec3,
    stun: f32,
}

/// What a projectile does when it stops.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum End {
    Fizzle,
    /// Neurotoxin: the poison jumps to zombies nearby.
    Spread,
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

    /// Enemies in a cone: within `range` of `origin` and `cos` of `dir`
    /// (or right on top of you), that you can see.
    fn cone(&self, origin: Vec3, dir: Vec3, range: f32, cos: f32) -> Vec<(Entity, Vec3)> {
        self.enemies
            .iter()
            .filter(|(_, p)| {
                let to = *p + Vec3::Y - origin;
                let d = to.length();
                d < range && (d < 2.0 || to.normalize().dot(dir) > cos) && line_of_sight(origin, *p + Vec3::Y, self.boxes)
            })
            .copied()
            .collect()
    }

    /// Enemies within `width` of the line from `a` to `b` (flat).
    fn along(&self, a: Vec3, b: Vec3, width: f32) -> Vec<(Entity, Vec3)> {
        let (a, b) = (a.with_y(0.0), b.with_y(0.0));
        let ab = b - a;
        self.enemies
            .iter()
            .filter(|(_, p)| {
                let q = p.with_y(0.0);
                let t = ((q - a).dot(ab) / ab.length_squared().max(1e-4)).clamp(0.0, 1.0);
                q.distance(a + ab * t) < width
            })
            .copied()
            .collect()
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
        heal: 0.0,
        grow: 0.0,
        max_radius: radius,
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
/// `origin` along `dir`, charged up to `charge` (0 to 1, unused since v12).
#[allow(clippy::too_many_arguments)]
pub(super) fn cast(
    w: &mut World,
    roster: &mut Roster,
    id: u8,
    slot: u8,
    origin: Vec3,
    dir: Vec3,
    _charge: f32,
) {
    use effect::{DRAIN, MARK, POISON};
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
    let throw = |speed: f32| dir * speed + Vec3::Y * crate::abilities::GRENADE_LIFT;
    let dash_end = |w: &World| {
        let len = DASH_SPEED * move_time(ability, tier);
        let dist = ray_world(feet + Vec3::Y * 0.5, flat, len, w.boxes);
        feet + flat * (dist - 0.4).max(0.0)
    };
    use Ability as A;
    match ability {
        // --- Bulwark ---------------------------------------------------------
        A::ShieldCharge => {
            // The charge itself is done locally; bowl over everything on
            // the way and shove it aside.
            let end = dash_end(w);
            let dmg = (90.0 + 35.0 * tier) * mult;
            for (e, pos) in w.along(feet, end, 1.9) {
                w.hit(e, dmg, id, el, 1.2 + 0.15 * tier);
                let side = if right.dot(pos - feet) >= 0.0 { 1.0 } else { -1.0 };
                w.forces.0.push(Force {
                    pos: pos - right * side,
                    radius: 3.0,
                    strength: -14.0,
                    life: 0.35,
                    only: Some(e),
                });
            }
            if let Some(p) = roster.0.get_mut(&id) {
                p.guard = p.guard.max(0.6);
                p.guard_cut = p.guard_cut.max(0.6);
            }
            w.send(Fx::Dash {
                player: id,
                a: feet.to_array(),
                b: end.to_array(),
            });
            w.spell(ability, feet, flat, feet.distance(end));
        }
        A::GroundPound => {
            let radius = 6.0 + 0.4 * tier;
            let dmg = (120.0 + 45.0 * tier) * mult;
            for (e, _) in w.near(feet, radius) {
                w.hit(e, dmg, id, el, 1.6 + 0.2 * tier);
            }
            w.spell(ability, feet, flat, radius);
        }
        A::Fortress => {
            let life = 8.0 + tier;
            if let Some(p) = roster.0.get_mut(&id) {
                p.guard = life;
                p.guard_cut = 0.8;
            }
            let mut z = zone(id, feet, 5.0, 60.0 * mult, 0.4, life);
            z.follow = Some(id);
            z.elements = fire;
            z.kind = zone::FORTRESS;
            w.zone(z);
            w.spell(ability, feet, flat, 5.0);
        }
        A::RallyCry => {
            let radius = 10.0;
            let time = 6.0 + tier;
            for other in roster.0.values_mut() {
                if other.alive && other.feet().distance(feet) < radius {
                    other.health = (other.health + other.max_health() * (0.3 + 0.05 * tier)).min(other.max_health());
                    other.guard = other.guard.max(time);
                    other.guard_cut = other.guard_cut.max(0.4);
                    other.stim = other.stim.max(time);
                }
            }
            w.spell(ability, feet, flat, radius);
        }
        A::Earthshaker => {
            // Three shockwaves, each wider than the last.
            for (i, radius) in [6.0, 10.0, 14.0].into_iter().enumerate() {
                let delay = 0.15 + 0.55 * i as f32;
                let mut s = Strike::new(id, feet, delay, radius, 450.0 * mult, el);
                s.stun = 1.5;
                s.fx = Some(Fx::Spell {
                    ability,
                    pos: feet.to_array(),
                    dir: flat.to_array(),
                    size: radius,
                });
                w.strike(s);
            }
            w.spell(ability, feet, flat, 0.0);
        }

        // --- Medic -----------------------------------------------------------
        A::HealingGrenade => {
            w.nade(
                NetKind::Missile(look::MEDKIT),
                origin + dir * 0.6,
                GrenadeBrain::new(
                    id,
                    throw(crate::abilities::GRENADE_SPEED),
                    1.2,
                    12.0 + 4.0 * tier,
                    5.0 + 0.3 * tier,
                    el,
                    Nade::Heal,
                ),
            );
        }
        A::NeurotoxinDart => {
            let mut m = Missile::new(id, dir * 55.0, 0.8, (80.0 + 30.0 * tier) * mult, el | POISON);
            m.stun = 2.0 + 0.2 * tier;
            m.hit_radius = 0.4;
            m.end = End::Spread;
            m.color = [0.7, 1.0, 0.2];
            w.missile(look::DART, hand, m);
        }
        A::Resurrection => {
            for other in roster.0.values_mut() {
                if !other.alive {
                    other.alive = true;
                    let at = other.feet();
                    w.emit(Fx::Spell {
                        ability,
                        pos: at.to_array(),
                        dir: flat.to_array(),
                        size: 1.0,
                    });
                }
                other.health = other.max_health();
                other.guard = other.guard.max(3.0);
                other.guard_cut = other.guard_cut.max(0.5);
            }
            w.spell(ability, feet, flat, 0.0);
        }
        A::MedDrone => {
            w.spawn(
                NetKind::Drone,
                Transform::from_translation(feet + Vec3::Y * 2.2 + right * 0.8),
                TurretBrain {
                    owner: id,
                    life: 18.0 + 4.0 * tier,
                    cooldown: 0.3,
                    damage: (14.0 + 5.0 * tier) * mult,
                    elements: el,
                    follow: Some(id),
                    heal: 8.0 + 3.0 * tier,
                    heal_timer: 0.5,
                    wraith: false,
                    slot: 0,
                },
            );
            w.emit(Fx::Ring {
                pos: (feet + Vec3::Y * 2.0).to_array(),
                radius: 1.2,
                color: [0.4, 1.0, 0.8],
            });
        }
        A::Sterilize => {
            let radius = 16.0;
            for (e, pos) in w.near(feet, radius) {
                // The wave reaches the far ones a moment later.
                let mut s = Strike::new(id, pos, pos.distance(feet) / 30.0, 0.5, 700.0 * mult, el);
                s.target = Some(e);
                s.stun = 1.0;
                w.strike(s);
            }
            w.spell(ability, feet, flat, radius);
        }

        // --- Revenant --------------------------------------------------------
        A::ScytheSweep => {
            let radius = 4.5 + 0.3 * tier;
            let dmg = (110.0 + 40.0 * tier) * mult;
            for (e, _) in w.near(feet, radius) {
                w.hit(e, dmg, id, el | DRAIN, 0.3);
            }
            w.spell(ability, feet + Vec3::Y * 1.0, flat, radius);
        }
        A::SoulChains => {
            // Chains lash out to the closest zombies in front and haul them in.
            let mut caught = w.cone(origin, dir, 15.0, 0.85);
            caught.sort_by(|a, b| a.1.distance(feet).total_cmp(&b.1.distance(feet)));
            let dmg = (60.0 + 25.0 * tier) * mult;
            for (e, pos) in caught.into_iter().take(5 + tier as usize / 2) {
                w.hit(e, dmg, id, el, 1.5 + 0.2 * tier);
                w.forces.0.push(Force {
                    pos: feet + flat * 1.2,
                    radius: 20.0,
                    strength: 18.0,
                    life: (pos.distance(feet) / 18.0).clamp(0.15, 0.8),
                    only: Some(e),
                });
                let chest = pos + Vec3::Y * 1.1;
                w.spell(ability, hand, chest - hand, 1.0);
            }
            if w.cone(origin, dir, 15.0, 0.85).is_empty() {
                // A miss still shows the chains lashing out.
                let reach = ray_world(origin, dir, 15.0, w.boxes);
                w.spell(ability, hand, dir * reach, 0.0);
            }
        }
        A::Reaper => {
            let life = 7.0 + tier;
            let mut z = zone(id, feet, 5.5, 90.0 * mult, 0.3, life);
            z.follow = Some(id);
            z.elements = el | DRAIN;
            z.kind = zone::REAPER;
            w.zone(z);
        }
        A::WraithStep => {
            // The drift is done locally; you're mist for a moment, and
            // everything you pass through is chilled to the bone.
            let end = dash_end(w);
            let dmg = (50.0 + 20.0 * tier) * mult;
            for (e, _) in w.along(feet, end, 1.6) {
                w.hit(e, dmg, id, el, 0.8);
            }
            if let Some(p) = roster.0.get_mut(&id) {
                p.vanish = p.vanish.max(1.2 + 0.1 * tier);
            }
            w.send(Fx::Dash {
                player: id,
                a: feet.to_array(),
                b: end.to_array(),
            });
            w.spell(ability, feet, flat, feet.distance(end));
        }
        A::ArmyOfTheDead => {
            for i in 0..4u8 {
                let ang = i as f32 / 4.0 * std::f32::consts::TAU;
                let at = feet + Vec3::new(ang.cos(), 0.0, ang.sin()) * 2.0;
                w.spawn(
                    NetKind::Wraith,
                    Transform::from_translation(at).looking_to(flat, Vec3::Y),
                    TurretBrain {
                        owner: id,
                        life: 12.0 + tier,
                        cooldown: 0.4 + 0.1 * i as f32,
                        damage: 90.0 * mult,
                        elements: el | DRAIN,
                        follow: Some(id),
                        heal: 0.0,
                        heal_timer: 0.0,
                        wraith: true,
                        slot: i,
                    },
                );
                w.emit(Fx::Spell {
                    ability,
                    pos: at.to_array(),
                    dir: flat.to_array(),
                    size: 1.0,
                });
            }
            w.spell(ability, feet, flat, 0.0);
        }

        // --- Demolisher ------------------------------------------------------
        A::StickyBomb => {
            w.nade(
                NetKind::Missile(look::STICKY),
                origin + dir * 0.6,
                GrenadeBrain::new(
                    id,
                    throw(crate::abilities::GRENADE_SPEED * 1.2),
                    1.6,
                    (220.0 + 80.0 * tier) * mult,
                    4.5,
                    el,
                    Nade::Sticky,
                ),
            );
        }
        A::BlastJump => {
            let radius = 4.5;
            let dmg = (130.0 + 50.0 * tier) * mult;
            for (e, _) in w.near(feet, radius) {
                w.hit(e, dmg, id, el, 0.5);
            }
            let end = dash_end(w);
            w.send(Fx::Dash {
                player: id,
                a: feet.to_array(),
                b: end.to_array(),
            });
            w.emit(blast(feet, radius, [1.0, 0.55, 0.15]));
            w.spell(ability, feet, flat, radius);
        }
        A::Payload => {
            let target = w.aim_ground(origin, dir, 70.0);
            let radius = 11.0;
            let delay = 1.8;
            w.emit(Fx::Falling {
                kind: falling::BOMB,
                from: (target - flat * 6.0 + Vec3::Y * 40.0).to_array(),
                to: target.to_array(),
                time: delay,
            });
            let mut s = Strike::new(id, target, delay, radius, 3500.0 * mult, fire);
            s.stun = 1.0;
            s.pool = 5.0;
            s.fx = Some(Fx::Spell {
                ability,
                pos: target.to_array(),
                dir: flat.to_array(),
                size: 0.0,
            });
            w.strike(s);
            w.spell(ability, target, flat, radius);
        }
        A::Claymore => {
            let dist = ray_world(feet + Vec3::Y * 0.5, flat, 2.5, w.boxes);
            let at = feet + flat * (dist - 0.5).max(0.4);
            w.spawn(
                NetKind::Missile(look::CLAYMORE),
                Transform::from_translation(at.with_y(0.0)).looking_to(flat, Vec3::Y),
                MineBrain {
                    owner: id,
                    arm: 0.8,
                    life: 45.0,
                    damage: (260.0 + 90.0 * tier) * mult,
                    radius: 8.0,
                    elements: el,
                    kind: MineKind::Claymore,
                    facing: flat,
                    stun: 0.6,
                },
            );
        }
        A::ChainReaction => {
            if let Some(p) = roster.0.get_mut(&id) {
                p.chain = 10.0 + tier;
            }
            w.spell(ability, feet, flat, 1.0);
        }

        // --- Chemist ---------------------------------------------------------
        A::AcidFlask => {
            w.nade(
                NetKind::Missile(look::FLASK),
                origin + dir * 0.6,
                GrenadeBrain::new(
                    id,
                    throw(crate::abilities::GRENADE_SPEED),
                    3.0,
                    (35.0 + 12.0 * tier) * mult,
                    3.5 + 0.3 * tier,
                    el | POISON,
                    Nade::Acid,
                ),
            );
        }
        A::ToxicCloud => {
            // Three puffs along the spray, wider further out.
            for (i, (d, r)) in [(2.5, 2.4), (5.5, 3.0), (9.0, 3.8)].into_iter().enumerate() {
                let reach = ray_world(feet + Vec3::Y, flat, d + 1.0, w.boxes) - 1.0;
                let at = feet + flat * reach.min(d).max(0.5);
                let mut z = zone(id, at, r, (20.0 + 8.0 * tier) * mult, 0.5, 6.0 + 0.5 * tier);
                z.timer = 0.1 * i as f32;
                z.elements = el | POISON;
                z.kind = zone::TOXIC;
                w.zone(z);
            }
            w.spell(ability, hand, dir, 10.0);
        }
        A::PlagueBloom => {
            let at = w.aim_ground(origin, dir, 50.0);
            let mut z = zone(id, at, PLAGUE_START, 90.0 * mult, 0.4, 12.0 + tier);
            z.elements = el | POISON;
            z.kind = zone::PLAGUE;
            z.grow = PLAGUE_GROW;
            z.max_radius = PLAGUE_MAX;
            w.zone(z);
        }
        A::Catalyst => {
            let dmg = (250.0 + 90.0 * tier) * mult;
            let mut pops = Vec::new();
            for z in w.zones.0.iter_mut() {
                if z.owner == id && matches!(z.kind, zone::ACID | zone::TOXIC | zone::PLAGUE) && z.life > 0.0 {
                    pops.push((z.pos, z.radius + 1.5, z.kind));
                    z.life = 0.0;
                }
            }
            for (pos, radius, kind) in pops {
                for (e, _) in w.near(pos, radius) {
                    w.hit(e, dmg, id, el | POISON, 0.6);
                }
                w.emit(Fx::ZoneEnd {
                    pos: pos.to_array(),
                    kind,
                });
                w.spell(ability, pos, flat, radius);
            }
            w.spell(ability, hand, dir, 0.0);
        }
        A::Petrify => {
            // Turned to stone where they stand, then they shatter.
            let hit = w.cone(origin, dir, 18.0, 0.7);
            for (i, (e, _)) in hit.into_iter().enumerate() {
                w.hit(e, 0.0, id, el, 4.0);
                let mut s = Strike::new(id, feet, 2.6 + 0.04 * i as f32, 1.0, 900.0 * mult, el);
                s.target = Some(e);
                s.fx = Some(Fx::Spell {
                    ability,
                    pos: [0.0; 3],
                    dir: flat.to_array(),
                    size: 0.0,
                });
                w.strike(s);
            }
            w.spell(ability, hand, dir, 18.0);
        }

        // --- Ranger ----------------------------------------------------------
        A::Grapple => {
            // The zip is done locally; everyone sees the line.
            let reach = ray_world(origin, dir, GRAPPLE_RANGE, w.boxes);
            w.spell(ability, hand, dir, reach);
        }
        A::HuntersMark => {
            let dmg = (30.0 + 10.0 * tier) * mult;
            for (e, pos) in w.cone(origin, dir, 28.0, 0.82) {
                w.hit(e, dmg, id, el | MARK, 0.0);
                w.emit(Fx::Spell {
                    ability,
                    pos: (pos + Vec3::Y * 2.3).to_array(),
                    dir: flat.to_array(),
                    size: 1.0,
                });
            }
            w.spell(ability, hand, dir, 0.0);
        }
        A::Deadeye => {
            // Lock on to the ten zombies nearest your aim, then fire.
            let mut targets = w.cone(origin, dir, 60.0, 0.45);
            let off = |p: Vec3| (p + Vec3::Y - origin).normalize_or_zero().dot(dir);
            targets.sort_by(|a, b| off(b.1).total_cmp(&off(a.1)));
            for (i, (e, pos)) in targets.into_iter().take(10).enumerate() {
                let delay = 0.7 + 0.12 * i as f32;
                let mut s = Strike::new(id, pos, delay, 1.0, 1400.0 * mult, el);
                s.target = Some(e);
                s.stun = 0.5;
                s.fx = Some(Fx::Spell {
                    ability,
                    pos: [0.0; 3],
                    dir: dir.to_array(),
                    size: 1.0,
                });
                w.strike(s);
                w.emit(Fx::Spell {
                    ability,
                    pos: (pos + Vec3::Y * 1.2).to_array(),
                    dir: dir.to_array(),
                    size: 0.0,
                });
            }
        }
        A::BearTrap => {
            let mut g = GrenadeBrain::new(
                id,
                throw(crate::abilities::GRENADE_SPEED * 0.85),
                45.0,
                (150.0 + 50.0 * tier) * mult,
                1.8,
                el,
                Nade::Trap,
            );
            g.stun = 5.0 + 0.5 * tier;
            w.nade(NetKind::Missile(look::TRAP), origin + dir * 0.6, g);
        }
        A::ArrowStorm => {
            let center = w.aim_ground(origin, dir, 60.0);
            let radius = 8.0;
            w.spell(ability, center, flat, radius);
            for i in 0..36 {
                let ang = rng.gen_range(0.0..std::f32::consts::TAU);
                let r = radius * rng.gen_range(0.0f32..1.0).sqrt();
                let at = center + Vec3::new(ang.cos(), 0.0, ang.sin()) * r;
                let delay = 0.6 + i as f32 * 0.08 + rng.gen_range(0.0..0.06);
                // Every other arrow is shown; all of them hit.
                if i % 2 == 0 {
                    w.emit(Fx::Falling {
                        kind: falling::ARROW,
                        from: (at - flat * 8.0 + Vec3::Y * 26.0).to_array(),
                        to: at.to_array(),
                        time: delay,
                    });
                }
                let mut s = Strike::new(id, at, delay, 2.0, 260.0 * mult, el);
                s.stun = 0.4;
                w.strike(s);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn event(target: Entity, amount: f32, from: u8, elements: u8, stun: f32) -> DamageEvent {
    DamageEvent {
        target,
        amount,
        from: Some(from),
        headshot: false,
        legs: false,
        elements,
        chained: false,
        stun,
    }
}

fn add_zone(z: Zone, zones: &mut Zones, fx: &mut FxQueue, out: &mut FxOutbox) {
    emit(
        fx,
        out,
        Fx::Zone {
            pos: z.pos.to_array(),
            radius: z.radius,
            life: z.life,
            follow: z.follow.unwrap_or(255),
            kind: z.kind,
        },
    );
    zones.0.push(z);
}

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
        if let Some(target) = s.target {
            let Ok((e, tf)) = enemies.get(target) else {
                continue;
            };
            damage.0.push(event(e, s.damage, s.owner, s.elements, s.stun));
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
                damage.0.push(event(e, s.damage, s.owner, s.elements, s.stun));
            }
        }
        if let Some(f) = s.fx.clone() {
            emit(&mut fx, &mut out, f);
        }
        if s.pool > 0.0 {
            let mut z = zone(s.owner, s.pos.with_y(0.0), s.pool, s.damage * 0.02, 0.3, 5.0);
            z.elements = s.elements | Element::Fire.bit();
            z.kind = zone::FIRE;
            add_zone(z, &mut zones, &mut fx, &mut out);
        }
    }
}

/// Pull and push areas wear off; buff timers run down.
pub fn timers(time: Res<Time>, mut forces: ResMut<Forces>, mut roster: ResMut<Roster>) {
    let dt = time.delta_secs();
    for f in forces.0.iter_mut() {
        f.life -= dt;
    }
    forces.0.retain(|f| f.life > 0.0);
    for p in roster.0.values_mut() {
        p.stim = (p.stim - dt).max(0.0);
        p.vanish = (p.vanish - dt).max(0.0);
        p.guard = (p.guard - dt).max(0.0);
        if p.guard <= 0.0 {
            p.guard_cut = 0.0;
        }
    }
}

/// Ability projectiles fly, hit, and burst.
#[allow(clippy::too_many_arguments)]
pub(super) fn missiles(
    mut commands: Commands,
    time: Res<Time>,
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
        let mut struck = None;
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
            struck = Some(*enemy);
            if m.damage > 0.0 || m.stun > 0.0 {
                damage.0.push(event(*enemy, m.damage, m.owner, m.elements, m.stun));
            }
            if !m.pierce {
                break;
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
                color,
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
        if m.end == End::Spread {
            if let Some(first) = struck {
                // The toxin jumps to the three nearest zombies.
                let mut near: Vec<&(Entity, Vec3)> = list
                    .iter()
                    .filter(|(o, p)| *o != first && p.distance(at) < 7.0)
                    .collect();
                near.sort_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)));
                for (o, p) in near.into_iter().take(3) {
                    let mut ev = event(*o, m.damage * 0.5, m.owner, m.elements, m.stun * 0.6);
                    ev.chained = true;
                    damage.0.push(ev);
                    emit(
                        &mut fx,
                        &mut out,
                        Fx::Spell {
                            ability: Ability::NeurotoxinDart,
                            pos: at.to_array(),
                            dir: (*p - at).to_array(),
                            size: 1.0,
                        },
                    );
                }
            }
        }
        commands.entity(e).despawn();
    }
}

/// Thrown things fly, stick or land, and go off.
#[allow(clippy::too_many_arguments)]
pub(super) fn grenades(
    mut commands: Commands,
    time: Res<Time>,
    mut zones: ResMut<Zones>,
    mut damage: ResMut<DamageQueue>,
    mut fx: ResMut<FxQueue>,
    mut out: ResMut<FxOutbox>,
    mut roster: ResMut<Roster>,
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
    for (e, mut tf, mut g) in &mut nades {
        g.fuse -= dt;
        // A sticky bomb rides along on its zombie (and goes off if it dies).
        if let Some((target, offset)) = g.stuck {
            match enemies.get(target) {
                Ok((_, t)) => tf.translation = t.translation + offset,
                Err(_) => g.fuse = g.fuse.min(0.0),
            }
        } else if !g.landed {
            g.velocity.y -= crate::abilities::GRENADE_GRAVITY * dt;
            let next = tf.translation + g.velocity * dt;
            if next.y < 0.1 {
                tf.translation.y = 0.1;
                g.landed = true;
            } else if !line_of_sight(tf.translation, next, &boxes) {
                // Sticky bombs cling to walls; the rest drop off them.
                if g.kind == Nade::Sticky {
                    g.landed = true;
                } else {
                    g.velocity = Vec3::new(-g.velocity.x * 0.2, g.velocity.y.min(0.0), -g.velocity.z * 0.2);
                }
            } else {
                tf.translation = next;
            }
            let touching = enemy_list
                .iter()
                .find(|(_, p)| (*p + Vec3::Y).distance(tf.translation) < 1.0);
            if let Some((target, p)) = touching {
                match g.kind {
                    Nade::Sticky => g.stuck = Some((*target, tf.translation - *p)),
                    Nade::Trap => {}
                    _ => g.landed = true,
                }
            }
        }
        let at = tf.translation;
        let ground = at.with_y(0.0);
        match g.kind {
            Nade::Sticky => {
                if g.fuse > 0.0 {
                    continue;
                }
                explode(
                    at + Vec3::Y * 0.2,
                    g.radius,
                    g.damage,
                    Some(g.owner),
                    g.elements,
                    &enemy_list,
                    &mut damage,
                    &mut fx,
                    &mut out,
                    Color::srgb(1.0, 0.45, 0.15),
                );
            }
            Nade::Heal => {
                if !g.landed && g.fuse > 0.0 {
                    continue;
                }
                // A burst of healing straight away, then the mist lingers.
                for p in roster.0.values_mut() {
                    if p.alive && p.feet().distance(ground) < g.radius {
                        p.health = (p.health + g.damage * 3.0).min(p.max_health());
                    }
                }
                let mut z = zone(g.owner, ground, g.radius, 0.0, 0.5, 6.0);
                z.heal = g.damage;
                z.kind = zone::HEAL;
                add_zone(z, &mut zones, &mut fx, &mut out);
                emit(
                    &mut fx,
                    &mut out,
                    Fx::Heal {
                        pos: ground.to_array(),
                        radius: g.radius,
                    },
                );
            }
            Nade::Acid => {
                if !g.landed && g.fuse > 0.0 {
                    continue;
                }
                let mut z = zone(g.owner, ground, g.radius, g.damage, 0.35, 7.0);
                z.elements = g.elements;
                z.kind = zone::ACID;
                add_zone(z, &mut zones, &mut fx, &mut out);
                emit(
                    &mut fx,
                    &mut out,
                    Fx::Spell {
                        ability: Ability::AcidFlask,
                        pos: ground.to_array(),
                        dir: Vec3::NEG_Z.to_array(),
                        size: g.radius,
                    },
                );
            }
            Nade::Trap => {
                if !g.landed {
                    continue;
                }
                // Lies open on the floor, keeping its model.
                tf.translation = ground;
                commands.entity(e).remove::<GrenadeBrain>().insert(MineBrain {
                    owner: g.owner,
                    arm: 0.5,
                    life: g.fuse,
                    damage: g.damage,
                    radius: g.radius,
                    elements: g.elements,
                    kind: MineKind::Trap,
                    facing: g.velocity.with_y(0.0).normalize_or(Vec3::NEG_Z),
                    stun: g.stun,
                });
                continue;
            }
        }
        commands.entity(e).despawn();
    }
}

/// Claymores and bear traps wait for a zombie to come close.
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
        let pos = tf.translation.with_y(0.0);
        if m.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        if m.arm > 0.0 {
            continue;
        }
        match m.kind {
            MineKind::Claymore => {
                // Trips on anything a few metres in front.
                let in_cone = |p: Vec3, range: f32| {
                    let to = (p - pos).with_y(0.0);
                    let d = to.length();
                    d < range && (d < 1.0 || to.normalize().dot(m.facing) > 0.45)
                };
                if !list.iter().any(|(_, p)| in_cone(*p, 4.5)) {
                    continue;
                }
                for (enemy, p) in &list {
                    if in_cone(*p, m.radius) {
                        damage.0.push(event(*enemy, m.damage, m.owner, m.elements, m.stun));
                    }
                }
                emit(
                    &mut fx,
                    &mut out,
                    Fx::Spell {
                        ability: Ability::Claymore,
                        pos: (pos + Vec3::Y * 0.3).to_array(),
                        dir: m.facing.to_array(),
                        size: m.radius,
                    },
                );
                commands.entity(e).despawn();
            }
            MineKind::Trap => {
                let Some((enemy, _)) = list
                    .iter()
                    .filter(|(_, p)| p.with_y(0.0).distance(pos) < m.radius)
                    .min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)))
                else {
                    continue;
                };
                damage.0.push(event(*enemy, m.damage, m.owner, m.elements, m.stun));
                emit(
                    &mut fx,
                    &mut out,
                    Fx::Spell {
                        ability: Ability::BearTrap,
                        pos: pos.to_array(),
                        dir: m.facing.to_array(),
                        size: 1.0,
                    },
                );
                commands.entity(e).despawn();
            }
        }
    }
}

/// The med drone hovers by its owner, healing and zapping; spectral
/// warriors hunt the zombies near their owner and cut them down.
#[allow(clippy::too_many_arguments)]
pub(super) fn turrets(
    mut commands: Commands,
    time: Res<Time>,
    mut roster: ResMut<Roster>,
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
        let (owner_feet, owner_yaw) = match owner {
            Some(p) if p.alive => (p.feet(), p.yaw),
            _ => {
                t.life = t.life.min(0.0);
                (Vec3::ZERO, 0.0)
            }
        };
        if t.life <= 0.0 {
            emit(
                &mut fx,
                &mut out,
                if t.wraith {
                    Fx::Spell {
                        ability: Ability::ArmyOfTheDead,
                        pos: tf.translation.to_array(),
                        dir: Vec3::NEG_Z.to_array(),
                        size: 1.0,
                    }
                } else {
                    Fx::Explosion {
                        pos: (tf.translation + Vec3::Y * 0.6).to_array(),
                        radius: 1.2,
                        color: [0.4, 1.0, 0.8],
                    }
                },
            );
            commands.entity(e).despawn();
            continue;
        }
        let fwd = Vec3::new(-owner_yaw.sin(), 0.0, -owner_yaw.cos());
        let right = Vec3::new(-fwd.z, 0.0, fwd.x);
        t.cooldown -= dt;
        if t.wraith {
            // Go for the zombie nearest the owner, or wait at their side.
            let target = enemies
                .iter()
                .map(|(e, et, _)| (e, et.translation))
                .filter(|(_, p)| p.distance(owner_feet) < 22.0)
                .min_by(|a, b| a.1.distance(tf.translation).total_cmp(&b.1.distance(tf.translation)));
            let pos = tf.translation;
            let (goal, enemy) = match target {
                Some((enemy, p)) => (p, Some(enemy)),
                None => {
                    let side = [right * 1.6, -right * 1.6, right * 2.6 - fwd, -right * 2.6 - fwd][t.slot as usize % 4];
                    (owner_feet + side - fwd * 0.8, None)
                }
            };
            let to = (goal - pos).with_y(0.0);
            let reach = if enemy.is_some() { 1.4 } else { 0.3 };
            if to.length() > reach {
                let step = to.normalize() * (9.0 * dt).min(to.length() - reach);
                tf.translation = (pos + step).with_y(0.0);
            }
            if to.length_squared() > 1e-4 {
                tf.look_to(to.normalize(), Vec3::Y);
            }
            if let Some(enemy) = enemy {
                if to.length() < reach + 0.4 && t.cooldown <= 0.0 {
                    t.cooldown = 0.65;
                    damage.0.push(event(enemy, t.damage, t.owner, t.elements, 0.4));
                    emit(
                        &mut fx,
                        &mut out,
                        Fx::Slash {
                            pos: (tf.translation + Vec3::Y * 1.1).to_array(),
                            dir: to.normalize_or(Vec3::NEG_Z).to_array(),
                            radius: 1.8,
                        },
                    );
                }
            }
            continue;
        }
        // The drone hovers over your left shoulder, bobbing.
        let want = owner_feet + Vec3::Y * (2.3 + 0.15 * (t_now * 2.5).sin()) - right * 1.0 - fwd * 0.3;
        let k = (dt * 5.0).min(1.0);
        tf.translation = tf.translation.lerp(want, k);
        t.heal_timer -= dt;
        if t.heal_timer <= 0.0 && t.heal > 0.0 {
            t.heal_timer = 1.0;
            for p in roster.0.values_mut() {
                if p.alive && p.feet().distance(owner_feet) < 8.0 && p.health < p.max_health() {
                    p.health = (p.health + t.heal).min(p.max_health());
                    emit(
                        &mut fx,
                        &mut out,
                        Fx::Spell {
                            ability: Ability::MedDrone,
                            pos: tf.translation.to_array(),
                            dir: (p.feet() + Vec3::Y * 1.0 - tf.translation).to_array(),
                            size: 1.0,
                        },
                    );
                }
            }
        }
        let gun = tf.translation;
        let target = enemies
            .iter()
            .map(|(e, et, b)| (e, et.translation + Vec3::Y * 1.1 * enemy_scale(b.kind)))
            .filter(|(_, p)| p.distance(gun) < 18.0 && line_of_sight(gun, *p, &boxes))
            .min_by(|a, b| a.1.distance(gun).total_cmp(&b.1.distance(gun)));
        let Some((enemy, at)) = target else { continue };
        let flat = (at - gun).with_y(0.0).normalize_or(Vec3::NEG_Z);
        tf.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, flat);
        if t.cooldown <= 0.0 {
            t.cooldown = 0.35;
            damage.0.push(event(enemy, t.damage, t.owner, t.elements, 0.0));
            emit(
                &mut fx,
                &mut out,
                Fx::Lightning {
                    a: (gun + (at - gun).normalize_or_zero() * 0.3).to_array(),
                    b: at.to_array(),
                },
            );
        }
    }
}
