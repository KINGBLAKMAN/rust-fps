//! Visual effects: bullet tracers, explosions, healing and frost rings,
//! lightning arcs and the orbital strike beam. The host broadcasts effects so
//! every player sees them.

use bevy::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{AppState, InGameEntity, Phase};

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FxQueue>()
            .init_resource::<FxOutbox>()
            .init_resource::<Lines>()
            .add_systems(PreStartup, setup)
            .add_systems(
                Update,
                (play, animate, draw_lines)
                    .chain()
                    .in_set(Phase::Present)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnExit(AppState::InGame), clear);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Fx {
    /// Bullet tracer from a player (`shooter`) so they can skip their own.
    Tracer { shooter: u8, a: [f32; 3], b: [f32; 3] },
    Explosion { pos: [f32; 3], radius: f32, color: [f32; 3] },
    Ring { pos: [f32; 3], radius: f32, color: [f32; 3] },
    Lightning { a: [f32; 3], b: [f32; 3] },
    Beam { pos: [f32; 3], radius: f32, delay: f32 },
}

/// Effects to show on this machine this frame.
#[derive(Resource, Default)]
pub struct FxQueue(pub Vec<Fx>);

/// Host only: effects to send to clients with the next snapshot.
#[derive(Resource, Default)]
pub struct FxOutbox(pub Vec<Fx>);

/// Short-lived lines: (start, end, color, time left).
#[derive(Resource, Default)]
struct Lines(Vec<(Vec3, Vec3, Color, f32)>);

#[derive(Resource)]
struct FxAssets {
    ball: Handle<Mesh>,
    disc: Handle<Mesh>,
    beam: Handle<Mesh>,
}

#[derive(Component)]
struct Burst {
    life: f32,
    max: f32,
    grow_to: f32,
    flat: bool,
}

#[derive(Component)]
struct Pending {
    delay: f32,
    pos: Vec3,
    radius: f32,
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(FxAssets {
        ball: meshes.add(Sphere::new(1.0).mesh().ico(3).unwrap()),
        disc: meshes.add(Cylinder::new(1.0, 0.08)),
        beam: meshes.add(Cylinder::new(1.0, 60.0)),
    });
}

pub fn rgb(c: Color) -> [f32; 3] {
    let s = c.to_srgba();
    [s.red, s.green, s.blue]
}

fn glow_material(materials: &mut Assets<StandardMaterial>, color: [f32; 3], alpha: f32) -> Handle<StandardMaterial> {
    let c = Color::srgba(color[0], color[1], color[2], alpha);
    materials.add(StandardMaterial {
        base_color: c,
        emissive: LinearRgba::rgb(color[0], color[1], color[2]) * 6.0,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    })
}

