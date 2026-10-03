//! The local player's guns: two weapon slots, ammo, reloading, firing modes,
//! and hit markers. The first-person model is in viewmodel.rs.

use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::Rng;

use crate::config::{Action, InputExt, Settings};
use crate::data::{gun_def, has_perk, FireMode, GunClass, Perk};
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
            .add_systems(
                Update,
                (sync_loadout, switch_weapon, reload, fire)
                    .chain()
                    .after(crate::player::movement)
                    .in_set(Phase::Local),
            )
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
    pub recoil: f32,
    pub flash: f32,
    /// Length of the reload in progress (for the animation).
    pub reload_total: f32,
    /// Counts shots fired (the viewmodel animates each one).
    pub shots: u32,
    last_ammo_seq: u32,
    last_spawn_seq: u32,
    last_round: u32,
    /// Seconds left to show the hit marker, and whether it was a headshot.
    pub hitmarker: f32,
    pub headshot: bool,
}

impl Loadout {
    pub fn current(&self) -> Option<&GunState> {
        self.slots[self.active].as_ref()
    }
}

fn reset_loadout(mut loadout: ResMut<Loadout>) {
    *loadout = Loadout::default();
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
        loadout.reload_total = loadout.reload;
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

pub fn fire(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    paused: Res<Paused>,
    cast: Res<crate::abilities::CastState>,
    view_muzzle: Res<crate::viewmodel::ViewMuzzle>,
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
    if !wants || loadout.fire_cd > 0.0 || loadout.reload > 0.0 || cast.blocks_fire {
        return;
    }
    if gun.mag == 0 && !overdrive {
        loadout.burst_left = 0;
        if gun.reserve > 0 {
            let speed = if has_perk(me.perks, Perk::QuickHands) { 2.0 } else { 1.0 };
            loadout.reload = def.reload / speed;
            loadout.reload_total = loadout.reload;
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
    loadout.shots += 1;

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
    let muzzle = origin + cam.rotation * view_muzzle.0;
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
