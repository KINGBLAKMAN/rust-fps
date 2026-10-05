# Rust FPS

Current version: **v12.0** (shown on the main menu and in the window title). Each version is one commit in this repo, its message starting with the version (v1 ... v12); small updates bump the minor number (v12.1, v12.2...).

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
- **Loadout**: pick which of your class's guns you bring (one primary, one secondary) and the attachments on each (see Classes below).
- **Characters**: pick one of 6 classes (Bulwark, Medic, Revenant, Demolisher, Chemist, Ranger), see their level and choose which abilities they take into a match.
- **Gun Crates**: open crates with your spins to win skins.
- **Gun Skins**: pick any gun and put one of your skins on it. It shows in game for you and your party.
- **Attachment Guide**: every attachment and exactly what it changes, with a tab per slot.
- **Sandbox**: a practice match with no waves and a tools panel (F1) to spawn zombies or a boss, switch on god mode and free abilities, add points, jump rounds and try any gun with any attachments.
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
| Right mouse | Alternate fire: underbarrel grenade (assault rifles), slug (shotguns), burst (SMGs and pistols). Snipers, marksman rifles and LMGs aim down sights instead |
| F | Buy / use (Armory, perk machines, ammo caches) |
| Q / E | Abilities 1 and 2 (see casting below) |
| X | Ultimate |
| 3 / 4 | Weapon abilities (your class's two gun power-ups) |
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
- **Quick** (default): hold the key to aim with a preview (throwing arc and landing ring, charge or grapple line, blast radius, cone, target circle), release to cast.
- **Confirm:** press the key to aim, left-click to cast, right-click or the key again to cancel.

Thrown abilities (healing canister, sticky bomb, acid flask, bear trap) are held in your hand while you aim, for as long as you like. Every ability has its own first-person animation (the Revenant swings a scythe, the Demolisher sets a claymore down, the Bulwark slams the ground...), and while an ability or ultimate is active a glow in its colour surrounds the caster and the edge of your screen.

## What you see

Since v8 the game aims to look like a painted illustration: soft warm sunlight with cool shadows, a faint brush grain on every surface, gently muted colour, a gradient sky that fades into the distance, and thin pencil lines round characters and nearby props. Heroes and zombies have realistic proportions. Ability previews and enemy attacks show as soft shapes painted on the ground: blue-grey for where your ability will land, ember red (filling up as it comes) for a Brute's slam or a Shooter's fireball. Hits flash zombies warm white, explosions shake the camera, and dust, pollen, fireflies or ash drift in the air depending on the map. The numbers behind the look are in `docs/art-direction.md`.

Every gun is modelled (23 in all) and held in gloved first-person hands, with animations for equipping, recoil, reloading (the magazine comes out and a fresh one goes in; revolvers, the double barrel and pump shotguns load shell by shell), pumping, sprinting and throwing. Teammates hold the gun they're using. The Twin Fangs are a pair of full-auto SMGs, one in each hand, and both fire on every shot. Every ability has its own effects: shockwaves, healing mist, spectral chains and scythes, acid pools and poison clouds, a falling bomb with a mushroom cloud, arrows raining from the sky and more. The Armory chest opens its lid and spins before showing its offer, and flies away when it moves; perk machines are vending machines; power-ups are little models. Every gun has its own recoil pattern and visible attachments, sights line up when you aim, tracers streak and fade, and hits show a hit marker (orange for headshots, red for kills). Zombies fall over with a death animation, and shooting their legs out turns them into crawlers. Everything has sound. The three maps are built room by room, with furnished interiors (offices, kitchens, classrooms, labs, cells, a diner, a museum and more), windows, lamps and roofs.

## Gameplay

- **The run:** a match is a run through 5 maps. On each map you survive 4 rounds, then its boss comes (7 seconds' warning). Kill the boss and its zombies fall with it, everyone gets 500 points, 300 XP and a free upgrade pick, and a purple teleporter opens. Get every living teammate into it for 3 seconds to go to the next map. You keep your guns, perks, levels and upgrades, and everyone comes back up at full health. The run goes through the three maps in order from the one the host picked, then the first two again at the other time of day. The fifth map ends with the final boss; beat it to win the run.
- **Rounds:** zombies come in rounds that get bigger and tougher, and the round number keeps counting up over the whole run. Grunts rush you, Shooters throw fireballs from a distance (round 3+), Brutes are big and tanky (round 6+). Each map's zombies hit 15% harder and have 10% more health than the last, and it sends more Brutes and Shooters. Downed players get back up at the start of the next round; if the whole team is down, it's game over.
- **Bosses:** each map has one: The Foreman (Shipping Yard), The Groundskeeper (Central Park) and The Landlord (The Neighborhood), huge rust-red Brutes. The final boss, The Abomination, is bigger, purple and twice as tough. Bosses slam the ground (a big red circle fills up first, so step out), throw fans of fireballs, and call in a ring of zombies at two-thirds and one-third health. The final boss throws more, calls in Brutes too, and speeds up below 30% health. Bosses get tougher with more players, shrug off stuns, Insta-Kill and Nukes, and can't be turned into crawlers. A bar at the top of the screen shows the boss's health.
- **Points:** 10 per hit, 60 per kill (Brutes 120), +40 for headshots. Spend them on the Armory, ammo and perks.
- **Two guns:** you start with your class's primary and secondary (see Classes below). Spare ammo is topped up at the start of each round.
- **Armory** (the old mystery box): 750 points a go. Its beam goes up into the sky so you can find it from anywhere. It offers a new random set of attachments for the gun in your hands (an optic, a muzzle, an underbarrel grip or laser and an extended mag, as the gun allows), or rarely (5%) a wonder weapon to swap it for (Ray Blaster: explosive shots; Thunder Cannon: lightning that chains between enemies). It sits at one of 5 spots and every few uses it may move to another (you get your points back).
- **Perk machines:** Quick Hands (3000, reload twice as fast), Stamina Rush (2000, faster sprint and slides), Boom Shot (3500, headshots can explode), Juggernaut (2500, 200 max health), Rapid Fire (2000, shoot 33% faster). You lose your perks when you go down.
- **Power-ups** sometimes drop from zombies (rarely, and never two within 25 seconds of each other): Nuke (kills everything, +400 points each), Insta-Kill, Double Points (30 seconds each) and Max Ammo.
- **Maps:** every map is a set of rooms, corridors, yards and buildings to explore, all open from the start (there are no doors to buy since v9). Areas loop into each other, so there's usually more than one way round. Zombies come from all over the map, and the perk machines are spread through it.
  - **Shipping Yard:** the Customs Hall and Gate Yard, the Container Stacks, the Warehouse (with a cold store), the Dockside, the Port Office, the Rail Yard, the Machine Shop and the Truck Depot.
  - **Central Park:** the Visitor Centre and Fountain Court, the Hedge Maze, Lakeside (boat shed and café), the Museum, the Old Zoo, the Conservatory, the Chapel and Bandstand Green.
  - **The Neighborhood:** the Hendersons' house and back yard, Main Street, the Diner and Corner Store, the School, the Police Station, the Community Centre, Maple Court and the Back Alley.
- **Melee:** press V for a quick knife slash at whatever is right in front of you, crawlers included.
- **Ammo caches:** the chalk boards on the walls (8 a map) refill the spare ammo of both your guns for 300 points.
- **Day and night:** the host picks Day or Night on the party screen (maps 4 and 5 of a run are the other one). At night the sky is dark, the street lamps are brighter and everyone has a flashlight.
- **Emotes and pings:** press G for the emote list; the camera pulls out so you can see your character (move or shoot to stop). Press Z to ping a spot (yellow beam) or an enemy (red marker that follows it), with your name and the distance.
- **Characters:** every character has their own model, two abilities (Q and E) and an ultimate (X). Ultimates charge over time and with kills. See Character levels below for every ability.
- **Levels:** kills give XP. Every level adds 3% damage, and every 2 levels (plus after each boss) you pick one of three upgrades (press B): a stronger ability tier (up to tier VI: more damage, size and duration, and a shorter cooldown, never below 40% of the start), an ability augment, an element, a stat, or a weapon upgrade (Mk II, III and IV: each adds 25% damage and magazine size to that gun). Augments change how your two abilities work, up to 3 times each: movement and blast abilities (Shield Charge, Ground Pound, Rally Cry, Med Drone, Scythe Sweep, Wraith Step, Blast Jump, Catalyst, Grapple) gain an extra charge, so you can use them several times in a row; everything else fires an extra copy fanned out to the side (two, three, then four canisters, darts, chains, bombs, claymores, flasks, clouds, marks or traps). The ability bar shows charges (2/4) and copies (x3). At every tenth level one of the choices is always an element. Elements: Fire (burns over time), Ice (slows) and Shock (arcs to nearby enemies), for your guns or your abilities. Stats stack up to 5 times each: Vitality (+20 max health), Firepower (+8% gun damage), Focus (6% shorter ability cooldowns), Swift (+5% move speed) and Sleight of Hand (10% faster reloads).
- **Zombies:** walkers (in three outfits), spitters that spit fireballs from range and hulking brutes.

## Classes and abilities

v12 replaced every class with a new one: new characters, new models and 30 new abilities. Each class levels up on its own from the XP you earn playing it (up to level 10). It starts with two abilities and an ultimate, unlocks a third ability at level 3 and a second ultimate at level 6, and on the Characters screen you choose which two abilities and which ultimate to take.

| Class | Abilities (start) | Ultimate (start) | Lv 3 ability | Lv 6 ultimate |
|---|---|---|---|---|
| **Bulwark**, riot-armour tank with a shield | Shield Charge (charge forward, bowling zombies aside and stunning them), Ground Pound (zombies around you are thrown up and stunned) | Fortress (take 80% less damage while a golden aura burns everything near you) | Rally Cry (you and nearby teammates heal, take 40% less damage and move faster) | Earthshaker (three shockwaves, each bigger than the last) |
| **Medic**, field medic | Healing Grenade (a canister bursts into a healing mist), Neurotoxin Dart (poisons and paralyses a zombie, then jumps to three more) | Resurrection (revives every downed teammate and heals the team to full) | Med Drone (follows you, heals you and teammates and zaps zombies) | Sterilize (an ultraviolet wave scorches every zombie around you) |
| **Revenant**, hooded reaper | Scythe Sweep (a full circle cut that heals you for every zombie hit), Soul Chains (spectral chains drag zombies to you and bind them) | Reaper (spectral scythes whirl round you, shredding zombies and healing you) | Wraith Step (drift forward as mist; nothing can hurt you for a moment) | Army of the Dead (four spectral warriors fight beside you) |
| **Demolisher**, demolition expert | Sticky Bomb (sticks to the first zombie or wall and blows), Blast Jump (blow yourself up and forward, blasting where you stood) | Payload (a huge bomb drops where you aim and levels the area) | Claymore (blasts a cone of shrapnel at the first zombie to come close) | Chain Reaction (for a while every zombie you kill explodes) |
| **Chemist**, hazmat specialist | Acid Flask (shatters into a pool of acid), Toxic Cloud (a choking cloud in front of you poisons and slows) | Plague Bloom (a toxic bloom that keeps spreading) | Catalyst (detonates all your acid pools and clouds) | Petrify (zombies in front of you turn to stone, then shatter) |
| **Ranger**, hooded hunter | Grapple (zip to wherever you aim, up onto roofs too), Hunter's Mark (zombies in front of you take 50% more damage) | Deadeye (locks on to up to 10 zombies and snipes every one) | Bear Trap (snaps shut on the first zombie, holding it fast) | Arrow Storm (arrows rain down where you aim) |

Poisoned zombies glow green and take damage over time; marked zombies glow red.

## Weapon abilities and guns

Each class decides your guns and your two weapon abilities. You bring one primary and one secondary, picked from two of each on the Loadout screen, and start every match holding them.

| Class | Primaries | Secondaries | Weapon abilities (3 / 4) |
|---|---|---|---|
| Bulwark | Breacher 12, Stormfront Auto | Viper .45, Judge Revolver | Suppressing Fire (every hit stuns briefly), Lock and Load (refill the magazine, hit 40% harder) |
| Medic | Kestrel SMG, Tempest Burst | M9 Sidearm, Hornet MP | Cryo Rounds (bullets slow), Auto-Loader (no ammo used, 30% faster fire) |
| Revenant | Twin Fangs, Ranger Rifle | Hammer .50, M9 Sidearm | Soul Siphon (every hit heals you for 8% of its damage), Overcharge (every shot chains lightning through three more) |
| Demolisher | Goliath LMG, Ripsaw LMG | Wasp PDW, Viper .45 | Dragon's Breath (burning rounds that burst on impact), Overheat (shoot 60% faster, hit 20% harder) |
| Chemist | Falcon AR, Double Barrel | Hornet MP, Judge Revolver | Toxic Rounds (bullets poison and slow), Shock Rounds (bullets arc to nearby zombies) |
| Ranger | Arbiter DMR, Longbow Sniper | Hammer .50, Mamba Machine Pistol | Executioner (hits finish anything under a quarter health), Quickdraw (shoot 40% faster, hit 80% harder) |

Weapon abilities power up whichever gun you're holding for 6 to 10 seconds, then go on cooldown. They're shown on the left of the ability bar, and glow in their colour while running.

Old saves carry over: each old character's XP moves to the class in the same slot (Striker to Bulwark, Warden to Medic, Ronin to Revenant, Tinker to Demolisher, Blaze to Chemist, Valkyrie to Ranger), and ability kits are reset to the new defaults.

## Career

Every match earns career XP (150 per round survived, 3 per kill, 400 per map cleared and 1500 for winning the run). Each career level unlocks an attachment or a class's second gun choice, 17 levels in all. Every class's first primary and secondary are unlocked from the start.

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

Winning the run gives the same spins as reaching that round; it doesn't double them.

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
| `src/progression.rs` | Career levels and unlocks |
| `src/game.rs` | Match start and end, in-game menus, cursor, Armory visuals, spin rewards |
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
| `src/sim/` | Host-only simulation: rounds, the run's maps and bosses, the teleporter, zombie AI, damage, elements, XP, box, perks, power-ups, sandbox (`mod.rs`), and every ability with its projectiles, grenades, mines, turrets, drones and air strikes (`powers.rs`) |
| `src/zombies.rs` | Zombie animation, crawlers and death falls |
| `src/physics.rs` | Box collisions and ray casts |
| `src/emotes.rs`, `src/pings.rs` | Emotes and the third-person emote camera; pings |
| `src/hud.rs` | In-game HUD, hit markers, scoreboard, level-up picker, end screen |
| `src/net.rs` | Command line, UDP messages, host and client networking, internet address lookup |
| `src/maps/` | The three maps (`mod.rs`), the floor-plan kit for rooms, walls and furniture (`interiors.rs`), props and ammo cache boards (`strips.rs`) and zombie pathfinding (`nav.rs`) |
| `src/models/` | The modelling kit, the 23 gun models, gun skins, hands, gun mounts, other players' models and ability projectiles and gadgets (`projectiles.rs`) |
| `src/rig/` | Skeleton and animation (`mod.rs`), the 6 hero models and the zombie models |
| `src/fx/` | Tracers, explosions, ability effects (`spells.rs` for the newer ones), and the ability auras |
| `src/audio/` | Sound effects, made by a small synthesizer at startup, and volume groups |
| `src/ui/` | Menus (`mod.rs`), the crate carousel, class loadout and attachment guide, sandbox tools |
