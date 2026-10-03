//! Static game data: guns, skins, characters, perks, power-ups, elements and
//! level-up upgrades. Everything here is plain tables so it's easy to tweak.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Guns
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GunClass {
    Pistol,
    Smg,
    Rifle,
    Shotgun,
    Lmg,
    Sniper,
    Wonder,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FireMode {
    Semi,
    Auto,
    Burst,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GunSpecial {
    None,
    /// Explodes on impact.
    Explosive { radius: f32 },
    /// Arcs to nearby enemies.
    Chain { jumps: u8 },
}

pub struct GunDef {
    pub name: &'static str,
    pub class: GunClass,
    pub mode: FireMode,
    pub damage: f32,
    pub rpm: f32,
    pub mag: u32,
    pub reserve: u32,
    pub reload: f32,
    pub pellets: u32,
    pub spread: f32,
    pub headshot: f32,
    pub rare: bool,
    pub special: GunSpecial,
}

const fn gun(
    name: &'static str,
    class: GunClass,
    mode: FireMode,
    damage: f32,
    rpm: f32,
    mag: u32,
    reserve: u32,
    reload: f32,
) -> GunDef {
    GunDef {
        name,
        class,
        mode,
        damage,
        rpm,
        mag,
        reserve,
        reload,
        pellets: 1,
        spread: 0.006,
        headshot: 2.0,
        rare: false,
        special: GunSpecial::None,
    }
}

const fn shotgun(
    name: &'static str,
    mode: FireMode,
    damage: f32,
    pellets: u32,
    rpm: f32,
    mag: u32,
    reserve: u32,
    reload: f32,
    spread: f32,
) -> GunDef {
    GunDef {
        pellets,
        spread,
        headshot: 1.5,
        ..gun(name, GunClass::Shotgun, mode, damage, rpm, mag, reserve, reload)
    }
}

use FireMode::*;
use GunClass::*;

/// Gun 0 is the starting pistol. Guns 1-20 come from the mystery box, and
/// 21-22 are the rare wonder weapons.
pub const GUNS: [GunDef; 23] = [
    gun("M9 Sidearm", Pistol, Semi, 34.0, 420.0, 12, 72, 1.3),
    gun("Viper .45", Pistol, Semi, 48.0, 380.0, 8, 64, 1.4),
    gun("Hornet MP", Smg, Auto, 24.0, 900.0, 32, 192, 1.8),
    gun("Kestrel SMG", Smg, Auto, 28.0, 760.0, 30, 180, 1.7),
    gun("Wasp PDW", Smg, Auto, 21.0, 1000.0, 50, 250, 2.2),
    gun("Mamba Machine Pistol", Pistol, Auto, 19.0, 1100.0, 20, 160, 1.3),
    gun("Falcon AR", Rifle, Auto, 36.0, 650.0, 30, 210, 2.0),
    gun("Ranger Rifle", Rifle, Auto, 40.0, 600.0, 30, 180, 2.2),
    gun("Tempest Burst", Rifle, Burst, 44.0, 820.0, 30, 180, 2.0),
    gun("Bulldog Carbine", Rifle, Auto, 32.0, 720.0, 35, 245, 1.9),
    shotgun("Breacher 12", Semi, 22.0, 8, 75.0, 6, 48, 2.6, 0.08),
    shotgun("Stormfront Auto", Auto, 16.0, 6, 300.0, 10, 80, 2.4, 0.09),
    shotgun("Double Barrel", Semi, 26.0, 10, 220.0, 2, 40, 1.8, 0.11),
    gun("Goliath LMG", Lmg, Auto, 38.0, 600.0, 100, 300, 4.0),
    gun("Ripsaw LMG", Lmg, Auto, 31.0, 850.0, 75, 300, 3.5),
    GunDef {
        headshot: 3.0,
        spread: 0.0,
        ..gun("Longbow Sniper", Sniper, Semi, 240.0, 50.0, 5, 40, 3.0)
    },
    GunDef {
        headshot: 2.5,
        spread: 0.002,
        ..gun("Arbiter DMR", Sniper, Semi, 95.0, 260.0, 15, 90, 2.4)
    },
    gun("Sentinel Marksman", Rifle, Semi, 75.0, 330.0, 20, 120, 2.2),
    gun("Judge Revolver", Pistol, Semi, 115.0, 160.0, 6, 54, 2.5),
    gun("Hammer .50", Pistol, Semi, 88.0, 220.0, 7, 49, 1.6),
    gun("Twin Fangs", Smg, Auto, 25.0, 1200.0, 40, 240, 2.5),
    GunDef {
        rare: true,
        special: GunSpecial::Explosive { radius: 3.5 },
        ..gun("Ray Blaster", Wonder, Semi, 260.0, 300.0, 20, 160, 2.0)
    },
    GunDef {
        rare: true,
        special: GunSpecial::Chain { jumps: 5 },
        headshot: 1.0,
        ..gun("Thunder Cannon", Wonder, Semi, 450.0, 120.0, 8, 48, 2.8)
    },
];

pub const STARTER_GUN: u8 = 0;
pub const BOX_COST: u32 = 750;

pub fn gun_def(id: u8) -> &'static GunDef {
    &GUNS[(id as usize).min(GUNS.len() - 1)]
}

/// Price of a gun bought off the wall (ammo for it costs half).
pub fn wall_cost(gun: u8) -> u32 {
    match gun_def(gun).class {
        GunClass::Pistol => 500,
        GunClass::Smg | GunClass::Shotgun => 1000,
        GunClass::Rifle => 1250,
        _ => 1500,
    }
}

/// Rolls a mystery box gun, never one the player already holds or one on
/// this map's walls.
pub fn roll_box_gun(rng: &mut impl rand::Rng, exclude: &[Option<u8>; 2], wall: &[u8]) -> u8 {
    loop {
        let id = if rng.gen_bool(0.06) {
            rng.gen_range(21..=22)
        } else {
            rng.gen_range(1..=20)
        };
        if !exclude.contains(&Some(id)) && !wall.contains(&id) {
            return id;
        }
    }
}

// ---------------------------------------------------------------------------
// Attachments (mystery box guns only)
// ---------------------------------------------------------------------------

/// A gun's attachments, packed in one byte: optic (bits 0-1), muzzle (2-3),
/// underbarrel (4-5), extended magazine (6). 0 in a slot means empty.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Attach(pub u8);

pub const OPTIC_NAMES: [&str; 4] = ["", "Red Dot", "Holo Sight", "3x Scope"];
pub const MUZZLE_NAMES: [&str; 3] = ["", "Suppressor", "Compensator"];
pub const UNDER_NAMES: [&str; 3] = ["", "Foregrip", "Laser"];

impl Attach {
    pub const NONE: Attach = Attach(0);

    pub fn optic(self) -> u8 {
        self.0 & 3
    }
    pub fn muzzle(self) -> u8 {
        (self.0 >> 2) & 3
    }
    pub fn under(self) -> u8 {
        (self.0 >> 4) & 3
    }
    pub fn ext_mag(self) -> bool {
        self.0 & 64 != 0
    }
    fn make(optic: u8, muzzle: u8, under: u8, mag: bool) -> Self {
        Attach(optic | (muzzle << 2) | (under << 4) | if mag { 64 } else { 0 })
    }

    /// Names of everything fitted, e.g. ["Red Dot", "Suppressor"].
    pub fn names(self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.optic() > 0 {
            v.push(OPTIC_NAMES[self.optic() as usize]);
        }
        if self.muzzle() > 0 {
            v.push(MUZZLE_NAMES[self.muzzle() as usize]);
        }
        if self.under() > 0 {
            v.push(UNDER_NAMES[self.under() as usize]);
        }
        if self.ext_mag() {
            v.push("Extended Mag");
        }
        v
    }

    /// How much recoil is left after the muzzle and grip (1 = none removed).
    pub fn recoil_scale(self) -> f32 {
        let m = match self.muzzle() {
            1 => 0.85,
            2 => 0.65,
            _ => 1.0,
        };
        let u = if self.under() == 1 { 0.75 } else { 1.0 };
        m * u
    }

    /// Hip-fire spread multiplier (the laser tightens it).
    pub fn hip_spread(self) -> f32 {
        if self.under() == 2 {
            0.6
        } else {
            1.0
        }
    }
}

/// Magazine size with attachments.
pub fn mag_size(gun: u8, attach: Attach) -> u32 {
    let m = gun_def(gun).mag;
    if attach.ext_mag() {
        (m as f32 * 1.5).round() as u32
    } else {
        m
    }
}

/// Which attachments a gun can take: (optics, muzzles, underbarrels, mag).
fn attach_options(gun: u8) -> (&'static [u8], &'static [u8], &'static [u8], bool) {
    let d = gun_def(gun);
    let (optics, muzzles, unders, mag) = attach_slots(gun, d.class);
    // Some models come with their own sight, suppressor or foregrip.
    let builtin_optic = matches!(gun, 3 | 4 | 5 | 6 | 8 | 9 | 14 | 15 | 16 | 17);
    let builtin_muzzle = matches!(gun, 3 | 16);
    let builtin_grip = matches!(gun, 3 | 5 | 8 | 9 | 11);
    (
        if builtin_optic { &[] } else { optics },
        if builtin_muzzle { &[] } else { muzzles },
        if builtin_grip { &[] } else { unders },
        mag,
    )
}

fn attach_slots(gun: u8, class: GunClass) -> (&'static [u8], &'static [u8], &'static [u8], bool) {
    match class {
        GunClass::Wonder => (&[], &[], &[], false),
        _ if gun == 12 || gun == 20 => (&[1], &[], &[2], false), // double barrel, dual SMGs
        GunClass::Pistol => (&[1], &[1, 2], &[2], gun != 18),
        GunClass::Smg => (&[1, 2], &[1, 2], &[1, 2], true),
        GunClass::Rifle | GunClass::Lmg => (&[1, 2, 3], &[1, 2], &[1, 2], true),
        GunClass::Shotgun => (&[1, 2], &[2], &[1, 2], true),
        // Snipers come with their own scope.
        GunClass::Sniper => (&[], &[1, 2], &[1, 2], true),
    }
}

/// Random attachments for a mystery box gun: about a quarter come bare,
/// half with some attachments and a quarter fully kitted out.
pub fn roll_attachments(gun: u8, rng: &mut impl rand::Rng) -> Attach {
    let (optics, muzzles, unders, mag) = attach_options(gun);
    let pick = |rng: &mut dyn rand::RngCore, list: &[u8]| -> u8 {
        if list.is_empty() {
            0
        } else {
            list[(rng.next_u32() as usize) % list.len()]
        }
    };
    let r: f32 = rng.gen_range(0.0..1.0);
    if r < 0.25 {
        return Attach::NONE;
    }
    let full = r >= 0.75;
    loop {
        let keep = |rng: &mut dyn rand::RngCore| full || rng.next_u32() % 100 < 45;
        let optic = if keep(rng) { pick(rng, optics) } else { 0 };
        let muzzle = if keep(rng) { pick(rng, muzzles) } else { 0 };
        let under = if keep(rng) { pick(rng, unders) } else { 0 };
        let ext = mag && keep(rng);
        let a = Attach::make(optic, muzzle, under, ext);
        // "Some" means at least one; guns that take nothing stay bare.
        if a != Attach::NONE || (optics.is_empty() && muzzles.is_empty() && unders.is_empty() && !mag) {
            return a;
        }
    }
}

// ---------------------------------------------------------------------------
// Skins
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rarity {
    Common,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    pub fn name(self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Rare => "Rare",
            Rarity::Epic => "Epic",
            Rarity::Legendary => "Legendary",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Rarity::Common => Color::srgb(0.75, 0.75, 0.75),
            Rarity::Rare => Color::srgb(0.3, 0.6, 1.0),
            Rarity::Epic => Color::srgb(0.75, 0.35, 1.0),
            Rarity::Legendary => Color::srgb(1.0, 0.75, 0.2),
        }
    }
}

