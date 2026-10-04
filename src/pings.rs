//! Pings: press Z (or the middle mouse button) to mark the spot you are
//! looking at for your whole party. Pinging an enemy marks it in red and the
//! marker follows it. Each player has one ping at a time; it fades after a
//! few seconds.

use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::abilities::{queue_action, ActionCounter};
use crate::config::{Action, InputExt, Settings};
use crate::fx::{Fx, FxQueue};
use crate::game::Paused;
use crate::kit::{c, Kit};
use crate::physics::{collect_boxes, trace_shot};
use crate::player::{can_act, LocalPlayer};
use crate::{
    ActionQueue, AppState, Collider, Enemy, InGameEntity, MatchState, Phase, PlayerAction,
    Replicated, Roster, Session,
};

pub struct PingPlugin;

impl Plugin for PingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, send_ping.in_set(Phase::Local))
            .add_systems(
                Update,
                (
                    spawn_pings.before(crate::fx::play),
                    place_pings.after(crate::fx::play),
                )
                    .in_set(Phase::Present)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

const PING_LIFE: f32 = 7.0;
const PING_RANGE: f32 = 120.0;

#[derive(Resource)]
struct PingAssets {
    marker: Handle<Mesh>,
    beam: Handle<Mesh>,
    place: Handle<StandardMaterial>,
    enemy: Handle<StandardMaterial>,
}

#[derive(Component)]
struct Ping {
    player: u8,
    pos: Vec3,
    target: Option<u32>,
    life: f32,
    label: Entity,
}

#[derive(Component)]
struct PingLabel;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // An upside-down diamond with a ring under it.
    let mut k = Kit::new();
    k.cone(
        Vec3::new(0.0, 0.0, 0.0),
        0.28,
        0.5,
        Quat::from_rotation_x(std::f32::consts::PI),
        c(1.0, 1.0, 1.0),
    );
    k.cone(
        Vec3::new(0.0, 0.42, 0.0),
        0.28,
        0.34,
        Quat::IDENTITY,
        c(1.0, 1.0, 1.0),
    );
    let glow = |materials: &mut Assets<StandardMaterial>, color: Color| {
        materials.add(StandardMaterial {
            base_color: color.with_alpha(0.85),
            emissive: LinearRgba::from(color) * 4.0,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })
    };
    commands.insert_resource(PingAssets {
        marker: meshes.add(k.build_or_empty()),
        beam: meshes.add(Cylinder::new(0.04, 1.0)),
        place: glow(&mut materials, Color::srgb(1.0, 0.85, 0.2)),
        enemy: glow(&mut materials, Color::srgb(1.0, 0.2, 0.15)),
    });
}

#[allow(clippy::too_many_arguments)]
fn send_ping(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<Settings>,
    window: Single<&Window, With<PrimaryWindow>>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    paused: Res<Paused>,
    mut counter: ResMut<ActionCounter>,
    mut queue: ResMut<ActionQueue>,
    player: Single<(&Transform, &LocalPlayer)>,
    colliders: Query<(&Transform, &Collider), Without<LocalPlayer>>,
    enemies: Query<(Entity, &Transform, &Replicated), With<Enemy>>,
    mut cooldown: Local<f32>,
) {
    *cooldown -= time.delta_secs();
    if !keys.tapped(&settings, Action::Ping) && !mouse.just_pressed(MouseButton::Middle) {
        return;
    }
    if *cooldown > 0.0 || !can_act(&session, &roster, &state, &paused, &window) {
        return;
    }
    *cooldown = 0.4;
    let (cam, p) = player.into_inner();
    // Aim from the eye even while the camera is out for an emote.
    let origin = if p.third_person() {
        p.eye_pos()
    } else {
        cam.translation
    };
    let dir = if p.third_person() {
        let (yaw, pitch) = (p.yaw, p.pitch);
        Vec3::new(
            -yaw.sin() * pitch.cos(),
            pitch.sin(),
            -yaw.cos() * pitch.cos(),
        )
    } else {
        cam.forward().as_vec3()
    };
    let boxes = collect_boxes(colliders.iter());
    let hit = trace_shot(
        origin,
        dir,
        PING_RANGE,
        &boxes,
        enemies
            .iter()
            .map(|(e, t, r)| (e, t.translation, crate::sim::enemy_scale(r.kind), false)),
    );
    let target = hit
        .enemy
        .and_then(|(e, _)| enemies.get(e).ok())
        .map(|(_, _, r)| r.id);
    let pos = origin + dir * (hit.dist - 0.05).max(0.5);
    queue_action(
        &session,
        &mut counter,
        &mut queue,
        PlayerAction::Ping {
            pos: pos.to_array(),
            target: target.unwrap_or(u32::MAX),
        },
    );
}

