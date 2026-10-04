//! The sandbox tools panel (F1 in a sandbox match).

use bevy::prelude::*;

use crate::data::{attach_options_for, gun_def, Attach, Slot, ATTACHMENTS, GUNS};
use crate::MatchState;

use super::{button_sized, label, row, UiAction, ACCENT, DIM};

/// The gun and attachments picked in the panel.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub(super) struct Tools {
    pub gun: u8,
    pub attach: Attach,
}

pub(super) fn panel(commands: &mut Commands, state: &MatchState, tools: &Tools) {
    super::panel(commands, true, |p| {
        label(p, "SANDBOX TOOLS", 28.0, ACCENT);
        label(p, "F1 or Esc to close. The game waits while this is open.", 14.0, DIM);
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(24.0),
            ..default()
        })
        .with_children(|cols| {
            column(cols, 450.0, |p| left(p, state, tools));
            column(cols, 680.0, |p| right(p, tools));
        });
    });
}

fn column(p: &mut ChildSpawnerCommands, width: f32, f: impl FnOnce(&mut ChildSpawnerCommands)) {
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(8.0),
        width: Val::Px(width),
        ..default()
    })
    .with_children(f);
}

/// Spawning, switches, quick actions and the attachments for the picked gun.
fn left(p: &mut ChildSpawnerCommands, state: &MatchState, tools: &Tools) {
    let sb = state.sandbox;
    label(p, "Spawn zombies in front of you:", 16.0, Color::WHITE);
    row(p, |r| {
        let spawns = [("Walker", 0, 1), ("5 Walkers", 0, 5), ("Spitter", 1, 1), ("Brute", 2, 1), ("Crawler", 3, 1)];
        for (name, kind, count) in spawns {
            button_sized(r, name, UiAction::SandboxSpawn(kind, count), Some(83.0), false);
        }
    });
    row(p, |r| {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        button_sized(r, format!("Waves {}", on(sb.waves)), UiAction::SandboxToggle(0), Some(140.0), sb.waves);
        button_sized(r, format!("God mode {}", on(sb.god)), UiAction::SandboxToggle(1), Some(140.0), sb.god);
        let free = format!("Free abilities {}", on(sb.free_abilities));
        button_sized(r, free, UiAction::SandboxToggle(2), Some(150.0), sb.free_abilities);
    });
    row(p, |r| {
        button_sized(r, "+10,000 pts", UiAction::SandboxPoints, Some(106.0), false);
        button_sized(r, "Level up", UiAction::SandboxLevel, Some(104.0), false);
        let next = format!("Round {}", state.round + 1);
        button_sized(r, next, UiAction::SandboxRound, Some(104.0), false);
        button_sized(r, "Kill all", UiAction::SandboxKill, Some(104.0), false);
    });
    label(p, format!("Attachments for the {}:", gun_def(tools.gun).name), 16.0, Color::WHITE);
    let opts = attach_options_for(tools.gun);
    for (i, slot) in Slot::ALL.into_iter().enumerate() {
        let fitted = match slot {
            Slot::Optic => tools.attach.optic(),
            Slot::Muzzle => tools.attach.muzzle(),
            Slot::Under => tools.attach.under(),
            Slot::Mag => tools.attach.ext_mag() as u8,
        };
        let choices = opts.for_slot(slot);
        if choices.is_empty() {
            continue;
        }
        row(p, |r| {
            r.spawn((
                Text::new(slot.name()),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(DIM),
                Node {
                    width: Val::Px(90.0),
                    ..default()
                },
            ));
            button_sized(r, "None", UiAction::SandboxAttach(i as u8, 0), Some(70.0), fitted == 0);
            for &id in choices {
                if let Some(a) = ATTACHMENTS.iter().find(|a| a.slot == slot && a.id == id) {
                    button_sized(r, a.name, UiAction::SandboxAttach(i as u8, id), Some(130.0), fitted == id);
                }
            }
        });
    }
    let take = format!("Take the {}", gun_def(tools.gun).name);
    button_sized(p, take, UiAction::SandboxGive, Some(440.0), false);
}

fn right(p: &mut ChildSpawnerCommands, tools: &Tools) {
    label(p, "Gun:", 16.0, Color::WHITE);
    row(p, |r| {
        for g in 0..GUNS.len() as u8 {
            button_sized(r, gun_def(g).name, UiAction::SandboxGun(g), Some(128.0), tools.gun == g);
        }
    });
}
