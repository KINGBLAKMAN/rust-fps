//! The local player's guns: two weapon slots, ammo, reloading, firing modes,
//! and hit markers. The first-person model is in viewmodel.rs.

use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::Rng;

use crate::config::{Action, InputExt, Settings};
use crate::data::{gun_def, has_perk, mag_size, Attach, FireMode, GunClass, Perk, Stat};
use crate::fx::{Fx, FxQueue};
use crate::game::Paused;
use crate::physics::{collect_boxes, trace_shot};
use crate::player::{can_act, LocalPlayer};
use crate::{
    AppState, Collider, Enemy, MatchState, Phase, PlayerAction, Replicated, Roster, Session, Shot,
    ShotQueue,
};

pub const GUN_RANGE: f32 = 120.0;

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Loadout>()
            .init_resource::<Aim>()
            .add_systems(
                Update,
                (sync_loadout, switch_weapon, reload, melee, aim, fire)
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
    pub attach: Attach,
    pub mag: u32,
    pub reserve: u32,
}

impl GunState {
    fn fresh(id: u8, attach: Attach) -> Self {
        Self {
            id,
            attach,
            mag: mag_size(id, attach),
            reserve: gun_def(id).reserve,
        }
    }

    pub fn mag_size(&self) -> u32 {
        mag_size(self.id, self.attach)
    }
}

/// Aiming down sights: how far the gun is raised (0 hip, 1 fully aimed) and
/// the zoom the current sight gives (field of view multiplier).
#[derive(Resource)]
pub struct Aim {
    pub amount: f32,
    pub zoom: f32,
    /// Looking through a magnified scope (draws the scope overlay).
    pub scoped: bool,
}

impl Default for Aim {
    fn default() -> Self {
        Self {
            amount: 0.0,
            zoom: 1.0,
            scoped: false,
        }
    }
}

