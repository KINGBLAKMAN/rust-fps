# Rust FPS

Current version: **v8.0** (shown on the main menu and in the window title). Each version is one commit in this repo, its message starting with the version (v1 ... v8); small updates bump the minor number (v8.1, v8.2...).

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

- **Play Solo**, **Host a Party** or **Join a Party** (type or paste the host's address with Ctrl+V).
- **Loadout**: pick the gun you bring into matches and its attachments (see Career below).
- **Characters**: pick one of 6 characters (Striker, Warden, Ronin, Tinker, Blaze, Valkyrie), see their level and choose which abilities they take into a match.
- **Gun Crates**: open crates with your spins to win skins.
- **Gun Skins**: pick any gun and put one of your skins on it. It shows in game for you and your party.
- **Attachment Guide**: every attachment and exactly what it changes, with a tab per slot.
- **Sandbox**: a practice match with no waves and a tools panel (F1) to spawn zombies, switch on god mode and free abilities, add points, jump rounds and try any gun with any attachments.
- **Settings**: field of view, mouse sensitivity, volume (master plus guns, enemies, movement, effects and interface), graphics (shadows, smoothed edges, glow, soft corner shading, outlines, camera shake, air particles), windowed / borderless / fullscreen, resolution and key remapping.

Before a match you're on the party screen: pick your character, the host picks the map and day or night, friends press Ready and the host presses Start. Friends can also join a match that's already running.

**Playing over the internet:** on the same Wi-Fi it just works. Otherwise the host forwards UDP port 7777 on their router and shares their internet address (the party screen shows it when you click "Click to reveal", so it never appears on stream by accident), or everyone uses a virtual LAN like Tailscale or ZeroTier. Everyone must run the same version. On Windows, allow the game through the firewall when asked.

Command-line shortcuts still work: `rust-fps solo`, `rust-fps host [port]`, `rust-fps join <address>`, plus `--name <name>`, `--map <0-2>` and `--start` (skip the party screen). `rust-fps lookdev --map <0-2> [--night] --view <spawn|street|overhead|lineup|side|guns|heads|markers>` holds the camera at a fixed view with the HUD hidden, for comparing screenshots.

## Controls (default, all remappable)

| Input | Action |
|---|---|
| Mouse | Look / shoot (hold for automatic guns) |
| V | Melee: a quick knife slash (cancels a reload) |
| W A S D | Move |
| Shift | Sprint (you can shoot while sprinting) |
| Space | Jump. Bunny hop by tapping it again just as you land: each well-timed tap adds speed (holding it does nothing) |
| C | Crouch. Press while running to slide; hold it as you land from a jump to chain jump-slides |
| R | Reload |
| Right mouse | Aim down sights (zooms in, steadier shots) |
| F | Buy / use (mystery box, perk machines, doors, wall guns) |
| Q / E | Abilities 1 and 2 (see casting below) |
| X | Ultimate |
| T, 1 / 2, mouse wheel | Swap between your two guns |
| B | Pick a level-up upgrade |
| G, then 1-5 | Emotes: dance, wave, flip off, point, backflip |
| Z or middle mouse | Ping: mark a spot or an enemy for your team |
| Tab | Scoreboard |
| Esc | Menu (pauses solo games) |
| F1 | Sandbox tools (sandbox matches only) |

## Casting abilities

Each ability can use its own cast mode (Settings > Casting):

- **Instant:** casts the moment you press the key.
- **Quick** (default): hold the key to aim with a preview (grenade arc and landing ring, dash arrow, blast radius, orbital target), release to cast.
- **Confirm:** press the key to aim, left-click to cast, right-click or the key again to cancel.

The frag grenade is held in your hand while you aim, for as long as you like: it only arms when it leaves your hand. Every ability has its own first-person animation (Ronin draws a katana for Iaido Slash, Valkyrie raises her spear, Tinker sets the turret down...), and while an ability or ultimate is active a glow in its colour surrounds the caster and the edge of your screen.

## What you see

Since v8 the game aims to look like a painted illustration: soft warm sunlight with cool shadows, a faint brush grain on every surface, gently muted colour, a gradient sky that fades into the distance, and thin pencil lines round characters and nearby props. Heroes and zombies have realistic proportions. Ability previews and enemy attacks show as soft shapes painted on the ground: blue-grey for where your ability will land, ember red (filling up as it comes) for a Brute's slam or a Shooter's fireball. Hits flash zombies warm white, explosions shake the camera, and dust, pollen, fireflies or ash drift in the air depending on the map. The numbers behind the look are in `docs/art-direction.md`.

Every gun is modelled (23 in all) and held in gloved first-person hands, with animations for equipping, recoil, reloading (the magazine comes out and a fresh one goes in; revolvers, the double barrel and pump shotguns load shell by shell), pumping, sprinting and throwing. Teammates hold the gun they're using. The Twin Fangs are a pair of full-auto SMGs, one in each hand, and both fire on every shot. Abilities have their own effects: dash trails, a grenade with a burning fuse and a fireball explosion with smoke and debris, a healing column, an ice nova with spikes, and an orbital strike with a targeting laser. The mystery box opens its lid and spins through the guns, and flies away when it moves; perk machines are vending machines; power-ups are little models. Every gun has its own recoil pattern and visible attachments, sights line up when you aim, tracers streak and fade, and hits show a hit marker (orange for headshots, red for kills). Zombies fall over with a death animation, and shooting their legs out turns them into crawlers. Everything has sound. The three maps are built room by room, with furnished interiors (offices, kitchens, classrooms, labs, cells, a diner, a museum and more), windows, lamps and roofs.

## Gameplay

- **Rounds:** zombies come in rounds that get bigger and tougher. Grunts rush you, Shooters throw fireballs from a distance (round 3+), Brutes are big and tanky (round 6+). Downed players get back up at the start of the next round; if the whole team is down, it's game over.
- **Points:** 10 per hit, 60 per kill (Brutes 120), +40 for headshots. Spend them on the mystery box and perks.
- **Two guns:** you start with the M9 Sidearm and can carry two guns. Spare ammo is topped up at the start of each round.
- **Mystery box:** 750 points per spin, always. Its beam goes up into the sky so you can find it from anywhere. It gives one of 20 guns, or rarely one of 2 wonder weapons (Ray Blaster: explosive shots; Thunder Cannon: lightning that chains between enemies). Box guns come with random attachments: all, some or none of an optic (Red Dot, Holo Sight, 3x Scope), a muzzle, an underbarrel grip or laser and an extended mag. The box sits at one of 5 spots and every few spins it may fly away to another (you get your points back).
- **Perk machines:** Quick Hands (3000, reload twice as fast), Stamina Rush (2000, faster sprint and slides), Boom Shot (3500, headshots can explode), Juggernaut (2500, 200 max health), Rapid Fire (2000, shoot 33% faster). You lose your perks when you go down.
- **Power-ups** drop from zombies: Nuke (kills everything, +400 points each), Insta-Kill, Double Points (30 seconds each) and Max Ammo.
- **Maps and doors:** every map is a set of rooms, corridors, yards and buildings to explore, split into a starting area and 7 more areas behind doors you buy open (750-1250 points). Areas loop into each other, so there's usually more than one way round. Zombies only come from areas that are open, and the perk machines are spread through the areas.
  - **Shipping Yard:** start in the Customs Hall and Gate Yard; open up the Container Stacks, the Warehouse (with a cold store), the Dockside, the Port Office, the Rail Yard, the Machine Shop and the Truck Depot.
  - **Central Park:** start in the Visitor Centre and Fountain Court; open up the Hedge Maze, Lakeside (boat shed and café), the Museum, the Old Zoo, the Conservatory, the Chapel and Bandstand Green.
  - **The Neighborhood:** start in the Hendersons' house and back yard; open up Main Street, the Diner and Corner Store, the School, the Police Station, the Community Centre, Maple Court and the Back Alley.
- **Melee:** press V for a quick knife slash at whatever is right in front of you, crawlers included.
- **Wall guns:** each map has 8 guns hanging on walls (chalk outline). Guns players bring from their loadout take the first two boards (one in the starting area, one just past the first door). Buy the gun (pistol 500, SMG or shotgun 1000, rifle 1250), or ammo for half price if you already have it. Wall guns never come out of the mystery box on that map.
- **Day and night:** the host picks Day or Night on the party screen. At night the sky is dark, the street lamps are brighter and everyone has a flashlight.
- **Emotes and pings:** press G for the emote list; the camera pulls out so you can see your character (move or shoot to stop). Press Z to ping a spot (yellow beam) or an enemy (red marker that follows it), with your name and the distance.
- **Characters:** every character has their own model, two abilities (Q and E) and an ultimate (X). Ultimates charge over time and with kills. See Character levels below for every ability.
- **Levels:** kills give XP. Every level adds 3% damage, and every 5 levels you pick an upgrade (press B): a stronger ability tier, or an element. At levels 5, 10, 15, 20 and 25 one of the choices is always an element. Elements: Fire (burns over time), Ice (slows) and Shock (arcs to nearby enemies), for your guns or your abilities.
- **Extraction:** after clearing round 15, and every 10 rounds after that (25, 35...), a green extraction beam opens for 45 seconds. Get every living teammate into it for 5 seconds to extract, or ignore it and keep fighting.
- **Zombies:** walkers (in three outfits), spitters that spit fireballs from range and hulking brutes.

## Character levels and abilities

Each character levels up on their own from the XP you earn playing them (up to level 10). New abilities unlock at levels 2, 4, 6 and 8, and on the Characters screen you choose which two abilities and which ultimate to take. The end-of-match screen shows when a character levels up and what it unlocked.

| Character | Starting kit | Lv 2 | Lv 4 | Lv 6 (ultimate) | Lv 8 |
|---|---|---|---|---|---|
| Striker | Dash, Frag Grenade, Overdrive | Cluster Grenade (scatters six bomblets) | Rocket Barrage (a fan of six mini rockets) | Airstrike (jets carpet-bomb a line) | Combat Stim (heal, move and shoot faster) |
| Warden | Heal Pulse, Frost Nova, Orbital Strike | Glacier Spike (a wall of ice spikes along the ground) | Barrier Dome (zombies can't get in, heals inside) | Blizzard (a freezing storm) | Cryo Orb (a slow orb that freezes, then shatters) |
| Ronin | Iaido Slash, Shadow Step, Blade Storm | Kunai Fan (seven piercing kunai) | Smoke Bomb (stuns zombies inside) | Thousand Cuts (vanish and cut down everything around you) | Rising Dragon (a flaming uppercut leap) |
| Tinker | Sentry Turret, Supply Drop, Tesla Coil | Proximity Mines (three mines) | Combat Drone (follows you and shoots) | Mortar Battery (shells rain down) | Grav Grenade (pulls zombies in, then implodes) |
| Blaze | Firebomb, Flame Wave, Inferno | Fireball (explodes and leaves flames) | Flame Dash (leaves a trail of fire) | Meteor Shower | Magma Geyser (a pillar of magma) |
| Valkyrie | Arc Spear, Storm Leap, Ragnarok | Thunder Clap (a stunning shockwave) | Chain Lightning (leaps from zombie to zombie) | Bifrost (a beam from the sky sweeps along) | Spear Rain (lightning spears rain down) |

Iaido Slash is charged: hold Q and Ronin draws the sword and focuses; let go and he cuts, sending a crescent of light flying forward. The longer you hold (up to a second), the bigger the crescent and the further it travels (12 to 40 m). It pierces through every zombie in its path.

## Career and loadout

Every match earns career XP (150 per round survived, 3 per kill, 600 for extracting). Each career level unlocks a gun or attachment for your loadout, 27 levels in all; the Viper .45, Hornet MP and Breacher 12 are unlocked from the start.

On the Loadout screen pick a gun to bring and its attachments. In every match it hangs on one of the map's wall boards with your attachments on it (with your name), and the mystery box leaves it out. With two or more players, the first two loadouts take the two boards.

## Gun crates and skins

Crates open with a carousel that slides through the skins and slows down onto your prize. Spins are earned by how far you get, so restarting round 1 over and over earns nothing:

| Rounds survived | Spins |
|---|---|
| under 5 | 0 |
| 5 | 1 |
| 10 | 3 |
| 15 | 6 |
| 20 | 10 |
| 25 | 15 |

Extracting gives the same spins as reaching that round; it doesn't double them.

There are 35 skins in 5 crates. 12 are finishes that fit every gun; the other 23 are patterned skins made for one gun each (camo, tiger stripes, carbon fibre, damascus, marble, dragon scales, glowing lava, circuits, starfields...).

On the Gun Skins screen pick a gun and click a skin to put it on: any finish you own, or a patterned skin made for that gun. Default finish sets the skin for every gun that doesn't have its own.

| Crate | Cost | Inside |
|---|---|---|
| Field Crate | 1 spin | 4 finishes + 5 gun skins |
| Street Crate | 1 spin | 4 finishes + 5 gun skins |
| Forge Crate | 1 spin | 3 finishes + 5 gun skins |
| Inferno Crate | 1 premium spin | 4 gun skins, Epic or Legendary only |
| Cosmos Crate | 1 premium spin | 4 gun skins, Epic or Legendary only |

Each crate's odds are Common 55%, Rare 30%, Epic 12%, Legendary 3%, shared out over the rarities that crate holds (premium crates: Epic 80%, Legendary 20%). Every 3 duplicate regular pulls give a free spin; a premium duplicate gives back a quarter of a premium spin.

**Premium spins:** trade 5 regular spins for 1 premium spin, or earn a quarter of a premium spin for every 20 rounds you survive (added up over all your matches).

Your profile and settings are saved in `%APPDATA%\RustFPS` on Windows (`~/.config/rust-fps` on Linux).

## How the multiplayer works

The host runs the game: rounds, zombies, damage, points, the box and perks. Each player moves their own character on their own machine (so movement never feels laggy) and sends their position, shots and actions to the host about 60 times a second. Purchases and abilities are numbered and resent until the host confirms them, so a lost packet never loses a purchase. The host sends everyone a snapshot of the party, the match and every zombie 30 times a second over UDP.

## Code layout

| Path | What's in it |
|---|---|
| `src/main.rs` | App setup, shared state (players, match state) and system ordering |
| `src/data.rs` | Guns, attachments, skins, crates, characters, abilities, perks, power-ups, elements, XP |
| `src/config.rs` | Settings, key bindings and the saved profile |
| `src/progression.rs` | Career levels, unlocks and loadouts |
| `src/game.rs` | Match start and end, in-game menus, cursor, box visuals, spin rewards |
| `src/graphics.rs` | Applies the graphics settings, tone mapping, colour grading, the sky and distance haze |
| `src/painted.rs` | The painted material (brush grain, cool shadows, rim light) swapped onto every lit surface |
| `src/outline.rs` | Pencil outlines (inverted hulls) |
| `src/markers.rs` | Ground markers for ability previews and enemy warnings |
| `src/feel.rs` | Camera shake, air particles and the effects warm-up |
| `src/lookdev.rs` | `rust-fps lookdev` views for checking the art |
| `src/player.rs` | Camera and movement (sprint, slide, crouch, bunny hop) |
| `src/weapons.rs` | Two gun slots, firing, recoil, reloading, melee |
| `src/abilities/` | Ability and buy input, cast modes and charged casts (`mod.rs`), aiming previews (`preview.rs`) |
| `src/viewmodel.rs` | First-person gun and hands, with all their animations and ability props |
| `src/sim/` | Host-only simulation: rounds, zombie AI, damage, elements, XP, box, perks, power-ups, extraction, sandbox (`mod.rs`), and every ability with its projectiles, grenades, mines, turrets, drones and air strikes (`powers.rs`) |
| `src/zombies.rs` | Zombie animation, crawlers and death falls |
| `src/physics.rs` | Box collisions and ray casts |
| `src/emotes.rs`, `src/pings.rs` | Emotes and the third-person emote camera; pings |
| `src/hud.rs` | In-game HUD, hit markers, scoreboard, level-up picker, end screen |
| `src/net.rs` | Command line, UDP messages, host and client networking, internet address lookup |
| `src/maps/` | The three maps (`mod.rs`), the floor-plan kit for rooms, walls and furniture (`interiors.rs`), props, doors (`strips.rs`) and zombie pathfinding (`nav.rs`) |
| `src/models/` | The modelling kit, the 23 gun models, gun skins, hands, gun mounts, other players' models and ability projectiles and gadgets (`projectiles.rs`) |
| `src/rig/` | Skeleton and animation (`mod.rs`), the 6 hero models and the zombie models |
| `src/fx/` | Tracers, explosions, ability effects (`spells.rs` for the newer ones), and the ability auras |
| `src/audio/` | Sound effects, made by a small synthesizer at startup, and volume groups |
| `src/ui/` | Menus (`mod.rs`), the crate carousel, loadout and attachment guide, sandbox tools |
