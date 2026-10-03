//! The arena, lights and the meshes/materials shared by spawned entities.

use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

use crate::{Collider, Enemy, EnemyLook, NetKind, Replicated, ARENA_HALF};

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup_level);
    }
}

#[derive(Resource)]
pub struct GameAssets {
    pub enemy_mesh: Handle<Mesh>,
    pub eye_mesh: Handle<Mesh>,
    pub eye_material: Handle<StandardMaterial>,
    pub projectile_mesh: Handle<Mesh>,
    pub projectile_material: Handle<StandardMaterial>,
    pub avatar_material: Handle<StandardMaterial>,
    pub gun_mesh: Handle<Mesh>,
    pub gun_material: Handle<StandardMaterial>,
    pub visor_mesh: Handle<Mesh>,
}

pub fn enemy_color(kind: NetKind) -> Color {
    match kind {
        NetKind::Shooter => Color::srgb(0.5, 0.2, 0.75),
        _ => Color::srgb(0.8, 0.15, 0.12),
    }
}

/// Spawns the visible part of an enemy or projectile. The host adds AI on top;
/// clients just move it to wherever the host says it is.
pub fn spawn_replicated(
    commands: &mut Commands,
    assets: &GameAssets,
    materials: &mut Assets<StandardMaterial>,
    id: u32,
    kind: NetKind,
    pos: Vec3,
) -> Entity {
    if kind == NetKind::Projectile {
        return commands
            .spawn((
                Replicated { id, kind },
                Mesh3d(assets.projectile_mesh.clone()),
                MeshMaterial3d(assets.projectile_material.clone()),
                Transform::from_translation(pos),
            ))
            .id();
    }
    let color = enemy_color(kind);
    commands
        .spawn((
            Replicated { id, kind },
            Enemy,
            EnemyLook {
                base_color: color,
                flash: 0.0,
            },
            Mesh3d(assets.enemy_mesh.clone()),
            // Each enemy gets its own material so it can flash on its own.
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.6,
                ..default()
            })),
            Transform::from_translation(pos),
        ))
        .with_children(|e| {
            for x in [-0.18, 0.18] {
                e.spawn((
                    Mesh3d(assets.eye_mesh.clone()),
                    MeshMaterial3d(assets.eye_material.clone()),
                    Transform::from_xyz(x, 0.45, -0.42),
                ));
            }
        })
        .id()
}

fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Floor
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(ARENA_HALF * 2.0, ARENA_HALF * 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.32, 0.36, 0.30),
            perceptual_roughness: 0.95,
            ..default()
        })),
    ));

    // Floor grid stripes for a sense of motion
    let stripe_mat = materials.add(Color::srgb(0.27, 0.30, 0.25));
    let stripe_mesh = meshes.add(Cuboid::new(ARENA_HALF * 2.0, 0.01, 0.08));
    for i in -6..=6 {
        let p = i as f32 * 5.0;
        commands.spawn((
            Mesh3d(stripe_mesh.clone()),
            MeshMaterial3d(stripe_mat.clone()),
            Transform::from_xyz(0.0, 0.005, p),
        ));
        commands.spawn((
            Mesh3d(stripe_mesh.clone()),
            MeshMaterial3d(stripe_mat.clone()),
            Transform::from_xyz(p, 0.005, 0.0).with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
        ));
    }

    let wall_mat = materials.add(Color::srgb(0.55, 0.52, 0.48));
    let pillar_mat = materials.add(Color::srgb(0.42, 0.45, 0.55));
    let crate_mat = materials.add(Color::srgb(0.62, 0.44, 0.24));

    let mut spawn_box = |pos: Vec3, size: Vec3, mat: &Handle<StandardMaterial>| {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(pos),
            Collider { half: size / 2.0 },
        ));
    };

    // Outer walls
    let h = 4.0;
    let len = ARENA_HALF * 2.0 + 2.0;
    spawn_box(Vec3::new(0.0, h / 2.0, -ARENA_HALF - 0.5), Vec3::new(len, h, 1.0), &wall_mat);
    spawn_box(Vec3::new(0.0, h / 2.0, ARENA_HALF + 0.5), Vec3::new(len, h, 1.0), &wall_mat);
    spawn_box(Vec3::new(-ARENA_HALF - 0.5, h / 2.0, 0.0), Vec3::new(1.0, h, len), &wall_mat);
    spawn_box(Vec3::new(ARENA_HALF + 0.5, h / 2.0, 0.0), Vec3::new(1.0, h, len), &wall_mat);

    // Pillars
    for (x, z) in [
        (-12.0, -12.0),
        (12.0, -12.0),
        (-12.0, 12.0),
        (12.0, 12.0),
        (0.0, -18.0),
        (-20.0, 0.0),
        (20.0, 0.0),
    ] {
        spawn_box(Vec3::new(x, 2.5, z), Vec3::new(2.0, 5.0, 2.0), &pillar_mat);
    }

    // Low cover walls
    spawn_box(Vec3::new(-6.0, 0.75, -4.0), Vec3::new(6.0, 1.5, 0.6), &wall_mat);
    spawn_box(Vec3::new(7.0, 0.75, 3.0), Vec3::new(0.6, 1.5, 6.0), &wall_mat);
    spawn_box(Vec3::new(0.0, 0.75, 22.0), Vec3::new(8.0, 1.5, 0.6), &wall_mat);

    // Crates (can be jumped on)
    for (x, z, s) in [
        (-3.0, 6.0, 1.2),
        (-1.8, 6.0, 1.0),
        (4.0, -10.0, 1.4),
        (15.0, 18.0, 1.2),
        (-17.0, -20.0, 1.4),
        (22.0, -15.0, 1.0),
        (-24.0, 16.0, 1.2),
        (-8.0, 14.0, 1.0),
    ] {
        spawn_box(Vec3::new(x, s / 2.0, z), Vec3::splat(s), &crate_mat);
    }
    // A stacked crate
    spawn_box(Vec3::new(-3.0, 1.2 + 0.5, 6.0), Vec3::splat(1.0), &crate_mat);

    // Lights
    commands.spawn((
        DirectionalLight {
            illuminance: 9000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(15.0, 30.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Shared enemy assets
    commands.insert_resource(GameAssets {
        enemy_mesh: meshes.add(Capsule3d::new(0.5, 1.0)),
        eye_mesh: meshes.add(Sphere::new(0.1)),
        eye_material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 0.6),
            emissive: LinearRgba::rgb(6.0, 6.0, 2.0),
            ..default()
        }),
        projectile_mesh: meshes.add(Sphere::new(0.2)),
        projectile_material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.5, 0.1),
            emissive: LinearRgba::rgb(12.0, 4.0, 0.5),
            ..default()
        }),
        avatar_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.45, 0.9),
            perceptual_roughness: 0.5,
            ..default()
        }),
        gun_mesh: meshes.add(Cuboid::new(0.1, 0.12, 0.6)),
        gun_material: materials.add(Color::srgb(0.12, 0.12, 0.14)),
        visor_mesh: meshes.add(Cuboid::new(0.6, 0.18, 0.2)),
    });
}