fn play(
    mut commands: Commands,
    mut queue: ResMut<FxQueue>,
    mut lines: ResMut<Lines>,
    assets: Res<FxAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();
    for fx in queue.0.drain(..) {
        match fx {
            Fx::Tracer { a, b, .. } => {
                lines.0.push((
                    Vec3::from_array(a),
                    Vec3::from_array(b),
                    Color::srgb(1.0, 0.9, 0.5),
                    0.06,
                ));
            }
            Fx::Explosion { pos, radius, color } => {
                commands.spawn((
                    InGameEntity,
                    Burst {
                        life: 0.45,
                        max: 0.45,
                        grow_to: radius,
                        flat: false,
                    },
                    Mesh3d(assets.ball.clone()),
                    MeshMaterial3d(glow_material(&mut materials, color, 0.7)),
                    Transform::from_translation(Vec3::from_array(pos)).with_scale(Vec3::splat(0.2)),
                ));
                commands.spawn((
                    InGameEntity,
                    Burst {
                        life: 0.25,
                        max: 0.25,
                        grow_to: 0.0,
                        flat: false,
                    },
                    PointLight {
                        intensity: 400_000.0,
                        color: Color::srgb(color[0], color[1], color[2]),
                        range: radius * 4.0,
                        ..default()
                    },
                    Transform::from_translation(Vec3::from_array(pos) + Vec3::Y),
                ));
            }
            Fx::Ring { pos, radius, color } => {
                commands.spawn((
                    InGameEntity,
                    Burst {
                        life: 0.6,
                        max: 0.6,
                        grow_to: radius,
                        flat: true,
                    },
                    Mesh3d(assets.disc.clone()),
                    MeshMaterial3d(glow_material(&mut materials, color, 0.45)),
                    Transform::from_translation(Vec3::from_array(pos) + Vec3::Y * 0.1)
                        .with_scale(Vec3::new(0.3, 1.0, 0.3)),
                ));
            }
            Fx::Lightning { a, b } => {
                let (a, b) = (Vec3::from_array(a), Vec3::from_array(b));
                let mut prev = a;
                for i in 1..=6 {
                    let t = i as f32 / 6.0;
                    let mut p = a.lerp(b, t);
                    if i < 6 {
                        p += Vec3::new(
                            rng.gen_range(-0.4..0.4),
                            rng.gen_range(-0.4..0.4),
                            rng.gen_range(-0.4..0.4),
                        );
                    }
                    lines.0.push((prev, p, Color::srgb(0.8, 0.85, 1.0), 0.15));
                    prev = p;
                }
            }
            Fx::Beam { pos, radius, delay } => {
                let p = Vec3::from_array(pos);
                // Target marker that turns into a blast when the delay ends.
                commands.spawn((
                    InGameEntity,
                    Pending {
                        delay,
                        pos: p,
                        radius,
                    },
                    Mesh3d(assets.disc.clone()),
                    MeshMaterial3d(glow_material(&mut materials, [1.0, 0.2, 0.2], 0.4)),
                    Transform::from_translation(p + Vec3::Y * 0.1)
                        .with_scale(Vec3::new(radius, 1.0, radius)),
                ));
            }
        }
    }
}

fn animate(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<FxAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut bursts: Query<(Entity, &mut Burst, &mut Transform, Option<&mut PointLight>), Without<Pending>>,
    mut pending: Query<(Entity, &mut Pending)>,
) {
    let dt = time.delta_secs();
    for (e, mut b, mut tf, light) in &mut bursts {
        b.life -= dt;
        if b.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let t = 1.0 - b.life / b.max;
        if let Some(mut light) = light {
            light.intensity *= 0.85;
        } else if b.flat {
            let r = b.grow_to * t.max(0.05);
            tf.scale = Vec3::new(r, 1.0, r);
        } else {
            tf.scale = Vec3::splat(b.grow_to * (0.3 + 0.7 * t.sqrt()));
        }
    }
    for (e, mut p) in &mut pending {
        p.delay -= dt;
        if p.delay > 0.0 {
            continue;
        }
        commands.entity(e).despawn();
        commands.spawn((
            InGameEntity,
            Burst {
                life: 0.6,
                max: 0.6,
                grow_to: 3.0,
                flat: false,
            },
            Mesh3d(assets.beam.clone()),
            MeshMaterial3d(glow_material(&mut materials, [1.0, 0.6, 0.3], 0.6)),
            Transform::from_translation(p.pos + Vec3::Y * 30.0),
        ));
        commands.spawn((
            InGameEntity,
            Burst {
                life: 0.7,
                max: 0.7,
                grow_to: p.radius,
                flat: false,
            },
            Mesh3d(assets.ball.clone()),
            MeshMaterial3d(glow_material(&mut materials, [1.0, 0.45, 0.1], 0.7)),
            Transform::from_translation(p.pos),
        ));
    }
}

fn draw_lines(time: Res<Time>, mut lines: ResMut<Lines>, mut gizmos: Gizmos) {
    let dt = time.delta_secs();
    for (a, b, color, life) in lines.0.iter_mut() {
        gizmos.line(*a, *b, *color);
        *life -= dt;
    }
    lines.0.retain(|l| l.3 > 0.0);
}

fn clear(mut lines: ResMut<Lines>, mut queue: ResMut<FxQueue>, mut out: ResMut<FxOutbox>) {
    lines.0.clear();
    queue.0.clear();
    out.0.clear();
}

/// Host helper: show an effect here and send it to everyone else.
pub fn emit(queue: &mut FxQueue, out: &mut FxOutbox, fx: Fx) {
    queue.0.push(fx.clone());
    out.0.push(fx);
}