/// Surface pattern of a gun-specific skin, painted over the base colour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pattern {
    Plain,
    Camo,
    Digital,
    Tiger,
    Zebra,
    Carbon,
    Hex,
    Scales,
    Damascus,
    Marble,
    Splatter,
    Woodgrain,
    /// Glowing patterns: the accent colour lights up.
    Circuit,
    Lava,
    Stars,
}

impl Pattern {
    /// Whether the accent colour of this pattern glows.
    pub fn glows(self) -> bool {
        matches!(self, Pattern::Circuit | Pattern::Lava | Pattern::Stars)
    }
}

pub struct SkinDef {
    pub name: &'static str,
    pub rarity: Rarity,
    pub color: [f32; 3],
    pub metallic: f32,
    pub glow: f32,
    /// `Some(gun)` for a skin made for one gun; `None` fits every gun.
    pub gun: Option<u8>,
    pub pattern: Pattern,
    /// Second colour of the pattern.
    pub accent: [f32; 3],
}

const fn skin(name: &'static str, rarity: Rarity, color: [f32; 3], metallic: f32, glow: f32) -> SkinDef {
    SkinDef {
        name,
        rarity,
        color,
        metallic,
        glow,
        gun: None,
        pattern: Pattern::Plain,
        accent: color,
    }
}

