//! Other players: a blue capsule with a visor and a gun, plus a floating name.

use bevy::prelude::*;
use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;

use crate::level::GameAssets;
use crate::player::LocalPlayer;
use crate::{Phase, Roster, Session};

pub struct AvatarPlugin;

impl Plugin for AvatarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (sync_avatars, place_name_tags).chain().in_set(Phase::Present),
        );
    }
}

#[derive(Component)]
struct Avatar {
    id: u8,
    tag: Entity,
}

#[derive(Component)]
struct NameTag;

fn sync_avatars(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<Session>,
    roster: Res<Roster>,
    assets: Res<GameAssets>,
    mut avatars: Query<(Entity, &Avatar, &mut Transform)>,
    mut tags: Query<&mut Text, With<NameTag>>,
) {
    let blend = 1.0 - (-15.0 * time.delta_secs()).exp();
    let mut have: HashMap<u8, ()> = HashMap::new();

    for (entity, avatar, mut tf) in &mut avatars {
        let Some(p) = roster.0.get(&avatar.id).filter(|_| avatar.id != session.my_id) else {
            commands.entity(avatar.tag).despawn();
            commands.entity(entity).despawn();
            continue;
        };
        have.insert(avatar.id, ());
        let (target_pos, target_rot) = if p.alive {
            (p.feet() + Vec3::Y, Quat::from_rotation_y(p.yaw))
        } else {
            // Downed players lie on the floor.
            (
                p.feet().with_y(0.5),
                Quat::from_rotation_y(p.yaw) * Quat::from_rotation_x(-FRAC_PI_2),
            )
        };
        if tf.translation.distance(target_pos) > 4.0 {
            tf.translation = target_pos;
        } else {
            tf.translation = tf.translation.lerp(target_pos, blend);
        }
        tf.rotation = tf.rotation.slerp(target_rot, blend);
        if let Ok(mut text) = tags.get_mut(avatar.tag) {
            let label = if p.alive {
                format!("{} {:.0}", p.name, p.health)
            } else {
                format!("{} (down)", p.name)
            };
            if text.0 != label {
                text.0 = label;
            }
        }
    }

    for p in roster.0.values() {
        if p.id == session.my_id || have.contains_key(&p.id) {
            continue;
        }
        let tag = commands
            .spawn((
                NameTag,
                Text::new(p.name.clone()),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.85, 1.0)),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Visibility::Hidden,
            ))
            .id();
        commands
            .spawn((
                Avatar { id: p.id, tag },
                Mesh3d(assets.enemy_mesh.clone()),
                MeshMaterial3d(assets.avatar_material.clone()),
                Transform::from_translation(p.feet() + Vec3::Y),
            ))
            .with_children(|a| {
                a.spawn((
                    Mesh3d(assets.visor_mesh.clone()),
                    MeshMaterial3d(assets.gun_material.clone()),
                    Transform::from_xyz(0.0, 0.5, -0.38),
                ));
                a.spawn((
                    Mesh3d(assets.gun_mesh.clone()),
                    MeshMaterial3d(assets.gun_material.clone()),
                    Transform::from_xyz(0.35, 0.15, -0.5),
                ));
            });
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
        let head = tf.translation + Vec3::Y * 1.3;
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
