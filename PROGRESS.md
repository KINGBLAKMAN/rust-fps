# PROGRESS.md

Where the project is up to. Update this at the end of every work session, and before compacting.

## Current version: v11.0 (2026-10-05); v12.0 in progress, does not build yet

Released: the Windows zip, the all-versions zip (v1 to v11), and the code pushed to GitHub (branch `claude/project-thread-7oz85t`, on top of v8.0; main still has v7.0).

## Version history

- v1: single-player FPS.
- v2: multiplayer wave survival (host and join).
- v3:
  - Menus, characters, abilities and levels.
  - Mystery box, perks and extraction.
  - 3 maps and gun skins.
- v4:
  - Gun and hand models, and cast modes.
  - Prop-built maps.
  - Per-gun skins and crates.
- v5:
  - 5 characters on a jointed rig, emotes and pings.
  - Aim down sights and box attachments.
  - Door-locked areas, wall guns and night mode.
- v6:
  - Audio with volume groups, melee, hit markers, crawlers and death animations.
  - New zombie models, per-gun recoil and real attachment effects.
  - Valkyrie, plus ability animations and auras.
  - Walk-through buildings between areas.
  - Career unlocks and loadouts, an attachment guide and sandbox mode.
  - Click-to-reveal public IP and working graphics settings.
  - Code split into folders.
- v7:
  - Character levels (1-10, XP per character) unlocking 4 new abilities each at levels 2, 4, 6 (ultimate) and 8; pick 2 abilities and an ultimate on the Characters screen. 42 abilities in all, with new projectiles, gadgets and effects (`sim/powers.rs`, `fx/spells.rs`, `models/projectiles.rs`).
  - Iaido is a charged cast: hold to draw, release to send a flying crescent (12-40 m by charge).
  - Gun Crates (opening) and a Gun Skins tab (pick a gun, put any fitting skin on it). Any "all guns" finish can now go on a single gun.
  - Twin Fangs fire both SMGs every shot.
  - All three maps rebuilt from rooms, corridors and buildings with 7 door areas each (`maps/interiors.rs` floor-plan kit); `cargo test` checks every map connects.
  - Spawns face the most open direction.
- v8: art pass towards a painted look (numbers in `docs/art-direction.md`).
  - AgX tone mapping, softer grading, a warm sun with a cool fill, a gradient sky with a sun disc, and fog that matches the horizon.
  - A painted material on everything: brush grain, cool shadows and a soft rim light (`painted.rs`).
  - Realistic proportions for heroes and zombies, with tapered limbs and faces.
  - Pencil outlines on characters, guns and nearby props (`outline.rs`).
  - Painted ground markers for ability previews and enemy warnings (`markers.rs`). Brutes now wind up for 0.45 s before slamming.
  - A warm fading hit flash, camera shake, air particles per map, and an effects warm-up at match start (`feel.rs`).
  - New Graphics settings for outlines, camera shake and air particles.
  - `rust-fps lookdev` views for checking the art (`lookdev.rs`).
