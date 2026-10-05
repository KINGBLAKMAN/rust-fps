//! Ground markers: soft painted shapes on the floor for ability previews
//! and enemy warnings. Each one is a flat quad drawn with a distance-field
//! shader (circle, donut, cone or line): a faint body that gets stronger
//! towards a crisp edge, an optional fill that grows as a countdown runs out
//! (with a bright front, and a pulse near the end), and a short flash when
//! it goes off. Anything that wants a marker pushes one into `Markers` each
//! frame; they're drawn once and cleared.

// The shader-type derive generates layout checks that are never called.
#![allow(dead_code)]

use bevy::asset::{weak_handle, RenderAssetUsages};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey, NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderRef, ShaderType, SpecializedMeshPipelineError,
};
use std::collections::HashMap;

use crate::physics::{collect_boxes, ray_world};
use crate::player::LocalPlayer;
use crate::rig::Rig;
use crate::sim::{BRUTE_SLAM_AHEAD, BRUTE_SLAM_RADIUS, BRUTE_WINDUP, FIREBALL_SPEED};
use crate::{AppState, Collider, EnemyStatus, NetKind, Phase, Replicated};

const SHADER: Handle<Shader> = weak_handle!("6c1d8e52-3f7a-4b90-a2e4-5d9c0b7f1e38");

/// Ability previews: a muted blue-grey, so they don't shout.
pub const PREVIEW: Color = Color::srgb(0.55, 0.75, 0.85);
/// Enemy attacks about to land: ember red.
pub const DANGER: Color = Color::srgb(0.85, 0.30, 0.15);

/// How long the flash lasts when a countdown goes off.
pub const FLASH: f32 = 0.4;

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Circle { radius: f32 },
    Donut { inner: f32, outer: f32 },
    /// A wedge from the centre, `half_angle` either side of the direction.
    Cone { radius: f32, half_angle: f32 },
    /// A strip from the centre along the direction.
    Line { length: f32, half_width: f32 },
}

#[derive(Clone, Copy, Debug)]
pub struct Marker {
    pub at: Vec3,
    /// Which way cones and lines point (flat).
    pub dir: Vec3,
    pub shape: Shape,
    pub color: Color,
    /// Countdown fill, 0 to 1 (none for previews).
    pub fill: Option<f32>,
    /// Goes-off flash, 1 fading to 0.
    pub flash: f32,
}

impl Marker {
    pub fn new(at: Vec3, shape: Shape, color: Color) -> Marker {
        Marker {
            at,
            dir: Vec3::NEG_Z,
            shape,
            color,
            fill: None,
            flash: 0.0,
        }
    }

    pub fn facing(mut self, dir: Vec3) -> Marker {
        self.dir = dir.with_y(0.0).normalize_or(Vec3::NEG_Z);
        self
    }

    pub fn fill(mut self, fill: f32) -> Marker {
        self.fill = Some(fill.clamp(0.0, 1.0));
        self
    }

    pub fn flash(mut self, flash: f32) -> Marker {
        self.flash = flash.clamp(0.0, 1.0);
        self
    }

    /// How far the quad has to reach from the centre.
    fn reach(&self) -> f32 {
        let r = match self.shape {
            Shape::Circle { radius } | Shape::Cone { radius, .. } => radius,
            Shape::Donut { outer, .. } => outer,
            Shape::Line { length, half_width } => length.max(half_width),
        };
        r + 0.3
    }
}

/// This frame's markers.
#[derive(Resource, Default)]
pub struct Markers(pub Vec<Marker>);

impl Markers {
    pub fn push(&mut self, m: Marker) {
        self.0.push(m);
    }
}

#[derive(Clone, Copy, ShaderType, Debug, Reflect, Default)]
struct MarkerParams {
    color: Vec4,
    /// 0 circle, 1 donut, 2 cone, 3 line.
    kind: f32,
    /// Shape sizes (see `Shape`).
    a: f32,
    b: f32,
    /// Half the quad's size in metres.
    half: f32,
    /// Countdown fill (below 0 = none).
    fill: f32,
    flash: f32,
    time: f32,
    /// Overall strength (lower at night, where they'd glare).
    level: f32,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct MarkerMaterial {
    #[uniform(0)]
    params: MarkerParams,
}

impl Material for MarkerMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn depth_bias(&self) -> f32 {
        40.0
    }
    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// The quads, reused frame to frame.
#[derive(Resource, Default)]
struct Pool {
    quad: Handle<Mesh>,
    slots: Vec<(Entity, Handle<MarkerMaterial>)>,
}

pub struct MarkersPlugin;

impl Plugin for MarkersPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut()
            .resource_mut::<Assets<Shader>>()
            .insert(SHADER.id(), Shader::from_wgsl(WGSL, "markers.wgsl"));
        app.add_plugins(MaterialPlugin::<MarkerMaterial>::default())
            .init_resource::<Markers>()
            .init_resource::<Pool>()
            .add_systems(Startup, make_quad)
            .add_systems(
                Update,
                enemy_warnings
                    .in_set(Phase::Present)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(PostUpdate, draw);
    }
}

