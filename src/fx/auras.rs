//! The glow around a character while an ability is working: a shimmering
//! shell and rising sparks in the ability's colour, lasting as long as the
//! ability does (Overdrive, Blade Storm, Inferno, Ragnarok) or a moment for
//! a quick cast. In first person it shows as a tint at the screen edges and
//! glowing hands (see viewmodel.rs).

use bevy::asset::RenderAssetUsages;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::Rng;
use std::collections::HashMap;
use std::f32::consts::TAU;

use crate::avatars::Avatar;
use crate::data::Ability;
use crate::rig::Rig;
use crate::{AppState, InGameEntity, Phase, Roster, Session};

pub struct AuraPlugin;

impl Plugin for AuraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Auras>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (tick, shells, motes, vignette)
                    .chain()
                    .in_set(Phase::Present)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnExit(AppState::InGame), |mut a: ResMut<Auras>| a.0.clear());
    }
}

/// How long a quick cast keeps its aura, and how long its body animation runs.
const CAST_AURA: f32 = 1.2;
pub const CAST_TIME: f32 = 0.75;

/// Who is glowing right now, by player id.
#[derive(Resource, Default)]
pub struct Auras(HashMap<u8, Aura>);

#[derive(Clone, Copy)]
pub struct Aura {
    pub color: Color,
    pub life: f32,
    pub max: f32,
    /// The ability just used and seconds since (drives the body animation).
    pub cast: Option<(Ability, f32)>,
}

impl Auras {
    /// A player used an ability.
    pub fn cast(&mut self, player: u8, ability: Ability) {
        let color = ability.color();
        let a = self.0.entry(player).or_insert(Aura {
            color,
            life: 0.0,
            max: CAST_AURA,
            cast: None,
        });
        if a.life < CAST_AURA {
            a.color = color;
            a.life = CAST_AURA;
            a.max = CAST_AURA;
        }
        a.cast = Some((ability, 0.0));
    }

    /// Keeps a player's aura lit in `color` for `life` seconds.
    pub fn hold(&mut self, player: u8, color: Color, life: f32) {
        let a = self.0.entry(player).or_insert(Aura {
            color,
            life: 0.0,
            max: life,
            cast: None,
        });
        if life >= a.life {
            a.color = color;
            a.life = life;
            a.max = life;
        }
    }

    /// The aura colour and how strong it is (0..1), if any.
    pub fn glow(&self, player: u8) -> Option<(Color, f32)> {
        let a = self.0.get(&player)?;
        // Fades in quickly, out over the last half second.
        let k = (a.life / 0.5)
            .min(1.0)
            .min((a.max - a.life) / 0.15 + 0.3)
            .clamp(0.0, 1.0);
        (a.life > 0.0).then_some((a.color, k))
    }
}

#[derive(Resource)]
struct AuraAssets {
    shell: Handle<Mesh>,
    disc: Handle<Mesh>,
    mote: Handle<Mesh>,
    vignette: Handle<Image>,
}

/// The shell around one player's model, and its own material for the colour.
#[derive(Component)]
struct Shell {
    player: u8,
    mat: Handle<StandardMaterial>,
    floor: Handle<StandardMaterial>,
}

#[derive(Component)]
struct ShellOf;

#[derive(Component)]
struct Mote {
    vel: Vec3,
    life: f32,
    max: f32,
}

#[derive(Component)]
struct Vignette;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
) {
    // A soft glow that is clear in the middle of the screen and fills in
    // towards the edges.
    let n = 128u32;
    let mut data = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let u = (x as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let v = (y as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let d = (u * u * 0.8 + v * v).sqrt();
            let a = ((d - 0.7) / 0.55).clamp(0.0, 1.0);
            data.extend_from_slice(&[255, 255, 255, (a * a * 255.0) as u8]);
        }
    }
    let vignette = images.add(Image::new(
        Extent3d {
            width: n,
            height: n,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ));
    commands.insert_resource(AuraAssets {
        shell: meshes.add(
            Capsule3d::new(0.42, 1.05)
                .mesh()
                .rings(4)
                .latitudes(12)
                .longitudes(20),
        ),
        disc: meshes.add(Circle::new(0.8)),
        mote: meshes.add(Sphere::new(0.035).mesh().ico(1).unwrap()),
        vignette,
    });
}

fn additive(materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: Color::NONE,
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        double_sided: true,
        ..default()
    })
}

/// Counts auras down, keeps Overdrive lit, and plays cast poses on models.
fn tick(
    time: Res<Time>,
    roster: Res<Roster>,
    mut auras: ResMut<Auras>,
    mut avatars: Query<(&Avatar, &mut Rig)>,
) {
    let dt = time.delta_secs();
    for p in roster.0.values() {
        // Overdrive and Combat Stim keep the aura lit while they last.
        let buff = if p.overdrive > 0.0 {
            Some((Ability::Overdrive, p.overdrive))
        } else if p.stim > 0.0 {
            Some((Ability::CombatStim, p.stim))
        } else {
            None
        };
        if let Some((ability, left)) = buff {
            let color = ability.color();
            let a = auras.0.entry(p.id).or_insert(Aura {
                color,
                life: 0.0,
                max: left,
                cast: None,
            });
            a.color = color;
            if a.life < left {
                a.max = a.max.max(left);
            }
            a.life = left;
        }
    }
    for a in auras.0.values_mut() {
        a.life -= dt;
        if let Some((_, t)) = a.cast.as_mut() {
            *t += dt;
        }
        if a.cast.is_some_and(|c| c.1 > CAST_TIME) {
            a.cast = None;
        }
    }
    auras
        .0
        .retain(|id, a| (a.life > 0.0 || a.cast.is_some()) && roster.0.contains_key(id));
    for (avatar, mut rig) in &mut avatars {
        rig.cast = auras
            .0
            .get(&avatar.id)
            .and_then(|a| a.cast)
            .map(|(a, t): (Ability, f32)| (a.style(), t));
    }
}