- v6.1: Ctrl+V pastes into the join address box (it used to type a "v", which made Windows report os error 11001), typed addresses are tidied (spaces, http://, commas), and clearer join errors.
- v8.1 (first step of the Game Overhaul list): power-ups drop half as often (3% a kill) and never within 25 s of the last drop; every ability cooldown is 30% longer (`COOLDOWN_SCALE` in `data.rs`).
- v9 (Game Overhaul part 2, the run):
  - A match is a run of 5 maps (`STAGES`, `ROUNDS_PER_STAGE` = 4 in `data.rs`): 4 rounds, then a boss; killing it opens a teleporter at the old extraction spot, and the whole team standing in it for 3 s moves everyone to the next map (`teleporter`, `next_stage` in `sim/mod.rs`). Maps go in order from the picked one; maps 4-5 flip day/night. Map 5's boss is the final boss; killing it wins the run (`MatchState::won`, was `extracted`).
  - Travel: `game.rs` watches `MatchState::stage` and passes through `AppState::Travel` for a frame, so the match is rebuilt on the new map on host and clients alike.
  - Bosses: `NetKind::Boss(0|1)`, a scaled, tinted Brute rig with a glow. Slam (`slam_spec`), fireball fans, summons at 2/3 and 1/3 health, final boss enrages under 30%. Immune to Insta-Kill, Nuke and crawling; 15% of stuns; knife does 1% instead of 20%. HP 35x (final 70x) a grunt, +75% per extra player. Boss bar on the HUD.
  - Doors are gone: every area is open, `DoorDef` is just a doorway cut in the walls; `strips.rs` only spawns wall guns. Zombies spawn anywhere.
  - Harder per map: zombie hits +15% per map, more Brutes and Shooters.
  - Level-up picks every 2 levels and after each boss; new stat upgrades (`data::Stat`: Vitality, Firepower, Focus, Swift, Sleight of Hand, 5 stacks each).
  - Career XP: 150/round, 3/kill, 400/map cleared, 1500 for a win. "Extractions" on the menu became "Runs won" (same save field).
  - Sandbox F1 panel can spawn a map boss or the final boss.
- v10 (Game Overhaul part 3, classes):
  - Each character is a class with 2 primaries and 2 secondaries (`Character::guns` in `data.rs`). You bring one of each (`PlayerInfo::class_guns`, saved per class in `Profile::class_guns`, attachments per gun in `Profile::gun_attach`) and start with both. The old single loadout gun, its wall board and `apply_loadouts` are gone.
  - Two weapon abilities per class on keys 3 and 4 (`WeaponAbility`, `GunBuff`, `PlayerAction::WeaponAbility`): a timed buff on your guns (elements, damage, fire rate, free ammo, stun, execute, blast, chain). Host checks `weapon_cd`; `resolve_shots` applies the buff.
  - Alternate fire on right mouse (`AltFire`, `alt_fire(gun)`): rifles fire an underbarrel grenade (8 s recharge, `grenade_cd`), shotguns a slug (one pellet worth the whole spread x0.9), SMGs and pistols a 5 or 3 shot burst. Snipers, DMRs, semi rifles and LMGs keep aim down sights. `Shot::alt` marks slug and grenade shots.
  - Weapon upgrades in level-up picks (`Upgrade::Weapon`, `gun_tiers`, up to Mk IV: +25% damage and magazine each).
  - The mystery box is the Armory: 750 for new random attachments on the gun in your hands, or a 5% wonder weapon (`roll_armory`). Wall boards are ammo caches (300, refill both guns).
  - Career unlocks cut to 16 levels: attachments and each class's second gun (`progression.rs`). Old career levels just mean more is unlocked.
  - Loadout screen rewritten: class guns in two columns with attachment pickers.
- v11 (Game Overhaul part 4, scaling ability upgrades):
  - `MAX_TIER` 3 to 5 (tier VI on the HUD); `Ability::cooldown` floors at 40% of the base.
  - Augments (`data::Augment`, `Upgrade::Augment`, `PlayerInfo::augments`, `MAX_AUGMENT` = 3) for the two abilities. `Ability::augment()` says which: Charges (dashes, leaps, Stim, Dome, Supply Drop) or Copies (everything else).
  - Charges: `PlayerInfo::charges`; `cooldowns[s]` is now the time to the next charge, refilled in `player_timers`. Clients check `charges > 0`.
  - Copies: the host calls `powers::cast` once per copy with the aim turned by `COPY_SPREAD` (0.2 rad) steps either side. Self-centred abilities just stack.
  - Zombie health +10% per map (`spawn_zombie`).

## Known issues and loose ends

- The public IP lookup (api.ipify.org, then checkip.amazonaws.com, then icanhazip.com) could only be tested against a fake service. Check it on a real PC.
- Multiplayer has only been tested on one machine running several copies of the game.
- Effects the host sends out (tracers, explosions, other players' pings) ride in snapshots and aren't resent if a packet is lost. Actions sent to the host (purchases, abilities, pings) are resent until confirmed.
- Sandbox spawns ignore walls: zombies always appear 9 m in front of you, even if that's inside a wall. With the indoor maps this happens more often.
- v7 abilities were only tested solo under lavapipe (2 fps). Their balance (damage, cooldowns) is a first pass and untested in co-op.
- The new maps haven't been played on a real GPU; map 2 has about 47 lights.
- Soft corner shading (SSAO) is untested on real GPUs.
- The v8 look was only seen under lavapipe, which bands colours. The painted shader, outlines and markers haven't been checked for speed on a real GPU.
- Camera shake and the hit flash were only checked in code; they can't be judged at 2 fps.
- v9 bosses were tested under lavapipe with test hooks (tiny boss health); their real health, damage and the summon counts are a first pass and need playing on a real PC, solo and in co-op.
- A full 5-map run hasn't been played start to finish; the stages were tested by jumping straight to boss rounds.
- Old "Extractions" from earlier versions count as "Runs won" on the main menu.
- v10 weapon abilities, alt fire and the Armory were checked solo under lavapipe (buff timer, grenade and its recharge, loadout screen). Balance (buff numbers, grenade damage, slug damage, Armory price) is a first pass. Not yet tested in co-op.
- Assault rifles lost aim down sights to the grenade; the Falcon AR still shows its built-in scope.
- v11 augments were tested with a hook giving full augments (4 fanned grenades, 4 dash charges). Copies of self-centred abilities (Frost Nova, Thunder Clap, Heal Pulse) simply stack their damage or healing; the aiming preview still shows one copy. Balance untested.

## Ideas not done yet

- Game Overhaul plan still to do: v12 character kit and model rework, v13 procedural maps plus 10 more map themes, v14 sound design and a menu rework (bigger text).

- Part 7 of the art brief: skin rarity looks (a different finish per rarity).
- Jonah likes big maps with many rooms to traverse and explore; keep new maps that way.
- Resend important effects (pings, power-up pickups) until clients confirm them.
- More loadout slots (a second gun, a perk).
- Show a 3D preview of a newly unlocked gun on the end screen (it only lists the name now).

## Next session

v12.0 (new classes) was stopped half way on 2026-10-05 and pushed as a WIP commit on `claude/project-thread-7oz85t`. It does NOT compile yet. When v12 is finished, fold the WIP commit into the single `v12.0: ...` commit (one commit per version).

Done in the WIP commit:
- `data.rs`: 6 new classes (Bulwark, Medic, Revenant, Demolisher, Chemist, Ranger; serde aliases map the old names so saves carry over), 30 new abilities (`Ability`, `ABILITY_DEFS`, 5 per class: two abilities and an ult at level 1, an ability at 3, an ult at 6), class guns and weapon abilities (Toxic Rounds replaces Incendiary Mag, Soul Siphon heals), augments per ability. Medic trim is green (no red cross anywhere).
- `sim/powers.rs`: all 30 abilities on the host, plus sticky bombs, heal/acid canisters, claymores, bear traps, the med drone and Army of the Dead warriors (`TurretBrain::wraith`). Damage-mask effect bits `powers::effect::{POISON, MARK, DRAIN}`.
- `sim/mod.rs`: poison and Hunter's Mark on zombies (`EnemyBrain::poison/marked`, `EnemyStatus::poisoned/marked`, net flags 64/128), drain healing and Chain Reaction kill blasts in `apply_damage`, zones with `heal`/`grow`, `Force::only` (Soul Chains), Soul Siphon lifesteal in `resolve_shots`.
- `main.rs`: `PlayerInfo` `chain`, `guard`/`guard_cut` (Fortress, Rally Cry), `overdrive` removed; NetKind Firebomb/Turret/Coil/Mine removed, `Wraith` added. `fx::Fx::ZoneEnd` added. `PROTOCOL_VERSION` 12.
- `abilities/mod.rs`: movement for Shield Charge, Wraith Step, Blast Jump, Grapple (`move_time`, `GRAPPLE_RANGE`).
- `progression.rs`: starting guns and unlocks for the new classes (17 career levels).
- `models/projectiles.rs` + `models/avatars.rs`: new gadget models (dart, sticky bomb, medkit, acid flask, bear trap, claymore), med drone, spectral warrior.
- `rig/heroes.rs`: six new hero models (built, never looked at on screen).
- `abilities/preview.rs`, `audio/`: previews and sounds were being rewritten (they compile; not checked).
- README class/ability/gun tables rewritten for v12.

Left to do for v12.0:
1. Fix the remaining compile errors: `fx/spells.rs` and `fx/mod.rs` (ability visuals and zone visuals half rewritten: `movers/sweeps/rumbles` systems, removed SpellAssets fields, the `falling()` call), `fx/auras.rs` (auras for guard, stim, chain instead of Overdrive/CombatStim), `viewmodel.rs` (old ability names, removed avatars gadget kits; held items should use `projectiles::missile_kit`, Revenant swings a scythe).
2. Check every ability has a visual per the spec: what each `Fx::Spell` means (pos/dir/size) is written in `cast()` in `sim/powers.rs`; zone kinds in `powers::zone`.
3. `cargo build` with no warnings, `cargo test`.
4. Test under Xvfb: lookdev lineup/side/heads for the six heroes, cast every ability (sandbox), check HUD text, Characters screen, loadout screen.
5. Balance pass on the new numbers (first guesses in `cast()`).
6. Bump Cargo.toml to 12.0.0, update PROGRESS.md history and known issues, Windows build and zips, push, reply to Jonah.

After v12: v13.0 procedural maps per run plus 10 new map themes; v14.0 sound design pass and a menu rework (bigger text, decorative but simple).
