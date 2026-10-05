//! Look-development views for checking the art: `rust-fps lookdev` starts a
//! sandbox match (no waves) with the camera fixed at a set view and the HUD
//! hidden, for side-by-side screenshots. Options: `--map 0-2`, `--night`,
//! `--view spawn|street|overhead|lineup|side|guns|heads|markers`. The lineup views show every
//! hero, one of each zombie, every gun and a wall section in a yard off to
//! the side of the map, under that map's light.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::data::Character;
use crate::kit::{c, vertex_material, Kit};
use crate::maps::CurrentMap;
use crate::net::Launch;
use crate::player::LocalPlayer;
use crate::rig::{Model, RigAssets};
use crate::{AppState, InGameEntity, MatchState};

/// Which view the camera is held at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    /// Where you spawn, at eye level (the normal game view).
    Spawn,
    /// Outside at eye level, at the extraction point looking into the map.
    Street,
    /// High up over a corner of the map.
    Overhead,
    /// Heroes, zombies, guns and a wall, from the front.
    Lineup,
    /// The same, turned side on.
    Side,
    /// Close up on the guns on the bench.
    Guns,
    /// Close up on three heroes' heads (Tinker, Blaze, Valkyrie).
    Heads,
    /// Every ground marker shape, from above the yard.
    Markers,
}

impl View {
    pub fn parse(s: &str) -> Option<View> {
        Some(match s {
            "spawn" => View::Spawn,
            "street" => View::Street,
            "overhead" => View::Overhead,
            "lineup" => View::Lineup,
            "side" => View::Side,
            "guns" => View::Guns,
            "heads" => View::Heads,
            "markers" => View::Markers,
            _ => return None,
        })
    }

    fn lineup(self) -> bool {
        !matches!(self, View::Spawn | View::Street | View::Overhead)
    }
}

/// Where the lineup yard is (well outside every map).
const YARD: Vec3 = Vec3::new(0.0, 0.0, 400.0);

pub struct LookdevPlugin;

