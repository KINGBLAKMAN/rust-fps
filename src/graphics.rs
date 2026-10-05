//! Applies the graphics settings (shadows, smoothed edges, glow and soft
//! corner shading) to the camera and sun, and the look of the picture:
//! film-like tone mapping, colour grading per map, the gradient sky and
//! distance haze. See docs/art-direction.md for the numbers.

use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::view::ColorGrading;
use bevy::pbr::{
    CascadeShadowConfig, CascadeShadowConfigBuilder, DirectionalLightShadowMap, DistanceFog,
    FogFalloff, ScreenSpaceAmbientOcclusion,
};
use bevy::prelude::*;
use bevy::render::view::Msaa;

use crate::config::Settings;
use crate::{AppState, MatchState};

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DirectionalLightShadowMap { size: 2048 })
            .add_systems(Update, (apply_camera, apply_sun))
            .add_systems(
                PostUpdate,
                follow_sky.before(bevy::transform::TransformSystem::TransformPropagate),
            );
    }
}

/// The weak, shadowless cool light from opposite the sun that keeps
/// shadows from going black.
#[derive(Component)]
pub struct FillLight;

/// The sky dome; it stays centred on the camera.
#[derive(Component)]
struct SkyDome;

/// Horizon and zenith colours for a map's sky colour. The horizon is lighter
/// and warmer, the zenith deeper and bluer. The fog uses the horizon.
pub fn sky_colors(sky: Color, night: bool) -> (Color, Color) {
    let s = sky.to_srgba();
    let mix = |a: Srgba, b: Srgba, t: f32| {
        Srgba::rgb(
            a.red + (b.red - a.red) * t,
            a.green + (b.green - a.green) * t,
            a.blue + (b.blue - a.blue) * t,
        )
    };
    if night {
        let h = mix(s, Srgba::rgb(0.09, 0.1, 0.14), 0.5);
        let z = mix(s, Srgba::rgb(0.0, 0.0, 0.02), 0.6);
        return (h.into(), z.into());
    }
    let h = mix(s, Srgba::rgb(1.0, 0.95, 0.86), 0.2);
    let z = mix(s, Srgba::rgb(0.2, 0.34, 0.66), 0.35) * 0.85;
    (h.into(), Srgba::rgb(z.red, z.green, z.blue).into())
}

/// Saturation after tone mapping. AgX already eases bright colours
/// towards white, so this is a little above the 0.85 first planned.
const SATURATION: f32 = 0.95;

/// How each map is graded: (white balance, saturation). The Neighborhood
/// leans warm, the Shipping Yard cool.
fn map_grade(map: u8, night: bool) -> (f32, f32) {
    // Bevy's white balance is strong: 0.1 is already a heavy tint.
    let warmth = match map {
        0 => -0.015,
        1 => 0.005,
        _ => 0.025,
    };
    if night {
        (warmth * 0.5 - 0.01, SATURATION)
    } else {
        (warmth, SATURATION)
    }
}

/// The sky: a big dome shaded from horizon to zenith, with a sun (or moon)
/// disc. Unlit and outside the fog.
pub fn spawn_sky(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sky: Color,
    night: bool,
    sun_dir: Vec3,
) {
    const R: f32 = 600.0;
    let (horizon, zenith) = sky_colors(sky, night);
    let (h, z) = (horizon.to_linear(), zenith.to_linear());
    let (rings, segs) = (24usize, 32usize);
    let mut pos = Vec::new();
    let mut col = Vec::new();
    for i in 0..=rings {
        // From straight down to straight up.
        let el = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / rings as f32;
        let up = el.sin();
        // Most of the change happens in the lower third of the sky.
        let t = (up.max(0.0) / 0.6).min(1.0).powf(0.8);
        let below = (-up).max(0.0).min(1.0);
        for j in 0..=segs {
            let a = std::f32::consts::TAU * j as f32 / segs as f32;
            pos.push([el.cos() * a.cos() * R, up * R, el.cos() * a.sin() * R]);
            let c = h.mix(&z, t) * (1.0 - 0.25 * below);
            col.push([c.red, c.green, c.blue, 1.0]);
        }
    }
    let mut idx = Vec::new();
    for i in 0..rings as u32 {
        for j in 0..segs as u32 {
            let a = i * (segs as u32 + 1) + j;
            let b = a + segs as u32 + 1;
            idx.extend([a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    let n = pos.len();
    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; n])
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32(idx));
    let dome = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        fog_enabled: false,
        cull_mode: None,
        ..default()
    });
    // The sun is bright enough for the bloom to give it a soft halo.
    let (disc, glow) = if night {
        (8.0, LinearRgba::rgb(1.4, 1.5, 1.7))
    } else {
        (14.0, LinearRgba::rgb(9.0, 8.2, 6.8))
    };
    let disc_mat = materials.add(StandardMaterial {
        base_color: Color::LinearRgba(glow),
        unlit: true,
        fog_enabled: false,
        ..default()
    });
    commands
        .spawn((
            crate::InGameEntity,
            SkyDome,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(dome),
            NotShadowCaster,
            NotShadowReceiver,
            Transform::default(),
        ))
        .with_child((
            Mesh3d(meshes.add(Sphere::new(disc).mesh().ico(3).unwrap())),
            MeshMaterial3d(disc_mat),
            NotShadowCaster,
            NotShadowReceiver,
            Transform::from_translation(sun_dir.normalize() * (R - 40.0)),
        ));
}

