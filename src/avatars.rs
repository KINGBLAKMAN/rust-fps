//! How things look: enemies (zombie-like people), projectiles, power-ups and
//! the other players in your party, with floating name tags.

use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

use crate::data::{skin_material, Character, PowerUp};
use crate::humanoid::{self, HumanoidMeshes, Look, Pose};
use crate::player::LocalPlayer;
use crate::sim::enemy_scale;
use crate::{AppState, Enemy, EnemyStatus, InGameEntity, NetKind, Phase, Replicated, Roster, Session};

pub struct AvatarPlugin;

impl Plugin for AvatarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(
                Update,
                (sync_avatars, place_name_tags, enemy_colors, spin)
                    .chain()
                    .in_set(Phase::Present)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

#[derive(Resource)]
pub struct ReplicatedAssets {
    zombie_skin: [Color; 3],
    zombie_shirt: [Color; 3],
    zombie_pants: Handle<StandardMaterial>,
    zombie_eyes: Handle<StandardMaterial>,
    ball: Handle<Mesh>,
    fireball: Handle<StandardMaterial>,
    grenade: Handle<StandardMaterial>,
    pickup: Handle<Mesh>,
    pickups: [Handle<StandardMaterial>; 4],
    eyes: Handle<StandardMaterial>,
    skin: Handle<StandardMaterial>,
}

fn glow(materials: &mut Assets<StandardMaterial>, color: Color, power: f32) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::from(color) * power,
        ..default()
    })
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let pickups = PowerUp::ALL.map(|p| glow(&mut materials, p.color(), 4.0));
    commands.insert_resource(ReplicatedAssets {
        // Grunt, Shooter, Brute.
        zombie_skin: [
            Color::srgb(0.45, 0.58, 0.38),
            Color::srgb(0.55, 0.42, 0.5),
            Color::srgb(0.5, 0.36, 0.3),
        ],
        zombie_shirt: [
            Color::srgb(0.35, 0.3, 0.26),
            Color::srgb(0.3, 0.12, 0.35),
            Color::srgb(0.45, 0.12, 0.1),
        ],
        zombie_pants: materials.add(Color::srgb(0.18, 0.2, 0.26)),
        zombie_eyes: glow(&mut materials, Color::srgb(1.0, 0.2, 0.1), 8.0),
        ball: meshes.add(Sphere::new(1.0).mesh().ico(2).unwrap()),
        fireball: glow(&mut materials, Color::srgb(1.0, 0.45, 0.1), 10.0),
        grenade: materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.3, 0.15),
            perceptual_roughness: 0.5,
            ..default()
        }),
        pickup: meshes.add(Cuboid::new(0.6, 0.6, 0.6)),
        pickups,
        eyes: glow(&mut materials, Color::srgb(0.6, 0.9, 1.0), 4.0),
        skin: materials.add(Color::srgb(0.85, 0.66, 0.52)),
    });
}

/// Per-enemy materials so hits, burning and slows can tint each one.
#[derive(Component)]
struct EnemyLook {
    skin: Handle<StandardMaterial>,
    shirt: Handle<StandardMaterial>,
    base_skin: Color,
    base_shirt: Color,
}

#[derive(Component)]
struct Spin;

