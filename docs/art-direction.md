# Art direction (v8)

The goal: the game should look like a painted illustration, not a cartoon. That means soft, warm light, slightly muted colour, a hint of brush texture, cool shadows and quiet pencil lines. Everything stays built from boxes, cylinders and spheres in code, and the maps and colour choices stay as they were.

What we don't want: hard toon shading bands, thick black outlines, neon glow, or very saturated colour.

This page records the numbers that were settled on. Tune from here.

## Finish (graphics.rs, maps/mod.rs)

- **HDR camera, AgX tone mapping.** Filmic was tried and was flatter.
- **Colour grading:**
  - Saturation 0.95. The brief said 0.85, but under AgX that washed out the grass and sky.
  - Midtone contrast 1.06.
  - Temperature per map: Shipping Yard -0.015 (cool), Central Park +0.005, The Neighborhood +0.025 (warm). At night it's half that, minus 0.01.
  - Anything above about ±0.05 tints the whole screen.
- **Light:**
  - The sun is warm, sRGB (1.0, 0.95, 0.86), at 0.7 of each map's old brightness.
  - A cool fill light (0.82, 0.87, 1.0) at 14% of the sun, with no shadows, by day only.
  - Ambient is 140 by day (near white) and 70 at night (blue), down from 350.
  - The moon is 6% of the sun.
- **Sky:**
  - A vertex-coloured dome, 600 m across, that follows the camera.
  - The horizon is the map's sky colour mixed 20% towards warm white. The clear colour and fog match it, so the far distance melts into the sky.
  - Sun disc: radius 14, linear (9, 8.2, 6.8). Moon: radius 8.
- **Bloom** intensity 0.1.
- **SSAO** stays a Settings option.

## Painted material (painted.rs)

Every lit, opaque material is swapped for a painted copy. Changes to the original material, such as hit flashes and tints, are copied across.

- **Brush grain:** triplanar value noise at 0.31 and 0.07 world scale, mixed 60/40, ±15% on the albedo. It fades out between 25 and 85 m.
- **Cool shadows:** lighting is tinted towards (0.80, 0.82, 0.95) where the sun doesn't reach, using N·L wrapped by 0.3 so the change is soft.
- **Rim light:** 12%, `pow(1 - N·V, 3)`, only on the lit side.
- **Emissive** is added after lighting, so glows aren't tinted or grained.
- **Kit colour jitter:** each shape's colour varies by ±4%, seeded by its position, so large flat areas aren't one exact colour.
- **Metal:** GUNMETAL, STEEL and BRASS parts carry vertex alpha 0.75, which the shader reads as metallic 1.0 and roughness 0.32.

## Proportions (rig/mod.rs, rig/heroes.rs, rig/zombie_models.rs)

- **Head:** 0.65 wide, 0.82 tall, 0.75 deep, compared with v7.
- **Shoulders** at x 0.20.
- **Chest** is a tapered torso: waist, flat chest front, collarbones and shoulder blades.
- **Limbs** taper towards the joint:
  - Upper arm 0.061 to 0.049, plus a deltoid.
  - Forearm 0.054 to 0.04.
  - Thigh 0.086 to 0.062.
  - Shin 0.058 to 0.042, plus a calf.
- **Faces:** a brow, cheekbones, jaw, chin and nose on the faces that show.
- **GUN_SCALE** is 1.0.
- **Zombies** get the same treatment. The Brute keeps its bulk (body scale about 0.9 rather than 0.8) and the Spitter keeps its hunch.
- **First-person forearm** has a muscle swell.

## Pencil outlines (outline.rs)

An inverted hull: each mesh is drawn again, pushed out along smoothed normals, with front faces culled.

- **Colour:** the surface colour × 0.12 in linear light (about 0.35 as you see it), at 50% opacity.
- **Width:** 0.0025 × distance, with the distance clamped to 2–60 m. The planned 0.0012 was under a pixel.
- **Drawn on:** characters, zombies, other players' guns, and props up to 4.5 m in size. Props fade out between 24 and 30 m.
- **First-person gun:** no outline. One was tried at 0.0006 × distance and 40% opacity, and it mostly made the gun look dirty.
- **Settings → Graphics → Outlines** turns them off.

## Ground markers (markers.rs)

Flat quads drawn with a distance-field shader, in four shapes: circle, donut, cone and line.

- **Body:** alpha 0.10 + 0.20·exp(d / 1.1), so it gets stronger towards the edge.
- **Edge:** a crisp 3.5 cm line at 0.55, antialiased with fwidth.
- **Fill:** countdowns fill outwards, with a 0.12 m bright front. In the last 20% they pulse up to ×1.45. When they go off they flash for 0.4 s.
- **Ability previews** are muted (0.55, 0.75, 0.85), with no fill. Gizmo lines are kept only for things in the air, such as throw arcs and beams.
- **Enemy warnings** are ember red (0.85, 0.30, 0.15):
  - A Brute's slam: a 2 m circle, 1 m in front of it. The Brute now winds up for 0.45 s and stops turning before it lands.
  - Where a Shooter's fireball will hit.
- At night markers are 40% as bright, so they don't glare.

## Feel (feel.rs, models/avatars.rs)

- **Hit flash:** (0.45, 0.42, 0.40) is added to the emissive and fades at 7 per second.
- **Camera shake:**
  - A shared value from 0 to 1. The camera moves value² × 0.09 m, and the shake fades at 2.5 per second.
  - Explosions and slams add up to 0.7 or 0.8, falling off with distance. A Brute slam within 8 m adds 0.5.
  - Settings → Graphics → Camera shake turns it off.
- **Air particles:** 140 specks wrapping round the camera in a 22 m box.
  - Shipping Yard: dust.
  - Central Park: pollen by day, fireflies at night.
  - The Neighborhood: ash.
  - Settings → Graphics → Air particles turns them off.
- **Warm-up:** a few frames into each match, one of every effect is set off 400 m away and 70 m below the horizon, so its shaders are built before anyone needs them.

## Checking the look

Run `rust-fps lookdev [--map 0|1|2] [--night] [--view spawn|street|overhead|lineup|side|guns|heads|markers]`.

It starts a sandbox with the HUD hidden and the camera held still, for side-by-side screenshots. The lineup views show every hero, one of each zombie, every gun and a wall section under that map's light. The markers view shows every marker shape, with a Brute slamming over and over.

## Not done yet

- Skin rarity looks (Part 7 of the brief) are left for a later version.
