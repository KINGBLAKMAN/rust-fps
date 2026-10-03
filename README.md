# Rust FPS

A co-op zombie-style wave-survival shooter written in Rust with [Bevy](https://bevyengine.org) 0.16. Play solo or with up to 8 friends.

![screenshot](screenshot.png)

## Build

1. Install Rust: https://rustup.rs
2. In this folder run `cargo build --release`. The first build compiles Bevy and takes a few minutes; later builds are fast.

**Windows:** the Rust installer asks for the Visual Studio C++ build tools; install them. (Or skip building and use the ready-made `RustFPS-windows.zip`.)

**Linux:** install the system libraries Bevy needs first, e.g. on Ubuntu/Debian:

```sh
sudo apt install g++ pkg-config libx11-dev libasound2-dev libudev-dev libxkbcommon-x11-0 libwayland-dev libxkbcommon-dev
```

## Play

Run the game (`cargo run --release`, or double-click `rust-fps.exe`) and use the menus:

- **Play Solo**, **Host a Party** or **Join a Party** (type the host's address).
- **Characters**: pick Striker or Warden.
- **Gun Skins**: spend gacha spins and equip skins.
- **Settings**: field of view, mouse sensitivity, volume, windowed / borderless / fullscreen, resolution and key remapping.

Before a match you're on the party screen: pick your character, the host picks the map, friends press Ready and the host presses Start. Friends can also join a match that's already running.

**Playing over the internet:** on the same Wi-Fi it just works. Otherwise the host forwards UDP port 7777 on their router (and shares their public IP), or everyone uses a virtual LAN like Tailscale or ZeroTier. Everyone must run the same version. On Windows, allow the game through the firewall when asked.

Command-line shortcuts still work: `rust-fps solo`, `rust-fps host [port]`, `rust-fps join <address>`, plus `--name <name>`, `--map <0-2>` and `--start` (skip the party screen).

## Controls (default, all remappable)

| Input | Action |
|---|---|
| Mouse | Look / shoot (hold for automatic guns) |
| W A S D | Move |
| Shift | Sprint (you can shoot while sprinting) |
| Space | Jump. Hold it to bunny hop: each hop right as you land adds speed |
| C | Crouch. Press while running to slide; hold it as you land from a jump to chain jump-slides |
| R | Reload |
| F | Buy / use (mystery box, perk machines, take a gun from the box) |
| Q / E | Abilities 1 and 2 |
| X | Ultimate |
| V, 1 / 2, mouse wheel | Swap between your two guns |
| B | Pick a level-up upgrade |
| Tab | Scoreboard |
| Esc | Menu (pauses solo games) |

## Gameplay

- **Rounds:** zombies come in rounds that get bigger and tougher. Grunts rush you, Shooters throw fireballs from a distance (round 3+), Brutes are big and tanky (round 6+). Downed players get back up at the start of the next round; if the whole team is down, it's game over.
- **Points:** 10 per hit, 60 per kill (Brutes 120), +40 for headshots. Spend them on the mystery box and perks.
- **Two guns:** you start with the M9 Sidearm and can carry two guns. Spare ammo is topped up at the start of each round.
- **Mystery box:** 750 points per spin, always. It gives one of 20 guns, or rarely one of 2 wonder weapons (Ray Blaster: explosive shots; Thunder Cannon: lightning that chains between enemies). The box sits at one of 5 spots and every few spins it may fly away to another (you get your points back).
- **Perk machines:** Quick Hands (3000, reload twice as fast), Stamina Rush (2000, faster sprint and slides), Boom Shot (3500, headshots can explode), Juggernaut (2500, 200 max health), Rapid Fire (2000, shoot 33% faster). You lose your perks when you go down.
- **Power-ups** drop from zombies: Nuke (kills everything, +400 points each), Insta-Kill, Double Points (30 seconds each) and Max Ammo.
- **Characters:** Striker has Dash, Frag Grenade and the Overdrive ultimate (more damage, faster fire, no ammo use). Warden has Heal Pulse (heals you and nearby teammates), Frost Nova (damages and slows enemies around you) and the Orbital Strike ultimate. Ultimates charge over time and with kills.
- **Levels:** kills give XP. Every level adds 3% damage, and every 5 levels you pick an upgrade (press B): a stronger ability tier, or an element. At levels 5, 10, 15, 20 and 25 one of the choices is always an element. Elements: Fire (burns over time), Ice (slows) and Shock (arcs to nearby enemies), for your guns or your abilities.
- **Extraction:** after clearing round 15, and every 10 rounds after that (25, 35...), a green extraction beam opens for 45 seconds. Get every living teammate into it for 5 seconds to extract, or ignore it and keep fighting.
- **Maps:** Shipping Yard, Central Park and The Neighborhood.

## Gacha spins and skins

Spins are earned by how far you get, so restarting round 1 over and over earns nothing:

| Rounds survived | Spins |
|---|---|
| under 5 | 0 |
| 5 | 1 |
| 10 | 3 |
| 15 | 6 |
| 20 | 10 |
| 25 | 15 |

Extracting doubles them. There are 12 gun skins: Common 55%, Rare 30%, Epic 12%, Legendary 3%. Every 3 duplicate pulls give a free spin. Your profile and settings are saved in `%APPDATA%\RustFPS` on Windows (`~/.config/rust-fps` on Linux).

## How the multiplayer works

The host runs the game: rounds, zombies, damage, points, the box and perks. Each player moves their own character on their own machine (so movement never feels laggy) and sends their position, shots and actions to the host about 60 times a second. Purchases and abilities are numbered and resent until the host confirms them, so a lost packet never loses a purchase. The host sends everyone a snapshot of the party, the match and every zombie 30 times a second over UDP.

## Code layout

| File | What's in it |
|---|---|
| `src/main.rs` | App setup, shared state (players, match state) and system ordering |
| `src/data.rs` | Guns, skins, characters, abilities, perks, power-ups, elements, XP |
| `src/config.rs` | Settings, key bindings and the saved profile |
| `src/ui.rs` | Main menu, characters, skins, settings, party screen, pause menu |
| `src/game.rs` | Match start and end, in-game menus, cursor, box visuals, spin rewards |
| `src/player.rs` | Camera and movement (sprint, slide, crouch, bunny hop) |
| `src/weapons.rs` | Two gun slots, firing, reloading, the first-person gun |
| `src/abilities.rs` | Ability and buy input |
| `src/sim.rs` | Host-only simulation: rounds, zombie AI, damage, elements, XP, box, perks, power-ups, extraction |
| `src/maps.rs` | The three maps |
| `src/nav.rs` | Zombie pathfinding |
| `src/humanoid.rs` | Person-shaped models and walk animation |
| `src/avatars.rs` | Zombie, pickup and teammate models, name tags |
| `src/fx.rs` | Tracers, explosions, rings, lightning, orbital strike |
| `src/hud.rs` | In-game HUD, scoreboard, level-up picker, end screen |
| `src/net.rs` | Command line, UDP messages, host and client networking |
| `src/physics.rs` | Box collisions and ray casts |
