//! Host-side game simulation: waves, enemy AI, projectiles, damage and score.
//! Only the host (or a solo player) runs this; clients mirror its results.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::Rng;

use crate::level::{spawn_replicated, GameAssets};
use crate::physics::{collect_boxes, resolve_collisions, trace_shot, Boxes};
use crate::player::GUN_RANGE;
use crate::{
    cursor_locked, Collider, Enemy, EnemyLook, MatchState, NetKind, Phase, Replicated, Roster, Session,
    ShotQueue, Tracers, EYE_HEIGHT, MAX_HEALTH,
};

const GUN_DAMAGE: f32 = 34.0;
const REVIVE_HEALTH: f32 = 50.0;

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TracerEvents>()
            .add_systems(
                Update,
                (start_match, resolve_shots, waves, enemy_ai, projectiles, check_game_over)
                    .chain()
                    .in_set(Phase::Sim),
            )
            .add_systems(Update, enemy_flash.in_set(Phase::Present));
    }
}

/// Shots the host resolved this frame, broadcast so everyone sees tracers:
/// (shooter id, start, end).
#[derive(Resource, Default)]
pub struct TracerEvents(pub Vec<(u8, [f32; 3], [f32; 3])>);

#[derive(Component)]
pub struct EnemyBrain {
    kind: NetKind,
    health: f32,
    speed: f32,
    attack_timer: f32,
}

#[derive(Component)]
pub struct ProjectileBrain {
    velocity: Vec3,
    life: f32,
}

fn resolve_shots(
    mut commands: Commands,
    session: Res<Session>,
    state: Res<MatchState>,
    mut roster: ResMut<Roster>,
    mut shots: ResMut<ShotQueue>,
    mut tracers: ResMut<Tracers>,
    mut events: ResMut<TracerEvents>,
    mut enemies: Query<(Entity, &Transform, &mut EnemyBrain, &mut EnemyLook)>,
    colliders: Query<(&Transform, &Collider)>,
) {
    if shots.0.is_empty() {
        return;
    }
    let boxes = collect_boxes(colliders.iter());
    for (shooter, origin, dir) in shots.0.drain(..) {
        let dir = dir.normalize_or_zero();
        if dir == Vec3::ZERO {
            continue;
        }
        let (dist, hit) = trace_shot(
            origin,
            dir,
            GUN_RANGE,
            &boxes,
            enemies
                .iter()
                .filter(|(_, _, b, _)| b.health > 0.0)
                .map(|(e, t, _, _)| (e, t.translation)),
        );
        let end = origin + dir * dist;
        // Remote players' shots: show their tracers here too.
        if shooter != session.my_id {
            let start = origin - Vec3::Y * 0.2;
            tracers.0.push((start, end, 0.06));
        }
        events.0.push((shooter, origin.to_array(), end.to_array()));

        let Some(entity) = hit else { continue };
        let Ok((_, _, mut brain, mut look)) = enemies.get_mut(entity) else {
            continue;
        };
        brain.health -= GUN_DAMAGE;
        look.flash = 0.1;
        if brain.health <= 0.0 {
            if let Some(p) = roster.0.get_mut(&shooter) {
                let base = match brain.kind {
                    NetKind::Shooter => 150,
                    _ => 100,
                };
                p.score += base * state.wave.max(1);
                p.kills += 1;
            }
            commands.entity(entity).despawn();
        }
    }
}

fn start_match(window: Single<&Window, With<PrimaryWindow>>, mut state: ResMut<MatchState>) {
    if !state.started && cursor_locked(&window) {
        state.started = true;
    }
}

