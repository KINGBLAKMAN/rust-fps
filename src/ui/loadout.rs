//! The loadout screen (the gun you bring into matches and its attachments)
//! and the attachment guide.

use bevy::prelude::*;

use crate::config::Profile;
use crate::data::{attach_options_for, gun_def, Attach, Slot, ATTACHMENTS};
use crate::progression::{
    attachment_unlocked, career, gun_unlocked, loadout_guns, unlock_level, Unlock,
};

use super::{button_sized, label, panel, row, UiAction, ACCENT, DIM};

/// A thin bar showing progress towards the next career level.
fn xp_bar(p: &mut ChildSpawnerCommands, into: u32, need: u32) {
    p.spawn((
        Node {
            width: Val::Px(420.0),
            height: Val::Px(10.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.16, 0.18, 0.26)),
        BorderRadius::all(Val::Px(5.0)),
    ))
    .with_children(|b| {
        b.spawn((
            Node {
                width: Val::Percent(100.0 * into as f32 / need.max(1) as f32),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.45, 0.8, 1.0)),
            BorderRadius::all(Val::Px(5.0)),
        ));
    });
}

pub(super) fn loadout_screen(commands: &mut Commands, profile: &Profile) {
    let (level, into, need) = career(profile.career_xp);
    let chosen = profile.loadout();
    panel(commands, false, |p| {
        label(p, "LOADOUT", 34.0, ACCENT);
        label(
            p,
            format!("Career level {level}   {into} / {need} XP to the next"),
            17.0,
            Color::WHITE,
        );
        xp_bar(p, into, need);
        label(
            p,
            "The gun you bring hangs on a wall board in every match, with\nyour attachments on it, and the mystery box leaves it out.\nPlay matches to earn XP: each level unlocks a gun or attachment.",
            14.0,
            DIM,
        );
        label(p, "Gun to bring:", 17.0, Color::WHITE);
        row(p, |r| {
            button_sized(
                r,
                "Nothing",
                UiAction::LoadoutGun(None),
                Some(150.0),
                chosen.is_none(),
            );
            for g in loadout_guns()
                .into_iter()
                .filter(|&g| gun_unlocked(level, g))
            {
                let selected = chosen.is_some_and(|(c, _)| c == g);
                button_sized(
                    r,
                    gun_def(g).name,
                    UiAction::LoadoutGun(Some(g)),
                    Some(150.0),
                    selected,
                );
            }
        });
        // Only the next gun to unlock is shown, so the list stays short.
        let locked: Vec<u8> = loadout_guns()
            .into_iter()
            .filter(|&g| !gun_unlocked(level, g))
            .collect();
        if let Some(&next) = locked.first() {
            label(
                p,
                format!(
                    "Next gun: {} at level {}  ({} more to unlock after that)",
                    gun_def(next).name,
                    unlock_level(Unlock::Gun(next)),
                    locked.len() - 1
                ),
                14.0,
                DIM,
            );
        }
        let Some((gun, attach)) = chosen else {
            button_sized(p, "Back", UiAction::BackToMain, Some(260.0), false);
            return;
        };
        let opts = attach_options_for(gun);
        label(
            p,
            format!("Attachments on the {}:", gun_def(gun).name),
            17.0,
            Color::WHITE,
        );
        for (i, slot) in Slot::ALL.into_iter().enumerate() {
            let fitted = match slot {
                Slot::Optic => attach.optic(),
                Slot::Muzzle => attach.muzzle(),
                Slot::Under => attach.under(),
                Slot::Mag => attach.ext_mag() as u8,
            };
            let choices = opts.for_slot(slot);
            row(p, |r| {
                r.spawn((
                    Text::new(slot.name()),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(DIM),
                    Node {
                        width: Val::Px(100.0),
                        ..default()
                    },
                ));
                if choices.is_empty() {
                    label(r, "Not on this gun (built in or doesn't fit)", 14.0, DIM);
                    return;
                }
                button_sized(
                    r,
                    "None",
                    UiAction::LoadoutAttach(i as u8, 0),
                    Some(110.0),
                    fitted == 0,
                );
                for &id in choices {
                    let Some(index) = ATTACHMENTS
                        .iter()
                        .position(|a| a.slot == slot && a.id == id)
                    else {
                        continue;
                    };
                    if attachment_unlocked(level, index) {
                        button_sized(
                            r,
                            ATTACHMENTS[index].name,
                            UiAction::LoadoutAttach(i as u8, id),
                            Some(130.0),
                            fitted == id,
                        );
                    } else {
                        let lv = unlock_level(Unlock::Attachment(index));
                        button_sized(
                            r,
                            format!("Level {lv}"),
                            UiAction::Locked,
                            Some(130.0),
                            false,
                        );
                    }
                }
            });
        }
        // What it all adds up to.
        let h = attach.handling(gun);
        let pct = |k: f32| ((k - 1.0) * 100.0).round() as i32;
        label(
            p,
            format!(
                "Aim time {:+}%   Aimed spread {:+}%   Hip spread {:+}%\nClimb {:+}%   Side recoil {:+}%   Reload {:+}%   Damage {:+}%",
                pct(h.ads_time),
                pct(h.ads_spread),
                pct(h.hip_spread),
                pct(h.recoil_up),
                pct(h.recoil_side),
                pct(h.reload),
                pct(h.damage)
            ),
            14.0,
            ACCENT,
        );
        let _ = Attach::NONE;
        row(p, |r| {
            button_sized(
                r,
                "Attachment Guide",
                UiAction::OpenGuide(0),
                Some(260.0),
                false,
            );
            button_sized(r, "Back", UiAction::BackToMain, Some(260.0), false);
        });
    });
}

