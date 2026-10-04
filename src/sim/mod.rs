//! Host-side game simulation: rounds, enemy AI, damage, elements, points, XP
//! and levels, abilities, the mystery box, perks, power-ups and extraction.
//! Only the host (or a solo player) runs this; clients mirror its results.

use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;

use crate::avatars::spawn_replicated;
use crate::data::{
    elements_in, gun_def, has_perk, roll_attachments, roll_box_gun, wall_cost,
    xp_to_next, Element, GunSpecial, Perk, PowerUp, Upgrade, BOX_COST, MAX_LEVEL,
    MAX_TIER,
};
use crate::fx::{emit, rgb, Fx, FxOutbox, FxQueue};
use crate::maps::{CurrentMap, EXTRACT_RADIUS};
use crate::nav::NavGrid;
use crate::physics::{
    collect_boxes, line_of_sight, resolve_collisions, trace_shot, Boxes,
};
use crate::weapons::GUN_RANGE;
use crate::{
    ActionQueue, AppState, BoxState, Collider, EnemyStatus, MatchState, NetKind, Phase,
    PlayerAction, PlayerInfo, Replicated, Roster, Session, ShotQueue, EYE_HEIGHT,
};

const REVIVE_HEALTH_FRACTION: f32 = 0.5;
const INTERACT_RANGE: f32 = 2.6;
const POWERUP_CHANCE: f64 = 0.06;
const REGEN_DELAY: f32 = 4.0;
const REGEN_RATE: f32 = 20.0;

pub mod powers;

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DamageQueue>()
            .init_resource::<DevQueue>()
            .init_resource::<Strikes>()
            .init_resource::<Zones>()
            .init_resource::<powers::Forces>()
            .init_resource::<LastHurt>()
            .add_systems(
                Update,
                (
                    sandbox,
                    resolve_shots,
                    player_timers,
                    status_effects,
                    projectiles,
                    powers::grenades,
                    powers::missiles,
                    powers::mines,
                    powers::strikes,
                    zones,
                    powers::turrets,
                    powers::timers,
                    apply_damage,
                    rounds,
                    enemy_ai,
                    powerups,
                    mystery_box,
                    extraction,
                    check_game_over,
                )
                    .chain()
                    .in_set(Phase::Sim),
            )
            // Actions also run while a solo game is paused, so level-up
            // picks made in the menu apply straight away.
            .add_systems(
                Update,
                process_actions
                    .after(Phase::Local)
                    .before(Phase::Sim)
                    .run_if(in_state(AppState::InGame).and(is_authority)),
            )
            .add_systems(OnEnter(AppState::InGame), clear_sim);
    }
}

fn is_authority(session: Res<Session>) -> bool {
    session.is_authority()
}

pub fn enemy_scale(kind: NetKind) -> f32 {
    match kind {
        NetKind::Brute => 1.35,
        NetKind::Shooter => 1.05,
        _ => 1.0,
    }
}

#[derive(Component)]
pub struct EnemyBrain {
    pub kind: NetKind,
    pub health: f32,
    speed: f32,
    attack_timer: f32,
    burn: f32,
    burn_dps: f32,
    burn_by: Option<u8>,
    slow: f32,
    max_health: f32,
    leg_damage: f32,
    /// Legs shot out: crawls along the ground.
    pub crawler: bool,
    /// Seconds since the last swing (for the attack animation).
    pub swing: f32,
    /// Stunned: can't move or attack.
    pub stun: f32,
}

#[derive(Component)]
pub struct FireballBrain {
    velocity: Vec3,
    life: f32,
    damage: f32,
}

/// A lasting damage area (see `Fx::Zone` for the kinds).
struct Zone {
    owner: u8,
    pos: Vec3,
    follow: Option<u8>,
    radius: f32,
    damage: f32,
    interval: f32,
    timer: f32,
    life: f32,
    elements: u8,
    kind: u8,
    /// A model that goes away with the zone (the tesla coil).
    model: Option<Entity>,
}

#[derive(Resource, Default)]
struct Zones(Vec<Zone>);

#[derive(Component)]
pub struct PowerUpBrain {
    kind: PowerUp,
    life: f32,
}

struct DamageEvent {
    target: Entity,
    amount: f32,
    from: Option<u8>,
    headshot: bool,
    /// Hit in the legs (enough of these make it a crawler).
    legs: bool,
    elements: u8,
    chained: bool,
    /// Seconds the target is stunned (frozen, choking, knocked down).
    stun: f32,
}

#[derive(Resource, Default)]
struct DamageQueue(Vec<DamageEvent>);

/// Delayed hits: orbital strikes, bombs, meteors, cuts (see powers.rs).
#[derive(Resource, Default)]
struct Strikes(Vec<powers::Strike>);

/// Sandbox tools waiting to run: (player, tool).
#[derive(Resource, Default)]
struct DevQueue(Vec<(u8, crate::DevCmd)>);

/// When each player last took damage (for health regeneration).
#[derive(Resource, Default)]
struct LastHurt(HashMap<u8, f32>);

fn clear_sim(
    mut damage: ResMut<DamageQueue>,
    mut strikes: ResMut<Strikes>,
    mut hurt: ResMut<LastHurt>,
    mut zones: ResMut<Zones>,
    mut forces: ResMut<powers::Forces>,
) {
    zones.0.clear();
    forces.0.clear();
    damage.0.clear();
    strikes.0.clear();
    hurt.0.clear();
}

fn hurt_player(p: &mut PlayerInfo, amount: f32, hurt: &mut LastHurt) {
    p.damage(amount);
    hurt.0.insert(p.id, 0.0);
}

/// Damage every enemy within `radius` of `pos`.
fn explode(
    pos: Vec3,
    radius: f32,
    damage: f32,
    from: Option<u8>,
    elements: u8,
    enemies: &[(Entity, Vec3)],
    queue: &mut DamageQueue,
    fx: &mut FxQueue,
    out: &mut FxOutbox,
    color: Color,
) {
    for (e, p) in enemies {
        let d = (*p + Vec3::Y).distance(pos);
        if d < radius {
            let falloff = 1.0 - 0.5 * (d / radius);
            queue.0.push(DamageEvent {
                target: *e,
                amount: damage * falloff,
                from,
                headshot: false,
                legs: false,
                elements,
                chained: false,
                stun: 0.0,
            });
        }
    }
    emit(
        fx,
        out,
        Fx::Explosion {
            pos: pos.to_array(),
            radius,
            color: rgb(color),
        },
    );
}

