//! The painted look: every lit, solid material is swapped for a copy that
//! keeps Bevy's lighting and adds a painted layer on top (brush grain in
//! world space, cool shadow sides, a soft rim of light and metal picked out
//! per part). Code keeps making and changing `StandardMaterial`s as before;
//! the copies follow every change. Glass, glows and other unlit or
//! see-through materials are left alone. See docs/art-direction.md.

// The shader-type derive generates layout checks that are never called.
#![allow(dead_code)]

use bevy::asset::{weak_handle, RenderAssetUsages};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderRef, ShaderType, TextureDimension, TextureFormat,
};
use std::collections::HashMap;

pub type PaintedMaterial = ExtendedMaterial<StandardMaterial, Painted>;

const SHADER: Handle<Shader> = weak_handle!("6c1f0a52-8d3e-4f7a-9b2c-3e5d7a9c1b40");

/// Settings for the painted layer (the numbers are in art-direction.md).
#[derive(Clone, Copy, ShaderType, Debug, Reflect)]
pub struct PaintParams {
    /// Brush grain: how much it changes the colour up close (0.15 = ±15%).
    pub grain: f32,
    /// Grain fades out between these distances.
    pub grain_near: f32,
    pub grain_far: f32,
    /// How far the shadow side wraps round (soft terminator).
    pub wrap: f32,
    /// Colour the side away from the sun is tinted towards.
    pub shadow_tint: Vec3,
    /// Rim light strength on the lit side of silhouettes.
    pub rim: f32,
}

impl Default for PaintParams {
    fn default() -> Self {
        Self {
            grain: 0.15,
            grain_near: 25.0,
            grain_far: 85.0,
            wrap: 0.3,
            shadow_tint: Vec3::new(0.80, 0.82, 0.95),
            rim: 0.12,
        }
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct Painted {
    #[uniform(100)]
    pub params: PaintParams,
    #[texture(101)]
    #[sampler(102)]
    pub grain: Handle<Image>,
}

impl MaterialExtension for Painted {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}

/// The shared brush-grain texture.
#[derive(Resource)]
struct Grain(Handle<Image>);

/// Which painted copy goes with each plain material.
#[derive(Resource, Default)]
struct Copies(HashMap<AssetId<StandardMaterial>, Handle<PaintedMaterial>>);

/// Put on anything that should keep its plain material.
#[derive(Component)]
pub struct Unpainted;

pub struct PaintedPlugin;

impl Plugin for PaintedPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut()
            .resource_mut::<Assets<Shader>>()
            .insert(SHADER.id(), Shader::from_wgsl(WGSL, "painted.wgsl"));
        app.add_plugins(MaterialPlugin::<PaintedMaterial>::default())
            .init_resource::<Copies>()
            .add_systems(PreStartup, make_grain)
            .add_systems(PostUpdate, (swap_materials, follow_changes).chain());
    }
}

fn make_grain(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(Grain(images.add(grain_texture())));
}

/// Plain materials that get the painted look: lit and solid.
fn paintable(m: &StandardMaterial) -> bool {
    !m.unlit && matches!(m.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_))
}

fn painted_copy(m: &StandardMaterial, grain: &Handle<Image>) -> PaintedMaterial {
    ExtendedMaterial {
        base: m.clone(),
        extension: Painted {
            params: PaintParams::default(),
            grain: grain.clone(),
        },
    }
}

#[allow(clippy::type_complexity)]
fn swap_materials(
    mut commands: Commands,
    grain: Res<Grain>,
    mut copies: ResMut<Copies>,
    plain: Res<Assets<StandardMaterial>>,
    mut painted: ResMut<Assets<PaintedMaterial>>,
    new: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>),
        (Changed<MeshMaterial3d<StandardMaterial>>, Without<Unpainted>),
    >,
) {
    for (e, mat) in &new {
        let id = mat.0.id();
        let handle = match copies.0.get(&id) {
            Some(h) => h.clone(),
            None => {
                let Some(m) = plain.get(id) else { continue };
                if !paintable(m) {
                    continue;
                }
                let h = painted.add(painted_copy(m, &grain.0));
                copies.0.insert(id, h.clone());
                h
            }
        };
        commands
            .entity(e)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle));
    }
}

/// Copies changes to plain materials (hit flashes, tints, glows) onto their
/// painted copies, and drops copies whose material is gone.
fn follow_changes(
    mut events: EventReader<AssetEvent<StandardMaterial>>,
    mut copies: ResMut<Copies>,
    plain: Res<Assets<StandardMaterial>>,
    mut painted: ResMut<Assets<PaintedMaterial>>,
) {
    for ev in events.read() {
        match ev {
            AssetEvent::Modified { id } => {
                if let (Some(h), Some(m)) = (copies.0.get(id), plain.get(*id)) {
                    if let Some(p) = painted.get_mut(h) {
                        p.base = m.clone();
                    }
                }
            }
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                copies.0.remove(id);
            }
            _ => {}
        }
    }
}

