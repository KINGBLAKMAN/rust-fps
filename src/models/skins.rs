//! Patterned gun skins: procedural, seamlessly tiling textures (camo, tiger
//! stripes, carbon weave, lava cracks...) and the texture coordinates that
//! wrap them around a gun model.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::data::{skin_def, skin_material, Pattern, SkinDef};
use crate::maps::vnoise;
use crate::props::hash;

/// Texture size in pixels (the pattern repeats every N pixels).
const N: usize = 128;
/// Pattern repeats per metre of gun.
const TILES_PER_METRE: f32 = 4.5;

/// The full material for a skin, with its pattern textures if it has one.
pub fn material(id: u8, images: &mut Assets<Image>) -> StandardMaterial {
    let def = skin_def(id);
    let mut mat = skin_material(id);
    if def.pattern == Pattern::Plain {
        return mat;
    }
    let (color, glow) = paint(def);
    mat.base_color = Color::WHITE;
    mat.base_color_texture = Some(images.add(image(color)));
    if def.pattern.glows() {
        mat.emissive = LinearRgba::WHITE * def.glow;
        mat.emissive_texture = Some(images.add(image(glow)));
    }
    mat
}

/// Box-projects texture coordinates from each vertex's position, so a
/// pattern runs continuously across every part of a gun.
pub fn project_uvs(mesh: &mut Mesh) {
    let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };
    let Some(VertexAttributeValues::Float32x3(nor)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) else {
        return;
    };
    let uvs: Vec<[f32; 2]> = pos
        .iter()
        .zip(nor)
        .map(|(p, n)| {
            let (ax, ay, az) = (n[0].abs(), n[1].abs(), n[2].abs());
            let uv = if ax >= ay && ax >= az {
                [p[2], p[1]]
            } else if ay >= az {
                [p[2], p[0]]
            } else {
                [p[0], p[1]]
            };
            [uv[0] * TILES_PER_METRE, uv[1] * TILES_PER_METRE]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
}

fn image(rgb: Vec<[f32; 3]>) -> Image {
    let mut data = Vec::with_capacity(N * N * 4);
    for c in rgb {
        for v in c {
            data.push((v.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8);
        }
        data.push(255);
    }
    let mut image = Image::new(
        Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Tiling fractal noise in 0..1 at pixel (x, y), starting at `base` cells
/// across the texture.
fn fbm(x: f32, y: f32, base: i32, seed: f32) -> f32 {
    let mut f = 0.0;
    let mut amp = 0.5;
    let mut period = base;
    let mut total = 0.0;
    for _ in 0..4 {
        let s = period as f32 / N as f32;
        f += vnoise(x * s + seed * 17.0, y * s + seed * 31.0, period) * amp;
        total += amp;
        amp *= 0.5;
        period *= 2;
    }
    f / total
}

/// Distances to the nearest and second-nearest of a tiling grid of jittered
/// points, `cells` per side. `row_offset` shifts every other row half a cell
/// (a honeycomb when jitter is 0). Returns (d1, d2, centre of nearest).
fn cells(
    x: f32,
    y: f32,
    cells: i32,
    jitter: f32,
    row_offset: bool,
    seed: f32,
) -> (f32, f32, [f32; 2]) {
    let size = N as f32 / cells as f32;
    let (cx, cy) = ((x / size).floor() as i32, (y / size).floor() as i32);
    let (mut d1, mut d2, mut best) = (f32::MAX, f32::MAX, [0.0, 0.0]);
    for j in -2..=2 {
        for i in -2..=2 {
            let (gx, gy) = (cx + i, cy + j);
            let (wx, wy) = (gx.rem_euclid(cells) as f32, gy.rem_euclid(cells) as f32);
            let shift = if row_offset && gy.rem_euclid(2) == 1 {
                0.5
            } else {
                0.0
            };
            let px = (gx as f32 + 0.5 + shift + (hash(wx + seed, wy) - 0.5) * jitter) * size;
            let py = (gy as f32 + 0.5 + (hash(wx, wy + seed) - 0.5) * jitter) * size;
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if d < d1 {
                d2 = d1;
                d1 = d;
                best = [px, py];
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    (d1, d2, best)
}

/// Paints a skin's pattern: returns the colour texture and the glow mask
/// (already tinted with the glow colour).
fn paint(def: &SkinDef) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let (a, b) = (def.color, def.accent);
    let mut col = vec![[0.0; 3]; N * N];
    let mut glow = vec![[0.0; 3]; N * N];
    let tau = std::f32::consts::TAU;
    let nf = N as f32;
    for y in 0..N {
        for x in 0..N {
            let (fx, fy) = (x as f32, y as f32);
            let (u, v) = (fx / nf, fy / nf);
            let i = y * N + x;
            let mut g = 0.0;
            col[i] = match def.pattern {
                Pattern::Plain => a,
                Pattern::Camo => {
                    let f = fbm(fx, fy, 4, 1.0);
                    let f2 = fbm(fx, fy, 4, 2.0);
                    if f < 0.42 {
                        b
                    } else if f2 > 0.58 {
                        mix(a, b, 0.5)
                    } else {
                        a
                    }
                }
                Pattern::Digital => {
                    // Blocky pixels, 8 to a side so they tile.
                    let (bx, by) = ((fx / 8.0).floor() * 8.0, (fy / 8.0).floor() * 8.0);
                    let f = fbm(bx, by, 4, 3.0);
                    let f2 = fbm(bx, by, 8, 4.0);
                    if f < 0.43 {
                        b
                    } else if f2 > 0.57 {
                        mix(a, b, 0.45)
                    } else {
                        a
                    }
                }
                Pattern::Tiger => {
                    let n = fbm(fx, fy, 4, 5.0);
                    let s = ((u + n * 0.3) * tau * 5.0).sin();
                    let width = 0.5 + fbm(fx, fy, 8, 6.0) * 0.4;
                    if s > width {
                        b
                    } else {
                        mix(a, scale(a, 1.15), fbm(fx, fy, 16, 7.0))
                    }
                }
                Pattern::Zebra => {
                    let n = fbm(fx, fy, 4, 8.0);
                    let s = ((u * 0.3 + v + n * 0.45) * tau * 4.0).sin();
                    if s > 0.05 {
                        b
                    } else {
                        a
                    }
                }
                Pattern::Carbon => {
                    let cell = 8.0;
                    let (cx, cy) = ((fx / cell).floor() as i32, (fy / cell).floor() as i32);
                    let (lx, ly) = (fx % cell / cell, fy % cell / cell);
                    let along = if (cx + cy) % 2 == 0 { lx } else { ly };
                    let sheen = (along * std::f32::consts::PI).sin();
                    mix(b, a, sheen * 0.9)
                }
                Pattern::Hex => {
                    let (d1, d2, _) = cells(fx, fy, 8, 0.0, true, 9.0);
                    let edge = d2 - d1;
                    if edge < 1.6 {
                        g = 1.0;
                        b
                    } else {
                        mix(a, scale(a, 0.8), d1 / 10.0)
                    }
                }
                Pattern::Scales => {
                    let (d1, _, c) = cells(fx, fy, 8, 0.0, true, 10.0);
                    // Each scale is lit from the top and rimmed at the bottom.
                    let t = d1 / 9.0;
                    let lit = smooth(4.0, -6.0, fy - c[1]);
                    let base = mix(a, scale(a, 1.35), lit * (1.0 - t));
                    mix(base, b, smooth(0.65, 1.0, t))
                }
                Pattern::Damascus => {
                    let n = fbm(fx, fy, 4, 11.0);
                    let s = ((v * 5.0 + u * 1.0 + n * 2.5) * tau).sin();
                    let line = smooth(0.75, 0.95, s.abs());
                    mix(a, b, line)
                }
                Pattern::Marble => {
                    let n = fbm(fx, fy, 4, 12.0);
                    let s = ((u * 2.0 + v + n * 3.0) * tau).sin().abs();
                    let vein = smooth(0.12, 0.0, s);
                    let cloud = fbm(fx, fy, 8, 13.0);
                    mix(mix(a, scale(a, 0.9), cloud), b, vein)
                }
                Pattern::Splatter => {
                    let mut c = mix(a, scale(a, 0.85), fbm(fx, fy, 8, 14.0));
                    for k in 0..36 {
                        let k = k as f32;
                        let (px, py) = (hash(k, 1.5) * nf, hash(k, 2.5) * nf);
                        let r = 2.0 + hash(k, 3.5).powi(2) * 9.0;
                        let dx = (fx - px + nf * 1.5).rem_euclid(nf) - nf * 0.5;
                        let dy = (fy - py + nf * 1.5).rem_euclid(nf) - nf * 0.5;
                        let wobble = 1.0 + (dy.atan2(dx) * 5.0 + k).sin() * 0.2;
                        if (dx * dx + dy * dy).sqrt() < r * wobble {
                            c = if hash(k, 4.5) > 0.3 {
                                b
                            } else {
                                mix(b, a, 0.5)
                            };
                        }
                    }
                    if hash(fx, fy) > 0.985 {
                        c = b;
                    }
                    c
                }
                Pattern::Woodgrain => {
                    let n = fbm(fx, fy, 4, 15.0);
                    let rings = ((v * 6.0 + n * 2.0) * tau).sin() * 0.5 + 0.5;
                    let streak = fbm(fx * 0.25, fy * 4.0, 4, 16.0);
                    mix(a, b, rings * 0.6 + streak * 0.3)
                }
                Pattern::Circuit => {
                    let cell = 16.0;
                    let (cx, cy) = ((fx / cell).floor(), (fy / cell).floor());
                    let (lx, ly) = (fx - cx * cell, fy - cy * cell);
                    let h = |s: f32| hash(cx + s * 7.0, cy + s * 3.0);
                    let mid = cell * 0.5;
                    let mut on = false;
                    if h(1.0) > 0.35 && (ly - mid).abs() < 1.0 {
                        on = true;
                    }
                    if h(2.0) > 0.5 && (lx - mid).abs() < 1.0 {
                        on = true;
                    }
                    if h(3.0) > 0.55 && (lx - mid).abs() < 2.5 && (ly - mid).abs() < 2.5 {
                        on = true;
                    }
                    if h(4.0) > 0.7 && (lx - ly).abs() < 1.0 {
                        on = true;
                    }
                    if on {
                        g = 1.0;
                        scale(b, 0.6)
                    } else {
                        mix(a, scale(a, 1.4), fbm(fx, fy, 16, 17.0) * 0.5)
                    }
                }
                Pattern::Lava => {
                    let (d1, d2, _) = cells(fx, fy, 6, 0.9, false, 18.0);
                    let edge = d2 - d1;
                    g = smooth(4.0, 0.5, edge) + smooth(1.0, 0.0, edge) * 0.5;
                    let rock = mix(a, scale(a, 1.6), fbm(fx, fy, 8, 19.0));
                    mix(rock, b, g)
                }
                Pattern::Stars => {
                    let neb = fbm(fx, fy, 4, 20.0);
                    let neb2 = fbm(fx, fy, 8, 21.0);
                    let sky = mix(
                        a,
                        mix(scale(a, 2.2), [0.8, 0.2, 0.6], 0.3),
                        smooth(0.45, 0.8, neb),
                    );
                    let sky = mix(sky, [0.1, 0.4, 0.8], smooth(0.55, 0.85, neb2) * 0.4);
                    let star = hash(fx, fy);
                    if star > 0.988 {
                        g = 1.0;
                        b
                    } else {
                        g = smooth(0.6, 0.9, neb) * 0.15;
                        sky
                    }
                }
            };
            glow[i] = scale(def.accent, g);
        }
    }
    (col, glow)
}
