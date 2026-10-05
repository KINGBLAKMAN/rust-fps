//! Game feel: camera shake, the dust (or pollen, or ash) drifting in the
//! air, and warming up the effects at the start of a match so the first
//! grenade doesn't stutter while its shaders are built.

use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;

use crate::config::Settings;
use crate::fx::{Fx, FxQueue};
use crate::player::LocalPlayer;
use crate::rig::Rig;
use crate::sim::BRUTE_WINDUP;
use crate::{AppState, InGameEntity, MatchState, NetKind, Phase, Replicated};

/// Camera shake, 0 to 1. Anything can add to it; it fades on its own.
#[derive(Resource, Default)]
pub struct Shake(pub f32);

impl Shake {
    pub fn add(&mut self, amount: f32) {
        self.0 = (self.0 + amount.max(0.0)).min(1.0);
    }
}

/// How far the camera moves at full shake (it goes with the square, so
/// small shakes stay small).
const SHAKE_MAX: f32 = 0.09;
/// Shake lost per second.
const SHAKE_FADE: f32 = 2.5;

pub struct FeelPlugin;

impl Plugin for FeelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Shake>()
            .add_systems(
                Update,
                (
                    shake_sources.before(crate::fx::play),
                    drift,
                    warm_up.before(crate::fx::play),
                )
                    .in_set(Phase::Present)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                OnEnter(AppState::InGame),
                (spawn_motes.after(crate::game::start_match), start_warm_up),
            )
            .add_systems(OnExit(AppState::InGame), |mut shake: ResMut<Shake>| shake.0 = 0.0)
            .add_systems(
                PostUpdate,
                shake_camera
                    .before(bevy::transform::TransformSystem::TransformPropagate)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

/// Explosions and slams near you, and Brutes slamming close by.
fn shake_sources(
    queue: Res<FxQueue>,
    mut shake: ResMut<Shake>,
    player: Option<Single<&LocalPlayer>>,
    brutes: Query<(Entity, &Replicated, &Transform, &Rig)>,
    mut swings: Local<HashMap<Entity, f32>>,
) {
    let Some(player) = player else { return };
    let eye = player.eye_pos();
    let near = |pos: [f32; 3], reach: f32| {
        (1.0 - Vec3::from_array(pos).distance(eye) / reach).max(0.0)
    };
    for fx in &queue.0 {
        match *fx {
            Fx::Explosion { pos, radius, .. } => {
                let size = (radius / 5.0).min(1.0);
                shake.add(0.7 * size * near(pos, radius * 3.0 + 8.0));
            }
            Fx::Slam { pos, radius } => shake.add(0.8 * near(pos, radius * 3.0 + 8.0)),
            _ => {}
        }
    }
    swings.retain(|e, _| brutes.contains(*e));
    for (e, r, tf, rig) in &brutes {
        if r.kind != NetKind::Brute {
            continue;
        }
        let before = swings.insert(e, rig.swing).unwrap_or(9.0);
        if before < BRUTE_WINDUP && rig.swing >= BRUTE_WINDUP && rig.dying.is_none() {
            shake.add(0.5 * near(tf.translation.to_array(), 8.0));
        }
    }
}

fn shake_camera(
    time: Res<Time>,
    settings: Res<Settings>,
    mut shake: ResMut<Shake>,
    camera: Option<Single<&mut Transform, With<LocalPlayer>>>,
) {
    shake.0 = (shake.0 - SHAKE_FADE * time.delta_secs()).max(0.0);
    let (Some(mut tf), true) = (camera, settings.camera_shake) else {
        return;
    };
    if shake.0 <= 0.0 {
        return;
    }
    // Smooth wobble (sums of sines), not a new random spot every frame.
    let t = time.elapsed_secs();
    let wobble = Vec3::new(
        (t * 23.0).sin() + 0.5 * (t * 41.0 + 1.3).sin(),
        (t * 19.0 + 0.7).sin() + 0.5 * (t * 37.0 + 2.1).sin(),
        (t * 29.0 + 2.9).sin() * 0.5,
    ) / 1.5;
    tf.translation += wobble * shake.0 * shake.0 * SHAKE_MAX;
}

/// A speck in the air, drifting.
#[derive(Component)]
struct Mote {
    /// Where it is relative to the camera's box.
    pos: Vec3,
    vel: Vec3,
    phase: f32,
}

/// The box of air round the camera the specks fill (they wrap round it).
const MOTE_BOX: f32 = 22.0;
const MOTES: usize = 140;

/// What drifts on each map: colour, size and how it moves.
fn mote_look(map: usize, night: bool) -> (Color, f32, Vec3) {
    match (map, night) {
        // Park: pollen by day, fireflies at night.
        (1, false) => (Color::srgba(1.0, 0.95, 0.7, 0.55), 0.018, Vec3::new(0.25, 0.05, 0.1)),
        (1, true) => (Color::srgba(0.9, 1.4, 0.5, 0.8), 0.022, Vec3::new(0.1, 0.05, 0.05)),
        // Neighbourhood: fine ash and dust settling.
        (2, _) => (Color::srgba(0.8, 0.78, 0.75, 0.6), 0.019, Vec3::new(0.15, -0.12, 0.05)),
        // Shipping yard: dust blowing in off the water.
        _ => (Color::srgba(0.85, 0.82, 0.75, 0.6), 0.018, Vec3::new(0.45, 0.0, 0.2)),
    }
}

fn spawn_motes(
    mut commands: Commands,
    settings: Res<Settings>,
    state: Res<MatchState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !settings.particles {
        return;
    }
    let (color, size, wind) = mote_look(state.map as usize, state.night);
    let mesh = meshes.add(Sphere::new(size).mesh().ico(1).unwrap());
    let material = materials.add(StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let mut rng = rand::thread_rng();
    for _ in 0..MOTES {
        let pos = Vec3::new(
            rng.gen_range(0.0..MOTE_BOX),
            rng.gen_range(0.0..MOTE_BOX * 0.4),
            rng.gen_range(0.0..MOTE_BOX),
        );
        let vel = wind * rng.gen_range(0.6..1.4)
            + Vec3::new(
                rng.gen_range(-0.05..0.05),
                rng.gen_range(-0.03..0.03),
                rng.gen_range(-0.05..0.05),
            );
        commands.spawn((
            InGameEntity,
            Mote {
                pos,
                vel,
                phase: rng.gen_range(0.0..std::f32::consts::TAU),
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(pos),
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }
}

fn drift(
    time: Res<Time>,
    settings: Res<Settings>,
    camera: Option<Single<&Transform, (With<LocalPlayer>, Without<Mote>)>>,
    mut motes: Query<(&mut Mote, &mut Transform, &mut Visibility)>,
) {
    let Some(cam) = camera else { return };
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let size = Vec3::new(MOTE_BOX, MOTE_BOX * 0.4, MOTE_BOX);
    // The box is centred on the camera, a little lower so most specks are
    // around head height and below.
    let corner = cam.translation - Vec3::new(MOTE_BOX * 0.5, MOTE_BOX * 0.2 + 1.0, MOTE_BOX * 0.5);
    let show = if settings.particles {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (mut m, mut tf, mut vis) in &mut motes {
        if *vis != show {
            *vis = show;
        }
        let wobble = Vec3::new(
            (t * 0.7 + m.phase).sin(),
            (t * 0.9 + m.phase * 1.7).sin() * 0.6,
            (t * 0.6 + m.phase * 2.3).cos(),
        ) * 0.08;
        let step = (m.vel + wobble) * dt;
        m.pos += step;
        // Wrap round the box as it follows the camera.
        let rel = (m.pos - corner).rem_euclid(size);
        m.pos = corner + rel;
        tf.translation = m.pos;
        // Shrink ones right by the camera so they don't show as big blobs.
        tf.scale = Vec3::splat((m.pos.distance(cam.translation) / 2.5).clamp(0.2, 1.0));
    }
}

/// Counts down a few frames into the match before warming up, so the
/// camera is in place.
#[derive(Resource)]
struct WarmUp(u8);

fn start_warm_up(mut commands: Commands) {
    commands.insert_resource(WarmUp(3));
}

/// Sets off one of every effect far away below the horizon (out of sight and
/// earshot) so their shaders are ready before they're needed.
fn warm_up(
    mut commands: Commands,
    warm: Option<ResMut<WarmUp>>,
    mut queue: ResMut<FxQueue>,
    camera: Option<Single<&Transform, With<LocalPlayer>>>,
) {
    let (Some(mut warm), Some(cam)) = (warm, camera) else {
        return;
    };
    if warm.0 > 0 {
        warm.0 -= 1;
        return;
    }
    commands.remove_resource::<WarmUp>();
    let flat = cam.forward().with_y(0.0).normalize_or(Vec3::NEG_Z);
    let at = (cam.translation + flat * 400.0 - Vec3::Y * 70.0).to_array();
    let dir = flat.to_array();
    let fx = &mut queue.0;
    fx.extend([
        Fx::Explosion {
            pos: at,
            radius: 4.0,
            color: [1.0, 0.5, 0.2],
        },
        Fx::Ring {
            pos: at,
            radius: 4.0,
            color: [0.5, 0.8, 1.0],
        },
        Fx::Slam {
            pos: at,
            radius: 4.0,
        },
        Fx::Heal {
            pos: at,
            radius: 4.0,
        },
        Fx::Nova {
            pos: at,
            radius: 4.0,
        },
        Fx::Slash {
            pos: at,
            dir,
            radius: 3.0,
        },
        Fx::Cone {
            pos: at,
            dir,
            range: 6.0,
        },
    ]);
    for ability in crate::data::Ability::ALL {
        fx.push(Fx::Spell {
            ability,
            pos: at,
            dir,
            size: 4.0,
        });
    }
}