/// Tileable value noise in a few octaves, as a greyscale image. Sampled in
/// world space by the shader at two sizes for the brush grain.
fn grain_texture() -> Image {
    const N: usize = 256;
    let hash = |x: i32, y: i32, o: u32| -> f32 {
        let mut h = (x as u32).wrapping_mul(374761393)
            ^ (y as u32).wrapping_mul(668265263)
            ^ o.wrapping_mul(2246822519);
        h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
    };
    let mut data = vec![0u8; N * N];
    for y in 0..N {
        for x in 0..N {
            let mut v = 0.0;
            let mut amp = 0.5;
            let mut total = 0.0;
            for o in 0..4u32 {
                let cells = 8 << o;
                let fx = x as f32 * cells as f32 / N as f32;
                let fy = y as f32 * cells as f32 / N as f32;
                let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
                let (tx, ty) = (fx - ix as f32, fy - iy as f32);
                let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
                let w = |i: i32| i.rem_euclid(cells);
                let a = hash(w(ix), w(iy), o);
                let b = hash(w(ix + 1), w(iy), o);
                let c = hash(w(ix), w(iy + 1), o);
                let d = hash(w(ix + 1), w(iy + 1), o);
                let n = a + (b - a) * sx + (c - a) * sy + (a - b - c + d) * sx * sy;
                v += n * amp;
                total += amp;
                amp *= 0.55;
            }
            data[y * N + x] = ((v / total).clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
    let mut img = Image::new(
        Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    img
}

const WGSL: &str = r#"
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::{view, lights},
}

struct PaintParams {
    grain: f32,
    grain_near: f32,
    grain_far: f32,
    wrap: f32,
    shadow_tint: vec3<f32>,
    rim: f32,
}

@group(2) @binding(100) var<uniform> paint: PaintParams;
@group(2) @binding(101) var grain_tex: texture_2d<f32>;
@group(2) @binding(102) var grain_samp: sampler;

// World-space grain seen from three sides, blended by the surface normal.
fn triplanar(p: vec3<f32>, w: vec3<f32>) -> f32 {
    let x = textureSample(grain_tex, grain_samp, p.zy).r;
    let y = textureSample(grain_tex, grain_samp, p.xz).r;
    let z = textureSample(grain_tex, grain_samp, p.xy).r;
    return x * w.x + y * w.y + z * w.z;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

#ifdef VERTEX_COLORS
    // Kit marks metal parts in the vertex alpha (1 = paint, 0.75 = metal).
    let metal = clamp((1.0 - in.color.a) * 4.0, 0.0, 1.0);
    pbr_input.material.metallic = mix(pbr_input.material.metallic, 1.0, metal);
    pbr_input.material.perceptual_roughness = mix(pbr_input.material.perceptual_roughness, 0.32, metal);
    pbr_input.material.base_color.a = 1.0;
#endif

    let pos = in.world_position.xyz;
    let n = normalize(pbr_input.N);
    let dist = distance(pos, view.world_position);

    // Brush grain, strongest up close and gone by the far distance.
    var w = pow(abs(n), vec3<f32>(4.0));
    w = w / (w.x + w.y + w.z);
    let g = triplanar(pos * 0.31, w) * 0.6 + triplanar(pos * 0.07, w) * 0.4;
    let fade = 1.0 - smoothstep(paint.grain_near, paint.grain_far, dist);
    let grain = 1.0 + (g - 0.5) * 2.0 * paint.grain * fade;
    pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * grain, pbr_input.material.base_color.a);

    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    // Light without the glow; the glow goes on last so it stays clean.
    let emissive = pbr_input.material.emissive;
    pbr_input.material.emissive = vec4<f32>(0.0, 0.0, 0.0, emissive.a);
    var color = apply_pbr_lighting(pbr_input);

    // The sun is the brightest directional light.
    var sun = vec3<f32>(0.0, 1.0, 0.0);
    var best = 0.0;
    for (var i = 0u; i < lights.n_directional_lights; i = i + 1u) {
        let l = lights.directional_lights[i];
        let b = dot(l.color.rgb, vec3<f32>(0.3, 0.5, 0.2));
        if b > best {
            best = b;
            sun = l.direction_to_light;
        }
    }
    let lit = clamp((dot(n, sun) + paint.wrap) / (1.0 + paint.wrap), 0.0, 1.0);
    // Cool shadow side instead of plain darkening.
    let tint = mix(paint.shadow_tint, vec3<f32>(1.0), lit);
    // A soft rim of light round the lit side of silhouettes.
    let v = normalize(view.world_position - pos);
    let rim = pow(1.0 - clamp(dot(n, v), 0.0, 1.0), 3.0) * lit * paint.rim;
    color = vec4<f32>(color.rgb * tint * (1.0 + rim), color.a);

    color = vec4<f32>(color.rgb + emissive.rgb * mix(1.0, view.exposure, emissive.a), color.a);

    var out: FragmentOutput;
    out.color = main_pass_post_lighting_processing(pbr_input, color);
    return out;
}
"#;
