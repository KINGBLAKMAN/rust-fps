//! Emotes: press G to open the emote wheel, then 1-5 (or click) to dance,
//! wave, flip off, point or backflip. The camera pulls out to show your whole
//! character while it plays; moving, shooting or jumping stops it.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::config::{Action, InputExt, Settings};
use crate::game::Paused;
use crate::physics::{collect_boxes, ray_world};
use crate::player::{can_act, LocalPlayer};
use crate::rig::{emote_length, EMOTES};
use crate::{AppState, Collider, InGameEntity, MatchState, Phase, Roster, Session};

pub struct EmotePlugin;

impl Plugin for EmotePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EmoteMenu>()
            .add_systems(OnEnter(AppState::InGame), spawn_menu)
            .add_systems(
                Update,
                emote_input
                    .in_set(Phase::Local)
                    .before(crate::player::movement),
            )
            .add_systems(
                Update,
                (
                    emote_camera
                        .after(crate::player::movement)
                        .in_set(Phase::Local),
                    show_menu.in_set(Phase::Present),
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

/// Whether the emote list is open.
#[derive(Resource, Default)]
pub struct EmoteMenu {
    pub open: bool,
}

#[derive(Component)]
struct MenuPanel;

const CAMERA_DISTANCE: f32 = 2.7;
const DIGITS: [KeyCode; 5] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
];

fn spawn_menu(mut commands: Commands, mut menu: ResMut<EmoteMenu>) {
    menu.open = false;
    commands
        .spawn((
            InGameEntity,
            MenuPanel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                top: Val::Percent(32.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.09, 0.85)),
            BorderRadius::all(Val::Px(8.0)),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("EMOTES"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.8, 0.3)),
            ));
            for (i, (name, _)) in EMOTES.iter().enumerate() {
                p.spawn((
                    Text::new(format!("{}  {}", i + 1, name)),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            }
            p.spawn((
                Text::new("G to close"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.65, 0.75)),
            ));
        });
}

fn show_menu(menu: Res<EmoteMenu>, mut panel: Query<&mut Visibility, With<MenuPanel>>) {
    for mut vis in &mut panel {
        let want = if menu.open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn emote_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<Settings>,
    window: Single<&Window, With<PrimaryWindow>>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    paused: Res<Paused>,
    app_state: Res<State<AppState>>,
    mut menu: ResMut<EmoteMenu>,
    mut player: Single<&mut LocalPlayer>,
) {
    let p = &mut **player;
    if *app_state.get() != AppState::InGame || !can_act(&session, &roster, &state, &paused, &window)
    {
        menu.open = false;
        p.emote = None;
        return;
    }
    if keys.tapped(&settings, Action::Emote) {
        menu.open = !menu.open;
    }
    if menu.open {
        if let Some(i) = DIGITS.iter().position(|k| keys.just_pressed(*k)) {
            let which = i as u8 + 1;
            menu.open = false;
            p.emote = Some((which, emote_length(which)));
            p.emote_seq = p.emote_seq.wrapping_add(1);
            p.orbit = Vec2::new(0.0, 0.25);
            if which == 5 && p.on_ground {
                p.vel.y = 7.5;
            }
        }
    }
    let Some((which, left)) = p.emote else { return };
    let left = left - time.delta_secs();
    // Moving, jumping, shooting or using anything stops the emote (the
    // backflip carries on through its own jump).
    let moved = [
        Action::Forward,
        Action::Back,
        Action::Left,
        Action::Right,
        Action::Jump,
        Action::Crouch,
    ]
    .iter()
    .any(|a| keys.held(&settings, *a))
        && which != 5;
    let busy = [
        Action::Ability1,
        Action::Ability2,
        Action::Ultimate,
        Action::Reload,
        Action::Interact,
    ]
    .iter()
    .any(|a| keys.tapped(&settings, *a))
        || mouse.just_pressed(MouseButton::Left)
        || mouse.just_pressed(MouseButton::Right);
    p.emote = if left <= 0.0 || moved || busy {
        None
    } else {
        Some((which, left))
    };
}

/// Pulls the camera out in front of you while you emote, so you can
/// see your character; the mouse orbits it.
fn emote_camera(
    time: Res<Time>,
    motion: Res<AccumulatedMouseMotion>,
    settings: Res<Settings>,
    player: Single<(&mut Transform, &mut LocalPlayer)>,
    colliders: Query<(&Transform, &Collider), Without<LocalPlayer>>,
) {
    let (mut tf, mut p) = player.into_inner();
    let dt = time.delta_secs();
    let target = if p.emoting() { 1.0 } else { 0.0 };
    p.cam_out = if target > 0.5 {
        (p.cam_out + dt * 3.5).min(1.0)
    } else {
        (p.cam_out - dt * 5.0).max(0.0)
    };
    if p.cam_out <= 0.0 {
        return;
    }
    if p.emoting() {
        let s = crate::player::MOUSE_SCALE * settings.sensitivity;
        p.orbit.x -= motion.delta.x * s;
        p.orbit.y = (p.orbit.y + motion.delta.y * s).clamp(-0.3, 1.2);
    }
    let out = p.cam_out * p.cam_out * (3.0 - 2.0 * p.cam_out);
    // Start in front of the character, looking back at them.
    let yaw = p.yaw + p.orbit.x;
    let pitch = p.orbit.y;
    let dir = Vec3::new(
        -yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    );
    let focus = p.feet + Vec3::Y * 1.15;
    let boxes = collect_boxes(colliders.iter());
    let dist = (ray_world(focus, dir, CAMERA_DISTANCE, &boxes) - 0.25).clamp(0.6, CAMERA_DISTANCE);
    let cam = focus + dir * dist;
    let look = Transform::from_translation(cam).looking_at(focus, Vec3::Y);
    tf.translation = tf.translation.lerp(cam, out);
    tf.rotation = tf.rotation.slerp(look.rotation, out);
}
