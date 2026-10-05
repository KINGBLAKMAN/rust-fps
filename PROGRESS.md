# PROGRESS.md

Where the project is up to. Update this at the end of every work session, and before compacting.

## Current version: v9.0 (2026-10-05)

Released: the Windows zip, the all-versions zip (v1 to v9), and the code pushed to GitHub (branch `claude/project-thread-7oz85t`, on top of v8.0; main still has v7.0).

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

## Ideas not done yet

- Part 7 of the art brief: skin rarity looks (a different finish per rarity).
- Jonah likes big maps with many rooms to traverse and explore; keep new maps that way.
- Resend important effects (pings, power-up pickups) until clients confirm them.
- More loadout slots (a second gun, a perk).
- Show a 3D preview of a newly unlocked gun on the end screen (it only lists the name now).

## Next session

Jonah's "Game Overhaul" list is split into versions (agreed plan, 2026-10-05):

- v8.1: fewer power-ups, slower cooldowns (done).
- v9.0: the run: no COD doors, survive rounds, level up and pick upgrades, map boss, teleporter to the next map, 5 maps then a final boss, harder each map (done).
- v10.0: classes (the 6 characters): 2 primaries and 2 secondaries each, 2 weapon abilities, 2 class abilities, weapon upgrades; alt fire on some guns instead of ADS.
- v11.0: ability upgrades that scale (more daggers in a fan, two fireballs, extra dashes, more grenades), tuned over a 5-map run.
- v12.0: character rework: detailed models, new ability visuals and animations.
- v13.0: procedural maps per run, plus 10 new map themes.
- v14.0: sound design pass and a menu rework (bigger text, decorative but simple).

Start v10.0 (classes and alt fire) next.
