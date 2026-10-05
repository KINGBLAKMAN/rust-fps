//! Career progress kept between matches. Every match earns career XP; each
//! career level unlocks a gun or an attachment for your loadout. The gun you
//! bring into a match hangs on one of the map's two wall-buy boards (with
//! your attachments on it), and the mystery box leaves it out.

use bevy::prelude::*;

use crate::config::Profile;
use crate::data::{gun_def, Attach, GunClass, ATTACHMENTS};
use crate::maps::MapLayout;
use crate::Roster;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unlock {
    Gun(u8),
    /// An entry in `ATTACHMENTS`.
    Attachment(usize),
}

/// Guns you can bring from the start.
pub const STARTING_GUNS: [u8; 3] = [1, 2, 10];

/// What each career level unlocks (level 2 onwards).
pub const UNLOCKS: [Unlock; 26] = [
    Unlock::Attachment(0), // 2: Red Dot
    Unlock::Gun(3),        // 3: Kestrel SMG
    Unlock::Attachment(3), // 4: Suppressor
    Unlock::Gun(6),        // 5: Falcon AR
    Unlock::Attachment(5), // 6: Foregrip
    Unlock::Gun(9),        // 7: Bulldog Carbine
    Unlock::Attachment(1), // 8: Holo Sight
    Unlock::Gun(8),        // 9: Tempest Burst
    Unlock::Attachment(7), // 10: Extended Mag
    Unlock::Gun(16),       // 11: Arbiter DMR
    Unlock::Attachment(4), // 12: Compensator
    Unlock::Gun(13),       // 13: Goliath LMG
    Unlock::Attachment(6), // 14: Laser
    Unlock::Gun(15),       // 15: Longbow Sniper
    Unlock::Attachment(2), // 16: 3x Scope
    Unlock::Gun(18),       // 17: Judge Revolver
    Unlock::Gun(11),       // 18: Stormfront Auto
    Unlock::Gun(7),        // 19: Ranger Rifle
    Unlock::Gun(14),       // 20: Ripsaw LMG
    Unlock::Gun(17),       // 21: Sentinel Marksman
    Unlock::Gun(19),       // 22: Hammer .50
    Unlock::Gun(4),        // 23: Wasp PDW
    Unlock::Gun(5),        // 24: Mamba Machine Pistol
    Unlock::Gun(20),       // 25: Twin Fangs
    Unlock::Gun(12),       // 26: Double Barrel
    Unlock::Gun(0),        // 27: M9 Sidearm (for the collectors)
];

pub const MAX_CAREER_LEVEL: u32 = UNLOCKS.len() as u32 + 1;

/// XP needed to go from `level` to the next.
pub fn xp_to_next(level: u32) -> u32 {
    500 + 100 * level
}

/// (career level, XP into it, XP needed for the next).
pub fn career(xp: u32) -> (u32, u32, u32) {
    let mut level = 1;
    let mut left = xp;
    while level < MAX_CAREER_LEVEL && left >= xp_to_next(level) {
        left -= xp_to_next(level);
        level += 1;
    }
    (level, left, xp_to_next(level))
}

/// The level that unlocks something (1 = from the start).
pub fn unlock_level(u: Unlock) -> u32 {
    if let Unlock::Gun(g) = u {
        if STARTING_GUNS.contains(&g) {
            return 1;
        }
    }
    UNLOCKS
        .iter()
        .position(|x| *x == u)
        .map_or(u32::MAX, |i| i as u32 + 2)
}

pub fn gun_unlocked(level: u32, gun: u8) -> bool {
    unlock_level(Unlock::Gun(gun)) <= level
}

pub fn attachment_unlocked(level: u32, index: usize) -> bool {
    unlock_level(Unlock::Attachment(index)) <= level
}

/// Every gun that can be brought in, in unlock order.
pub fn loadout_guns() -> Vec<u8> {
    let mut v: Vec<u8> = STARTING_GUNS.to_vec();
    for u in UNLOCKS {
        if let Unlock::Gun(g) = u {
            v.push(g);
        }
    }
    v
}

/// What a finished match is worth.
/// Career XP for a run: rounds survived, kills, maps cleared and a bonus
/// for beating the final boss.
pub fn match_xp(rounds: u32, kills: u32, maps_cleared: u32, won: bool) -> u32 {
    rounds * 150 + kills * 3 + maps_cleared * 400 + if won { 1500 } else { 0 }
}

/// Is this a valid loadout (a gun that can be brought, attachments it takes)?
pub fn valid_loadout(gun: u8, attach: Attach) -> bool {
    (gun as usize) < crate::data::GUNS.len()
        && gun_def(gun).class != GunClass::Wonder
        && crate::data::attach_options_for(gun).fits(attach)
}

/// Turns attachments off that the player hasn't unlocked yet or that the
/// gun can't take.
pub fn allowed_attach(level: u32, gun: u8, attach: Attach) -> Attach {
    let opts = crate::data::attach_options_for(gun);
    let ok = |slot: crate::data::Slot, id: u8| {
        id == 0
            || ATTACHMENTS
                .iter()
                .position(|a| a.slot == slot && a.id == id)
                .is_some_and(|i| attachment_unlocked(level, i))
    };
    let optic =
        if ok(crate::data::Slot::Optic, attach.optic()) && opts.optics.contains(&attach.optic()) {
            attach.optic()
        } else {
            0
        };
    let muzzle = if ok(crate::data::Slot::Muzzle, attach.muzzle())
        && opts.muzzles.contains(&attach.muzzle())
    {
        attach.muzzle()
    } else {
        0
    };
    let under =
        if ok(crate::data::Slot::Under, attach.under()) && opts.unders.contains(&attach.under()) {
            attach.under()
        } else {
            0
        };
    let mag = attach.ext_mag() && opts.mag && ok(crate::data::Slot::Mag, 1);
    Attach::new(optic, muzzle, under, mag)
}

/// Puts the guns players brought onto the wall boards: the first two
/// players (by id) with a loadout take the two boards in order.
pub fn apply_loadouts(layout: &mut MapLayout, roster: &Roster) {
    let mut boards = layout.wall_buys.iter_mut();
    for p in roster.0.values() {
        let Some((gun, attach)) = p.loadout else {
            continue;
        };
        if !valid_loadout(gun, attach) {
            continue;
        }
        let Some(board) = boards.next() else { break };
        board.gun = gun;
        board.attach = attach;
        board.owner = Some(p.name.clone());
    }
}

pub struct ProgressionPlugin;

impl Plugin for ProgressionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, back_pay);
    }
}

/// Players from before career levels existed get XP for what they'd done.
fn back_pay(mut profile: ResMut<Profile>) {
    if profile.career_xp == 0 && (profile.best_round > 0 || profile.extractions > 0) {
        profile.career_xp = profile.best_round * 150 + profile.extractions * 600;
    }
}