const fn gun_skin(
    name: &'static str,
    gun: u8,
    rarity: Rarity,
    pattern: Pattern,
    color: [f32; 3],
    accent: [f32; 3],
    metallic: f32,
    glow: f32,
) -> SkinDef {
    SkinDef {
        name,
        rarity,
        color,
        metallic,
        glow,
        gun: Some(gun),
        pattern,
        accent,
    }
}

use Pattern::*;
use Rarity::*;

/// Skins 0-11 fit every gun; 12 onwards are made for one gun each.
pub const SKINS: [SkinDef; 35] = [
    skin("Factory", Common, [0.12, 0.12, 0.14], 0.6, 0.0),
    skin("Desert", Common, [0.76, 0.64, 0.42], 0.1, 0.0),
    skin("Forest", Common, [0.22, 0.36, 0.2], 0.1, 0.0),
    skin("Arctic", Common, [0.88, 0.9, 0.95], 0.1, 0.0),
    skin("Crimson", Rare, [0.65, 0.05, 0.08], 0.4, 0.0),
    skin("Cobalt", Rare, [0.1, 0.25, 0.75], 0.5, 0.0),
    skin("Hazard", Rare, [0.95, 0.75, 0.05], 0.2, 0.0),
    skin("Toxic", Epic, [0.2, 0.9, 0.2], 0.0, 2.0),
    skin("Chrome", Epic, [0.9, 0.9, 0.95], 1.0, 0.0),
    skin("Neon", Epic, [1.0, 0.2, 0.7], 0.0, 2.5),
    skin("Gold", Legendary, [1.0, 0.75, 0.2], 1.0, 0.3),
    skin("Galaxy", Legendary, [0.45, 0.15, 0.9], 0.3, 3.0),
    // 12: Field crate
    gun_skin("Woodland", 0, Common, Camo, [0.3, 0.36, 0.2], [0.16, 0.13, 0.08], 0.1, 0.0),
    gun_skin("Urban Pixel", 2, Common, Digital, [0.45, 0.47, 0.5], [0.18, 0.19, 0.22], 0.1, 0.0),
    gun_skin("Jungle Tiger", 6, Rare, Tiger, [0.85, 0.5, 0.12], [0.08, 0.07, 0.05], 0.2, 0.0),
    gun_skin("Rust Belt", 10, Common, Splatter, [0.45, 0.3, 0.2], [0.62, 0.3, 0.1], 0.5, 0.0),
    gun_skin("Sandstorm", 13, Rare, Digital, [0.78, 0.66, 0.45], [0.5, 0.4, 0.26], 0.1, 0.0),
    // 17: Street crate
    gun_skin("Carbon Fibre", 1, Rare, Carbon, [0.16, 0.16, 0.18], [0.04, 0.04, 0.05], 0.5, 0.0),
    gun_skin("Zebra", 3, Common, Zebra, [0.92, 0.92, 0.9], [0.06, 0.06, 0.06], 0.1, 0.0),
    gun_skin("Snowdrift", 7, Common, Camo, [0.9, 0.92, 0.96], [0.55, 0.6, 0.66], 0.1, 0.0),
    gun_skin("Graffiti", 11, Rare, Splatter, [0.15, 0.15, 0.2], [1.0, 0.3, 0.6], 0.2, 0.0),
    gun_skin("White Marble", 17, Epic, Marble, [0.95, 0.94, 0.92], [0.35, 0.33, 0.35], 0.3, 0.0),
    // 22: Forge crate
    gun_skin("Hex Plate", 4, Rare, Hex, [0.3, 0.33, 0.38], [0.1, 0.11, 0.13], 0.8, 0.0),
    gun_skin("Python", 5, Epic, Scales, [0.4, 0.6, 0.2], [0.12, 0.18, 0.06], 0.3, 0.0),
    gun_skin("Damascus", 8, Epic, Damascus, [0.6, 0.62, 0.66], [0.22, 0.23, 0.26], 1.0, 0.0),
    gun_skin("Bengal", 9, Common, Tiger, [0.95, 0.6, 0.2], [0.1, 0.06, 0.03], 0.1, 0.0),
    gun_skin("Walnut Inlay", 12, Rare, Woodgrain, [0.45, 0.27, 0.13], [0.25, 0.13, 0.05], 0.1, 0.0),
    // 27: Inferno crate (premium)
    gun_skin("Molten Core", 14, Legendary, Lava, [0.12, 0.08, 0.07], [1.0, 0.4, 0.05], 0.2, 1.6),
    gun_skin("Dragonscale", 19, Epic, Scales, [0.6, 0.08, 0.06], [1.0, 0.72, 0.2], 0.8, 0.0),
    gun_skin("Hellfire", 18, Epic, Lava, [0.2, 0.05, 0.04], [1.0, 0.2, 0.05], 0.4, 1.3),
    gun_skin("Stormcaller", 22, Legendary, Circuit, [0.1, 0.12, 0.2], [0.3, 0.8, 1.0], 0.6, 2.0),
    // 31: Cosmos crate (premium)
    gun_skin("Nebula", 15, Legendary, Stars, [0.2, 0.06, 0.35], [0.9, 0.85, 1.0], 0.3, 3.0),
    gun_skin("Mainframe", 16, Epic, Circuit, [0.04, 0.12, 0.06], [0.2, 1.0, 0.4], 0.4, 1.8),
    gun_skin("Void Hex", 20, Epic, Hex, [0.08, 0.04, 0.14], [0.7, 0.2, 1.0], 0.6, 0.0),
    gun_skin("Supernova", 21, Legendary, Stars, [0.05, 0.08, 0.25], [1.0, 0.9, 0.5], 0.3, 3.5),
];