/// Spawns the visible model for something the host replicates. Used by the
/// host's simulation and by clients when a new entity appears.
pub fn spawn_replicated(
    commands: &mut Commands,
    assets: &ReplicatedAssets,
    meshes: &HumanoidMeshes,
    materials: &mut Assets<StandardMaterial>,
    id: u32,
    kind: NetKind,
    pos: Vec3,
) -> Entity {
    let root = commands
        .spawn((
            InGameEntity,
            Replicated { id, kind },
            Transform::from_translation(pos),
            Visibility::default(),
        ))
        .id();
    match kind {
        NetKind::Grunt | NetKind::Shooter | NetKind::Brute => {
            let i = match kind {
                NetKind::Grunt => 0,
                NetKind::Shooter => 1,
                _ => 2,
            };
            let skin = materials.add(StandardMaterial {
                base_color: assets.zombie_skin[i],
                perceptual_roughness: 0.9,
                ..default()
            });
            let shirt = materials.add(StandardMaterial {
                base_color: assets.zombie_shirt[i],
                perceptual_roughness: 0.9,
                ..default()
            });
            let scale = enemy_scale(kind);
            commands.entity(root).insert((
                Enemy,
                EnemyStatus::default(),
                EnemyLook {
                    skin: skin.clone(),
                    shirt: shirt.clone(),
                    base_skin: assets.zombie_skin[i],
                    base_shirt: assets.zombie_shirt[i],
                },
                Transform::from_translation(pos).with_scale(Vec3::splat(scale)),
            ));
            humanoid::build(
                commands,
                root,
                meshes,
                Look {
                    skin,
                    shirt,
                    pants: assets.zombie_pants.clone(),
                    eyes: assets.zombie_eyes.clone(),
                    gun: None,
                    visor: false,
                    pose: Pose::Reach,
                },
            );
        }
        NetKind::Fireball => {
            commands.entity(root).with_children(|p| {
                p.spawn((
                    Mesh3d(assets.ball.clone()),
                    MeshMaterial3d(assets.fireball.clone()),
                    Transform::from_scale(Vec3::splat(0.25)),
                ));
            });
        }
        NetKind::Grenade => {
            commands.entity(root).with_children(|p| {
                p.spawn((
                    Mesh3d(assets.ball.clone()),
                    MeshMaterial3d(assets.grenade.clone()),
                    Transform::from_scale(Vec3::splat(0.14)),
                ));
            });
        }
        NetKind::PowerUp(kind) => {
            let i = PowerUp::ALL.iter().position(|p| *p == kind).unwrap_or(0);
            commands.entity(root).with_children(|p| {
                p.spawn((
                    Spin,
                    Mesh3d(assets.pickup.clone()),
                    MeshMaterial3d(assets.pickups[i].clone()),
                    Transform::default(),
                ));
                p.spawn((
                    PointLight {
                        intensity: 30_000.0,
                        color: kind.color(),
                        range: 5.0,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.8, 0.0),
                ));
            });
        }
    }
    root
}

fn enemy_colors(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut enemies: Query<(&mut EnemyStatus, &EnemyLook)>,
) {
    let dt = time.delta_secs();
    for (mut status, look) in &mut enemies {
        status.flash -= dt;
        let (tint, glow): (Option<Color>, LinearRgba) = if status.flash > 0.0 {
            (None, LinearRgba::rgb(3.0, 3.0, 3.0))
        } else if status.burning {
            (None, LinearRgba::rgb(2.0, 0.6, 0.1) * (0.6 + 0.4 * (time.elapsed_secs() * 12.0).sin().abs()))
        } else if status.slowed {
            (Some(Color::srgb(0.55, 0.8, 1.0)), LinearRgba::rgb(0.1, 0.3, 0.6))
        } else {
            (None, LinearRgba::BLACK)
        };
        for (handle, base) in [(&look.skin, look.base_skin), (&look.shirt, look.base_shirt)] {
            if let Some(m) = materials.get_mut(handle) {
                m.base_color = tint.unwrap_or(base);
                m.emissive = glow;
            }
        }
    }
}

fn spin(time: Res<Time>, mut q: Query<&mut Transform, With<Spin>>) {
    let t = time.elapsed_secs();
    for mut tf in &mut q {
        tf.rotation = Quat::from_rotation_y(t * 2.0) * Quat::from_rotation_x(0.4);
        tf.translation.y = (t * 3.0).sin() * 0.15;
    }
}

// ---------------------------------------------------------------------------
// Other players
// ---------------------------------------------------------------------------

#[derive(Component)]
struct Avatar {
    id: u8,
    tag: Entity,
    character: Character,
    skin: u8,
}

#[derive(Component)]
struct NameTag;

