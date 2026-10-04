//! Applies the graphics settings (shadows, smoothed edges, glow and soft
//! corner shading) to the camera and sun, and adds distance haze in matches.

use bevy::core_pipeline::bloom::Bloom;
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
            .add_systems(Update, (apply_camera, apply_sun));
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
        camera.hdr = settings.bloom;
        if settings.bloom {
            ec.insert(Bloom {
                intensity: 0.12,
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
    mut suns: Query<(&mut DirectionalLight, &mut CascadeShadowConfig)>,
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