pub fn skin_def(id: u8) -> &'static SkinDef {
    &SKINS[(id as usize).min(SKINS.len() - 1)]
}

/// The material for a skin, without its pattern texture.
pub fn skin_material(id: u8) -> StandardMaterial {
    let s = skin_def(id);
    let base = Color::srgb(s.color[0], s.color[1], s.color[2]);
    let glow_color = if s.pattern == Plain { base } else { Color::BLACK };
    StandardMaterial {
        base_color: base,
        metallic: s.metallic,
        perceptual_roughness: if s.metallic > 0.8 { 0.15 } else { 0.45 },
        emissive: LinearRgba::from(glow_color) * s.glow,
        ..default()
    }
}

/// Which skin a gun shows: its own equipped skin if it has one, else the
/// finish that fits every gun. `gun_skins[gun]` is 255 for none.
pub fn skin_for(default: u8, gun_skins: &[u8], gun: u8) -> u8 {
    match gun_skins.get(gun as usize) {
        Some(&s) if (s as usize) < SKINS.len() && SKINS[s as usize].gun == Some(gun) => s,
        _ => default,
    }
}

/// A gacha crate: what's inside and what it costs to open.
pub struct CrateDef {
    pub name: &'static str,
    pub blurb: &'static str,
    /// Premium crates cost a premium spin instead of a regular one.
    pub premium: bool,
    pub skins: &'static [u8],
}