/// Turns ping effects into markers (one per player).
fn spawn_pings(
    mut commands: Commands,
    queue: Res<FxQueue>,
    assets: Res<PingAssets>,
    roster: Res<Roster>,
    existing: Query<(Entity, &Ping)>,
) {
    for fx in &queue.0 {
        let Fx::Ping {
            player,
            pos,
            target,
        } = *fx
        else {
            continue;
        };
        for (e, ping) in &existing {
            if ping.player == player {
                commands.entity(ping.label).despawn();
                commands.entity(e).despawn();
            }
        }
        let enemy = target != u32::MAX;
        let name = roster
            .0
            .get(&player)
            .map_or("?".to_string(), |p| p.name.clone());
        let color = if enemy {
            Color::srgb(1.0, 0.35, 0.3)
        } else {
            Color::srgb(1.0, 0.85, 0.3)
        };
        let label = commands
            .spawn((
                InGameEntity,
                PingLabel,
                Text::new(name),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(color),
                TextLayout::new_with_justify(JustifyText::Center),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Visibility::Hidden,
            ))
            .id();
        let mat = if enemy {
            assets.enemy.clone()
        } else {
            assets.place.clone()
        };
        commands
            .spawn((
                InGameEntity,
                Ping {
                    player,
                    pos: Vec3::from_array(pos),
                    target: enemy.then_some(target),
                    life: PING_LIFE,
                    label,
                },
                Transform::from_translation(Vec3::from_array(pos)),
                Visibility::default(),
            ))
            .with_children(|p| {
                p.spawn((
                    Mesh3d(assets.marker.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(0.0, 0.6, 0.0),
                    NotShadowCaster,
                ));
                if !enemy {
                    p.spawn((
                        Mesh3d(assets.beam.clone()),
                        MeshMaterial3d(mat),
                        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(1.0, 1.0, 1.0)),
                        NotShadowCaster,
                    ));
                }
            });
    }
}

/// Keeps markers on their enemy, bobbing and big enough to see from afar,
/// with the pinger's name and the distance on screen.
#[allow(clippy::type_complexity)]
fn place_pings(
    mut commands: Commands,
    time: Res<Time>,
    camera: Single<(&Camera, &GlobalTransform), With<LocalPlayer>>,
    targets: Query<(&Replicated, &Transform), (Without<Ping>, Without<LocalPlayer>)>,
    mut pings: Query<(Entity, &mut Ping, &mut Transform, &Children)>,
    mut parts: Query<&mut Transform, (Without<Ping>, Without<Replicated>, Without<LocalPlayer>)>,
    mut labels: Query<(&mut Node, &mut Visibility, &mut Text, &ComputedNode), With<PingLabel>>,
    roster: Res<Roster>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let (cam, cam_tf) = *camera;
    for (e, mut ping, mut tf, children) in &mut pings {
        ping.life -= dt;
        if let Some(id) = ping.target {
            match targets.iter().find(|(r, _)| r.id == id) {
                Some((_, et)) => ping.pos = et.translation + Vec3::Y * 2.1 * et.scale.y,
                // The enemy died: the ping goes with it.
                None => ping.life = ping.life.min(0.3),
            }
        }
        if ping.life <= 0.0 {
            commands.entity(ping.label).despawn();
            commands.entity(e).despawn();
            continue;
        }
        tf.translation = ping.pos;
        let dist = cam_tf.translation().distance(ping.pos);
        // Stay a readable size at range.
        let size = (dist / 14.0).clamp(0.6, 4.0) * (ping.life / 0.3).min(1.0);
        for (i, child) in children.iter().enumerate() {
            if let Ok(mut ctf) = parts.get_mut(child) {
                if i == 0 {
                    ctf.translation.y = 0.5 * size + 0.12 * size * (t * 4.0).sin();
                    ctf.scale = Vec3::splat(size);
                    ctf.rotation = Quat::from_rotation_y(t * 2.0);
                } else {
                    ctf.scale = Vec3::new(size.min(2.0), 60.0, size.min(2.0));
                    ctf.translation.y = 30.0;
                }
            }
        }
        let Ok((mut node, mut vis, mut text, computed)) = labels.get_mut(ping.label) else {
            continue;
        };
        let head = ping.pos + Vec3::Y * 1.3 * size;
        let in_front = cam_tf.forward().dot(head - cam_tf.translation()) > 0.0;
        match cam.world_to_viewport(cam_tf, head) {
            Ok(screen) if in_front => {
                let name = roster.0.get(&ping.player).map_or("?", |p| p.name.as_str());
                let label = format!("{name}\n{dist:.0} m");
                if text.0 != label {
                    text.0 = label;
                }
                let size = computed.size() * computed.inverse_scale_factor();
                node.left = Val::Px(screen.x - size.x / 2.0);
                node.top = Val::Px(screen.y - size.y);
                *vis = Visibility::Inherited;
            }
            _ => *vis = Visibility::Hidden,
        }
    }
}
