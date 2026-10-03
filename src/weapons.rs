//! The local player's guns: two weapon slots, ammo, reloading, firing modes,
//! the first-person gun model (in your equipped skin) and hit markers.

use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::Rng;

use crate::config::{Action, InputExt, Settings};
use crate::data::{gun_def, has_perk, skin_material, FireMode, GunClass, Perk};
use crate::fx::{Fx, FxQueue};
use crate::game::Paused;
use crate::physics::{collect_boxes, trace_shot};
use crate::player::{can_act, LocalPlayer};
use crate::{
    AppState, Collider, Enemy, MatchState, Phase, Replicated, Roster, Session, Shot, ShotQueue,
};

pub const GUN_RANGE: f32 = 120.0;

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Loadout>()
            .add_systems(Startup, spawn_viewmodel)
            .add_systems(
                Update,
                (sync_loadout, switch_weapon, reload, fire)
                    .chain()
                    .after(crate::player::movement)
                    .in_set(Phase::Local),
            )
            .add_systems(Update, update_viewmodel.in_set(Phase::Present))
            .add_systems(OnEnter(AppState::InGame), reset_loadout);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GunState {
    pub id: u8,
    pub mag: u32,
    pub reserve: u32,
}

impl GunState {
    fn fresh(id: u8) -> Self {
        let d = gun_def(id);
        Self {
            id,
            mag: d.mag,
            reserve: d.reserve,
        }
    }
}

#[derive(Resource, Default)]
pub struct Loadout {
    pub slots: [Option<GunState>; 2],
    pub active: usize,
    fire_cd: f32,
    pub reload: f32,
    burst_left: u32,
    switch_cd: f32,
    recoil: f32,
    flash: f32,
    last_ammo_seq: u32,
    last_spawn_seq: u32,
    last_round: u32,
    /// Seconds left to show the hit marker, and whether it was a headshot.
    pub hitmarker: f32,
    pub headshot: bool,
    shown: Option<(u8, u8)>,
}

impl Loadout {
    pub fn current(&self) -> Option<&GunState> {
        self.slots[self.active].as_ref()
    }
}

fn reset_loadout(mut loadout: ResMut<Loadout>) {
    *loadout = Loadout::default();
}

#[derive(Component)]
struct ViewModel;

#[derive(Component)]
struct MuzzleFlash;

#[derive(Component)]
struct GunPart;

fn spawn_viewmodel(mut commands: Commands, camera: Single<Entity, With<LocalPlayer>>) {
    commands.entity(*camera).with_children(|cam| {
        cam.spawn((
            ViewModel,
            Transform::from_xyz(0.25, -0.22, -0.45),
            Visibility::Hidden,
        ))
        .with_children(|v| {
            v.spawn((
                MuzzleFlash,
                PointLight {
                    intensity: 0.0,
                    color: Color::srgb(1.0, 0.8, 0.4),
                    range: 12.0,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.03, -0.6),
            ));
        });
    });
}

/// Keeps our guns in step with what the host says we hold (mystery box,
/// respawns) and refills ammo on Max Ammo.
fn sync_loadout(
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    mut loadout: ResMut<Loadout>,
) {
    let Some(me) = roster.me(&session) else {
        return;
    };
    if me.spawn_seq != loadout.last_spawn_seq {
        loadout.last_spawn_seq = me.spawn_seq;
        loadout.slots = [None, None];
        loadout.reload = 0.0;
    }
    for i in 0..2 {
        let want = me.guns[i];
        let have = loadout.slots[i].map(|g| g.id);
        if want != have {
            loadout.slots[i] = want.map(GunState::fresh);
            if want.is_some() {
                // A new gun from the box goes straight into your hands.
                loadout.active = i;
                loadout.reload = 0.0;
            }
        }
    }
    if loadout.slots[loadout.active].is_none() {
        loadout.active = if loadout.slots[0].is_some() { 0 } else { 1 };
    }
    // Spare ammo is topped up at the start of every round (Max Ammo also
    // refills the magazine), so nobody gets stuck with an empty gun.
    if state.round != loadout.last_round {
        loadout.last_round = state.round;
        for g in loadout.slots.iter_mut().flatten() {
            g.reserve = g.reserve.max(gun_def(g.id).reserve);
        }
    }
    if state.max_ammo_seq != loadout.last_ammo_seq {
        loadout.last_ammo_seq = state.max_ammo_seq;
        for g in loadout.slots.iter_mut().flatten() {
            let d = gun_def(g.id);
            g.reserve = d.reserve;
            g.mag = d.mag;
        }
    }
}