/// A flat square from -1 to 1, with the UVs the same as x and z.
fn make_quad(mut pool: ResMut<Pool>, mut meshes: ResMut<Assets<Mesh>>) {
    let pos = vec![[-1.0, 0.0, -1.0], [1.0, 0.0, -1.0], [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0]];
    let uv: Vec<[f32; 2]> = pos.iter().map(|p| [p[0], p[2]]).collect();
    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_indices(Indices::U32(vec![0, 2, 1, 0, 3, 2]));
    pool.quad = meshes.add(mesh);
}

fn draw(
    mut commands: Commands,
    time: Res<Time>,
    mut markers: ResMut<Markers>,
    mut pool: ResMut<Pool>,
    mut mats: ResMut<Assets<MarkerMaterial>>,
    mut quads: Query<(&mut Transform, &mut Visibility)>,
    state: Option<Res<crate::MatchState>>,
) {
    let level = if state.is_some_and(|s| s.night) { 0.4 } else { 1.0 };
    let list = std::mem::take(&mut markers.0);
    while pool.slots.len() < list.len() {
        let mat = mats.add(MarkerMaterial {
            params: MarkerParams::default(),
        });
        let e = commands
            .spawn((
                Mesh3d(pool.quad.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::default(),
                Visibility::Hidden,
                NotShadowCaster,
                NotShadowReceiver,
            ))
            .id();
        pool.slots.push((e, mat));
    }
    let t = time.elapsed_secs();
    for (i, (e, mat)) in pool.slots.iter().enumerate() {
        let Some(m) = list.get(i) else {
            if let Ok((_, mut vis)) = quads.get_mut(*e) {
                if *vis != Visibility::Hidden {
                    *vis = Visibility::Hidden;
                }
            }
            continue;
        };
        // Spawned this frame: picked up next frame.
        let Ok((mut tf, mut vis)) = quads.get_mut(*e) else {
            continue;
        };
        let half = m.reach();
        *tf = Transform::from_translation(m.at + Vec3::Y * 0.04)
            .looking_to(m.dir, Vec3::Y)
            .with_scale(Vec3::new(half, 1.0, half));
        *vis = Visibility::Visible;
        let (kind, a, b) = match m.shape {
            Shape::Circle { radius } => (0.0, radius, 0.0),
            Shape::Donut { inner, outer } => (1.0, inner, outer),
            Shape::Cone { radius, half_angle } => (2.0, radius, half_angle),
            Shape::Line { length, half_width } => (3.0, length, half_width),
        };
        if let Some(mat) = mats.get_mut(mat) {
            mat.params = MarkerParams {
                color: m.color.to_linear().to_vec4(),
                kind,
                a,
                b,
                half,
                fill: m.fill.unwrap_or(-1.0),
                flash: m.flash,
                time: t,
                level,
            };
        }
    }
}

/// A fireball being tracked: where it was first seen.
struct Shot {
    first: Vec3,
    /// Seconds to impact when the direction was first known.
    total: Option<f32>,
}

/// Red warnings where enemy attacks are about to land: a Brute's slam, and
/// where a spat fireball is heading.
#[allow(clippy::type_complexity)]
fn enemy_warnings(
    time: Res<Time>,
    mut markers: ResMut<Markers>,
    mut shots: Local<HashMap<Entity, Shot>>,
    player: Option<Single<&LocalPlayer>>,
    enemies: Query<(&Replicated, &Transform, &Rig, &EnemyStatus)>,
    projectiles: Query<(Entity, &Replicated, &Transform), Without<Rig>>,
    colliders: Query<(&Transform, &Collider)>,
) {
    for (r, tf, rig, status) in &enemies {
        if r.kind != NetKind::Brute || status.stunned || rig.dying.is_some() {
            continue;
        }
        let s = rig.swing;
        if s > BRUTE_WINDUP + FLASH {
            continue;
        }
        let ahead = (tf.rotation * Vec3::NEG_Z).with_y(0.0).normalize_or(Vec3::NEG_Z);
        let m = Marker::new(
            tf.translation + ahead * BRUTE_SLAM_AHEAD,
            Shape::Circle {
                radius: BRUTE_SLAM_RADIUS,
            },
            DANGER,
        );
        markers.push(if s < BRUTE_WINDUP {
            m.fill(s / BRUTE_WINDUP)
        } else {
            m.fill(1.0).flash(1.0 - (s - BRUTE_WINDUP) / FLASH)
        });
    }

    // Fireballs fly straight, so the line from where one was first seen
    // gives where it's going.
    let me = player.map(|p| p.feet);
    let mut boxes = None;
    let dt = time.delta_secs();
    shots.retain(|e, _| projectiles.contains(*e));
    for (e, r, tf) in &projectiles {
        if r.kind != NetKind::Fireball {
            continue;
        }
        let pos = tf.translation;
        let shot = shots.entry(e).or_insert(Shot {
            first: pos,
            total: None,
        });
        let travelled = pos - shot.first;
        if travelled.length() < 0.4 {
            continue;
        }
        let dir = travelled.normalize();
        let boxes = boxes.get_or_insert_with(|| collect_boxes(colliders.iter()));
        let mut dist = ray_world(pos, dir, FIREBALL_SPEED * 4.0, boxes);
        if dir.y < -1e-3 {
            dist = dist.min(pos.y / -dir.y);
        }
        // Coming at you: it bursts where it reaches you.
        if let Some(feet) = me {
            let chest = feet + Vec3::Y;
            let along = (chest - pos).dot(dir);
            if along > 0.0 && along < dist && (pos + dir * along).distance(chest) < 1.2 {
                dist = along;
            }
        }
        let left = dist / FIREBALL_SPEED;
        let total = *shot.total.get_or_insert(left + dt);
        let hit = pos + dir * dist;
        let ground = if let Some(feet) = me.filter(|f| f.distance(hit) < 2.5) {
            feet.y
        } else {
            0.0
        };
        markers.push(
            Marker::new(hit.with_y(ground), Shape::Circle { radius: 1.0 }, DANGER)
                .fill(1.0 - left / total.max(1e-3)),
        );
    }
}

const WGSL: &str = r#"
#import bevy_pbr::forward_io::VertexOutput

struct MarkerParams {
    color: vec4<f32>,
    kind: f32,
    a: f32,
    b: f32,
    half: f32,
    fill: f32,
    flash: f32,
    time: f32,
    level: f32,
}

@group(2) @binding(0) var<uniform> marker: MarkerParams;

// Distance to a wedge of radius r opening `ang` either side of +y.
fn sd_cone(p_in: vec2<f32>, r: f32, ang: f32) -> f32 {
    let c = vec2<f32>(sin(ang), cos(ang));
    let p = vec2<f32>(abs(p_in.x), p_in.y);
    let l = length(p) - r;
    let m = length(p - c * clamp(dot(p, c), 0.0, r));
    return max(l, m * sign(c.y * p.x - c.x * p.y));
}

fn sd_box(p: vec2<f32>, b: vec2<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Metres from the centre, +y the way the marker faces.
    let q = vec2<f32>(in.uv.x, -in.uv.y) * marker.half;
    let r = length(q);
    var d: f32;
    // How far the fill has to travel, and where this point is along it.
    var t: f32;
    var len: f32;
    let kind = i32(marker.kind + 0.5);
    if kind == 0 {
        d = r - marker.a;
        t = r;
        len = marker.a;
    } else if kind == 1 {
        let mid = 0.5 * (marker.a + marker.b);
        d = abs(r - mid) - 0.5 * (marker.b - marker.a);
        t = r - marker.a;
        len = marker.b - marker.a;
    } else if kind == 2 {
        d = sd_cone(q, marker.a, marker.b);
        t = r;
        len = marker.a;
    } else {
        d = sd_box(q - vec2<f32>(0.0, 0.5 * marker.a), vec2<f32>(marker.b, 0.5 * marker.a));
        t = q.y;
        len = marker.a;
    }

    let fw = max(fwidth(d), 1e-4);
    let edge_w = 0.035;
    // Inside, out to the outer side of the edge line (antialiased).
    let cover = 1.0 - smoothstep(edge_w - fw, edge_w + fw, d);
    if cover <= 0.0 {
        discard;
    }
    let inside = 1.0 - smoothstep(-fw, fw, d);
    // Faint body, stronger towards the edge.
    var alpha = (0.10 + 0.20 * exp(min(d, 0.0) / 1.1)) * inside;
    // The crisp edge line.
    let line = 1.0 - smoothstep(edge_w - fw, edge_w + fw, abs(d));
    alpha = max(alpha, 0.55 * line);
    var bright = 1.0;

    if marker.fill >= 0.0 {
        let front = marker.fill * len;
        let filled = (1.0 - smoothstep(front - fw, front + fw, t)) * inside;
        // A bright band just behind the front.
        let band = filled * (1.0 - smoothstep(0.0, 0.12, front - t));
        alpha += 0.16 * filled + 0.3 * band;
        bright += 0.5 * band;
        // Pulse in the last fifth.
        if marker.fill > 0.8 {
            let wave = 0.5 + 0.5 * sin(marker.time * 20.0);
            let k = 1.0 + 0.45 * wave;
            alpha *= k;
            bright *= k;
        }
    }
    alpha = mix(alpha, 0.65 * inside + 0.35 * line, marker.flash);
    bright += 0.8 * marker.flash;

    return vec4<f32>(
        marker.color.rgb * bright * marker.level,
        min(alpha * cover, 0.9) * marker.color.a * mix(0.75, 1.0, marker.level),
    );
}
"#;