pub(super) fn guide_screen(commands: &mut Commands, profile: &Profile, tab: u8) {
    let level = career(profile.career_xp).0;
    panel(commands, false, |p| {
        label(p, "ATTACHMENT GUIDE", 34.0, ACCENT);
        row(p, |r| {
            button_sized(r, "All", UiAction::OpenGuide(0), Some(80.0), tab == 0);
            for (i, slot) in Slot::ALL.into_iter().enumerate() {
                button_sized(
                    r,
                    slot.name(),
                    UiAction::OpenGuide(i as u8 + 1),
                    Some(120.0),
                    tab == i as u8 + 1,
                );
            }
        });
        label(
            p,
            "Guns from the mystery box come with random attachments.\nYour loadout gun uses the ones you pick.",
            14.0,
            DIM,
        );
        let shown = ATTACHMENTS
            .iter()
            .enumerate()
            .filter(|(_, a)| tab == 0 || Slot::ALL.get(tab as usize - 1) == Some(&a.slot));
        // Cards two to a row.
        row(p, |r| {
            for (i, a) in shown {
                r.spawn((
                    Node {
                        width: Val::Px(250.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(3.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.13, 0.19, 0.9)),
                    BorderColor(Color::srgb(0.3, 0.32, 0.42)),
                    BorderRadius::all(Val::Px(6.0)),
                ))
                .with_children(|c| {
                    label(c, a.name, 18.0, ACCENT);
                    let lock = if attachment_unlocked(level, i) {
                        "Unlocked for your loadout".to_string()
                    } else {
                        format!(
                            "Loadout unlock at career level {}",
                            unlock_level(Unlock::Attachment(i))
                        )
                    };
                    label(c, format!("{}  -  {lock}", a.slot.name()), 12.0, DIM);
                    label(c, a.blurb, 13.0, Color::WHITE);
                    for (good, e) in a.effects() {
                        let color = match good {
                            Some(true) => Color::srgb(0.5, 1.0, 0.6),
                            Some(false) => Color::srgb(1.0, 0.55, 0.45),
                            None => Color::srgb(0.6, 0.85, 1.0),
                        };
                        label(c, e, 13.0, color);
                    }
                });
            }
        });
        button_sized(p, "Back", UiAction::BackToMain, Some(260.0), false);
    });
}