fn level_multiplier(p: &PlayerInfo) -> f32 {
    let mut m = 1.0 + 0.03 * (p.level.saturating_sub(1)) as f32;
    if p.overdrive > 0.0 {
        m *= 1.5;
    }
    m
}

/// Three random level-up rewards. Milestone levels 5-25 always include an
/// element if one is left.
fn roll_choices(p: &PlayerInfo) -> Vec<Upgrade> {
    let mut rng = rand::thread_rng();
    let mut abilities: Vec<Upgrade> = (0..3u8)
        .filter(|s| p.tiers[*s as usize] < MAX_TIER)
        .map(Upgrade::Ability)
        .collect();
    let mut elements: Vec<Upgrade> = Vec::new();
    for (i, e) in Element::ALL.iter().enumerate() {
        if p.gun_elements & e.bit() == 0 {
            elements.push(Upgrade::GunElement(i as u8));
        }
        if p.ability_elements & e.bit() == 0 {
            elements.push(Upgrade::AbilityElement(i as u8));
        }
    }
    let mut picks = Vec::new();
    if p.level <= 25 && !elements.is_empty() {
        picks.push(elements.swap_remove(rng.gen_range(0..elements.len())));
    }
    let mut pool: Vec<Upgrade> = abilities.drain(..).chain(elements).collect();
    while picks.len() < 3 && !pool.is_empty() {
        picks.push(pool.swap_remove(rng.gen_range(0..pool.len())));
    }
    picks
}

fn give_xp(p: &mut PlayerInfo, amount: u32) {
    if p.level >= MAX_LEVEL {
        return;
    }
    p.xp += amount;
    while p.level < MAX_LEVEL && p.xp >= xp_to_next(p.level) {
        p.xp -= xp_to_next(p.level);
        p.level += 1;
        if p.level % 5 == 0 {
            p.pending_picks += 1;
        }
    }
    if p.pending_picks > 0 && p.choices.is_empty() {
        p.choices = roll_choices(p);
        if p.choices.is_empty() {
            p.pending_picks = 0;
        }
    }
}

fn give_points(p: &mut PlayerInfo, amount: u32, state: &MatchState) {
    let amount = if state.double_points > 0.0 {
        amount * 2
    } else {
        amount
    };
    p.points += amount;
    p.score += amount;
}

// ---------------------------------------------------------------------------
// Player actions: interact, abilities, level-up picks
// ---------------------------------------------------------------------------

