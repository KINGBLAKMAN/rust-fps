# PROGRESS.md

Where the project is up to. Update this at the end of every work session, and before compacting.

## Current version: v6.0 (2026-10-04)

Released: the Windows zip, the all-versions zip (v1 to v6), and the code pushed to GitHub.

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

## Known issues and loose ends

- The public IP lookup (api.ipify.org, then checkip.amazonaws.com, then icanhazip.com) could only be tested against a fake service. Check it on a real PC.
- Multiplayer has only been tested on one machine with several game instances. FX and pings are not resent if a packet is lost (purchases and abilities are).
- Spawning in the sandbox ignores walls: zombies always appear 9 m in front of you.
- Soft corner shading (SSAO) is untested on real GPUs.

## Ideas not done yet

- Reliable resend for FX and pings.
- More loadout slots (a second gun, a perk).
- An end-of-match screen that shows the unlock you just earned with a model preview.

## Next session

Nothing is in progress. Start new work from Jonah's next request.