impl Plugin for LookdevPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, night_before_start.before(crate::ui::autostart))
            .add_systems(OnEnter(AppState::InGame), spawn_yard.after(crate::game::start_match))
            .add_systems(Update, slam_demo.run_if(in_state(AppState::InGame)))
            .add_systems(
                PostUpdate,
                (hold_camera, hide_ui, sample_markers)
                    .before(bevy::transform::TransformSystem::TransformPropagate)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

fn view(launch: &Launch) -> Option<View> {
    launch.lookdev
}

/// The night switch lives in the match state, which the sandbox request
/// resets, so set it again until the match starts.
fn night_before_start(launch: Res<Launch>, mut state: ResMut<MatchState>) {
    if view(&launch).is_some() && !state.started && state.night != launch.night {
        state.night = launch.night;
    }
}

fn hold_camera(
    launch: Res<Launch>,
    map: Option<Res<CurrentMap>>,
    mut cam: Single<(&mut Transform, &LocalPlayer, &Children)>,
    mut vis: Query<&mut Visibility>,
) {
    let (Some(v), Some(map)) = (view(&launch), map) else {
        return;
    };
    let (tf, _, children) = &mut *cam;
    if v == View::Spawn {
        return;
    }
    // No first-person gun away from the player.
    for c in children.iter() {
        if let Ok(mut vis) = vis.get_mut(c) {
            *vis = Visibility::Hidden;
        }
    }
    let (eye, target) = match v {
        View::Spawn => return,
        View::Street => (map.0.extraction + Vec3::Y * 1.6, Vec3::new(0.0, 1.4, 0.0)),
        View::Overhead => {
            let h = map.0.half;
            (Vec3::new(h * 0.75, h * 0.6, h * 0.75), Vec3::new(-h * 0.1, 0.0, -h * 0.1))
        }
        View::Guns => (
            YARD + Vec3::new(-1.6, 1.05, -0.25),
            YARD + Vec3::new(-1.6, 0.6, -1.15),
        ),
        View::Heads => (
            YARD + Vec3::new(-1.1, 1.62, 1.35),
            YARD + Vec3::new(-1.1, 1.5, 0.3),
        ),
        View::Markers => (
            YARD + Vec3::new(0.0, 6.5, 9.5),
            YARD + Vec3::new(0.0, 0.0, 3.2),
        ),
        View::Lineup | View::Side => (
            YARD + Vec3::new(0.0, 1.3, 4.6),
            YARD + Vec3::new(0.0, 1.0, 0.0),
        ),
    };
    **tf = Transform::from_translation(eye).looking_at(target, Vec3::Y);
}

/// One of each marker shape: previews on the left, warnings on the right
/// (counting down over and over, then flashing).
fn sample_markers(
    launch: Res<Launch>,
    time: Res<Time>,
    mut markers: ResMut<crate::markers::Markers>,
) {
    use crate::markers::{Marker, Shape, DANGER, FLASH, PREVIEW};
    if view(&launch) != Some(View::Markers) {
        return;
    }
    let at = |x: f32, z: f32| YARD + Vec3::new(x, 0.0, z);
    markers.push(Marker::new(at(-5.0, 2.5), Shape::Circle { radius: 1.6 }, PREVIEW));
    markers.push(Marker::new(
        at(-1.8, 2.5),
        Shape::Donut {
            inner: 1.0,
            outer: 1.6,
        },
        PREVIEW,
    ));
    markers.push(
        Marker::new(
            at(-5.5, 6.0),
            Shape::Cone {
                radius: 2.4,
                half_angle: 0.6,
            },
            PREVIEW,
        )
        .facing(Vec3::new(1.0, 0.0, -1.0)),
    );
    markers.push(
        Marker::new(
            at(-3.5, 6.5),
            Shape::Line {
                length: 3.5,
                half_width: 0.5,
            },
            PREVIEW,
        )
        .facing(Vec3::X),
    );
    // Countdowns, each at a different point.
    let cycle = 1.5 + FLASH;
    for (i, x) in [1.8, 5.0].into_iter().enumerate() {
        let t = (time.elapsed_secs() + i as f32 * 0.8) % cycle;
        let m = Marker::new(at(x, 2.5), Shape::Circle { radius: 1.5 }, DANGER);
        markers.push(if t < 1.5 {
            m.fill(t / 1.5)
        } else {
            m.fill(1.0).flash(1.0 - (t - 1.5) / FLASH)
        });
    }
    // Fixed fills for the screenshot: half way, and in the last fifth.
    for (fill, x) in [(0.5, 1.8), (0.9, 5.0)] {
        markers.push(
            Marker::new(
                at(x, 6.0),
                Shape::Line {
                    length: 3.0,
                    half_width: 0.6,
                },
                DANGER,
            )
            .facing(Vec3::Z)
            .fill(fill),
        );
    }
}

fn hide_ui(launch: Res<Launch>, mut roots: Query<&mut Visibility, (With<Node>, Without<ChildOf>)>) {
    if view(&launch).is_none() {
        return;
    }
    for mut vis in &mut roots {
        if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}

/// The lineup: a wall section at the back, a row of guns on a bench, the
/// heroes on the left and the zombies on the right.
fn spawn_yard(
    mut commands: Commands,
    launch: Res<Launch>,
    rigs: Res<RigAssets>,
    guns: Res<crate::gunmodels::GunAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(v) = view(&launch).filter(|v| v.lineup()) else {
        return;
    };
    let mat = materials.add(vertex_material(0.85, 0.0));
    let mut k = Kit::new();
    // Paving, and a wall with siding, a window and a door frame behind.
    k.cuboid(Vec3::new(0.0, -0.05, 0.0), Vec3::new(16.0, 0.1, 10.0), c(0.55, 0.53, 0.5));
    if v == View::Markers {
        // More ground towards the camera.
        k.cuboid(Vec3::new(0.0, -0.05, 8.5), Vec3::new(16.0, 0.1, 7.0), c(0.5, 0.52, 0.47));
    }
    let back = -2.2;
    k.cuboid(Vec3::new(0.0, 0.15, back), Vec3::new(14.0, 0.3, 0.3), c(0.45, 0.43, 0.42));
    for i in 0..18 {
        let y = 0.35 + i as f32 * 0.16;
        k.cuboid(
            Vec3::new(0.0, y, back),
            Vec3::new(14.0, 0.15, 0.12 + (i % 2) as f32 * 0.01),
            c(0.62, 0.7, 0.76),
        );
    }
    k.cuboid(Vec3::new(-3.0, 1.7, back + 0.11), Vec3::new(1.2, 1.1, 0.05), c(0.18, 0.22, 0.26));
    k.cuboid(Vec3::new(-3.0, 1.7, back + 0.08), Vec3::new(1.35, 1.25, 0.04), c(0.92, 0.9, 0.86));
    k.cuboid(Vec3::new(3.5, 1.1, back + 0.11), Vec3::new(1.0, 2.2, 0.05), c(0.42, 0.25, 0.14));
    k.cuboid(Vec3::new(3.5, 1.15, back + 0.08), Vec3::new(1.15, 2.3, 0.04), c(0.92, 0.9, 0.86));
    k.wedge(
        Vec3::new(0.0, 3.6, back - 0.3),
        Vec3::new(14.4, 0.9, 1.2),
        Quat::IDENTITY,
        c(0.35, 0.3, 0.3),
    );
    // Bench for the guns.
    k.cuboid(Vec3::new(0.0, 0.45, -1.2), Vec3::new(9.0, 0.06, 0.7), c(0.5, 0.36, 0.22));
    for x in [-4.2, 0.0, 4.2] {
        k.cuboid(Vec3::new(x, 0.21, -1.2), Vec3::new(0.08, 0.42, 0.6), c(0.3, 0.3, 0.32));
    }
    commands.spawn((
        InGameEntity,
        Mesh3d(meshes.add(k.build_or_empty())),
        MeshMaterial3d(mat),
        Transform::from_translation(YARD),
    ));

    // Every gun, lying across the bench, muzzles to the right.
    let n = crate::data::GUNS.len();
    for id in 0..n as u8 {
        let x = -4.0 + 8.0 * (id as f32 + 0.5) / n as f32;
        commands
            .spawn((
                InGameEntity,
                Transform::from_translation(YARD + Vec3::new(x, 0.62, -1.15))
                    .with_rotation(Quat::from_rotation_y(-FRAC_PI_2) * Quat::from_rotation_x(0.5))
                    .with_scale(Vec3::splat(1.0)),
                Visibility::default(),
            ))
            .with_children(|p| {
                crate::gunmodels::spawn_gun(
                    p,
                    &guns,
                    id,
                    crate::data::Attach::NONE,
                    guns.skin(0),
                    false,
                    Some(crate::outline::Outline::Prop),
                )
            });
    }

    // Heroes then zombies, facing the camera (or side on).
    let turn = if v == View::Side { -FRAC_PI_2 } else { PI };
    let models: Vec<(Model, f32)> = Character::ALL
        .iter()
        .map(|ch| (Model::Hero(*ch), 1.0))
        .chain([
            (Model::Walker(0), 1.0),
            (Model::Walker(1), 1.0),
            (Model::Walker(2), 1.0),
            (Model::Spitter, crate::sim::enemy_scale(crate::NetKind::Shooter)),
            (Model::Brute, crate::sim::enemy_scale(crate::NetKind::Brute)),
        ])
        .collect();
    let count = models.len();
    for (i, (model, scale)) in models.into_iter().enumerate() {
        let x = -6.0 + 12.0 * (i as f32 + 0.5) / count as f32;
        let root = commands
            .spawn((
                InGameEntity,
                Transform::from_translation(YARD + Vec3::new(x, 0.0, 0.3))
                    .with_rotation(Quat::from_rotation_y(turn))
                    .with_scale(Vec3::splat(scale)),
                Visibility::default(),
            ))
            .id();
        let body = (!matches!(model, Model::Hero(_)))
            .then(|| materials.add(vertex_material(0.75, 0.05)));
        crate::rig::spawn_rig_with(&mut commands, &rigs, root, model, None, body);
        if v == View::Markers && model == Model::Brute {
            // Slams over and over, to show its warning.
            commands.entity(root).insert((
                SlamDemo,
                crate::Replicated {
                    id: u32::MAX,
                    kind: crate::NetKind::Brute,
                },
                crate::EnemyStatus::default(),
            ));
        }
    }
}

#[derive(Component)]
struct SlamDemo;

fn slam_demo(mut rigs: Query<&mut crate::rig::Rig, With<SlamDemo>>) {
    for mut rig in &mut rigs {
        if rig.swing > 1.6 {
            rig.swing = 0.0;
        }
    }
}
