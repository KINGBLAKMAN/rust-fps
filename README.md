# Rust FPS

A co-op wave-survival first-person shooter written in Rust with [Bevy](https://bevyengine.org) 0.16. Play solo or with up to 8 friends.

![screenshot](screenshot.png)

## Build

1. Install Rust: https://rustup.rs
2. In this folder run `cargo build --release`. The first build compiles Bevy and takes a few minutes; later builds are fast.

**Linux only:** install the system libraries Bevy needs first, e.g. on Ubuntu/Debian:

```sh
sudo apt install g++ pkg-config libx11-dev libasound2-dev libudev-dev libxkbcommon-x11-0 libwayland-dev libxkbcommon-dev
```

Windows and macOS need nothing extra.

## Play

```sh
# Solo
cargo run --release

# Host a co-op game (port 7777 by default)
cargo run --release -- host --name Jonah

# Join a friend's game
cargo run --release -- join 192.168.1.20 --name Buddy
```

The host's screen shows the exact `join` address to give your friends. Waves start when the host clicks into the game, so let everyone join first.

- **Same Wi-Fi / LAN:** use the address the host's screen shows.
- **Over the internet:** the host needs to forward UDP port 7777 on their router to their PC and share their public IP, or you can all use a virtual LAN tool such as Tailscale or ZeroTier and join with that address.
- Everyone must run the same version of the game. On Windows, allow the game through the firewall when asked.

You can also run the built binary directly: `target/release/rust-fps host`, etc. Use `host 9000` to pick another port, and `join host:9000` to match it.

## Controls

| Input | Action |
|---|---|
| Mouse | Look |
| W A S D | Move |
| Shift | Sprint |
| Space | Jump (you can stand on crates) |
| Left click | Shoot (hold for auto-fire) / capture the mouse |
| R | Reload |
| Esc | Release the mouse (pauses solo games) |
| Enter | Restart after a game over (host or solo) |

## Gameplay

- Enemies come in waves from the corners of the arena and hunt the nearest player. Waves get bigger with each wave and with each extra player.
- **Red grunts** rush you and hit you up close.
- **Purple shooters** (from wave 2) keep their distance and lob glowing fireballs you can dodge or hide from behind cover.
- Clearing a wave heals everyone still standing by 25 HP.
- If you go down, you lie on the floor until the next wave starts, then come back with 50 HP. When the whole team is down, it's game over.
- Score per kill is multiplied by the wave number. The scoreboard is in the top right.

## How the multiplayer works

The host runs the game: enemies, waves, damage and score. Each player moves their own character on their own machine (so movement never feels laggy) and sends their position and shots to the host about 60 times a second. The host checks what each shot hit and sends everyone a snapshot of all players, enemies and fireballs 30 times a second over UDP.

## Code layout

| File | What's in it |
|---|---|
| `src/main.rs` | App setup, shared state (players, match state) and system ordering |
| `src/player.rs` | Your character: camera, movement, gun, reloading, tracers |
| `src/sim.rs` | Host-only simulation: waves, enemy AI, fireballs, hits, revive and restart |
| `src/net.rs` | Command line, UDP messages, host and client networking |
| `src/avatars.rs` | Other players' characters and floating name tags |
| `src/hud.rs` | Health, ammo, score, scoreboard, messages, game-over screen |
| `src/level.rs` | The arena and shared meshes |
| `src/physics.rs` | Box collisions and ray casts |

Tuning constants (speeds, damage, fire rate, magazine size) are at the top of `player.rs` and `sim.rs`.
