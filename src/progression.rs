//! Career progress kept between matches. Every match earns career XP; each
//! career level unlocks an attachment or a class's second gun choice for
//! your loadout.

use bevy::prelude::*;

use crate::config::Profile;
use crate::data::{Attach, Character, ATTACHMENTS};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unlock {
    Gun(u8),
    /// An entry in `ATTACHMENTS`.
    Attachment(usize),
}

/// Guns you can bring from the start: every class's first choices.
pub const STARTING_GUNS: [u8; 11] = [10, 3, 20, 13, 6, 16, 1, 0, 19, 4, 2];

/// What each career level unlocks (level 2 onwards): attachments, and the
/// second gun choice of each class.
pub const UNLOCKS: [Unlock; 16] = [
    Unlock::Attachment(0), // 2: Red Dot
    Unlock::Gun(11),       // 3: Stormfront Auto (Bulwark)
    Unlock::Attachment(3), // 4: Suppressor
    Unlock::Gun(8),        // 5: Tempest Burst (Medic)
    Unlock::Attachment(5), // 6: Foregrip
    Unlock::Gun(7),        // 7: Ranger Rifle (Revenant)
    Unlock::Attachment(1), // 8: Holo Sight
    Unlock::Gun(14),       // 9: Ripsaw LMG (Demolisher)
    Unlock::Attachment(7), // 10: Extended Mag
    Unlock::Gun(12),       // 11: Double Barrel (Chemist)
    Unlock::Attachment(4), // 12: Compensator
    Unlock::Gun(15),       // 13: Longbow Sniper (Ranger)
    Unlock::Attachment(6), // 14: Laser
    Unlock::Gun(18),       // 15: Judge Revolver (Bulwark, Chemist)
    Unlock::Attachment(2), // 16: 3x Scope
    Unlock::Gun(5),        // 17: Mamba Machine Pistol (Ranger)
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

/// What a finished match is worth.
/// Career XP for a run: rounds survived, kills, maps cleared and a bonus
/// for beating the final boss.
pub fn match_xp(rounds: u32, kills: u32, maps_cleared: u32, won: bool) -> u32 {
    rounds * 150 + kills * 3 + maps_cleared * 400 + if won { 1500 } else { 0 }
}

/// Are these guns (primary, secondary) ones the class can bring, with
/// attachments they take?
pub fn valid_class_guns(c: Character, guns: [(u8, Attach); 2]) -> bool {
    guns.iter().enumerate().all(|(slot, (g, a))| {
        c.has_gun(slot, *g) && crate::data::attach_options_for(*g).fits(*a)
    })
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