fn switch_weapon(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
    settings: Res<Settings>,
    session: Res<Session>,
    mut roster: ResMut<Roster>,
    mut loadout: ResMut<Loadout>,
) {
    loadout.switch_cd -= time.delta_secs();
    let mut target = None;
    if keys.just_pressed(KeyCode::Digit1) {
        target = Some(0);
    } else if keys.just_pressed(KeyCode::Digit2) {
        target = Some(1);
    } else if keys.tapped(&settings, Action::SwapWeapon) || scroll.delta.y.abs() > 0.0 {
        target = Some(1 - loadout.active);
    }
    if let Some(t) = target {
        if t != loadout.active && loadout.slots[t].is_some() && loadout.switch_cd <= 0.0 {
            loadout.active = t;
            loadout.reload = 0.0;
            loadout.burst_left = 0;
            loadout.switch_cd = 0.25;
            loadout.fire_cd = loadout.fire_cd.max(0.25);
        }
    }
    let active = loadout.active as u8;
    if let Some(me) = roster.0.get_mut(&session.my_id) {
        me.active_slot = active;
    }
}

fn reload(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    session: Res<Session>,
    roster: Res<Roster>,
    mut loadout: ResMut<Loadout>,
) {
    let perks = roster.me(&session).map(|m| m.perks).unwrap_or(0);
    let speed = if has_perk(perks, Perk::QuickHands) { 2.0 } else { 1.0 };
    let active = loadout.active;
    let Some(gun) = loadout.slots[active] else {
        return;
    };
    let def = gun_def(gun.id);
    if keys.tapped(&settings, Action::Reload)
        && loadout.reload <= 0.0
        && gun.mag < def.mag
        && gun.reserve > 0
    {
        loadout.reload = def.reload / speed;
    }
    if loadout.reload > 0.0 {
        loadout.reload -= time.delta_secs();
        if loadout.reload <= 0.0 {
            loadout.reload = 0.0;
            if let Some(g) = loadout.slots[active].as_mut() {
                let take = (def.mag - g.mag).min(g.reserve);
                g.mag += take;
                g.reserve -= take;
            }
        }
    }
}