pub const CRATES: [CrateDef; 5] = [
    CrateDef {
        name: "Field Crate",
        blurb: "Camo and outdoor finishes.",
        premium: false,
        skins: &[1, 2, 4, 7, 12, 13, 14, 15, 16],
    },
    CrateDef {
        name: "Street Crate",
        blurb: "City colours, carbon and marble.",
        premium: false,
        skins: &[3, 5, 6, 9, 17, 18, 19, 20, 21],
    },
    CrateDef {
        name: "Forge Crate",
        blurb: "Metalwork, scales and precious finishes.",
        premium: false,
        skins: &[8, 10, 11, 22, 23, 24, 25, 26],
    },
    CrateDef {
        name: "Inferno Crate",
        blurb: "PREMIUM: molten, burning and electric skins. Epic or better.",
        premium: true,
        skins: &[27, 28, 29, 30],
    },
    CrateDef {
        name: "Cosmos Crate",
        blurb: "PREMIUM: starfields, circuits and void. Epic or better.",
        premium: true,
        skins: &[31, 32, 33, 34],
    },
];

/// Regular spins it takes to make one premium spin.
pub const PREMIUM_TRADE_COST: u32 = 5;
/// Every this many rounds survived (added up over all matches) earns a
/// quarter of a premium spin.
pub const ROUNDS_PER_PREMIUM_QUARTER: u32 = 20;