fn waves(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<GameAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<MatchState>,
    mut roster: ResMut<Roster>,
    enemies: Query<(), With<EnemyBrain>>,
) {
    if state.game_over || !state.started {
        return;
    }
    let dt = time.delta_secs();

    if state.to_spawn == 0 && enemies.is_empty() {
        if state.intermission <= 0.0 {
            // Wave cleared: heal survivors a bit.
            state.intermission = 3.0;
            if state.wave > 0 {
                for p in roster.0.values_mut().filter(|p| p.alive) {
                    p.health = (p.health + 25.0).min(MAX_HEALTH);
                }
            }
        }
        state.intermission -= dt;
        if state.intermission <= 0.0 {
            state.wave += 1;
            let players = roster.0.len().max(1) as u32;
            // More players, more enemies.
            state.to_spawn = 3 + state.wave * 2 + (players - 1) * (2 + state.wave);
            state.spawn_timer = 0.0;
            // Downed players come back at the start of each wave.
            for p in roster.0.values_mut().filter(|p| !p.alive) {
                p.alive = true;
                p.health = REVIVE_HEALTH;
                p.spawn_seq += 1;
            }
        }
        return;
    }

    if state.to_spawn == 0 {
        return;
    }
    state.spawn_timer -= dt;
    if state.spawn_timer > 0.0 {
        return;
    }
    state.spawn_timer = (1.2 - state.wave as f32 * 0.08).max(0.35);
    state.to_spawn -= 1;

    let mut rng = rand::thread_rng();
    let spawn_points = [
        Vec3::new(-26.0, 0.0, -26.0),
        Vec3::new(26.0, 0.0, -26.0),
        Vec3::new(-26.0, 0.0, 26.0),
        Vec3::new(26.0, 0.0, 26.0),
        Vec3::new(0.0, 0.0, -26.0),
    ];
    // Prefer spawn points that aren't right next to any player.
    let far: Vec<Vec3> = spawn_points
        .iter()
        .copied()
        .filter(|s| roster.0.values().all(|p| p.feet().with_y(0.0).distance(*s) > 12.0))
        .collect();
    let base = if far.is_empty() {
        spawn_points[rng.gen_range(0..spawn_points.len())]
    } else {
        far[rng.gen_range(0..far.len())]
    };
    let pos = base + Vec3::new(rng.gen_range(-2.0..2.0), 1.0, rng.gen_range(-2.0..2.0));

    let kind = if state.wave >= 2 && rng.gen_bool(0.35) {
        NetKind::Shooter
    } else {
        NetKind::Grunt
    };
    let (health, speed) = match kind {
        NetKind::Shooter => (70.0, 2.5),
        _ => (100.0, (3.2 + state.wave as f32 * 0.25).min(7.0)),
    };

    let id = state.next_net_id;
    state.next_net_id += 1;
    let e = spawn_replicated(&mut commands, &assets, &mut materials, id, kind, pos);
    commands.entity(e).insert(EnemyBrain {
        kind,
        health,
        speed,
        attack_timer: 1.0,
    });
}

