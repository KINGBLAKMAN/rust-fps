# CLAUDE.md

Notes for Claude (or anyone) working on this repo. Read PROGRESS.md too: it says where the work is up to.

## What this is

Rust FPS: a co-op zombie wave-survival shooter in Rust with Bevy 0.16. Solo, or up to 8 players over UDP (the host runs the game). The README covers gameplay and controls.

## Build and run

- `cargo run` for a debug build (optimised enough to play). `cargo build --release` for a real build.
- Windows exe for Jonah (no MSVC on their PC): `cargo build --release --target x86_64-pc-windows-gnu`, then `x86_64-w64-mingw32-strip`.
- Command line: `rust-fps solo --start --map 0|1|2` skips the menus. Also `host [port]` and `join <address>`.
- `rust-fps lookdev --map N [--night] --view spawn|street|overhead|lineup|side|guns|heads|markers` holds the camera still with the HUD hidden. Use it for before and after screenshots of art changes; `docs/art-direction.md` has the look's numbers.
- Save data goes in `%APPDATA%\RustFPS` or `~/.config/rust-fps`. Set `RUST_FPS_DATA=<dir>` to use a throwaway folder when testing.

## Versions and releases

- Each version is one commit whose message starts with the version, e.g. `v6.0: ...`. No git tags.
- Big updates bump the major (v7.0); fixes bump the minor (v6.1). Update `version` in Cargo.toml. The game shows major.minor on the main menu and in the window title.
- When the network messages change, bump `PROTOCOL_VERSION` in `src/net.rs`.
- Every release: update README.md and PROGRESS.md, build the Windows zip (exe plus a CRLF README.txt), and add a folder to the all-versions zip.

## Code map

- `main.rs`: app setup, shared state (`Roster`, `MatchState`, `PlayerAction`) and system ordering. Systems run in `Phase` sets in this order: NetIn, Local (only in a match), Sim (only on the host, while the match runs), NetOut, Present.
- `data.rs`: all tables (guns, attachments, characters, abilities, skins, perks). Start here when balancing.
- `sim.rs`: the host-only simulation (rounds, zombie AI, damage, box, perks, sandbox). Clients never run it.
- `net.rs`: UDP and serde messages. The host is authoritative; clients send actions, and the host sends snapshots.
- `player.rs`, `weapons.rs`, `abilities.rs`, `viewmodel.rs`: the local player, guns, ability input and first-person animation.
- `maps/`: maps (`mod.rs`), props, door areas (`strips.rs`), walk-through buildings (`interiors.rs`) and pathfinding (`nav.rs`).
- `models/`: the shape kit (`kit.rs`) that everything is modelled with, gun models, skins, hands and other players.
- `rig/`: skeleton and animation for heroes and zombies. `fx/`: effects and auras. `audio/`: synthesised sounds. `ui/`: menus.
- `progression.rs`: career XP, unlocks and loadouts. `graphics.rs`: applies the graphics settings.

## Working with Jonah

- Jonah owns the game and plays it on Windows. Every build they get must carry a version number.
- Jonah sends feature lists as markdown files. Flesh the ideas out properly, but don't go overboard.
- Deliver the Windows zip, push to GitHub, and say plainly what changed and what couldn't be tested.

## Conventions

- Models are built in code from boxes, cylinders and spheres through `Kit`. There are no asset files, and sounds are synthesised too.
- Anything gameplay-related goes through the host: add a `PlayerAction` or `Fx` variant rather than changing state on the client.
- Keep comments short and plain. Match the style of the surrounding code.
- Testing-only environment hooks are marked `// TEST-HOOK` and must be removed before a release commit.

## Testing without a GPU (cloud)

- Use Xvfb on `:99` and lavapipe (`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`). It runs at about 2 fps, so wait about 20 seconds before taking a screenshot.
- Take screenshots with `import -window root`, and send input with `xdotool`. After clicking, run `xdotool windowfocus --sync` or key presses get lost.
- At 2 fps a click sometimes misses and the game stays on "Paused - click to play". If that happens, take the screenshot again.
