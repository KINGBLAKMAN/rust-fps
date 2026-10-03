//! Host-side game simulation: rounds, enemy AI, damage, elements, points, XP
//! and levels, abilities, the mystery box, perks, power-ups and extraction.
//! Only the host (or a solo player) runs this; clients mirror its results.

use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;

use crate::avatars::spawn_replicated;
use crate::data::{
    ability_cooldown, elements_in, gun_def, has_perk, roll_box_gun, xp_to_next, Character,
    Element, GunSpecial, Perk, PowerUp, Upgrade, BOX_COST, MAX_LEVEL, MAX_TIER,
};
use crate::fx::{emit, rgb, Fx, FxOutbox, FxQueue};
use crate::maps::{CurrentMap, EXTRACT_RADIUS};
use crate::nav::NavGrid;
use crate::physics::{collect_boxes, line_of_sight, ray_world, resolve_collisions, trace_shot, Boxes};
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

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DamageQueue>()
            .init_resource::<Strikes>()
            .init_resource::<LastHurt>()
            .add_systems(
                Update,
                (
                    resolve_shots,
                    player_timers,
                    status_effects,
                    projectiles,
                    grenades,
                    orbital_strikes,
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
}

#[derive(Component)]
pub struct FireballBrain {
    velocity: Vec3,
    life: f32,
    damage: f32,
}

#[derive(Component)]
pub struct GrenadeBrain {
    owner: u8,
    velocity: Vec3,
    fuse: f32,
    damage: f32,
    radius: f32,
    elements: u8,
}

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
    elements: u8,
    chained: bool,
}

#[derive(Resource, Default)]
struct DamageQueue(Vec<DamageEvent>);

/// Pending orbital strikes: (owner, position, delay, radius, damage, elements).
#[derive(Resource, Default)]
struct Strikes(Vec<(u8, Vec3, f32, f32, f32, u8)>);

/// When each player last took damage (for health regeneration).
#[derive(Resource, Default)]
struct LastHurt(HashMap<u8, f32>);