fn process_actions(
    mut commands: Commands,
    mut actions: ResMut<ActionQueue>,
    mut roster: ResMut<Roster>,
    mut state: ResMut<MatchState>,
    map: Res<CurrentMap>,
    mut damage: ResMut<DamageQueue>,
    mut strikes: ResMut<Strikes>,
    mut zones: ResMut<Zones>,
    mut forces: ResMut<powers::Forces>,
    mut fx: ResMut<FxQueue>,
    mut out: ResMut<FxOutbox>,
    enemies: Query<(Entity, &Transform, &EnemyBrain)>,
    colliders: Query<(&Transform, &Collider)>,
    mut dev: ResMut<DevQueue>,
) {
    if actions.0.is_empty() {
        return;
    }
    let enemy_list: Vec<(Entity, Vec3)> =
        enemies.iter().map(|(e, t, _)| (e, t.translation)).collect();
    for (id, seq, action) in std::mem::take(&mut actions.0) {
        let Some(p) = roster.0.get_mut(&id) else {
            continue;
        };
        if seq <= p.action_ack {
            continue;
        }
        p.action_ack = seq;
        if state.game_over || state.extracted {
            continue;
        }
        match action {
            PlayerAction::Dev(cmd) => {
                if !state.sandbox.on {
                    continue;
                }
                use crate::DevCmd;
                match cmd {
                    DevCmd::Toggle(n) => {
                        let s = &mut state.sandbox;
                        match n {
                            0 => s.waves = !s.waves,
                            1 => s.god = !s.god,
                            _ => s.free_abilities = !s.free_abilities,
                        }
                    }
                    DevCmd::NextRound => state.round += 1,
                    DevCmd::Points => p.points += 10_000,
                    DevCmd::LevelUp => {
                        let need = (p.level * 100).max(100);
                        give_xp(p, need);
                    }
                    DevCmd::Gun { gun, attach } => {
                        if (gun as usize) < crate::data::GUNS.len()
                            && crate::data::attach_options_for(gun).fits(attach)
                        {
                            let slot = if let Some(i) = p.guns.iter().position(|g| *g == Some(gun))
                            {
                                i
                            } else if p.guns[1].is_none() {
                                1
                            } else {
                                p.active_slot as usize
                            };
                            p.guns[slot] = Some(gun);
                            p.attach[slot] = attach;
                            p.active_slot = slot as u8;
                            p.supply_seq = p.supply_seq.wrapping_add(1);
                        }
                    }
                    DevCmd::Spawn { .. } | DevCmd::KillAll => dev.0.push((id, cmd)),
                }
            }
            PlayerAction::Choose(i) => {
                let Some(choice) = p.choices.get(i as usize).copied() else {
                    continue;
                };
                match choice {
                    Upgrade::Ability(s) => {
                        let t = &mut p.tiers[s as usize];
                        *t = (*t + 1).min(MAX_TIER);
                    }
                    Upgrade::GunElement(e) => p.gun_elements |= Element::ALL[e as usize].bit(),
                    Upgrade::AbilityElement(e) => {
                        p.ability_elements |= Element::ALL[e as usize].bit()
                    }
                }
                p.pending_picks = p.pending_picks.saturating_sub(1);
                p.choices = if p.pending_picks > 0 {
                    roll_choices(p)
                } else {
                    Vec::new()
                };
            }
            PlayerAction::Ping { pos, target } => {
                if pos.iter().all(|v| v.is_finite()) {
                    emit(
                        &mut fx,
                        &mut out,
                        Fx::Ping {
                            player: id,
                            pos,
                            target,
                        },
                    );
                }
            }
            PlayerAction::Melee { origin, dir } => {
                let origin = Vec3::from_array(origin);
                let dir = Vec3::from_array(dir).normalize_or_zero();
                if !p.alive || !origin.is_finite() || dir == Vec3::ZERO {
                    continue;
                }
                // The nearest zombie in reach takes the slash.
                let target = enemies
                    .iter()
                    .filter(|(_, _, b)| b.health > 0.0)
                    .filter_map(|(e, t, b)| {
                        crate::weapons::melee_reach(
                            origin,
                            dir,
                            t.translation,
                            enemy_scale(b.kind),
                            b.crawler,
                        )
                        .map(|d| (e, d, b))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((e, _, b)) = target {
                    let amount = (150.0 + 0.2 * b.max_health) * level_multiplier(p);
                    // A melee kill is worth more than a shot kill.
                    if amount >= b.health || state.insta_kill > 0.0 {
                        give_points(p, 70, &state);
                    }
                    damage.0.push(DamageEvent {
                        target: e,
                        amount,
                        from: Some(id),
                        headshot: false,
                        legs: false,
                        elements: 0,
                        chained: false,
                        stun: 0.0,
                    });
                }
            }
            PlayerAction::Interact => {
                if !p.alive {
                    continue;
                }
                let feet = p.feet();
                let box_pos = map.0.box_spots[state.box_spot as usize];
                let near_box = feet.with_y(0.0).distance(box_pos) < INTERACT_RANGE;
                match state.box_state {
                    BoxState::Offer {
                        player,
                        gun,
                        attach,
                        ..
                    } if near_box && player == id => {
                        let slot = if p.guns[1].is_none() {
                            1
                        } else {
                            p.active_slot as usize
                        };
                        p.guns[slot] = Some(gun);
                        p.attach[slot] = attach;
                        p.active_slot = slot as u8;
                        state.box_state = BoxState::Idle;
                        continue;
                    }
                    BoxState::Idle if near_box => {
                        if p.points >= BOX_COST {
                            p.points -= BOX_COST;
                            state.box_uses += 1;
                            state.box_state = BoxState::Rolling {
                                player: id,
                                time: 3.0,
                            };
                        }
                        continue;
                    }
                    _ => {}
                }
                // Doors to the other areas.
                if let Some(door) = map
                    .0
                    .doors
                    .iter()
                    .find(|d| d.near(feet) && d.locked(state.doors))
                {
                    if p.points >= door.cost {
                        p.points -= door.cost;
                        state.doors |= door.opens();
                    }
                    continue;
                }
                // Guns on the wall: buy the gun, or ammo for it if you have it.
                if let Some(wall) = map.0.wall_buys.iter().find(|w| w.near(feet)) {
                    let cost = wall_cost(wall.gun);
                    if p.guns.contains(&Some(wall.gun)) {
                        if p.points >= cost / 2 {
                            p.points -= cost / 2;
                            p.supply_seq = p.supply_seq.wrapping_add(1);
                        }
                    } else if p.points >= cost {
                        p.points -= cost;
                        let slot = if p.guns[1].is_none() {
                            1
                        } else {
                            p.active_slot as usize
                        };
                        p.guns[slot] = Some(wall.gun);
                        p.attach[slot] = wall.attach;
                        p.active_slot = slot as u8;
                    }
                    continue;
                }
                for (spot, perk) in map.0.perk_spots.iter().zip(Perk::ALL) {
                    if feet.with_y(0.0).distance(*spot) < INTERACT_RANGE
                        && !has_perk(p.perks, perk)
                        && p.points >= perk.cost()
                    {
                        p.points -= perk.cost();
                        p.perks |= perk.bit();
                        if perk == Perk::Juggernaut {
                            p.health = p.max_health();
                        }
                    }
                }
            }
            PlayerAction::Ability {
                slot,
                origin,
                dir,
                charge,
            } => {
                if !p.alive || slot > 2 {
                    continue;
                }
                let s = slot as usize;
                let ability = p.kit[s];
                if slot == 2 {
                    if p.ult_charge < 100.0 {
                        continue;
                    }
                    p.ult_charge = 0.0;
                } else {
                    if p.cooldowns[s] > 0.0 {
                        continue;
                    }
                    p.cooldowns[s] = ability.cooldown(p.tiers[s]);
                }
                let origin = Vec3::from_array(origin);
                let dir = Vec3::from_array(dir).normalize_or_zero();
                if !origin.is_finite() || dir == Vec3::ZERO {
                    continue;
                }
                let charge = if charge.is_finite() {
                    charge.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                emit(&mut fx, &mut out, Fx::Cast { player: id, slot });
                let boxes = collect_boxes(colliders.iter());
                let mut world = powers::World {
                    commands: &mut commands,
                    state: &mut state,
                    damage: &mut damage,
                    strikes: &mut strikes,
                    zones: &mut zones,
                    forces: &mut forces,
                    fx: &mut fx,
                    out: &mut out,
                    enemies: &enemy_list,
                    boxes: &boxes,
                };
                powers::cast(&mut world, &mut roster, id, slot, origin, dir, charge);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shooting and damage
// ---------------------------------------------------------------------------

fn resolve_shots(
    session: Res<Session>,
    state: Res<MatchState>,
    roster: Res<Roster>,
    mut shots: ResMut<ShotQueue>,
    mut damage: ResMut<DamageQueue>,
    mut fx: ResMut<FxQueue>,
    mut out: ResMut<FxOutbox>,
    enemies: Query<(Entity, &Transform, &EnemyBrain)>,
    colliders: Query<(&Transform, &Collider)>,
) {
    if shots.0.is_empty() {
        return;
    }
    let boxes = collect_boxes(colliders.iter());
    let enemy_list: Vec<(Entity, Vec3)> =
        enemies.iter().map(|(e, t, _)| (e, t.translation)).collect();
    let mut rng = rand::thread_rng();
    for (shooter, shot) in shots.0.drain(..) {
        let Some(p) = roster.0.get(&shooter) else {
            continue;
        };
        // Only guns you actually hold.
        if !p.alive || !p.guns.contains(&Some(shot.gun)) {
            continue;
        }
        let origin = Vec3::from_array(shot.origin);
        let dir = Vec3::from_array(shot.dir).normalize_or_zero();
        if !origin.is_finite() || dir == Vec3::ZERO {
            continue;
        }
        let def = gun_def(shot.gun);
        let hit = trace_shot(
            origin,
            dir,
            GUN_RANGE,
            &boxes,
            enemies
                .iter()
                .filter(|(_, _, b)| b.health > 0.0)
                .map(|(e, t, b)| (e, t.translation, enemy_scale(b.kind), b.crawler)),
        );
        let end = origin + dir * hit.dist;
        let tracer = Fx::Tracer {
            shooter,
            a: (origin - Vec3::Y * 0.25).to_array(),
            b: end.to_array(),
        };
        if shooter == session.my_id {
            out.0.push(tracer);
        } else {
            emit(&mut fx, &mut out, tracer);
        }

        let slot = p
            .guns
            .iter()
            .position(|g| *g == Some(shot.gun))
            .unwrap_or(0);
        let mult = level_multiplier(p) * p.attach[slot].handling(shot.gun).damage;
        let elements = p.gun_elements;
        if let GunSpecial::Explosive { radius } = def.special {
            explode(
                end - dir * 0.3,
                radius,
                def.damage * mult,
                Some(shooter),
                elements,
                &enemy_list,
                &mut damage,
                &mut fx,
                &mut out,
                Color::srgb(0.3, 1.0, 0.4),
            );
            continue;
        }
        let Some((entity, headshot)) = hit.enemy else {
            continue;
        };
        let amount = def.damage * mult * if headshot { def.headshot } else { 1.0 };
        damage.0.push(DamageEvent {
            target: entity,
            amount,
            from: Some(shooter),
            headshot,
            legs: hit.legs,
            elements,
            chained: false,
            stun: 0.0,
        });
        if let GunSpecial::Chain { jumps } = def.special {
            let mut from = end;
            let mut others: Vec<(Entity, Vec3)> = enemy_list
                .iter()
                .filter(|(e, _)| *e != entity)
                .copied()
                .collect();
            others.sort_by(|a, b| a.1.distance(end).total_cmp(&b.1.distance(end)));
            for (e, pos) in others.into_iter().take(jumps as usize) {
                if pos.distance(end) > 14.0 {
                    break;
                }
                let to = pos + Vec3::Y * 1.2;
                emit(
                    &mut fx,
                    &mut out,
                    Fx::Lightning {
                        a: from.to_array(),
                        b: to.to_array(),
                    },
                );
                from = to;
                damage.0.push(DamageEvent {
                    target: e,
                    amount: def.damage * mult * 0.8,
                    from: Some(shooter),
                    headshot: false,
                    legs: false,
                    elements,
                    chained: true,
                    stun: 0.0,
                });
            }
        }
        if headshot && has_perk(p.perks, Perk::BoomShot) && rng.gen_bool(0.3) {
            explode(
                end,
                3.0,
                (100.0 + 10.0 * state.round as f32) * mult,
                Some(shooter),
                elements,
                &enemy_list,
                &mut damage,
                &mut fx,
                &mut out,
                Color::srgb(1.0, 0.5, 0.1),
            );
        }
    }
}

fn status_effects(
    time: Res<Time>,
    mut damage: ResMut<DamageQueue>,
    mut enemies: Query<(Entity, &mut EnemyBrain, &mut EnemyStatus)>,
) {
    let dt = time.delta_secs();
    for (e, mut b, mut status) in &mut enemies {
        status.flash -= dt;
        b.slow -= dt;
        b.stun -= dt;
        if b.burn > 0.0 {
            b.burn -= dt;
            damage.0.push(DamageEvent {
                target: e,
                amount: b.burn_dps * dt,
                from: b.burn_by,
                headshot: false,
                legs: false,
                elements: 0,
                chained: true,
                stun: 0.0,
            });
        }
        status.burning = b.burn > 0.0;
        status.slowed = b.slow > 0.0;
        status.stunned = b.stun > 0.0;
        status.crawler = b.crawler;
        status.attacking = b.swing < 0.6;
    }
}

fn apply_damage(
    mut commands: Commands,
    mut queue: ResMut<DamageQueue>,
    mut roster: ResMut<Roster>,
    mut state: ResMut<MatchState>,
    mut fx: ResMut<FxQueue>,
    mut out: ResMut<FxOutbox>,
    mut enemies: Query<(Entity, &Transform, &mut EnemyBrain, &mut EnemyStatus)>,
) {
    if queue.0.is_empty() {
        return;
    }
    let positions: Vec<(Entity, Vec3)> = enemies
        .iter()
        .map(|(e, t, _, _)| (e, t.translation))
        .collect();
    let mut rng = rand::thread_rng();
    let round = state.round;
    let mut i = 0;
    while i < queue.0.len() {
        let ev = &queue.0[i];
        let (target, from, headshot, legs, elements, chained, stun) = (
            ev.target,
            ev.from,
            ev.headshot,
            ev.legs,
            ev.elements,
            ev.chained,
            ev.stun,
        );
        let mut amount = ev.amount;
        i += 1;
        let Ok((_, tf, mut brain, mut status)) = enemies.get_mut(target) else {
            continue;
        };
        if brain.health <= 0.0 {
            continue;
        }
        if state.insta_kill > 0.0 && from.is_some() && !chained {
            amount = amount.max(brain.health);
        }
        brain.health -= amount;
        // Brutes shrug off half of it.
        let resist = if brain.kind == NetKind::Brute { 0.5 } else { 1.0 };
        brain.stun = brain.stun.max(stun * resist);
        if !chained || amount > 5.0 {
            status.flash = 0.08;
        }
        let pos = tf.translation;
        for el in elements_in(elements) {
            match el {
                Element::Fire => {
                    brain.burn = 3.0;
                    brain.burn_dps = 15.0 + 3.0 * round as f32;
                    brain.burn_by = from;
                }
                Element::Ice => brain.slow = 2.5,
                Element::Shock if !chained => {
                    let mut near: Vec<&(Entity, Vec3)> = positions
                        .iter()
                        .filter(|(e, p)| *e != target && p.distance(pos) < 6.0)
                        .collect();
                    near.sort_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)));
                    for (e, p) in near.into_iter().take(2) {
                        emit(
                            &mut fx,
                            &mut out,
                            Fx::Lightning {
                                a: (pos + Vec3::Y * 1.2).to_array(),
                                b: (*p + Vec3::Y * 1.2).to_array(),
                            },
                        );
                        queue.0.push(DamageEvent {
                            target: *e,
                            amount: amount * 0.6,
                            from,
                            headshot: false,
                            legs: false,
                            elements: 0,
                            chained: true,
                            stun: 0.0,
                        });
                    }
                }
                _ => {}
            }
        }
        let killed = brain.health <= 0.0;
        // Enough damage to the legs and it goes down and crawls.
        if legs && !killed {
            brain.leg_damage += amount;
            let needed = if brain.kind == NetKind::Brute {
                0.55
            } else {
                0.35
            };
            if !brain.crawler && brain.leg_damage >= needed * brain.max_health {
                brain.crawler = true;
                brain.speed *= 0.45;
            }
        }
        if let Some(pid) = from {
            if let Some(p) = roster.0.get_mut(&pid) {
                if killed {
                    let bonus = if headshot { 40 } else { 0 };
                    let base = if brain.kind == NetKind::Brute {
                        120
                    } else {
                        60
                    };
                    give_points(p, base + bonus, &state);
                    p.kills += 1;
                    p.ult_charge = (p.ult_charge + 3.0).min(100.0);
                    let xp = if brain.kind == NetKind::Brute { 60 } else { 25 }
                        + if headshot { 15 } else { 0 };
                    give_xp(p, xp);
                } else if !chained {
                    give_points(p, 10, &state);
                }
            }
        }
        if killed {
            crate::zombies::kill(&mut commands, target);
            if rng.gen_bool(POWERUP_CHANCE) {
                let kind = PowerUp::ALL[rng.gen_range(0..PowerUp::ALL.len())];
                let id = state.next_net_id;
                state.next_net_id += 1;
                commands.spawn((
                    crate::InGameEntity,
                    Replicated {
                        id,
                        kind: NetKind::PowerUp(kind),
                    },
                    PowerUpBrain { kind, life: 25.0 },
                    Transform::from_translation(pos.with_y(0.9)),
                ));
            }
        }
    }
    queue.0.clear();
}

// ---------------------------------------------------------------------------
// Players
// ---------------------------------------------------------------------------

fn player_timers(
    time: Res<Time>,
    mut roster: ResMut<Roster>,
    mut state: ResMut<MatchState>,
    mut hurt: ResMut<LastHurt>,
) {
    let dt = time.delta_secs();
    for p in roster.0.values_mut() {
        for cd in p.cooldowns.iter_mut() {
            *cd = (*cd - dt).max(0.0);
        }
        p.overdrive = (p.overdrive - dt).max(0.0);
        if p.alive && state.started && !state.game_over {
            p.ult_charge = (p.ult_charge + dt * 0.8).min(100.0);
            let since = hurt.0.entry(p.id).or_insert(99.0);
            *since += dt;
            if *since > REGEN_DELAY {
                p.health = (p.health + REGEN_RATE * dt).min(p.max_health());
            }
        }
    }
    state.insta_kill = (state.insta_kill - dt).max(0.0);
    state.double_points = (state.double_points - dt).max(0.0);
}

// ---------------------------------------------------------------------------
// Rounds and spawning
// ---------------------------------------------------------------------------

fn rounds(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<crate::avatars::ReplicatedAssets>,
    rigs: Res<crate::rig::RigAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    map: Res<CurrentMap>,
    mut state: ResMut<MatchState>,
    mut roster: ResMut<Roster>,
    enemies: Query<(), With<EnemyBrain>>,
) {
    if state.game_over || state.extracted || !state.started {
        return;
    }
    if state.sandbox.on && !state.sandbox.waves {
        return;
    }
    let dt = time.delta_secs();
    let alive_enemies = enemies.iter().count() as u32;

    if state.to_spawn == 0 && alive_enemies == 0 {
        if state.intermission <= 0.0 {
            // Round cleared.
            state.intermission = 4.0;
            let r = state.round;
            if r >= 15 && (r - 15) % 10 == 0 {
                state.extraction = 45.0;
                state.intermission = 45.0;
                state.extract_hold = 0.0;
            }
        }
        state.intermission -= dt;
        if state.intermission <= 0.0 {
            state.extraction = 0.0;
            state.round += 1;
            let r = state.round;
            let players = roster.0.len().max(1) as u32;
            state.to_spawn = 5 + 3 * r + (players - 1) * (2 + r);
            state.spawn_timer = 0.5;
            // Downed players get back up at the start of each round.
            for p in roster.0.values_mut().filter(|p| !p.alive) {
                p.alive = true;
                p.health = p.max_health() * REVIVE_HEALTH_FRACTION;
                p.spawn_seq += 1;
            }
        }
        return;
    }

    if state.to_spawn == 0 {
        return;
    }
    let players = roster.0.len().max(1) as u32;
    if alive_enemies >= 24 + 4 * players {
        return;
    }
    state.spawn_timer -= dt;
    if state.spawn_timer > 0.0 {
        return;
    }
    let r = state.round;
    state.spawn_timer = (1.6 - r as f32 * 0.08).max(0.3);
    state.to_spawn -= 1;

    let mut rng = rand::thread_rng();
    let living: Vec<Vec3> = roster
        .0
        .values()
        .filter(|p| p.alive)
        .map(|p| p.feet())
        .collect();
    // Only areas the team has opened spawn enemies.
    let open: Vec<Vec3> = map
        .0
        .enemy_spawns
        .iter()
        .filter(|(_, zone)| crate::maps::MapLayout::zone_open(state.doors, *zone))
        .map(|(p, _)| *p)
        .collect();
    let far: Vec<Vec3> = open
        .iter()
        .copied()
        .filter(|s| living.iter().all(|p| p.distance(*s) > 14.0))
        .collect();
    let pool = if far.is_empty() { &open } else { &far };
    let base = pool[rng.gen_range(0..pool.len())];
    let pos = base + Vec3::new(rng.gen_range(-1.5..1.5), 0.0, rng.gen_range(-1.5..1.5));

    let kind = if r >= 6 && rng.gen_bool(0.1) {
        NetKind::Brute
    } else if r >= 3 && rng.gen_bool(0.2) {
        NetKind::Shooter
    } else {
        NetKind::Grunt
    };
    spawn_zombie(
        &mut commands,
        &assets,
        &rigs,
        &mut materials,
        &mut state,
        kind,
        pos,
        false,
    );
}

/// Spawns a zombie of `kind` at `pos`, as tough as the current round makes it.
#[allow(clippy::too_many_arguments)]
fn spawn_zombie(
    commands: &mut Commands,
    assets: &crate::avatars::ReplicatedAssets,
    rigs: &crate::rig::RigAssets,
    materials: &mut Assets<StandardMaterial>,
    state: &mut MatchState,
    kind: NetKind,
    pos: Vec3,
    crawler: bool,
) {
    let mut rng = rand::thread_rng();
    let r = state.round.max(1);
    let base_hp = if r <= 10 {
        75.0 + 30.0 * r as f32
    } else {
        375.0 * 1.1f32.powi(r as i32 - 10)
    };
    let (health, speed) = match kind {
        NetKind::Shooter => (base_hp * 0.7, 2.6),
        NetKind::Brute => (base_hp * 3.0, (2.4 + 0.1 * r as f32).min(4.5)),
        _ => (
            base_hp,
            (2.8 + 0.25 * r as f32).min(7.0) * rng.gen_range(0.9..1.1),
        ),
    };
    let id = state.next_net_id;
    state.next_net_id += 1;
    let e = spawn_replicated(commands, assets, rigs, materials, id, kind, pos);
    commands.entity(e).insert(EnemyBrain {
        kind,
        health,
        speed,
        attack_timer: 1.0,
        burn: 0.0,
        burn_dps: 0.0,
        burn_by: None,
        slow: 0.0,
        max_health: health,
        leg_damage: 0.0,
        crawler,
        swing: 9.0,
        stun: 0.0,
    });
}

/// Runs sandbox tools, and keeps god mode and free abilities going.
#[allow(clippy::too_many_arguments)]
fn sandbox(
    mut commands: Commands,
    assets: Res<crate::avatars::ReplicatedAssets>,
    rigs: Res<crate::rig::RigAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<MatchState>,
    mut roster: ResMut<Roster>,
    mut dev: ResMut<DevQueue>,
    enemies: Query<Entity, With<EnemyBrain>>,
) {
    if !state.sandbox.on {
        dev.0.clear();
        return;
    }
    use crate::DevCmd;
    for (_, cmd) in std::mem::take(&mut dev.0) {
        match cmd {
            DevCmd::Spawn {
                kind,
                count,
                at,
                dir,
            } => {
                let at = Vec3::from_array(at);
                let fwd = Vec3::from_array(dir).with_y(0.0).normalize_or(Vec3::NEG_Z);
                let side = Vec3::new(-fwd.z, 0.0, fwd.x);
                let net = match kind {
                    1 => NetKind::Shooter,
                    2 => NetKind::Brute,
                    _ => NetKind::Grunt,
                };
                for i in 0..count.min(20) {
                    let spread = (i as f32 - (count as f32 - 1.0) / 2.0) * 1.4;
                    let pos = (at + fwd * 9.0 + side * spread).with_y(0.0);
                    spawn_zombie(
                        &mut commands,
                        &assets,
                        &rigs,
                        &mut materials,
                        &mut state,
                        net,
                        pos,
                        kind == 3,
                    );
                }
            }
            DevCmd::KillAll => {
                for e in &enemies {
                    crate::zombies::kill(&mut commands, e);
                }
            }
            _ => {}
        }
    }
    let sb = state.sandbox;
    for p in roster.0.values_mut() {
        if sb.god {
            p.health = p.max_health();
        }
        if sb.free_abilities {
            p.cooldowns = [0.0; 2];
            p.ult_charge = 100.0;
        }
    }
}

fn enemy_ai(
    mut commands: Commands,
    time: Res<Time>,
    map: Res<CurrentMap>,
    mut nav: ResMut<NavGrid>,
    mut state: ResMut<MatchState>,
    mut roster: ResMut<Roster>,
    mut hurt: ResMut<LastHurt>,
    forces: Res<powers::Forces>,
    mut enemies: Query<(Entity, &mut Transform, &mut EnemyBrain)>,
    colliders: Query<(&Transform, &Collider), Without<EnemyBrain>>,
) {
    let dt = time.delta_secs();
    let boxes: Boxes = collect_boxes(colliders.iter());
    // Players who have vanished (Thousand Cuts) can't be chased.
    let targets: Vec<(u8, Vec3)> = roster
        .0
        .values()
        .filter(|p| p.alive && p.vanish <= 0.0)
        .map(|p| (p.id, p.feet()))
        .collect();
    let target_points: Vec<Vec3> = targets.iter().map(|t| t.1).collect();
    nav.update(dt, &boxes, &target_points);
    let positions: Vec<(Entity, Vec3)> =
        enemies.iter().map(|(e, t, _)| (e, t.translation)).collect();
    let half = map.0.half;

    for (entity, mut tf, mut enemy) in &mut enemies {
        let pos = tf.translation;
        let scale = enemy_scale(enemy.kind);
        let target = targets.iter().min_by(|a, b| {
            a.1.distance_squared(pos)
                .total_cmp(&b.1.distance_squared(pos))
        });
        let mut velocity = Vec3::ZERO;
        let Some(&(target_id, target_feet)) = target else {
            continue;
        };
        let to_target = (target_feet - pos).with_y(0.0);
        let dist = to_target.length();
        let direct = to_target.normalize_or_zero();
        let chest = Vec3::Y * 1.3;
        let sees = dist < 30.0 && line_of_sight(pos + chest, target_feet + chest, &boxes);
        let path = if sees && dist < 18.0 {
            direct
        } else {
            nav.direction(pos).unwrap_or(direct)
        };
        let speed = enemy.speed * if enemy.slow > 0.0 { 0.5 } else { 1.0 };
        let desired = match enemy.kind {
            NetKind::Shooter if sees => 12.0,
            NetKind::Brute => 1.6,
            _ => 1.1,
        };
        if dist > desired {
            velocity = path * speed;
        } else if enemy.kind == NetKind::Shooter && dist < desired - 4.0 {
            velocity = -direct * speed * 0.6;
        }
        if enemy.stun > 0.0 {
            velocity = Vec3::ZERO;
        }
        // Singularities pull, barrier domes push out.
        for f in &forces.0 {
            let to = (f.pos - pos).with_y(0.0);
            let d = to.length();
            if d < f.radius && d > 1e-3 {
                let dir = to / d;
                if f.strength > 0.0 {
                    velocity = velocity * 0.3 + dir * f.strength * (0.4 + 0.6 * d / f.radius);
                } else {
                    let inward = velocity.dot(dir).max(0.0);
                    velocity += -dir * (inward - f.strength * (1.0 - d / f.radius) * 2.0);
                }
            }
        }
        for (other, opos) in &positions {
            if *other == entity {
                continue;
            }
            let away = (pos - *opos).with_y(0.0);
            let d = away.length();
            if d < 1.1 && d > 1e-3 {
                velocity += away / d * (1.1 - d) * 5.0;
            }
        }
        let mut new_pos = pos + velocity * dt;
        new_pos.y = 0.0;
        resolve_collisions(&mut new_pos, 0.38 * scale, 0.0, &boxes);
        new_pos.x = new_pos.x.clamp(-half + 0.5, half - 0.5);
        new_pos.z = new_pos.z.clamp(-half + 0.5, half - 0.5);
        tf.translation = new_pos;
        let face = if sees || velocity.length_squared() < 0.01 {
            direct
        } else {
            velocity.with_y(0.0).normalize_or_zero()
        };
        if face != Vec3::ZERO {
            let look = Quat::from_rotation_arc(Vec3::NEG_Z, face);
            tf.rotation = tf.rotation.slerp(look, (dt * 10.0).min(1.0));
        }

        enemy.attack_timer -= dt;
        enemy.swing += dt;
        if enemy.stun > 0.0 {
            continue;
        }
        match enemy.kind {
            NetKind::Shooter => {
                if sees && dist < 28.0 && enemy.attack_timer <= 0.0 {
                    enemy.attack_timer = 2.4;
                    enemy.swing = 0.0;
                    let start = new_pos + Vec3::Y * 1.5 * scale + direct * 0.5;
                    let eye = target_feet + Vec3::Y * (EYE_HEIGHT - 0.3);
                    let aim = (eye - start).normalize_or_zero();
                    let id = state.next_net_id;
                    state.next_net_id += 1;
                    commands.spawn((
                        crate::InGameEntity,
                        Replicated {
                            id,
                            kind: NetKind::Fireball,
                        },
                        FireballBrain {
                            velocity: aim * 15.0,
                            life: 4.0,
                            damage: 12.0 + state.round as f32 * 0.5,
                        },
                        Transform::from_translation(start),
                    ));
                }
            }
            kind => {
                let reach = if kind == NetKind::Brute { 2.1 } else { 1.6 };
                if dist < reach && enemy.attack_timer <= 0.0 {
                    let (cd, dmg) = if kind == NetKind::Brute {
                        (1.4, 35.0)
                    } else {
                        (0.9, 15.0)
                    };
                    enemy.attack_timer = cd;
                    enemy.swing = 0.0;
                    if let Some(p) = roster.0.get_mut(&target_id) {
                        hurt_player(p, dmg, &mut hurt);
                    }
                }
            }
        }
    }
}

fn projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut roster: ResMut<Roster>,
    mut hurt: ResMut<LastHurt>,
    mut shots: Query<(Entity, &mut Transform, &mut FireballBrain)>,
    colliders: Query<(&Transform, &Collider), Without<FireballBrain>>,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut shot) in &mut shots {
        tf.translation += shot.velocity * dt;
        shot.life -= dt;
        let pos = tf.translation;
        let mut hit = false;
        for p in roster.0.values_mut().filter(|p| p.alive) {
            if pos.distance(p.feet() + Vec3::Y * 1.0) < 0.9 {
                hurt_player(p, shot.damage, &mut hurt);
                hit = true;
                break;
            }
        }
        let hit_wall = pos.y < 0.0
            || colliders.iter().any(|(ct, c)| {
                let d = (pos - ct.translation).abs();
                d.x < c.half.x && d.y < c.half.y && d.z < c.half.z
            });
        if hit || hit_wall || shot.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}


fn zones(
    mut commands: Commands,
    time: Res<Time>,
    mut roster: ResMut<Roster>,
    mut zones: ResMut<Zones>,
    mut damage: ResMut<DamageQueue>,
    mut fx: ResMut<FxQueue>,
    mut out: ResMut<FxOutbox>,
    enemies: Query<(Entity, &Transform), With<EnemyBrain>>,
) {
    let dt = time.delta_secs();
    for z in zones.0.iter_mut() {
        z.life -= dt;
        if let Some(p) = z.follow.and_then(|id| roster.0.get(&id)) {
            if p.alive {
                z.pos = p.feet();
            } else {
                z.life = z.life.min(0.0);
            }
        }
        z.timer -= dt;
        if z.timer > 0.0 || z.life <= 0.0 {
            continue;
        }
        z.timer += z.interval;
        if z.kind == powers::zone::DOME {
            // Barrier Dome heals instead of hurting.
            for p in roster.0.values_mut() {
                if p.alive && p.feet().distance(z.pos) < z.radius {
                    p.health = (p.health + z.damage).min(p.max_health());
                }
            }
            continue;
        }
        let stun = if z.kind == powers::zone::SMOKE { 0.8 } else { 0.0 };
        if z.kind == 4 {
            // Ragnarok: lightning from the sky on a couple of enemies in reach.
            let mut near: Vec<(Entity, Vec3)> = enemies
                .iter()
                .filter(|(_, t)| t.translation.with_y(0.0).distance(z.pos.with_y(0.0)) < z.radius)
                .map(|(e, t)| (e, t.translation))
                .collect();
            let mut rng = rand::thread_rng();
            for _ in 0..2.min(near.len()) {
                let (e, at) = near.swap_remove(rng.gen_range(0..near.len()));
                damage.0.push(DamageEvent {
                    target: e,
                    amount: z.damage,
                    from: Some(z.owner),
                    headshot: false,
                    legs: false,
                    elements: z.elements,
                    chained: true,
                    stun: 0.0,
                });
                emit(
                    &mut fx,
                    &mut out,
                    Fx::Lightning {
                        a: (at
                            + Vec3::new(rng.gen_range(-2.0..2.0), 16.0, rng.gen_range(-2.0..2.0)))
                        .to_array(),
                        b: (at + Vec3::Y * 0.9).to_array(),
                    },
                );
                emit(
                    &mut fx,
                    &mut out,
                    Fx::Ring {
                        pos: at.to_array(),
                        radius: 1.6,
                        color: [0.6, 0.85, 1.0],
                    },
                );
            }
            continue;
        }
        let mut arcs = 0;
        for (e, t) in &enemies {
            let d = t.translation.with_y(0.0).distance(z.pos.with_y(0.0));
            if d < z.radius && (t.translation.y - z.pos.y).abs() < 3.0 {
                damage.0.push(DamageEvent {
                    target: e,
                    amount: z.damage,
                    from: Some(z.owner),
                    headshot: false,
                    legs: false,
                    elements: z.elements,
                    chained: true,
                    stun,
                });
                if z.kind == 1 && arcs < 4 {
                    arcs += 1;
                    emit(
                        &mut fx,
                        &mut out,
                        Fx::Lightning {
                            a: (z.pos + Vec3::Y * 2.5).to_array(),
                            b: (t.translation + Vec3::Y * 1.2).to_array(),
                        },
                    );
                }
            }
        }
    }
    zones.0.retain(|z| {
        if z.life > 0.0 {
            return true;
        }
        if let Some(m) = z.model {
            if let Ok(mut e) = commands.get_entity(m) {
                e.try_despawn();
            }
        }
        false
    });
}



// ---------------------------------------------------------------------------
// Power-ups, mystery box, extraction, game over
// ---------------------------------------------------------------------------

fn powerups(
    mut commands: Commands,
    time: Res<Time>,
    mut state: ResMut<MatchState>,
    mut roster: ResMut<Roster>,
    mut damage: ResMut<DamageQueue>,
    mut fx: ResMut<FxQueue>,
    mut out: ResMut<FxOutbox>,
    mut pickups: Query<(Entity, &Transform, &mut PowerUpBrain)>,
    enemies: Query<(Entity, &Transform), With<EnemyBrain>>,
) {
    let dt = time.delta_secs();
    for (e, tf, mut p) in &mut pickups {
        p.life -= dt;
        if p.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let grabbed = roster
            .0
            .values()
            .any(|pl| pl.alive && pl.feet().with_y(0.0).distance(tf.translation.with_y(0.0)) < 1.6);
        if !grabbed {
            continue;
        }
        commands.entity(e).despawn();
        state.last_powerup = Some(p.kind);
        state.powerup_seq += 1;
        match p.kind {
            PowerUp::Nuke => {
                for (enemy, t) in &enemies {
                    damage.0.push(DamageEvent {
                        target: enemy,
                        amount: f32::MAX / 4.0,
                        from: None,
                        headshot: false,
                        legs: false,
                        elements: 0,
                        chained: true,
                        stun: 0.0,
                    });
                    emit(
                        &mut fx,
                        &mut out,
                        Fx::Explosion {
                            pos: (t.translation + Vec3::Y).to_array(),
                            radius: 1.5,
                            color: [1.0, 0.6, 0.2],
                        },
                    );
                }
                for pl in roster.0.values_mut() {
                    give_points(pl, 400, &state);
                }
            }
            PowerUp::InstaKill => state.insta_kill = 30.0,
            PowerUp::DoublePoints => state.double_points = 30.0,
            PowerUp::MaxAmmo => state.max_ammo_seq += 1,
        }
    }
}

fn mystery_box(
    time: Res<Time>,
    map: Res<CurrentMap>,
    mut state: ResMut<MatchState>,
    mut roster: ResMut<Roster>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    state.box_state = match state.box_state {
        BoxState::Rolling { player, time } if time - dt <= 0.0 => {
            if state.box_uses >= state.box_move_after && rng.gen_bool(0.5) {
                // The box flies away; refund the spin.
                if let Some(p) = roster.0.get_mut(&player) {
                    p.points += BOX_COST;
                }
                let mut spot = state.box_spot;
                while spot == state.box_spot {
                    spot = rng.gen_range(0..map.0.box_spots.len() as u8);
                }
                state.box_spot = spot;
                state.box_uses = 0;
                state.box_move_after = rng.gen_range(4..9);
                BoxState::Moving { time: 4.0 }
            } else {
                let exclude = roster
                    .0
                    .get(&player)
                    .map(|p| p.guns)
                    .unwrap_or([None, None]);
                let wall: Vec<u8> = map.0.wall_buys.iter().map(|w| w.gun).collect();
                let gun = roll_box_gun(&mut rng, &exclude, &wall);
                BoxState::Offer {
                    player,
                    gun,
                    attach: roll_attachments(gun, &mut rng),
                    time: 9.0,
                }
            }
        }
        BoxState::Rolling { player, time } => BoxState::Rolling {
            player,
            time: time - dt,
        },
        BoxState::Offer { time, .. } if time - dt <= 0.0 => BoxState::Idle,
        BoxState::Offer {
            player,
            gun,
            attach,
            time,
        } => BoxState::Offer {
            player,
            gun,
            attach,
            time: time - dt,
        },
        BoxState::Moving { time } if time - dt <= 0.0 => BoxState::Idle,
        BoxState::Moving { time } => BoxState::Moving { time: time - dt },
        BoxState::Idle => BoxState::Idle,
    };
}

fn extraction(
    time: Res<Time>,
    map: Res<CurrentMap>,
    mut state: ResMut<MatchState>,
    roster: Res<Roster>,
) {
    if state.extraction <= 0.0 || state.game_over {
        return;
    }
    let dt = time.delta_secs();
    state.extraction -= dt;
    let living: Vec<&PlayerInfo> = roster.0.values().filter(|p| p.alive).collect();
    let all_in = !living.is_empty()
        && living
            .iter()
            .all(|p| p.feet().with_y(0.0).distance(map.0.extraction) < EXTRACT_RADIUS);
    if all_in {
        state.extract_hold += dt;
        if state.extract_hold >= 5.0 {
            state.extracted = true;
            state.extraction = 0.0;
        }
    } else {
        state.extract_hold = 0.0;
    }
    if state.extraction <= 0.0 && !state.extracted {
        state.intermission = 0.01;
    }
}

fn check_game_over(roster: Res<Roster>, mut state: ResMut<MatchState>) {
    if state.started
        && !state.game_over
        && !roster.0.is_empty()
        && roster.0.values().all(|p| !p.alive)
    {
        state.game_over = true;
    }
}

/// Fresh match state for the host when a match begins.
pub fn new_match(state: &mut MatchState, roster: &mut Roster, map: u8) {
    let mut rng = rand::thread_rng();
    let (night, sandbox) = (state.night, state.sandbox);
    *state = MatchState::new(map);
    state.night = night;
    state.sandbox = sandbox;
    if sandbox.on {
        // Everything open and plenty of points to try things with.
        state.doors = 0xFE;
    }
    state.started = true;
    state.box_spot = rng.gen_range(0..5);
    state.box_move_after = rng.gen_range(4..9);
    for p in roster.0.values_mut() {
        p.reset_for_match();
        if sandbox.on {
            p.points = 50_000;
        }
    }
}
