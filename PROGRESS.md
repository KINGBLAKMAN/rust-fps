# PROGRESS.md

Where the project is up to. Update this at the end of every work session, and before compacting.

## Current version: v8.0 (2026-10-05)

Released: the Windows zip, the all-versions zip (v1 to v8), and the code pushed to GitHub (branch `claude/v8-art-pass-ymt4mt`).

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

## Ideas not done yet

- Part 7 of the art brief: skin rarity looks (a different finish per rarity).
- Jonah likes big maps with many rooms to traverse and explore; keep new maps that way.
- Resend important effects (pings, power-up pickups) until clients confirm them.
- More loadout slots (a second gun, a perk).
- Show a 3D preview of a newly unlocked gun on the end screen (it only lists the name now).

## Next session

Nothing is in progress. Start new work from Jonah's next request.