fn enemy_ai(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<GameAssets>,
    mut state: ResMut<MatchState>,
    mut roster: ResMut<Roster>,
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
    let positions: Vec<(Entity, Vec3)> = enemies.iter().map(|(e, t, _)| (e, t.translation)).collect();

    for (entity, mut tf, mut enemy) in &mut enemies {
        // Chase the nearest living player.
        let target = targets.iter().min_by(|a, b| {
            let da = a.1.distance_squared(tf.translation);
            let db = b.1.distance_squared(tf.translation);
            da.total_cmp(&db)
        });

        let mut velocity = Vec3::ZERO;
        let mut dist = f32::MAX;
        let mut dir = Vec3::ZERO;
        if let Some((_, target_feet)) = target {
            let to_target = (*target_feet - tf.translation).with_y(0.0);
            dist = to_target.length();
            dir = to_target.normalize_or_zero();
            if dist > 0.01 {
                let look = Vec3::new(target_feet.x, tf.translation.y, target_feet.z);
                tf.look_at(look, Vec3::Y);
            }
            let desired = match enemy.kind {
                NetKind::Shooter => 12.0,
                _ => 1.2,
            };
            if dist > desired {
                velocity = dir * enemy.speed;
            } else if enemy.kind == NetKind::Shooter && dist < desired - 3.0 {
                velocity = -dir * enemy.speed * 0.6;
            }
        }

        // Separation from other enemies.
        for (other, pos) in &positions {
            if *other == entity {
                continue;
            }
            let away = (tf.translation - *pos).with_y(0.0);
            let d = away.length();
            if d < 1.3 && d > 1e-3 {
                velocity += away / d * (1.3 - d) * 6.0;
            }
        }

        let mut pos = tf.translation + velocity * dt;
        let feet_y = pos.y - 1.0;
        resolve_collisions(&mut pos, 0.5, feet_y, &boxes);
        tf.translation = pos;

        enemy.attack_timer -= dt;
        let Some((target_id, target_feet)) = target else {
            continue;
        };
        match enemy.kind {
            NetKind::Shooter => {
                if dist < 30.0 && enemy.attack_timer <= 0.0 {
                    enemy.attack_timer = 2.2;
                    let start = tf.translation + Vec3::Y * 0.45 + dir * 0.6;
                    let eye = *target_feet + Vec3::Y * (EYE_HEIGHT - 0.4);
                    let aim = (eye - start).normalize_or_zero();
                    let id = state.next_net_id;
                    state.next_net_id += 1;
                    commands.spawn((
                        Replicated {
                            id,
                            kind: NetKind::Projectile,
                        },
                        ProjectileBrain {
                            velocity: aim * 14.0,
                            life: 4.0,
                        },
                        Mesh3d(assets.projectile_mesh.clone()),
                        MeshMaterial3d(assets.projectile_material.clone()),
                        Transform::from_translation(start),
                    ));
                }
            }
            _ => {
                if dist < 1.8 && enemy.attack_timer <= 0.0 {
                    enemy.attack_timer = 0.9;
                    if let Some(p) = roster.0.get_mut(target_id) {
                        p.damage(12.0);
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
    mut shots: Query<(Entity, &mut Transform, &mut ProjectileBrain)>,
    colliders: Query<(&Transform, &Collider), Without<ProjectileBrain>>,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut shot) in &mut shots {
        tf.translation += shot.velocity * dt;
        shot.life -= dt;

        let hit_player = roster
            .0
            .values_mut()
            .filter(|p| p.alive)
            .find(|p| tf.translation.distance(p.feet() + Vec3::Y * 0.9) < 0.9);
        let hit = hit_player.is_some();
        if let Some(p) = hit_player {
            p.damage(10.0);
        }
        let hit_wall = tf.translation.y < 0.0
            || colliders.iter().any(|(ct, c)| {
                let d = (tf.translation - ct.translation).abs();
                d.x < c.half.x && d.y < c.half.y && d.z < c.half.z
            });
        if hit || hit_wall || shot.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

fn check_game_over(roster: Res<Roster>, mut state: ResMut<MatchState>) {
    if !state.game_over && !roster.0.is_empty() && roster.0.values().all(|p| !p.alive) {
        state.game_over = true;
    }
}

/// Host presses Enter after a game over to start again with everyone.
pub fn restart(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MatchState>,
    mut roster: ResMut<Roster>,
    mut tracers: ResMut<Tracers>,
    things: Query<Entity, With<Replicated>>,
) {
    if !state.game_over || !keys.just_pressed(KeyCode::Enter) {
        return;
    }
    for e in &things {
        commands.entity(e).despawn();
    }
    let next_net_id = state.next_net_id;
    *state = MatchState {
        intermission: 1.5,
        started: true,
        next_net_id,
        ..default()
    };
    tracers.0.clear();
    for p in roster.0.values_mut() {
        p.health = MAX_HEALTH;
        p.alive = true;
        p.score = 0;
        p.kills = 0;
        p.spawn_seq += 1;
    }
}

/// Flash enemies white briefly when hit (host and clients).
fn enemy_flash(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut enemies: Query<(&mut EnemyLook, &MeshMaterial3d<StandardMaterial>), With<Enemy>>,
) {
    for (mut look, mat) in &mut enemies {
        if look.flash <= 0.0 {
            continue;
        }
        look.flash -= time.delta_secs();
        if let Some(m) = materials.get_mut(&mat.0) {
            m.base_color = if look.flash > 0.0 {
                Color::WHITE
            } else {
                look.base_color
            };
        }
    }
}
