//! Drawn outlines: a pencil line round characters, zombies, other players'
//! guns and nearby props. Each outlined mesh is drawn a second time, pushed
//! out along its smoothed normals with the front faces culled (an inverted
//! hull), in the surface's own colour darkened, half see-through. The width
//! follows the camera distance so the line stays the same on screen.

// The shader-type derive generates layout checks that are never called.
#![allow(dead_code)]

use bevy::pbr::{MaterialPipeline, MaterialPipelineKey, NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef, VertexAttributeValues};
use bevy::render::render_resource::{
    AsBindGroup, Face, RenderPipelineDescriptor, ShaderRef, ShaderType,
    SpecializedMeshPipelineError, VertexFormat,
};
use bevy::asset::weak_handle;
use std::collections::{HashMap, HashSet};

use crate::config::Settings;

const SHADER: Handle<Shader> = weak_handle!("2a7e9f14-5b3c-4d61-8e0f-91c4a6b7d203");

/// The normal averaged over every vertex at the same spot, so the pushed-out
/// hull doesn't split open at the hard edges of boxes and cylinders.
pub const ATTRIBUTE_SMOOTH_NORMAL: MeshVertexAttribute =
    MeshVertexAttribute::new("SmoothNormal", 988_540_917, VertexFormat::Float32x3);

/// What gets an outline, and so how it's drawn.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Outline {
    /// Characters, zombies and their guns: always.
    Figure,
    /// Props: fades out by about 30 m.
    Prop,
    /// The first-person gun and hands: thinner, and very close.
    FirstPerson,
}

/// The hull drawn for an outlined mesh.
#[derive(Component)]
struct Hull;

#[derive(Clone, Copy, ShaderType, Debug, Reflect)]
pub struct OutlineParams {
    /// Line width per metre of distance.
    width: f32,
    /// Distance is clamped to this range before working out the width.
    near: f32,
    far: f32,
    /// See-through: 0.5 = half.
    alpha: f32,
    /// The line is the surface colour times this.
    darken: f32,
    /// Fades out between these distances.
    fade_start: f32,
    fade_end: f32,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct OutlineMaterial {
    #[uniform(0)]
    params: OutlineParams,
}

impl Material for OutlineMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(5),
            ATTRIBUTE_SMOOTH_NORMAL.at_shader_location(8),
        ])?;
        descriptor.vertex.buffers = vec![vertex];
        descriptor.primitive.cull_mode = Some(Face::Front);
        Ok(())
    }
}

#[derive(Resource)]
struct OutlineMats(HashMap<Outline, Handle<OutlineMaterial>>);

/// Meshes already given smoothed normals.
#[derive(Resource, Default)]
struct Smoothed(HashSet<AssetId<Mesh>>);

pub struct OutlinePlugin;

impl Plugin for OutlinePlugin {
    fn build(&self, app: &mut App) {
        app.world_mut()
            .resource_mut::<Assets<Shader>>()
            .insert(SHADER.id(), Shader::from_wgsl(WGSL, "outline.wgsl"));
        app.add_plugins(MaterialPlugin::<OutlineMaterial>::default())
            .init_resource::<Smoothed>()
            .add_systems(Startup, make_materials)
            .add_systems(PostUpdate, (add_hulls, show_hulls).chain());
    }
}

fn make_materials(mut commands: Commands, mut mats: ResMut<Assets<OutlineMaterial>>) {
    let figure = OutlineParams {
        // 0.0012 per metre (as planned) is under a pixel at 1080p, so it's
        // a little wider.
        width: 0.0025,
        near: 2.0,
        far: 60.0,
        alpha: 0.5,
        // 0.35 of the colour as you see it (the shader works in linear
        // light, where that's about 0.12).
        darken: 0.12,
        fade_start: 1.0e6,
        fade_end: 2.0e6,
    };
    let all = [
        (Outline::Figure, figure),
        (
            Outline::Prop,
            OutlineParams {
                fade_start: 24.0,
                fade_end: 30.0,
                ..figure
            },
        ),
        // Held 0.3-0.7 m away: no clamp, and half the width.
        (
            Outline::FirstPerson,
            OutlineParams {
                width: 0.0006,
                near: 0.0,
                alpha: 0.4,
                ..figure
            },
        ),
    ];
    commands.insert_resource(OutlineMats(
        all.into_iter()
            .map(|(k, p)| (k, mats.add(OutlineMaterial { params: p })))
            .collect(),
    ));
}

/// Works out the smoothed normals for a mesh (once).
fn smooth_normals(mesh: &mut Mesh) {
    let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };
    let Some(VertexAttributeValues::Float32x3(nor)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    else {
        return;
    };
    let key = |p: &[f32; 3]| {
        let q = |x: f32| (x * 10_000.0).round() as i64;
        (q(p[0]), q(p[1]), q(p[2]))
    };
    let mut sum: HashMap<(i64, i64, i64), Vec3> = HashMap::new();
    for (p, n) in pos.iter().zip(nor) {
        *sum.entry(key(p)).or_default() += Vec3::from_array(*n);
    }
    let out: Vec<[f32; 3]> = pos
        .iter()
        .zip(nor)
        .map(|(p, n)| {
            sum[&key(p)]
                .try_normalize()
                .unwrap_or(Vec3::from_array(*n))
                .to_array()
        })
        .collect();
    mesh.insert_attribute(ATTRIBUTE_SMOOTH_NORMAL, out);
}

fn add_hulls(
    mut commands: Commands,
    mats: Res<OutlineMats>,
    mut smoothed: ResMut<Smoothed>,
    mut meshes: ResMut<Assets<Mesh>>,
    new: Query<(Entity, &Outline, &Mesh3d), Added<Outline>>,
) {
    for (e, kind, mesh) in &new {
        let id = mesh.0.id();
        if !smoothed.0.contains(&id) {
            let Some(m) = meshes.get_mut(id) else { continue };
            if m.attribute(Mesh::ATTRIBUTE_COLOR).is_none() {
                continue;
            }
            smooth_normals(m);
            smoothed.0.insert(id);
        }
        commands.entity(e).with_child((
            Hull,
            Mesh3d(mesh.0.clone()),
            MeshMaterial3d(mats.0[kind].clone()),
            NotShadowCaster,
            NotShadowReceiver,
            Transform::default(),
            Visibility::Inherited,
        ));
    }
}

/// The Settings switch.
fn show_hulls(
    settings: Res<Settings>,
    mut hulls: Query<(&mut Visibility, Ref<Hull>)>,
) {
    let want = if settings.outlines {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (mut vis, hull) in &mut hulls {
        if (settings.is_changed() || hull.is_added()) && *vis != want {
            *vis = want;
        }
    }
}

const WGSL: &str = r#"
#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::view,
}

struct OutlineParams {
    width: f32,
    near: f32,
    far: f32,
    alpha: f32,
    darken: f32,
    fade_start: f32,
    fade_end: f32,
}

@group(2) @binding(0) var<uniform> outline: OutlineParams;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(5) color: vec4<f32>,
    @location(8) smooth_normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(v.position, 1.0));
    let n = normalize(mesh_functions::mesh_normal_local_to_world(v.smooth_normal, v.instance_index));
    let d = distance(world.xyz, view.world_position);
    let push = outline.width * clamp(d, outline.near, outline.far);
    var out: VertexOutput;
    out.clip_position = position_world_to_clip(world.xyz + n * push);
    let fade = 1.0 - smoothstep(outline.fade_start, outline.fade_end, d);
    out.color = vec4<f32>(v.color.rgb * outline.darken, outline.alpha * fade);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;