/// Base odds of each rarity, in percent.
pub fn rarity_odds(r: Rarity) -> f32 {
    match r {
        Common => 55.0,
        Rare => 30.0,
        Epic => 12.0,
        Legendary => 3.0,
    }
}

/// The odds of each rarity in a crate: the base odds, shared out over the
/// rarities the crate actually holds.
pub fn crate_odds(crate_id: usize) -> Vec<(Rarity, f32)> {
    let c = &CRATES[crate_id.min(CRATES.len() - 1)];
    let mut held: Vec<Rarity> = Vec::new();
    for &s in c.skins {
        let r = SKINS[s as usize].rarity;
        if !held.contains(&r) {
            held.push(r);
        }
    }
    let total: f32 = held.iter().map(|r| rarity_odds(*r)).sum();
    [Common, Rare, Epic, Legendary]
        .into_iter()
        .filter(|r| held.contains(r))
        .map(|r| (r, rarity_odds(r) / total * 100.0))
        .collect()
}

/// One pull from a crate.
pub fn roll_crate(crate_id: usize, rng: &mut impl rand::Rng) -> u8 {
    let c = &CRATES[crate_id.min(CRATES.len() - 1)];
    let odds = crate_odds(crate_id);
    let mut r: f32 = rng.gen_range(0.0..100.0);
    let mut rarity = odds[0].0;
    for (rar, pct) in odds {
        rarity = rar;
        if r < pct {
            break;
        }
        r -= pct;
    }
    let pool: Vec<u8> = c.skins.iter().copied().filter(|i| SKINS[*i as usize].rarity == rarity).collect();
    pool[rng.gen_range(0..pool.len())]
}

/// Gacha spins for reaching a round: round 5 gives 1, round 10 gives 2 more,
/// round 15 gives 3 more, and so on. Nothing below round 5, so restarting
/// early rounds never pays. Extracting gives the same as dying on that round.
pub fn spins_for_round(round: u32) -> u32 {
    let milestones = round / 5;
    milestones * (milestones + 1) / 2
}

// ---------------------------------------------------------------------------
// Characters and abilities
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum Character {
    #[default]
    Striker,
    Warden,
    Ronin,
    Tinker,
    Blaze,
}

