//! Local input for abilities and interacting. The host checks cooldowns and
//! does the actual work (see sim.rs); Dash moves you right away here.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::config::{Action, InputExt, Settings};
use crate::data::Character;
use crate::game::Paused;
use crate::player::{can_act, LocalPlayer};
use crate::{ActionQueue, MatchState, Phase, PlayerAction, Roster, Session};

pub struct AbilityPlugin;

impl Plugin for AbilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionCounter>().add_systems(
            Update,
            use_abilities
                .after(crate::player::movement)
                .in_set(Phase::Local),
        );
    }
}

/// Numbers our actions so the host handles each exactly once.
#[derive(Resource, Default)]
pub struct ActionCounter(pub u32);

pub fn queue_action(
    session: &Session,
    counter: &mut ActionCounter,
    queue: &mut ActionQueue,
    action: PlayerAction,
) {
    counter.0 += 1;
    queue.0.push((session.my_id, counter.0, action));
}

fn use_abilities(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    window: Single<&Window, With<PrimaryWindow>>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    paused: Res<Paused>,
    mut counter: ResMut<ActionCounter>,
    mut queue: ResMut<ActionQueue>,
    player: Single<(&Transform, &mut LocalPlayer)>,
    mut local_cd: Local<[f32; 3]>,
    time: Res<Time>,
) {
    for cd in local_cd.iter_mut() {
        *cd -= time.delta_secs();
    }
    if !can_act(&session, &roster, &state, &paused, &window) {
        return;
    }
    let Some(me) = roster.me(&session) else {
        return;
    };
    let (cam, mut p) = player.into_inner();

    if keys.tapped(&settings, Action::Interact) {
        queue_action(&session, &mut counter, &mut queue, PlayerAction::Interact);
    }

    for (slot, action) in [(0u8, Action::Ability1), (1, Action::Ability2), (2, Action::Ultimate)] {
        if !keys.tapped(&settings, action) {
            continue;
        }
        let ready = if slot == 2 {
            me.ult_charge >= 100.0
        } else {
            me.cooldowns[slot as usize] <= 0.0
        };
        // The local timer stops double-firing while the host catches up.
        if !ready || local_cd[slot as usize] > 0.0 {
            continue;
        }
        local_cd[slot as usize] = 0.5;
        if me.character == Character::Striker && slot == 0 {
            let fwd = Vec3::new(-p.yaw.sin(), 0.0, -p.yaw.cos());
            let right = Vec3::new(-fwd.z, 0.0, fwd.x);
            let mut dir = Vec3::ZERO;
            if keys.held(&settings, Action::Forward) {
                dir += fwd;
            }
            if keys.held(&settings, Action::Back) {
                dir -= fwd;
            }
            if keys.held(&settings, Action::Right) {
                dir += right;
            }
            if keys.held(&settings, Action::Left) {
                dir -= right;
            }
            p.dash_dir = if dir == Vec3::ZERO { fwd } else { dir.normalize() };
            p.dash_time = 0.18 + 0.03 * me.tiers[0] as f32;
        }
        queue_action(
            &session,
            &mut counter,
            &mut queue,
            PlayerAction::Ability {
                slot,
                origin: cam.translation.to_array(),
                dir: cam.forward().as_vec3().to_array(),
            },
        );
    }
}
