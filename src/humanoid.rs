//! Person-shaped models built from boxes, with a simple walk cycle: legs and
//! arms swing with movement speed. Used for other players and for enemies.

use bevy::prelude::*;

use crate::Phase;

pub struct HumanoidPlugin;

impl Plugin for HumanoidPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup_meshes)
            .add_systems(Update, (animate.in_set(Phase::Present), fill_gun_mounts));
    }
}

#[derive(Resource)]
pub struct HumanoidMeshes {
    leg: Handle<Mesh>,
    torso: Handle<Mesh>,
    arm: Handle<Mesh>,
    head: Handle<Mesh>,
    eye: Handle<Mesh>,
    visor: Handle<Mesh>,
}

fn setup_meshes(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(HumanoidMeshes {
        leg: meshes.add(Cuboid::new(0.22, 0.88, 0.26)),
        torso: meshes.add(Cuboid::new(0.6, 0.72, 0.32)),
        arm: meshes.add(Cuboid::new(0.17, 0.7, 0.2)),
        head: meshes.add(Cuboid::new(0.34, 0.36, 0.34)),
        eye: meshes.add(Cuboid::new(0.08, 0.05, 0.02)),
        visor: meshes.add(Cuboid::new(0.3, 0.1, 0.04)),
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Pose {
    /// Arms hanging, swinging when walking.
    Normal,
    /// Holding a gun forward.
    Rifle,
    /// Zombie arms reaching forward.
    Reach,
}

pub struct Look {
    pub skin: Handle<StandardMaterial>,
    pub shirt: Handle<StandardMaterial>,
    pub pants: Handle<StandardMaterial>,
    pub eyes: Handle<StandardMaterial>,
    /// (gun id, skin) held in the right hand.
    pub gun: Option<(u8, u8)>,
    pub visor: bool,
    pub pose: Pose,
}

/// Animation state on the model's root (whose origin is at the feet).
#[derive(Component)]
pub struct Walker {
    phase: f32,
    last: Vec3,
    speed: f32,
    pose: Pose,
    /// Extra speed of the walk cycle (fast zombies look frantic).
    pub cadence: f32,
}

#[derive(Component)]
struct Limb {
    owner: Entity,
    side: f32,
    leg: bool,
}

/// Adds the body parts as children of `root`.
/// Where a model holds its gun. Set `want` and the gun model appears.
#[derive(Component)]
pub struct GunMount {
    pub want: Option<u8>,
    pub skin: u8,
    shown: Option<(u8, u8)>,
}

fn fill_gun_mounts(
    mut commands: Commands,
    guns: Option<Res<crate::gunmodels::GunAssets>>,
    mut mounts: Query<(Entity, &mut GunMount)>,
) {
    let Some(guns) = guns else { return };
    for (e, mut m) in &mut mounts {
        let want = m.want.map(|g| (g, m.skin));
        if m.shown == want {
            continue;
        }
        m.shown = want;
        commands.entity(e).despawn_related::<Children>();
        if let Some((gun, skin)) = want {
            commands
                .entity(e)
                .with_children(|p| crate::gunmodels::spawn_gun(p, &guns, gun, guns.skin(skin), false));
        }
    }
}

/// Adds the body parts as children of `root`. Returns the gun mount, if
/// the model holds a gun.
pub fn build(commands: &mut Commands, root: Entity, meshes: &HumanoidMeshes, look: Look) -> Option<Entity> {
    let mut mount = None;
    commands.entity(root).insert(Walker {
        phase: 0.0,
        last: Vec3::ZERO,
        speed: 0.0,
        pose: look.pose,
        cadence: 1.0,
    });
    commands.entity(root).with_children(|p| {
        for side in [-1.0f32, 1.0] {
            // Hip pivot with the leg hanging below it.
            p.spawn((
                Limb {
                    owner: root,
                    side,
                    leg: true,
                },
                Transform::from_xyz(side * 0.15, 0.9, 0.0),
                Visibility::default(),
            ))
            .with_children(|hip| {
                hip.spawn((
                    Mesh3d(meshes.leg.clone()),
                    MeshMaterial3d(look.pants.clone()),
                    Transform::from_xyz(0.0, -0.44, 0.0),
                ));
            });
            // Shoulder pivot with the arm hanging below it.
            p.spawn((
                Limb {
                    owner: root,
                    side,
                    leg: false,
                },
                Transform::from_xyz(side * 0.39, 1.56, 0.0),
                Visibility::default(),
            ))
            .with_children(|sh| {
                sh.spawn((
                    Mesh3d(meshes.arm.clone()),
                    MeshMaterial3d(look.shirt.clone()),
                    Transform::from_xyz(0.0, -0.3, 0.0),
                ));
                sh.spawn((
                    Mesh3d(meshes.eye.clone()),
                    MeshMaterial3d(look.skin.clone()),
                    Transform::from_xyz(0.0, -0.66, 0.0).with_scale(Vec3::new(2.0, 2.0, 8.0)),
                ));
                if side > 0.0 {
                    if let Some((gun, skin)) = look.gun {
                        mount = Some(
                            sh.spawn((
                                GunMount {
                                    want: Some(gun),
                                    skin,
                                    shown: None,
                                },
                                Transform::from_xyz(-0.02, -0.68, 0.0)
                                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                                    .with_scale(Vec3::splat(1.25)),
                                Visibility::default(),
                            ))
                            .id(),
                        );
                    }
                }
            });
        }
        p.spawn((
            Mesh3d(meshes.torso.clone()),
            MeshMaterial3d(look.shirt.clone()),
            Transform::from_xyz(0.0, 1.25, 0.0),
        ));
        p.spawn((
            Mesh3d(meshes.head.clone()),
            MeshMaterial3d(look.skin.clone()),
            Transform::from_xyz(0.0, 1.8, 0.0),
        ));
        if look.visor {
            p.spawn((
                Mesh3d(meshes.visor.clone()),
                MeshMaterial3d(look.eyes.clone()),
                Transform::from_xyz(0.0, 1.84, -0.18),
            ));
        } else {
            for x in [-0.08, 0.08] {
                p.spawn((
                    Mesh3d(meshes.eye.clone()),
                    MeshMaterial3d(look.eyes.clone()),
                    Transform::from_xyz(x, 1.84, -0.175),
                ));
            }
        }
    });
    mount
}

fn animate(
    time: Res<Time>,
    mut walkers: Query<(&mut Walker, &GlobalTransform)>,
    mut limbs: Query<(&Limb, &mut Transform)>,
) {
    let dt = time.delta_secs().max(1e-4);
    for (mut w, gt) in &mut walkers {
        let pos = gt.translation();
        let moved = (pos - w.last).with_y(0.0).length() / dt;
        w.last = pos;
        // Smooth out jittery network updates.
        let target = if moved > 30.0 { 0.0 } else { moved };
        w.speed += (target - w.speed) * (1.0 - (-10.0 * dt).exp());
        let speed = w.speed;
        w.phase += speed * dt * 1.9 * w.cadence;
    }
    for (limb, mut tf) in &mut limbs {
        let Ok((w, _)) = walkers.get(limb.owner) else {
            continue;
        };
        let amp = (w.speed / 4.0).min(1.0);
        let swing = (w.phase + if limb.side > 0.0 { 0.0 } else { std::f32::consts::PI }).sin();
        let angle = if limb.leg {
            swing * 0.7 * amp
        } else {
            match w.pose {
                Pose::Normal => -swing * 0.6 * amp,
                // Arms forward (-X rotation lifts them towards -Z).
                Pose::Rifle => 1.35 + swing * 0.05 * amp,
                Pose::Reach => 1.45 + swing * 0.2 * amp,
            }
        };
        tf.rotation = Quat::from_rotation_x(angle);
    }
}