impl Aim {
    /// Field of view multiplier right now.
    pub fn fov_scale(&self) -> f32 {
        1.0 + (self.zoom - 1.0) * self.amount
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
    last_supply_seq: u8,
    last_spawn_seq: u32,
    last_round: u32,
    /// Seconds left to show the hit marker, and whether it was a headshot.
    pub hitmarker: f32,
    pub headshot: bool,
    /// Muzzle climb from recoil still to be recovered, and how long the
    /// trigger has been held (auto guns drift sideways the longer you hold).
    climb: f32,
    spray: u32,
    /// Melee: seconds into the swing (None when not swinging), the cooldown,
    /// and whether this swing has landed yet.
    pub melee: Option<f32>,
    melee_cd: f32,
    melee_struck: bool,
    /// Seconds left to show the kill marker.
    pub kill_marker: f32,
    last_kills: u32,
}

/// How long a melee swing takes, and when in it the blade connects.
pub const MELEE_TIME: f32 = 0.5;
const MELEE_STRIKE: f32 = 0.13;
pub const MELEE_RANGE: f32 = 2.3;

impl Loadout {
    pub fn current(&self) -> Option<&GunState> {
        self.slots[self.active].as_ref()
    }
}

fn reset_loadout(mut loadout: ResMut<Loadout>, mut aim: ResMut<Aim>) {
    *loadout = Loadout::default();
    *aim = Aim::default();
}

/// Right mouse raises the sights (not while sprinting or aiming an ability).
#[allow(clippy::too_many_arguments)]
fn aim(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    paused: Res<Paused>,
    cast: Res<crate::abilities::CastState>,
    loadout: Res<Loadout>,
    player: Single<&LocalPlayer>,
    mut aim: ResMut<Aim>,
) {
    let Some(gun) = loadout.current() else {
        aim.amount = 0.0;
        return;
    };
    let def = gun_def(gun.id);
    let want = can_act(&session, &roster, &state, &paused, &window)
        && mouse.pressed(MouseButton::Right)
        && cast.aiming.is_none()
        && !player.emoting()
        && player.dash_time <= 0.0
        && loadout.melee.is_none();
    // Heavier guns take longer to raise; attachments change it too.
    let handling = gun.attach.handling(gun.id);
    let time_to_aim = match def.class {
        GunClass::Pistol => 0.13,
        GunClass::Smg => 0.16,
        GunClass::Lmg | GunClass::Sniper => 0.3,
        _ => 0.21,
    } * handling.ads_time;
    let step = time.delta_secs() / time_to_aim;
    aim.amount = if want {
        (aim.amount + step).min(1.0)
    } else {
        (aim.amount - step * 1.3).max(0.0)
    };
    aim.zoom = handling.zoom;
    aim.scoped = handling.scoped && aim.amount > 0.9;
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
        let want = me.guns[i].map(|g| (g, me.attach[i]));
        let have = loadout.slots[i].map(|g| (g.id, g.attach));
        if want != have {
            loadout.slots[i] = want.map(|(g, a)| GunState::fresh(g, a));
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
    if me.supply_seq != loadout.last_supply_seq {
        loadout.last_supply_seq = me.supply_seq;
        for g in loadout.slots.iter_mut().flatten() {
            g.reserve = g.reserve.max(gun_def(g.id).reserve);
        }
    }
    if state.max_ammo_seq != loadout.last_ammo_seq {
        loadout.last_ammo_seq = state.max_ammo_seq;
        for g in loadout.slots.iter_mut().flatten() {
            let d = gun_def(g.id);
            g.reserve = d.reserve;
            g.mag = g.mag_size();
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
    menu: Res<crate::emotes::EmoteMenu>,
    player: Single<&LocalPlayer>,
) {
    loadout.switch_cd -= time.delta_secs();
    // The number keys pick emotes while that list is open.
    if menu.open || player.emoting() {
        return;
    }
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

/// How fast you reload: the Quick Hands perk doubles it, and each Sleight
/// of Hand upgrade adds a bit.
fn reload_speed(me: &crate::PlayerInfo) -> f32 {
    let perk = if has_perk(me.perks, Perk::QuickHands) {
        2.0
    } else {
        1.0
    };
    perk * (1.0 + me.stat(Stat::Sleight) * Stat::Sleight.per_stack())
}

fn reload(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    session: Res<Session>,
    roster: Res<Roster>,
    mut loadout: ResMut<Loadout>,
) {
    let speed = roster.me(&session).map_or(1.0, reload_speed);
    let active = loadout.active;
    let Some(gun) = loadout.slots[active] else {
        return;
    };
    let def = gun_def(gun.id);
    if keys.tapped(&settings, Action::Reload)
        && loadout.reload <= 0.0
        && gun.mag < gun.mag_size()
        && gun.reserve > 0
    {
        loadout.reload = def.reload * gun.attach.handling(gun.id).reload / speed;
        loadout.reload_total = loadout.reload;
    }
    if loadout.reload > 0.0 {
        loadout.reload -= time.delta_secs();
        if loadout.reload <= 0.0 {
            loadout.reload = 0.0;
            if let Some(g) = loadout.slots[active].as_mut() {
                let take = (g.mag_size() - g.mag).min(g.reserve);
                g.mag += take;
                g.reserve -= take;
            }
        }
    }
}

/// Melee: a quick knife slash. The host decides what it hits; we show the
/// hit marker straight away if someone is in reach.
#[allow(clippy::too_many_arguments)]
fn melee(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    window: Single<&Window, With<PrimaryWindow>>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    paused: Res<Paused>,
    mut counter: ResMut<crate::abilities::ActionCounter>,
    mut actions: ResMut<crate::ActionQueue>,
    mut loadout: ResMut<Loadout>,
    player: Single<(&Transform, &LocalPlayer)>,
    enemies: Query<(&Transform, &Replicated, Option<&crate::EnemyStatus>), With<Enemy>>,
) {
    let dt = time.delta_secs();
    loadout.melee_cd -= dt;
    loadout.kill_marker -= dt;
    let kills = roster.me(&session).map_or(0, |m| m.kills);
    if kills > loadout.last_kills {
        loadout.kill_marker = 0.35;
    }
    loadout.last_kills = kills;
    let (cam, p) = player.into_inner();
    if let Some(t) = loadout.melee.as_mut() {
        *t += dt;
        let t = *t;
        if t >= MELEE_STRIKE && !loadout.melee_struck {
            loadout.melee_struck = true;
            let origin = cam.translation;
            let dir = cam.forward().as_vec3();
            crate::abilities::queue_action(
                &session,
                &mut counter,
                &mut actions,
                PlayerAction::Melee {
                    origin: origin.to_array(),
                    dir: dir.to_array(),
                },
            );
            let reach = enemies.iter().any(|(tf, r, s)| {
                melee_reach(
                    origin,
                    dir,
                    tf.translation,
                    crate::sim::enemy_scale(r.kind),
                    s.is_some_and(|s| s.crawler),
                )
                .is_some()
            });
            if reach {
                loadout.hitmarker = 0.15;
                loadout.headshot = false;
            }
        }
        if t >= MELEE_TIME {
            loadout.melee = None;
        }
        return;
    }
    if keys.tapped(&settings, Action::Melee)
        && loadout.melee_cd <= 0.0
        && can_act(&session, &roster, &state, &paused, &window)
        && !p.emoting()
    {
        loadout.melee = Some(0.0);
        loadout.melee_struck = false;
        loadout.melee_cd = MELEE_TIME + 0.1;
        loadout.reload = 0.0;
        loadout.burst_left = 0;
        loadout.fire_cd = loadout.fire_cd.max(MELEE_TIME - 0.05);
    }
}

/// Distance to an enemy if a swing from `origin` along `dir` reaches it.
pub fn melee_reach(origin: Vec3, dir: Vec3, feet: Vec3, scale: f32, crawl: bool) -> Option<f32> {
    let chest = feet + Vec3::Y * if crawl { 0.35 } else { 1.0 * scale };
    let to = chest - origin;
    let flat = to.with_y(0.0);
    let dist = flat.length();
    let reach = MELEE_RANGE + 0.25 * (scale - 1.0);
    // In front of you (a wide arc), within reach and not far above or below.
    let facing = flat
        .normalize_or_zero()
        .dot(dir.with_y(0.0).normalize_or_zero());
    (dist < reach && (facing > 0.45 || dist < 0.7) && to.y.abs() < 1.8).then_some(dist)
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
    aim: Res<Aim>,
    mut loadout: ResMut<Loadout>,
    mut shots: ResMut<ShotQueue>,
    mut fx: ResMut<FxQueue>,
    mut player: Single<(&Transform, &mut LocalPlayer)>,
    enemies: Query<(Entity, &Transform, &Replicated, Option<&crate::EnemyStatus>), With<Enemy>>,
    colliders: Query<(&Transform, &Collider)>,
) {
    let dt = time.delta_secs();
    loadout.fire_cd -= dt;
    loadout.hitmarker -= dt;
    loadout.recoil = (loadout.recoil - dt * 8.0).max(0.0);
    loadout.flash -= dt;
    // Once the trigger is let go the muzzle settles most of the way back.
    let (cam_tf, p) = &mut *player;
    let cam_tf = **cam_tf;
    let holding = mouse.pressed(MouseButton::Left) && loadout.fire_cd > -0.08;
    if !holding {
        loadout.spray = 0;
        if loadout.climb > 0.0 {
            let back = (loadout.climb * (1.0 - (-9.0 * dt).exp())).min(loadout.climb);
            loadout.climb -= back;
            p.pitch -= back * 0.75;
        }
    }

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
    if !wants
        || loadout.fire_cd > 0.0
        || loadout.reload > 0.0
        || cast.blocks_fire
        || loadout.melee.is_some()
    {
        return;
    }
    if gun.mag == 0 && !overdrive {
        loadout.burst_left = 0;
        if gun.reserve > 0 {
            let speed = reload_speed(me);
            loadout.reload = def.reload * gun.attach.handling(gun.id).reload / speed;
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
    if me.stim > 0.0 {
        interval /= 1.25;
    }
    if def.mode == FireMode::Burst {
        if loadout.burst_left == 0 {
            loadout.burst_left = 3;
        }
        loadout.burst_left -= 1;
        loadout.fire_cd = if loadout.burst_left == 0 {
            0.3
        } else {
            interval * 0.6
        };
    } else {
        loadout.fire_cd = interval;
    }
    if !overdrive && !state.sandbox.god {
        if let Some(g) = loadout.slots[active].as_mut() {
            let rounds = if crate::data::is_dual(g.id) { 2 } else { 1 };
            g.mag = g.mag.saturating_sub(rounds);
        }
    }
    loadout.recoil = 1.0;
    loadout.flash = 0.05;
    loadout.shots += 1;

    let cam = cam_tf;
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
    // Aimed shots are much tighter; the laser tightens hip fire.
    let handling = gun.attach.handling(gun.id);
    let hip = handling.hip_spread;
    let aimed = match def.class {
        GunClass::Sniper => 0.02,
        GunClass::Shotgun => 0.7,
        _ => 0.3,
    } * handling.ads_spread;
    spread *= hip + (aimed - hip) * aim.amount;
    if def.class == GunClass::Sniper {
        // Snipers are wild from the hip.
        spread += 0.05 * (1.0 - aim.amount);
    }
    // Recoil: the muzzle climbs (and auto guns walk sideways the longer you
    // hold), plus a jolt that snaps straight back. Aiming and crouching
    // steady it; attachments change it.
    let rc = crate::data::recoil(gun.id);
    let steady = (1.0 - 0.3 * aim.amount) * if p.crouching { 0.8 } else { 1.0 };
    let mut rng = rand::thread_rng();
    let first = if loadout.spray == 0 { 1.25 } else { 1.0 };
    let climb = rc.up * handling.recoil_up * steady * first;
    let side = (rng.gen_range(-1.0..1.0) * rc.side
        + rc.drift * (loadout.spray.min(12) as f32 / 4.0))
        * handling.recoil_side
        * steady;
    p.pitch += climb;
    p.yaw -= side;
    loadout.climb += climb;
    loadout.spray += 1;
    p.kick += rc.kick * handling.recoil_up * steady;

    let boxes = collect_boxes(colliders.iter());
    // Twin Fangs: both guns fire on every pull.
    let mut muzzles = vec![origin + cam.rotation * view_muzzle.0];
    if let (true, Some(left)) = (crate::data::is_dual(gun.id), view_muzzle.1) {
        muzzles.push(origin + cam.rotation * left);
    }
    let mut any_hit = false;
    let mut head = false;
    for muzzle in muzzles.iter().flat_map(|m| std::iter::repeat_n(*m, def.pellets as usize)) {
        let a = rng.gen_range(0.0..std::f32::consts::TAU);
        let r = spread * rng.gen_range(0.0f32..1.0).sqrt();
        let dir = (forward + right * a.cos() * r + up * a.sin() * r).normalize();
        let hit = trace_shot(
            origin,
            dir,
            GUN_RANGE,
            &boxes,
            enemies.iter().map(|(e, t, r, s)| {
                (
                    e,
                    t.translation,
                    crate::sim::enemy_scale(r.kind),
                    s.is_some_and(|s| s.crawler),
                )
            }),
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