impl Character {
    pub const ALL: [Character; 5] = [
        Character::Striker,
        Character::Warden,
        Character::Ronin,
        Character::Tinker,
        Character::Blaze,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Character::Striker => "Striker",
            Character::Warden => "Warden",
            Character::Ronin => "Ronin",
            Character::Tinker => "Tinker",
            Character::Blaze => "Blaze",
        }
    }

    pub fn tagline(self) -> &'static str {
        match self {
            Character::Striker => "Aggressive assault specialist",
            Character::Warden => "Support and crowd control",
            Character::Ronin => "Blade master who strikes up close",
            Character::Tinker => "Engineer with turrets and supplies",
            Character::Blaze => "Pyro who sets the horde ablaze",
        }
    }

    /// (name, description) for ability 1, ability 2 and the ultimate.
    pub fn abilities(self) -> [(&'static str, &'static str); 3] {
        match self {
            Character::Striker => [
                ("Dash", "Burst forward at high speed."),
                ("Frag Grenade", "Throw a grenade that explodes in an area."),
                ("Overdrive", "ULT: bigger damage, faster fire, no ammo use."),
            ],
            Character::Warden => [
                ("Heal Pulse", "Heal yourself and nearby teammates."),
                ("Frost Nova", "Damage and slow every enemy around you."),
                ("Orbital Strike", "ULT: call a huge blast where you aim."),
            ],
            Character::Ronin => [
                ("Iaido Slash", "Draw and cut everything in front of you."),
                ("Shadow Step", "Blink forward, slicing enemies you pass through."),
                ("Blade Storm", "ULT: whirling blades shred enemies around you."),
            ],
            Character::Tinker => [
                ("Sentry Turret", "Deploy a turret that shoots nearby enemies."),
                ("Supply Drop", "Refill ammo and patch up teammates nearby."),
                ("Tesla Coil", "ULT: plant a coil that shocks everything near it."),
            ],
            Character::Blaze => [
                ("Firebomb", "Throw a bomb that leaves a pool of fire."),
                ("Flame Wave", "Blast a cone of fire that sets enemies alight."),
                ("Inferno", "ULT: wreathe yourself in a ring of fire."),
            ],
        }
    }

    pub fn suit_color(self) -> Color {
        match self {
            Character::Striker => Color::srgb(0.15, 0.35, 0.85),
            Character::Warden => Color::srgb(0.2, 0.65, 0.35),
            Character::Ronin => Color::srgb(0.6, 0.08, 0.08),
            Character::Tinker => Color::srgb(0.95, 0.75, 0.12),
            Character::Blaze => Color::srgb(0.25, 0.22, 0.2),
        }
    }

    pub fn trim_color(self) -> Color {
        match self {
            Character::Striker => Color::srgb(0.95, 0.55, 0.1),
            Character::Warden => Color::srgb(0.92, 0.92, 0.95),
            Character::Ronin => Color::srgb(0.9, 0.7, 0.25),
            Character::Tinker => Color::srgb(0.3, 0.9, 1.0),
            Character::Blaze => Color::srgb(1.0, 0.45, 0.08),
        }
    }
}

/// Ability cooldown in seconds by slot (0 or 1) and upgrade tier (0-3).
pub fn ability_cooldown(c: Character, slot: usize, tier: u8) -> f32 {
    let t = tier as f32;
    match (c, slot) {
        (Character::Striker, 0) => 6.0 - t,
        (Character::Striker, _) => 12.0 - 2.0 * t,
        (Character::Warden, 0) => 14.0 - 2.0 * t,
        (Character::Warden, _) => 12.0 - 2.0 * t,
        (Character::Ronin, 0) => 5.0 - t,
        (Character::Ronin, _) => 9.0 - 1.5 * t,
        (Character::Tinker, 0) => 20.0 - 3.0 * t,
        (Character::Tinker, _) => 16.0 - 2.0 * t,
        (Character::Blaze, 0) => 10.0 - 2.0 * t,
        (Character::Blaze, _) => 7.0 - t,
    }
}

pub const MAX_TIER: u8 = 3;

/// Abilities you hold in your hand and throw (with an arc preview).
pub fn is_thrown(c: Character, slot: u8) -> bool {
    matches!((c, slot), (Character::Striker, 1) | (Character::Blaze, 0))
}

/// Abilities that move you rather than use your hand.
pub fn is_dash(c: Character, slot: u8) -> bool {
    matches!((c, slot), (Character::Striker, 0) | (Character::Ronin, 1))
}

// ---------------------------------------------------------------------------
// Perks, power-ups, elements, upgrades
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Perk {
    QuickHands,
    Stamina,
    BoomShot,
    Juggernaut,
    RapidFire,
}

impl Perk {
    pub const ALL: [Perk; 5] = [
        Perk::QuickHands,
        Perk::Stamina,
        Perk::BoomShot,
        Perk::Juggernaut,
        Perk::RapidFire,
    ];

    pub fn bit(self) -> u8 {
        1 << (self as u8)
    }