/// A glowing shell around each glowing player, pulsing, with a pool of light
/// at their feet.
fn shells(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<AuraAssets>,
    auras: Res<Auras>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    avatars: Query<(Entity, &Avatar), Without<ShellOf>>,
    mut shells: Query<(&Shell, &mut Transform, &mut Visibility)>,
) {
    let t = time.elapsed_secs();
    for (e, avatar) in &avatars {
        let mat = additive(&mut materials);
        let floor = additive(&mut materials);
        commands.entity(e).insert(ShellOf).with_children(|c| {
            c.spawn((
                Shell {
                    player: avatar.id,
                    mat: mat.clone(),
                    floor: floor.clone(),
                },
                Transform::from_xyz(0.0, 0.95, 0.0),
                Visibility::Hidden,
            ))
            .with_children(|s| {
                s.spawn((
                    Mesh3d(assets.shell.clone()),
                    MeshMaterial3d(mat),
                    NotShadowCaster,
                ));
                s.spawn((
                    Mesh3d(assets.disc.clone()),
                    MeshMaterial3d(floor),
                    Transform::from_xyz(0.0, -0.92, 0.0)
                        .with_rotation(Quat::from_rotation_x(-TAU / 4.0)),
                    NotShadowCaster,
                ));
            });
        });
    }
    for (shell, mut tf, mut vis) in &mut shells {
        let Some((color, k)) = auras.glow(shell.player) else {
            *vis = Visibility::Hidden;
            continue;
        };
        *vis = Visibility::Inherited;
        let pulse = 0.5 + 0.5 * (t * 7.0).sin();
        tf.scale =
            Vec3::new(1.0 + 0.05 * pulse, 1.0 + 0.02 * pulse, 1.0 + 0.05 * pulse) * (0.9 + 0.1 * k);
        let lin = LinearRgba::from(color);
        if let Some(m) = materials.get_mut(&shell.mat) {
            m.base_color = Color::LinearRgba(lin * (0.18 + 0.1 * pulse) * k);
        }
        if let Some(m) = materials.get_mut(&shell.floor) {
            m.base_color = Color::LinearRgba(lin * (0.35 + 0.15 * pulse) * k);
        }
    }
}

/// Sparks rising around glowing players (you see your own around you too).
fn motes(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<AuraAssets>,
    auras: Res<Auras>,
    roster: Res<Roster>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mats: Local<HashMap<u32, Handle<StandardMaterial>>>,
    mut acc: Local<f32>,
    mut live: Query<(Entity, &mut Mote, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (e, mut m, mut tf) in &mut live {
        m.life -= dt;
        if m.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        tf.translation += m.vel * dt;
        m.vel *= 1.0 - dt * 0.8;
        tf.scale = Vec3::splat((m.life / m.max).min(1.0));
    }
    *acc += dt;
    if *acc < 0.04 {
        return;
    }
    *acc = 0.0;
    let mut rng = rand::thread_rng();
    for p in roster.0.values().filter(|p| p.alive) {
        let Some((color, k)) = auras.glow(p.id) else {
            continue;
        };
        if !rng.gen_bool((0.4 + 0.6 * k as f64).min(1.0)) {
            continue;
        }
        // One shared material per colour.
        let s = color.to_srgba();
        let key =
            ((s.red * 31.0) as u32) << 10 | ((s.green * 31.0) as u32) << 5 | (s.blue * 31.0) as u32;
        let mat = mats
            .entry(key)
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: color,
                    emissive: LinearRgba::from(color) * 8.0,
                    unlit: true,
                    ..default()
                })
            })
            .clone();
        for _ in 0..2 {
            let ang = rng.gen_range(0.0..TAU);
            let r = rng.gen_range(0.45..0.75);
            let pos = p.feet() + Vec3::new(ang.cos() * r, rng.gen_range(0.0..1.4), ang.sin() * r);
            let tan = Vec3::new(-ang.sin(), 0.0, ang.cos());
            let life = rng.gen_range(0.5..0.9);
            commands.spawn((
                InGameEntity,
                Mote {
                    vel: Vec3::Y * rng.gen_range(1.0..2.2) + tan * 0.8,
                    life,
                    max: life,
                },
                Mesh3d(assets.mote.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(pos),
                NotShadowCaster,
            ));
        }
    }
}

/// In first person, your own aura tints the edges of the screen.
fn vignette(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<AuraAssets>,
    auras: Res<Auras>,
    session: Res<Session>,
    mut nodes: Query<&mut ImageNode, With<Vignette>>,
) {
    let Ok(mut node) = nodes.single_mut() else {
        commands.spawn((
            InGameEntity,
            Vignette,
            ImageNode::new(assets.vignette.clone()).with_color(Color::NONE),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
            GlobalZIndex(-5),
        ));
        return;
    };
    let pulse = 0.5 + 0.5 * (time.elapsed_secs() * 6.0).sin();
    node.color = match auras.glow(session.my_id) {
        Some((c, k)) => c.with_alpha(k * (0.22 + 0.08 * pulse)),
        None => Color::NONE,
    };
}