/// Builds a person model for a player with their character colours and gun skin.
pub fn spawn_person(
    commands: &mut Commands,
    meshes: &HumanoidMeshes,
    materials: &mut Assets<StandardMaterial>,
    assets: &ReplicatedAssets,
    character: Character,
    skin: u8,
    transform: Transform,
) -> Entity {
    let root = commands.spawn((transform, Visibility::default())).id();
    humanoid::build(
        commands,
        root,
        meshes,
        Look {
            skin: assets.skin.clone(),
            shirt: materials.add(StandardMaterial {
                base_color: character.suit_color(),
                perceptual_roughness: 0.6,
                ..default()
            }),
            pants: materials.add(StandardMaterial {
                base_color: character.trim_color().darker(0.35),
                perceptual_roughness: 0.7,
                ..default()
            }),
            eyes: assets.eyes.clone(),
            gun: Some(materials.add(skin_material(skin))),
            visor: character == Character::Striker,
            pose: Pose::Rifle,
        },
    );
    root
}

fn sync_avatars(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<Session>,
    roster: Res<Roster>,
    assets: Res<ReplicatedAssets>,
    meshes: Res<HumanoidMeshes>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut avatars: Query<(Entity, &Avatar, &mut Transform)>,
    mut tags: Query<&mut Text, With<NameTag>>,
) {
    let blend = 1.0 - (-15.0 * time.delta_secs()).exp();
    let mut have = Vec::new();

    for (entity, avatar, mut tf) in &mut avatars {
        let p = roster
            .0
            .get(&avatar.id)
            .filter(|p| p.id != session.my_id && p.character == avatar.character && p.skin == avatar.skin);
        let Some(p) = p else {
            commands.entity(avatar.tag).despawn();
            commands.entity(entity).despawn();
            continue;
        };
        have.push(avatar.id);
        // The model faces -Z, the same as yaw 0.
        let (target_pos, target_rot) = if p.alive {
            (p.feet(), Quat::from_rotation_y(p.yaw))
        } else {
            // Downed players lie on the floor.
            (
                p.feet() + Vec3::Y * 0.2,
                Quat::from_rotation_y(p.yaw) * Quat::from_rotation_x(-FRAC_PI_2),
            )
        };
        if tf.translation.distance(target_pos) > 4.0 {
            tf.translation = target_pos;
        } else {
            tf.translation = tf.translation.lerp(target_pos, blend);
        }
        tf.rotation = tf.rotation.slerp(target_rot, blend);
        let crouch = if p.alive && p.stance > 0 { 0.72 } else { 1.0 };
        tf.scale.y += (crouch - tf.scale.y) * blend;
        if let Ok(mut text) = tags.get_mut(avatar.tag) {
            let label = if p.alive {
                format!("{} [{}] {:.0} HP", p.name, p.level, p.health)
            } else {
                format!("{} (down)", p.name)
            };
            if text.0 != label {
                text.0 = label;
            }
        }
    }

    for p in roster.0.values() {
        if p.id == session.my_id || have.contains(&p.id) {
            continue;
        }
        let tag = commands
            .spawn((
                InGameEntity,
                NameTag,
                Text::new(p.name.clone()),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.9, 1.0)),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Visibility::Hidden,
            ))
            .id();
        let root = spawn_person(
            &mut commands,
            &meshes,
            &mut materials,
            &assets,
            p.character,
            p.skin,
            Transform::from_translation(p.feet()),
        );
        commands.entity(root).insert((
            InGameEntity,
            Avatar {
                id: p.id,
                tag,
                character: p.character,
                skin: p.skin,
            },
        ));
    }
}

/// Projects each avatar's head into screen space to position its name.
fn place_name_tags(
    camera: Single<(&Camera, &GlobalTransform), With<LocalPlayer>>,
    avatars: Query<(&Avatar, &Transform)>,
    mut tags: Query<(&mut Node, &mut Visibility, &ComputedNode), With<NameTag>>,
) {
    let (cam, cam_tf) = *camera;
    for (avatar, tf) in &avatars {
        let Ok((mut node, mut vis, computed)) = tags.get_mut(avatar.tag) else {
            continue;
        };
        let head = tf.translation + Vec3::Y * 2.25;
        let in_front = cam_tf.forward().dot(head - cam_tf.translation()) > 0.0;
        match cam.world_to_viewport(cam_tf, head) {
            Ok(screen) if in_front => {
                let size = computed.size() * computed.inverse_scale_factor();
                node.left = Val::Px(screen.x - size.x / 2.0);
                node.top = Val::Px(screen.y - size.y);
                *vis = Visibility::Inherited;
            }
            _ => *vis = Visibility::Hidden,
        }
    }
}