    pub fn name(self) -> &'static str {
        match self {
            Perk::QuickHands => "Quick Hands",
            Perk::Stamina => "Stamina Rush",
            Perk::BoomShot => "Boom Shot",
            Perk::Juggernaut => "Juggernaut",
            Perk::RapidFire => "Rapid Fire",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Perk::QuickHands => "Reload twice as fast",
            Perk::Stamina => "Sprint and slide 30% faster",
            Perk::BoomShot => "Headshots can explode",
            Perk::Juggernaut => "Max health 200",
            Perk::RapidFire => "Fire 33% faster",
        }
    }

    pub fn cost(self) -> u32 {
        match self {
            Perk::QuickHands => 3000,
            Perk::Stamina => 2000,
            Perk::BoomShot => 3500,
            Perk::Juggernaut => 2500,
            Perk::RapidFire => 2000,
        }
    }

    pub fn color(self) -> Color {
        match self {
            Perk::QuickHands => Color::srgb(0.2, 0.9, 0.4),
            Perk::Stamina => Color::srgb(1.0, 0.85, 0.2),
            Perk::BoomShot => Color::srgb(1.0, 0.4, 0.1),
            Perk::Juggernaut => Color::srgb(0.9, 0.15, 0.15),
            Perk::RapidFire => Color::srgb(0.3, 0.6, 1.0),
        }
    }
}

pub fn has_perk(perks: u8, perk: Perk) -> bool {
    perks & perk.bit() != 0
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PowerUp {
    Nuke,
    InstaKill,
    DoublePoints,
    MaxAmmo,
}

impl PowerUp {
    pub const ALL: [PowerUp; 4] = [
        PowerUp::Nuke,
        PowerUp::InstaKill,
        PowerUp::DoublePoints,
        PowerUp::MaxAmmo,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PowerUp::Nuke => "NUKE",
            PowerUp::InstaKill => "INSTA-KILL",
            PowerUp::DoublePoints => "DOUBLE POINTS",
            PowerUp::MaxAmmo => "MAX AMMO",
        }
    }

    pub fn color(self) -> Color {
        match self {
            PowerUp::Nuke => Color::srgb(1.0, 0.45, 0.1),
            PowerUp::InstaKill => Color::srgb(0.9, 0.1, 0.1),
            PowerUp::DoublePoints => Color::srgb(0.2, 1.0, 0.3),
            PowerUp::MaxAmmo => Color::srgb(0.3, 0.6, 1.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Element {
    Fire,
    Ice,
    Shock,
}

impl Element {
    pub const ALL: [Element; 3] = [Element::Fire, Element::Ice, Element::Shock];

    pub fn bit(self) -> u8 {
        1 << (self as u8)
    }

    pub fn name(self) -> &'static str {
        match self {
            Element::Fire => "Fire",
            Element::Ice => "Ice",
            Element::Shock => "Shock",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Element::Fire => Color::srgb(1.0, 0.45, 0.1),
            Element::Ice => Color::srgb(0.5, 0.85, 1.0),
            Element::Shock => Color::srgb(0.85, 0.75, 1.0),
        }
    }

    pub fn effect(self) -> &'static str {
        match self {
            Element::Fire => "burns enemies over time",
            Element::Ice => "slows enemies down",
            Element::Shock => "arcs to nearby enemies",
        }
    }
}

pub fn elements_in(mask: u8) -> impl Iterator<Item = Element> {
    Element::ALL.into_iter().filter(move |e| mask & e.bit() != 0)
}

/// A level-up reward the player can pick.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Upgrade {
    /// Upgrade ability slot 0, 1 or 2 (ultimate).
    Ability(u8),
    GunElement(u8),
    AbilityElement(u8),
}

impl Upgrade {
    pub fn label(self, c: Character, tiers: [u8; 3]) -> String {
        match self {
            Upgrade::Ability(slot) => {
                let name = c.abilities()[slot as usize].0;
                format!("{name} tier {}", tiers[slot as usize] + 2)
            }
            Upgrade::GunElement(e) => {
                let e = Element::ALL[e as usize];
                format!("{} rounds: guns {}", e.name(), e.effect())
            }
            Upgrade::AbilityElement(e) => {
                let e = Element::ALL[e as usize];
                format!("{} abilities: abilities {}", e.name(), e.effect())
            }
        }
    }
}

pub const MAX_LEVEL: u32 = 50;

/// XP needed to go from `level` to `level + 1`.
pub fn xp_to_next(level: u32) -> u32 {
    100 + 30 * level.saturating_sub(1)
}