fn follow_sky(
    cam: Query<&GlobalTransform, With<Camera3d>>,
    mut sky: Query<&mut Transform, With<SkyDome>>,
) {
    let Some(cam) = cam.iter().next() else { return };
    for mut tf in &mut sky {
        tf.translation = cam.translation();
    }
}

fn apply_camera(
    mut commands: Commands,
    settings: Res<Settings>,
    app_state: Res<State<AppState>>,
    state: Res<MatchState>,
    clear: Res<ClearColor>,
    mut cams: Query<(Entity, &mut Camera, Ref<Camera3d>)>,
) {
    let changed = settings.is_changed() || app_state.is_changed() || clear.is_changed();
    for (e, mut camera, added) in &mut cams {
        if !changed && !added.is_added() {
            continue;
        }
        let mut ec = commands.entity(e);
        // Soft corner shading needs the edge smoothing off (the settings
        // screen keeps them exclusive).
        let ao = settings.ambient_occlusion;
        ec.insert(if settings.antialias && !ao { Msaa::Sample4 } else { Msaa::Off });
        if ao {
            ec.insert(ScreenSpaceAmbientOcclusion::default());
        } else {
            ec.remove::<ScreenSpaceAmbientOcclusion>();
        }
        // Always HDR, rolled off to the screen like film.
        camera.hdr = true;
        ec.insert(Tonemapping::AgX);
        let (temperature, saturation) = if *app_state.get() == AppState::InGame {
            map_grade(state.map, state.night)
        } else {
            (0.0, SATURATION)
        };
        let mut grade = ColorGrading::default();
        grade.global.temperature = temperature;
        grade.global.post_saturation = saturation;
        grade.midtones.contrast = 1.06;
        ec.insert(grade);
        if settings.bloom {
            ec.insert(Bloom {
                intensity: 0.1,
                ..Bloom::NATURAL
            });
        } else {
            ec.remove::<Bloom>();
        }
        // Haze in the distance, the colour of the sky, so far things fade
        // instead of ending at a hard edge.
        if *app_state.get() == AppState::InGame {
            let (start, end) = if state.night { (14.0, 70.0) } else { (90.0, 280.0) };
            ec.insert(DistanceFog {
                color: clear.0,
                falloff: FogFalloff::Linear { start, end },
                ..default()
            });
        } else {
            ec.remove::<DistanceFog>();
        }
    }
}

fn apply_sun(
    settings: Res<Settings>,
    mut map: ResMut<DirectionalLightShadowMap>,
    mut suns: Query<(&mut DirectionalLight, &mut CascadeShadowConfig), Without<FillLight>>,
) {
    let size = if settings.shadows >= 2 { 4096 } else { 2048 };
    if map.size != size {
        map.size = size;
    }
    for (mut sun, mut cascades) in &mut suns {
        if !settings.is_changed() && !sun.is_added() {
            continue;
        }
        sun.shadows_enabled = settings.shadows > 0;
        *cascades = if settings.shadows >= 2 {
            CascadeShadowConfigBuilder {
                num_cascades: 4,
                first_cascade_far_bound: 8.0,
                maximum_distance: 120.0,
                ..default()
            }
        } else {
            CascadeShadowConfigBuilder {
                num_cascades: 2,
                first_cascade_far_bound: 15.0,
                maximum_distance: 60.0,
                ..default()
            }
        }
        .build();
    }
}
