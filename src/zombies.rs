//! Zombie bodies: drives the rig from what the host says about each zombie
//! (crawling, swinging, flinching) and plays the death animation before the
//! body sinks away.

use bevy::prelude::*;
use rand::Rng;

use crate::rig::Rig;
use crate::{AppState, Collider, Enemy, EnemyStatus, Phase, Replicated};

pub struct ZombiePlugin;

impl Plugin for ZombiePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (start_dying, sync_rigs, dying)
                .chain()
                .in_set(Phase::Present)
                .run_if(in_state(AppState::InGame)),
        );
    }
}

/// Marks something the game is done with. Zombies fall over first; anything
/// else is removed straight away.
#[derive(Component)]
struct Killed;

/// A zombie playing its death animation.
#[derive(Component)]
struct Dying {
    t: f32,
    kind: u8,
}

/// How long a body lies there before it starts sinking, and how long it sinks.
const LIE: f32 = 3.0;
const SINK: f32 = 1.0;

/// Removes `e` from play: zombies collapse, everything else just goes.
pub fn kill(commands: &mut Commands, e: Entity) {
    if let Ok(mut ent) = commands.get_entity(e) {
        ent.try_insert(Killed);
    }
}

fn start_dying(
    mut commands: Commands,
    killed: Query<(Entity, Has<Enemy>, Has<Rig>), Added<Killed>>,
) {
    let mut rng = rand::thread_rng();
    for (e, enemy, rig) in &killed {
        if enemy && rig {
            // Out of the game straight away (no more shots, nav, snapshots),
            // but the body stays for its fall.
            commands
                .entity(e)
                .remove::<(
                    Enemy,
                    Replicated,
                    Collider,
                    crate::sim::EnemyBrain,
                    crate::net::Puppet,
                )>()
                .insert(Dying {
                    t: 0.0,
                    kind: rng.gen_range(0..3),
                });
        } else {
            commands.entity(e).despawn();
        }
    }
}

/// Copies the host's status onto the rig so every machine animates the same.
fn sync_rigs(mut zombies: Query<(&EnemyStatus, &mut Rig), (With<Enemy>, Without<Dying>)>) {
    for (status, mut rig) in &mut zombies {
        rig.crawl = status.crawler;
        if status.attacking && rig.swing > 0.6 {
            rig.swing = 0.0;
        }
        if status.flash > 0.0 {
            rig.flinch = 1.0;
        }
    }
}

fn dying(
    mut commands: Commands,
    time: Res<Time>,
    mut bodies: Query<(Entity, &mut Dying, &mut Rig, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (e, mut d, mut rig, mut tf) in &mut bodies {
        d.t += dt;
        rig.dying = Some((d.t, d.kind));
        if d.t > LIE {
            tf.translation.y -= dt * 0.6 / SINK;
        }
        if d.t > LIE + SINK {
            commands.entity(e).despawn();
        }
    }
}