fn fire(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    paused: Res<Paused>,
    mut loadout: ResMut<Loadout>,
    mut shots: ResMut<ShotQueue>,
    mut fx: ResMut<FxQueue>,
    player: Single<(&Transform, &mut LocalPlayer)>,
    enemies: Query<(Entity, &Transform, &Replicated), With<Enemy>>,
    colliders: Query<(&Transform, &Collider)>,
) {
    let dt = time.delta_secs();
    loadout.fire_cd -= dt;
    loadout.hitmarker -= dt;
    loadout.recoil = (loadout.recoil - dt * 8.0).max(0.0);
    loadout.flash -= dt;

    if !can_act(&session, &roster, &state, &paused, &window) {
        loadout.burst_left = 0;
        return;
    }
    let Some(me) = roster.me(&session) else {
        return;
    };
    let active = loadout.active;
    let Some(gun) = loadout.slots[active] else {
        return;
    };
    let def = gun_def(gun.id);
    let overdrive = me.overdrive > 0.0;

    let wants = match def.mode {
        FireMode::Auto => mouse.pressed(MouseButton::Left),
        FireMode::Semi => mouse.just_pressed(MouseButton::Left),
        FireMode::Burst => mouse.just_pressed(MouseButton::Left) || loadout.burst_left > 0,
    };
    if !wants || loadout.fire_cd > 0.0 || loadout.reload > 0.0 {
        return;
    }
    if gun.mag == 0 && !overdrive {
        loadout.burst_left = 0;
        if gun.reserve > 0 {
            let speed = if has_perk(me.perks, Perk::QuickHands) { 2.0 } else { 1.0 };
            loadout.reload = def.reload / speed;
        }
        return;
    }

    let mut interval = 60.0 / def.rpm;
    if has_perk(me.perks, Perk::RapidFire) {
        interval /= 1.33;
    }
    if overdrive {
        interval /= 1.5;
    }
    if def.mode == FireMode::Burst {
        if loadout.burst_left == 0 {
            loadout.burst_left = 3;
        }
        loadout.burst_left -= 1;
        loadout.fire_cd = if loadout.burst_left == 0 { 0.3 } else { interval * 0.6 };
    } else {
        loadout.fire_cd = interval;
    }
    if !overdrive {
        if let Some(g) = loadout.slots[active].as_mut() {
            g.mag -= 1;
        }
    }
    loadout.recoil = 1.0;
    loadout.flash = 0.05;

    let (cam, mut p) = player.into_inner();
    let origin = cam.translation;
    let forward = cam.forward().as_vec3();
    let right = cam.right().as_vec3();
    let up = cam.up().as_vec3();
    let mut spread = def.spread;
    if def.pellets == 1 {
        if !p.on_ground {
            spread += 0.03;
        } else if p.horizontal_speed() > 1.0 {
            spread += if p.sprinting { 0.015 } else { 0.008 };
        }
        if p.crouching {
            spread *= 0.6;
        }
    }
    let kick = match def.class {
        GunClass::Sniper => 0.05,
        GunClass::Shotgun => 0.045,
        GunClass::Lmg => 0.012,
        GunClass::Pistol => 0.018,
        _ => 0.01,
    };
    p.kick += kick;

    let boxes = collect_boxes(colliders.iter());
    let muzzle = origin + cam.rotation * Vec3::new(0.25, -0.18, -0.9);
    let mut rng = rand::thread_rng();
    let mut any_hit = false;
    let mut head = false;
    for _ in 0..def.pellets {
        let a = rng.gen_range(0.0..std::f32::consts::TAU);
        let r = spread * rng.gen_range(0.0f32..1.0).sqrt();
        let dir = (forward + right * a.cos() * r + up * a.sin() * r).normalize();
        let hit = trace_shot(
            origin,
            dir,
            GUN_RANGE,
            &boxes,
            enemies
                .iter()
                .map(|(e, t, r)| (e, t.translation, crate::sim::enemy_scale(r.kind))),
        );
        if let Some((_, h)) = hit.enemy {
            any_hit = true;
            head |= h;
        }
        fx.0.push(Fx::Tracer {
            shooter: session.my_id,
            a: muzzle.to_array(),
            b: (origin + dir * hit.dist).to_array(),
        });
        shots.0.push((
            session.my_id,
            Shot {
                origin: origin.to_array(),
                dir: dir.to_array(),
                gun: gun.id,
            },
        ));
    }
    if any_hit {
        loadout.hitmarker = 0.12;
        loadout.headshot = head;
    }
}