fn clear_sim(mut damage: ResMut<DamageQueue>, mut strikes: ResMut<Strikes>, mut hurt: ResMut<LastHurt>) {
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
                elements,
                chained: false,
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
    let amount = if state.double_points > 0.0 { amount * 2 } else { amount };
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
    mut fx: ResMut<FxQueue>,
    mut out: ResMut<FxOutbox>,
    enemies: Query<(Entity, &Transform), With<EnemyBrain>>,
    colliders: Query<(&Transform, &Collider)>,
) {
    if actions.0.is_empty() {
        return;
    }
    let enemy_list: Vec<(Entity, Vec3)> = enemies.iter().map(|(e, t)| (e, t.translation)).collect();
    let mut rng = rand::thread_rng();
    for (id, seq, action) in std::mem::take(&mut actions.0) {
        let Some(p) = roster.0.get_mut(&id) else { continue };
        if seq <= p.action_ack {
            continue;
        }
        p.action_ack = seq;
        if state.game_over || state.extracted {
            continue;
        }
        match action {
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
                p.choices = if p.pending_picks > 0 { roll_choices(p) } else { Vec::new() };
            }
            PlayerAction::Interact => {
                if !p.alive {
                    continue;
                }
                let feet = p.feet();
                let box_pos = map.0.box_spots[state.box_spot as usize];
                let near_box = feet.with_y(0.0).distance(box_pos) < INTERACT_RANGE;
                match state.box_state {
                    BoxState::Offer { player, gun, .. } if near_box && player == id => {
                        let slot = if p.guns[1].is_none() { 1 } else { p.active_slot as usize };
                        p.guns[slot] = Some(gun);
                        p.active_slot = slot as u8;
                        state.box_state = BoxState::Idle;
                        continue;
                    }
                    BoxState::Idle if near_box => {
                        if p.points >= BOX_COST {
                            p.points -= BOX_COST;
                            state.box_uses += 1;
                            state.box_state = BoxState::Rolling { player: id, time: 3.0 };
                        }
                        continue;
                    }
                    _ => {}
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
            PlayerAction::Ability { slot, origin, dir, cook } => {
                if !p.alive || slot > 2 {
                    continue;
                }
                let s = slot as usize;
                if slot == 2 {
                    if p.ult_charge < 100.0 {
                        continue;
                    }
                    p.ult_charge = 0.0;
                } else {
                    if p.cooldowns[s] > 0.0 {
                        continue;
                    }
                    p.cooldowns[s] = ability_cooldown(p.character, s, p.tiers[s]);
                }
                let origin = Vec3::from_array(origin);
                let dir = Vec3::from_array(dir).normalize_or_zero();
                if !origin.is_finite() || dir == Vec3::ZERO {
                    continue;
                }
                let tier = p.tiers[s] as f32;
                let mult = level_multiplier(p);
                let elements = p.ability_elements;
                let feet = p.feet();
                match (p.character, slot) {
                    (Character::Striker, 0) => {
                        // Dash is movement, done locally; everyone else sees
                        // the streak.
                        let flat = dir.with_y(0.0).normalize_or_zero();
                        let dash = Fx::Dash {
                            player: id,
                            a: feet.to_array(),
                            b: (feet + flat * 22.0 * (0.18 + 0.03 * tier)).to_array(),
                        };
                        out.0.push(dash);
                    }
                    (Character::Striker, 1) => {
                        let id_net = state.next_net_id;
                        state.next_net_id += 1;
                        commands.spawn((
                            crate::InGameEntity,
                            Replicated {
                                id: id_net,
                                kind: NetKind::Grenade,
                            },
                            GrenadeBrain {
                                owner: id,
                                velocity: dir * crate::abilities::GRENADE_SPEED
                                    + Vec3::Y * crate::abilities::GRENADE_LIFT,
                                fuse: (crate::abilities::GRENADE_FUSE
                                    - cook.clamp(0.0, crate::abilities::MAX_COOK))
                                .max(0.15),
                                damage: (150.0 + 60.0 * tier) * mult,
                                radius: 4.0 + 0.5 * tier,
                                elements,
                            },
                            Transform::from_translation(origin + dir * 0.6),
                        ));
                    }
                    (Character::Striker, _) => {
                        p.overdrive = 8.0 + 2.0 * tier;
                    }
                    (Character::Warden, 0) => {
                        let heal = 40.0 + 20.0 * tier;
                        let center = feet;
                        for other in roster.0.values_mut() {
                            if other.alive && other.feet().distance(center) < 8.0 {
                                other.health = (other.health + heal).min(other.max_health());
                            }
                        }
                        emit(
                            &mut fx,
                            &mut out,
                            Fx::Heal {
                                pos: center.to_array(),
                                radius: 8.0,
                            },
                        );
                    }
                    (Character::Warden, 1) => {
                        let dmg = (40.0 + 30.0 * tier) * mult;
                        for (e, pos) in &enemy_list {
                            if pos.distance(feet) < 7.0 {
                                damage.0.push(DamageEvent {
                                    target: *e,
                                    amount: dmg,
                                    from: Some(id),
                                    headshot: false,
                                    elements: elements | Element::Ice.bit(),
                                    chained: false,
                                });
                            }
                        }
                        emit(
                            &mut fx,
                            &mut out,
                            Fx::Nova {
                                pos: feet.to_array(),
                                radius: 7.0,
                            },
                        );
                    }
                    (Character::Warden, _) => {
                        let boxes = collect_boxes(colliders.iter());
                        let dist = ray_world(origin, dir, 80.0, &boxes);
                        let target = (origin + dir * dist).with_y(0.0);
                        let radius = 9.0 + tier;
                        strikes.0.push((id, target, 1.2, radius, 3000.0 * mult, elements));
                        emit(
                            &mut fx,
                            &mut out,
                            Fx::Beam {
                                pos: target.to_array(),
                                radius,
                                delay: 1.2,
                            },
                        );
                    }
                }
                let _ = &mut rng;
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
    let enemy_list: Vec<(Entity, Vec3)> = enemies.iter().map(|(e, t, _)| (e, t.translation)).collect();
    let mut rng = rand::thread_rng();
    for (shooter, shot) in shots.0.drain(..) {
        let Some(p) = roster.0.get(&shooter) else { continue };
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
                .map(|(e, t, b)| (e, t.translation, enemy_scale(b.kind))),
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

        let mult = level_multiplier(p);
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
        let Some((entity, headshot)) = hit.enemy else { continue };
        let amount = def.damage * mult * if headshot { def.headshot } else { 1.0 };
        damage.0.push(DamageEvent {
            target: entity,
            amount,
            from: Some(shooter),
            headshot,
            elements,
            chained: false,
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
                    elements,
                    chained: true,
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
        if b.burn > 0.0 {
            b.burn -= dt;
            damage.0.push(DamageEvent {
                target: e,
                amount: b.burn_dps * dt,
                from: b.burn_by,
                headshot: false,
                elements: 0,
                chained: true,
            });
        }
        status.burning = b.burn > 0.0;
        status.slowed = b.slow > 0.0;
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
    let positions: Vec<(Entity, Vec3)> = enemies.iter().map(|(e, t, _, _)| (e, t.translation)).collect();
    let mut rng = rand::thread_rng();
    let round = state.round;
    let mut i = 0;
    while i < queue.0.len() {
        let ev = &queue.0[i];
        let (target, from, headshot, elements, chained) =
            (ev.target, ev.from, ev.headshot, ev.elements, ev.chained);
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
                            elements: 0,
                            chained: true,
                        });
                    }
                }
                _ => {}
            }
        }
        let killed = brain.health <= 0.0;
        if let Some(pid) = from {
            if let Some(p) = roster.0.get_mut(&pid) {
                if killed {
                    let bonus = if headshot { 40 } else { 0 };
                    let base = if brain.kind == NetKind::Brute { 120 } else { 60 };
                    give_points(p, base + bonus, &state);
                    p.kills += 1;
                    p.ult_charge = (p.ult_charge + 3.0).min(100.0);
                    let xp = if brain.kind == NetKind::Brute { 60 } else { 25 } + if headshot { 15 } else { 0 };
                    give_xp(p, xp);
                } else if !chained {
                    give_points(p, 10, &state);
                }
            }
        }
        if killed {
            commands.entity(target).despawn();
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
    meshes: Res<crate::humanoid::HumanoidMeshes>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    map: Res<CurrentMap>,
    mut state: ResMut<MatchState>,
    mut roster: ResMut<Roster>,
    enemies: Query<(), With<EnemyBrain>>,
) {
    if state.game_over || state.extracted || !state.started {
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
    let living: Vec<Vec3> = roster.0.values().filter(|p| p.alive).map(|p| p.feet()).collect();
    let far: Vec<Vec3> = map
        .0
        .enemy_spawns
        .iter()
        .copied()
        .filter(|s| living.iter().all(|p| p.distance(*s) > 14.0))
        .collect();
    let pool = if far.is_empty() { &map.0.enemy_spawns } else { &far };
    let base = pool[rng.gen_range(0..pool.len())];
    let pos = base + Vec3::new(rng.gen_range(-1.5..1.5), 0.0, rng.gen_range(-1.5..1.5));

    let kind = if r >= 6 && rng.gen_bool(0.1) {
        NetKind::Brute
    } else if r >= 3 && rng.gen_bool(0.2) {
        NetKind::Shooter
    } else {
        NetKind::Grunt
    };
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
    let e = spawn_replicated(&mut commands, &assets, &meshes, &mut materials, id, kind, pos);
    commands.entity(e).insert(EnemyBrain {
        kind,
        health,
        speed,
        attack_timer: 1.0,
        burn: 0.0,
        burn_dps: 0.0,
        burn_by: None,
        slow: 0.0,
    });
}

fn enemy_ai(
    mut commands: Commands,
    time: Res<Time>,
    map: Res<CurrentMap>,
    mut nav: ResMut<NavGrid>,
    mut state: ResMut<MatchState>,
    mut roster: ResMut<Roster>,
    mut hurt: ResMut<LastHurt>,
    mut enemies: Query<(Entity, &mut Transform, &mut EnemyBrain)>,
    colliders: Query<(&Transform, &Collider), Without<EnemyBrain>>,
) {
    let dt = time.delta_secs();
    let boxes: Boxes = collect_boxes(colliders.iter());
    let targets: Vec<(u8, Vec3)> = roster
        .0
        .values()
        .filter(|p| p.alive)
        .map(|p| (p.id, p.feet()))
        .collect();
    let target_points: Vec<Vec3> = targets.iter().map(|t| t.1).collect();
    nav.update(dt, &boxes, &target_points);
    let positions: Vec<(Entity, Vec3)> = enemies.iter().map(|(e, t, _)| (e, t.translation)).collect();
    let half = map.0.half;

    for (entity, mut tf, mut enemy) in &mut enemies {
        let pos = tf.translation;
        let scale = enemy_scale(enemy.kind);
        let target = targets
            .iter()
            .min_by(|a, b| a.1.distance_squared(pos).total_cmp(&b.1.distance_squared(pos)));
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
        let face = if sees || velocity.length_squared() < 0.01 { direct } else { velocity.with_y(0.0).normalize_or_zero() };
        if face != Vec3::ZERO {
            let look = Quat::from_rotation_arc(Vec3::NEG_Z, face);
            tf.rotation = tf.rotation.slerp(look, (dt * 10.0).min(1.0));
        }

        enemy.attack_timer -= dt;
        match enemy.kind {
            NetKind::Shooter => {
                if sees && dist < 28.0 && enemy.attack_timer <= 0.0 {
                    enemy.attack_timer = 2.4;
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
                    let (cd, dmg) = if kind == NetKind::Brute { (1.4, 35.0) } else { (0.9, 15.0) };
                    enemy.attack_timer = cd;
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

fn grenades(
    mut commands: Commands,
    time: Res<Time>,
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
    for (e, mut tf, mut g) in &mut nades {
        g.fuse -= dt;
        g.velocity.y -= crate::abilities::GRENADE_GRAVITY * dt;
        let next = tf.translation + g.velocity * dt;
        // Bounce off the floor and stop at walls.
        if next.y < 0.1 {
            g.velocity.y = -g.velocity.y * 0.35;
            g.velocity *= 0.6;
            tf.translation.y = 0.1;
        } else if !line_of_sight(tf.translation, next, &boxes) {
            g.velocity = -g.velocity * 0.3;
        } else {
            tf.translation = next;
        }
        let touching = enemy_list
            .iter()
            .any(|(_, p)| (*p + Vec3::Y).distance(tf.translation) < 1.0);
        if g.fuse <= 0.0 || touching {
            let mut color = Color::srgb(1.0, 0.55, 0.15);
            if let Some(el) = elements_in(g.elements).next() {
                color = el.color();
            }
            explode(
                tf.translation + Vec3::Y * 0.3,
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
}

fn orbital_strikes(
    time: Res<Time>,
    mut strikes: ResMut<Strikes>,
    mut damage: ResMut<DamageQueue>,
    enemies: Query<(Entity, &Transform), With<EnemyBrain>>,
) {
    let dt = time.delta_secs();
    let mut done = Vec::new();
    for (i, s) in strikes.0.iter_mut().enumerate() {
        s.2 -= dt;
        if s.2 <= 0.0 {
            done.push(i);
        }
    }
    for i in done.into_iter().rev() {
        let (owner, pos, _, radius, dmg, elements) = strikes.0.remove(i);
        for (e, t) in &enemies {
            if t.translation.with_y(0.0).distance(pos) < radius {
                damage.0.push(DamageEvent {
                    target: e,
                    amount: dmg,
                    from: Some(owner),
                    headshot: false,
                    elements,
                    chained: false,
                });
            }
        }
    }
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
                        elements: 0,
                        chained: true,
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

fn mystery_box(time: Res<Time>, map: Res<CurrentMap>, mut state: ResMut<MatchState>, mut roster: ResMut<Roster>) {
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
                let exclude = roster.0.get(&player).map(|p| p.guns).unwrap_or([None, None]);
                BoxState::Offer {
                    player,
                    gun: roll_box_gun(&mut rng, &exclude),
                    time: 9.0,
                }
            }
        }
        BoxState::Rolling { player, time } => BoxState::Rolling {
            player,
            time: time - dt,
        },
        BoxState::Offer { time, .. } if time - dt <= 0.0 => BoxState::Idle,
        BoxState::Offer { player, gun, time } => BoxState::Offer {
            player,
            gun,
            time: time - dt,
        },
        BoxState::Moving { time } if time - dt <= 0.0 => BoxState::Idle,
        BoxState::Moving { time } => BoxState::Moving { time: time - dt },
        BoxState::Idle => BoxState::Idle,
    };
}

fn extraction(time: Res<Time>, map: Res<CurrentMap>, mut state: ResMut<MatchState>, roster: Res<Roster>) {
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
    *state = MatchState::new(map);
    state.started = true;
    state.box_spot = rng.gen_range(0..5);
    state.box_move_after = rng.gen_range(4..9);
    for p in roster.0.values_mut() {
        p.reset_for_match();
    }
}