/// Rebuilds the first-person gun when you switch guns or skins, and animates
/// recoil, reload and the muzzle flash.
fn update_viewmodel(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<State<AppState>>,
    mut loadout: ResMut<Loadout>,
    view: Single<(Entity, &mut Transform, &mut Visibility), With<ViewModel>>,
    parts: Query<Entity, With<GunPart>>,
    mut flash: Single<&mut PointLight, With<MuzzleFlash>>,
    player: Single<&LocalPlayer>,
) {
    let (view_entity, mut tf, mut vis) = view.into_inner();
    let me = roster.me(&session);
    let in_game = *state.get() == AppState::InGame;
    let alive = me.is_some_and(|m| m.alive);
    let gun = loadout.current().map(|g| g.id);
    *vis = if in_game && alive && gun.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let Some(gun) = gun else { return };
    let skin = me.map(|m| m.skin).unwrap_or(0);

    if loadout.shown != Some((gun, skin)) {
        loadout.shown = Some((gun, skin));
        for e in &parts {
            commands.entity(e).despawn();
        }
        let body = materials.add(skin_material(skin));
        let dark = materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.08, 0.09),
            perceptual_roughness: 0.6,
            ..default()
        });
        let def = gun_def(gun);
        let accent = if def.rare {
            let c = if gun == 21 {
                LinearRgba::rgb(0.3, 1.0, 0.4)
            } else {
                LinearRgba::rgb(0.5, 0.6, 1.0)
            };
            materials.add(StandardMaterial {
                base_color: Color::from(c),
                emissive: c * 6.0,
                ..default()
            })
        } else {
            dark.clone()
        };
        let mut part = |size: Vec3, pos: Vec3, mat: &Handle<StandardMaterial>| {
            commands.entity(view_entity).with_children(|v| {
                v.spawn((
                    GunPart,
                    Mesh3d(meshes.add(Cuboid::from_size(size))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_translation(pos),
                ));
            });
        };
        let (len, h) = match def.class {
            GunClass::Pistol => (0.26, 0.12),
            GunClass::Smg => (0.42, 0.13),
            GunClass::Rifle => (0.55, 0.13),
            GunClass::Shotgun => (0.55, 0.12),
            GunClass::Lmg => (0.62, 0.16),
            GunClass::Sniper => (0.7, 0.12),
            GunClass::Wonder => (0.5, 0.18),
        };
        part(Vec3::new(0.09, h, len), Vec3::new(0.0, 0.0, -len / 2.0 + 0.1), &body);
        // Barrel.
        let barrel = match def.class {
            GunClass::Pistol => 0.06,
            GunClass::Sniper => 0.35,
            GunClass::Shotgun => 0.25,
            _ => 0.18,
        };
        let bw = if def.class == GunClass::Shotgun { 0.06 } else { 0.035 };
        part(
            Vec3::new(bw, bw, barrel),
            Vec3::new(0.0, 0.02, -len + 0.1 - barrel / 2.0),
            &dark,
        );
        // Grip.
        part(Vec3::new(0.07, 0.15, 0.08), Vec3::new(0.0, -0.12, 0.02), &dark);
        match def.class {
            GunClass::Smg | GunClass::Rifle => {
                part(Vec3::new(0.06, 0.18, 0.08), Vec3::new(0.0, -0.13, -0.18), &dark);
                part(Vec3::new(0.07, 0.1, 0.22), Vec3::new(0.0, -0.02, 0.2), &body);
            }
            GunClass::Lmg => {
                part(Vec3::new(0.14, 0.14, 0.16), Vec3::new(0.0, -0.13, -0.22), &dark);
                part(Vec3::new(0.08, 0.12, 0.24), Vec3::new(0.0, -0.02, 0.2), &body);
            }
            GunClass::Shotgun => {
                part(Vec3::new(0.09, 0.07, 0.16), Vec3::new(0.0, -0.08, -0.36), &dark);
                part(Vec3::new(0.07, 0.11, 0.22), Vec3::new(0.0, -0.03, 0.2), &body);
            }
            GunClass::Sniper => {
                part(Vec3::new(0.06, 0.06, 0.26), Vec3::new(0.0, 0.11, -0.25), &dark);
                part(Vec3::new(0.07, 0.11, 0.24), Vec3::new(0.0, -0.02, 0.2), &body);
            }
            GunClass::Wonder => {
                for z in [-0.12, -0.24, -0.36] {
                    part(Vec3::new(0.14, 0.14, 0.04), Vec3::new(0.0, 0.0, z), &accent);
                }
            }
            GunClass::Pistol => {}
        }
    }

    // Animation.
    let dip = if loadout.reload > 0.0 { 0.18 } else { 0.0 };
    let bob = if player.on_ground {
        (time.elapsed_secs() * 9.0).sin() * 0.008 * (player.horizontal_speed() / 6.0).min(1.5)
    } else {
        0.0
    };
    let sprint_tilt = if player.sprinting { 0.25 } else { 0.0 };
    tf.translation = Vec3::new(
        0.25,
        -0.22 - dip + bob,
        -0.45 + loadout.recoil * 0.06,
    );
    tf.rotation = Quat::from_euler(
        EulerRot::YXZ,
        sprint_tilt * 0.6,
        loadout.recoil * 0.12 - dip * 2.0,
        sprint_tilt * 0.3,
    );
    flash.intensity = if loadout.flash > 0.0 { 60_000.0 } else { 0.0 };
}
