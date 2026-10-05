//! The in-game HUD: crosshair, health, level and XP, points, ammo, ability
//! cooldowns, perks, prompts for the box and perk machines, power-up and
//! round banners, the boss bar, the teleporter, the scoreboard, the level-up
//! picker and the end screen.

/// Quiet text colour for the sandbox hint.
const DIM_HINT: Color = Color::srgba(1.0, 1.0, 1.0, 0.55);

use bevy::prelude::*;

use crate::config::{key_name, Action, Settings};
use crate::data::{
    boss_name, elements_in, gun_def, has_perk, skin_def, xp_to_next, Perk, AMMO_COST, BOX_COST, alt_fire, tier_name, AltFire,
    FINAL_BOSS_NAME, MAX_LEVEL, ROUNDS_PER_STAGE, STAGES,
};
use crate::sim::TELEPORT_HOLD;
use crate::game::{match_ended, MatchResult, Overlay};
use crate::maps::{map_name, CurrentMap};
use crate::ui::{button, UiAction, ACCENT, PANEL};
use crate::weapons::Loadout;
use crate::{AppState, BoxState, Enemy, InGameEntity, MatchState, Phase, Roster, Session};

const INTERACT_RANGE: f32 = 2.4;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), spawn_hud)
            .add_systems(
                Update,
                (
                    update_hud,
                    update_sights,
                    update_prompt,
                    update_banner,
                    update_scoreboard,
                    upgrade_panel,
                    end_screen,
                    boss_bar,
                )
                    .in_set(Phase::Present)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

/// Which HUD text a node shows.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum HudText {
    Round,
    Info,
    Health,
    Level,
    Perks,
    Points,
    Gun,
    Ammo,
    OtherGun,
    Ability(usize),
}

/// Which bar a node fills.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum HudFill {
    Health,
    Xp,
    Ability(usize),
}

#[derive(Component)]
struct AbilityBox(usize);
#[derive(Component)]
struct Hitmarker;

/// One of the four slashes of the hit marker.
#[derive(Component)]
struct HitmarkerBar;
#[derive(Component)]
struct PromptText;
#[derive(Component)]
struct BannerText;
#[derive(Component)]
struct HurtFlash;
#[derive(Component)]
struct Scoreboard;
#[derive(Component)]
struct ScoreboardText;
#[derive(Component)]
struct UpgradePanel;
#[derive(Component)]
struct EndPanel;
/// The boss's name and health bar at the top of the screen.
#[derive(Component)]
struct BossBar;
#[derive(Component)]
struct BossName;
#[derive(Component)]
struct BossFill;

fn text(value: impl Into<String>, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (
        Text::new(value),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
    )
}

/// The four crosshair lines (hidden while aiming down sights).
#[derive(Component)]
struct CrosshairLine;

/// The black scope view shown while looking through a magnified scope.
#[derive(Component)]
struct ScopeOverlay;

/// A scope's view: clear in a circle with fine crosshairs and range marks,
/// black outside.
fn scope_image() -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const N: usize = 512;
    let mut data = vec![0u8; N * N * 4];
    let c = N as f32 / 2.0;
    for y in 0..N {
        for x in 0..N {
            let (dx, dy) = (x as f32 + 0.5 - c, y as f32 + 0.5 - c);
            let r = (dx * dx + dy * dy).sqrt() / c;
            // Dark outside the lens, with a soft shaded rim inside it.
            let mut alpha = ((r - 0.93) / 0.03).clamp(0.0, 1.0);
            alpha = alpha.max(((r - 0.75) / 0.18).clamp(0.0, 1.0).powi(3) * 0.55);
            // Thin crosshair lines, thicker posts towards the edge.
            let thick = if r > 0.55 { 3.0 } else { 0.9 };
            if dx.abs() < thick || dy.abs() < thick {
                alpha = alpha.max(0.95);
            }
            // Range marks along the lines.
            for k in 1..6 {
                let m = k as f32 * 0.08 * c;
                if (dx.abs() < 1.2 && (dy.abs() - m).abs() < 1.6 && dy.abs() < 0.5 * c)
                    || (dy.abs() < 1.2 && (dx.abs() - m).abs() < 1.6 && dx.abs() < 0.5 * c)
                {
                    alpha = alpha.max(0.9);
                }
            }
            let i = (y * N + x) * 4;
            data[i + 3] = (alpha * 255.0) as u8;
        }
    }
    Image::new(
        Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Hides the crosshair while aiming and shows the scope view when scoped in.
fn update_sights(
    aim: Res<crate::weapons::Aim>,
    player: Single<&crate::player::LocalPlayer>,
    mut lines: Query<&mut Visibility, (With<CrosshairLine>, Without<ScopeOverlay>)>,
    mut scope: Query<&mut Visibility, (With<ScopeOverlay>, Without<CrosshairLine>)>,
) {
    let hide = aim.amount > 0.4 || player.third_person();
    for mut v in &mut lines {
        *v = if hide {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for mut v in &mut scope {
        *v = if aim.scoped {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn spawn_hud(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Scope view: black bars either side of a square lens image.
    let lens = images.add(scope_image());
    commands
        .spawn((
            InGameEntity,
            ScopeOverlay,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .with_children(|o| {
            o.spawn((
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                BackgroundColor(Color::BLACK),
            ));
            o.spawn((
                Node {
                    height: Val::Percent(100.0),
                    aspect_ratio: Some(1.0),
                    ..default()
                },
                ImageNode::new(lens),
            ));
            o.spawn((
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                BackgroundColor(Color::BLACK),
            ));
        });
    let white = Color::WHITE;
    let dim = Color::srgb(0.75, 0.78, 0.85);

    // Red flash when you get hurt.
    commands.spawn((
        InGameEntity,
        HurtFlash,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.8, 0.0, 0.0, 0.0)),
        Pickable::IGNORE,
    ));

    // Crosshair and hit marker.
    commands
        .spawn((
            InGameEntity,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|c| {
            for (w, h, x, y) in [
                (2.0, 7.0, 0.0, -9.0),
                (2.0, 7.0, 0.0, 9.0),
                (7.0, 2.0, -9.0, 0.0),
                (7.0, 2.0, 9.0, 0.0),
                (2.0, 2.0, 0.0, 0.0),
            ] {
                c.spawn((
                    CrosshairLine,
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(w),
                        height: Val::Px(h),
                        margin: UiRect {
                            left: Val::Px(x * 2.0),
                            top: Val::Px(y * 2.0),
                            ..default()
                        },
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
                ));
            }
            // Hit marker: four short slashes around the crosshair.
            c.spawn((
                Hitmarker,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(40.0),
                    height: Val::Px(40.0),
                    ..default()
                },
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|h| {
                for (x, y, a) in [
                    (-1.0, -1.0, 1.0),
                    (1.0, -1.0, -1.0),
                    (-1.0, 1.0, -1.0),
                    (1.0, 1.0, 1.0),
                ] {
                    h.spawn((
                        HitmarkerBar,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(20.0 + x * 10.0 - 1.5),
                            top: Val::Px(20.0 + y * 10.0 - 6.0),
                            width: Val::Px(3.0),
                            height: Val::Px(12.0),
                            ..default()
                        },
                        Transform::from_rotation(Quat::from_rotation_z(
                            a * std::f32::consts::FRAC_PI_4,
                        )),
                        BackgroundColor(Color::WHITE),
                    ));
                }
            });
        });

    // Top left: round and info.
    commands
        .spawn((
            InGameEntity,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(18.0),
                top: Val::Px(12.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|c| {
            c.spawn((HudText::Round, text("", 44.0, Color::srgb(0.85, 0.12, 0.1))));
            c.spawn((HudText::Info, text("", 17.0, dim)));
        });

    // Top centre: the boss's health bar (hidden until a boss is out).
    commands
        .spawn((
            InGameEntity,
            BossBar,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .with_children(|c| {
            c.spawn((BossName, text("", 24.0, Color::srgb(1.0, 0.55, 0.4))));
            bar(c, 520.0, 16.0, Color::srgb(0.85, 0.18, 0.12), BossFill);
        });

    // Top centre banner and centre prompt.
    commands
        .spawn((
            InGameEntity,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Px(70.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|c| {
            c.spawn((
                BannerText,
                text("", 30.0, ACCENT),
                TextLayout::new_with_justify(JustifyText::Center),
            ));
        });
    commands
        .spawn((
            InGameEntity,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Percent(58.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|c| {
            c.spawn((
                PromptText,
                text("", 20.0, white),
                TextLayout::new_with_justify(JustifyText::Center),
            ));
        });

    // Bottom left: health, level, perks.
    commands
        .spawn((
            InGameEntity,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(18.0),
                bottom: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|c| {
            c.spawn((HudText::Perks, text("", 15.0, white)));
            c.spawn((HudText::Level, text("", 16.0, dim)));
            bar(c, 240.0, 6.0, Color::srgb(0.45, 0.7, 1.0), HudFill::Xp);
            c.spawn((HudText::Health, text("", 18.0, white)));
            bar(
                c,
                240.0,
                14.0,
                Color::srgb(0.25, 0.85, 0.35),
                HudFill::Health,
            );
        });

    // Bottom centre: abilities.
    commands
        .spawn((
            InGameEntity,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                bottom: Val::Px(16.0),
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(10.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|c| {
            // Weapon abilities first, then the class abilities.
            for i in [3, 4, 0, 1, 2] {
                c.spawn((
                    AbilityBox(i),
                    Node {
                        width: Val::Px(134.0),
                        height: Val::Px(46.0),
                        border: UiRect::all(Val::Px(2.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.05, 0.06, 0.1, 0.75)),
                    BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                    BorderRadius::all(Val::Px(6.0)),
                ))
                .with_children(|b| {
                    b.spawn((
                        HudFill::Ability(i),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            bottom: Val::Px(0.0),
                            width: Val::Percent(100.0),
                            height: Val::Percent(0.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.3, 0.6, 1.0, 0.35)),
                    ));
                    b.spawn((
                        HudText::Ability(i),
                        text("", 13.0, white),
                        TextLayout::new_with_justify(JustifyText::Center),
                    ));
                });
            }
        });

    // Bottom right: points and guns.
    commands
        .spawn((
            InGameEntity,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(18.0),
                bottom: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(2.0),
                ..default()
            },
        ))
        .with_children(|c| {
            c.spawn((HudText::Points, text("", 30.0, Color::srgb(1.0, 0.85, 0.3))));
            c.spawn((HudText::OtherGun, text("", 15.0, dim)));
            c.spawn((HudText::Gun, text("", 18.0, white)));
            c.spawn((HudText::Ammo, text("", 34.0, white)));
        });

    // Scoreboard (hold Tab).
    commands
        .spawn((
            InGameEntity,
            Scoreboard,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Px(120.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|c| {
            c.spawn((
                Node {
                    padding: UiRect::all(Val::Px(16.0)),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderRadius::all(Val::Px(8.0)),
            ))
            .with_children(|p| {
                p.spawn((ScoreboardText, text("", 17.0, white)));
            });
        });
}

fn bar(
    c: &mut ChildSpawnerCommands,
    width: f32,
    height: f32,
    color: Color,
    marker: impl Component,
) {
    c.spawn((
        Node {
            width: Val::Px(width),
            height: Val::Px(height),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        BorderRadius::all(Val::Px(3.0)),
    ))
    .with_children(|b| {
        b.spawn((
            marker,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(color),
            BorderRadius::all(Val::Px(3.0)),
        ));
    });
}

fn set(text: &mut Text, value: String) {
    if text.0 != value {
        text.0 = value;
    }
}

#[allow(clippy::too_many_arguments)]
fn update_hud(
    time: Res<Time>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    loadout: Res<Loadout>,
    settings: Res<Settings>,
    enemies: Query<(), With<Enemy>>,
    mut texts: Query<(&HudText, &mut Text)>,
    mut fills: Query<(&HudFill, &mut Node, &mut BackgroundColor)>,
    mut ability_box: Query<(&AbilityBox, &mut BorderColor)>,
    hitmarker: Single<(&mut Visibility, &mut Transform), With<Hitmarker>>,
    mut hitmarker_bars: Query<
        &mut BackgroundColor,
        (With<HitmarkerBar>, Without<HurtFlash>, Without<HudFill>),
    >,
    mut hurt: Single<&mut BackgroundColor, (With<HurtFlash>, Without<HudFill>)>,
    mut last_health: Local<f32>,
    mut flash: Local<f32>,
) {
    let Some(me) = roster.me(&session) else {
        return;
    };
    let enemy_count = enemies.iter().count() as u32;
    let max = me.max_health();
    let abilities = me.kit;
    let keys = [
        Action::Ability1,
        Action::Ability2,
        Action::Ultimate,
        Action::WeaponAbility1,
        Action::WeaponAbility2,
    ];
    let weapon = me.character.weapon_abilities();
    let ready = |i: usize| match i {
        2 => me.ult_charge >= 100.0,
        3 | 4 => me.weapon_cd[i - 3] <= 0.0,
        _ => me.charges[i] > 0,
    };
    // The weapon ability running right now (box index).
    let running = |i: usize| i >= 3 && me.buff_time > 0.0 && me.buff == Some(weapon[i - 3]);

    for (kind, mut t) in &mut texts {
        let value = match *kind {
            HudText::Round => {
                if state.round == 0 {
                    String::new()
                } else {
                    format!("{}", state.round)
                }
            }
            HudText::Info => {
                let mut info = map_name(state.map).to_string();
                if !state.sandbox.on {
                    info += &format!("  -  map {} of {}", state.stage + 1, STAGES);
                    if state.teleport {
                        info += &format!("\nBoss down: on to map {}", state.stage + 2);
                    } else if state.stage_round > ROUNDS_PER_STAGE {
                        info += "\nBOSS ROUND";
                    } else if state.stage_round > 0 {
                        info += &format!(
                            "\nRound {} of {}, then the boss",
                            state.stage_round, ROUNDS_PER_STAGE
                        );
                    }
                }
                if state.round > 0 && !state.teleport {
                    info += &format!("\nEnemies left: {}", enemy_count + state.to_spawn);
                }
                if state.insta_kill > 0.0 {
                    info += &format!("\nINSTA KILL {:.0}s", state.insta_kill);
                }
                if state.double_points > 0.0 {
                    info += &format!("\nDOUBLE POINTS {:.0}s", state.double_points);
                }
                if me.overdrive > 0.0 {
                    info += &format!("\nOVERDRIVE {:.0}s", me.overdrive);
                }
                if !session.status.is_empty() && session.role == crate::Role::Host {
                    info += &format!("\n{}", session.status);
                }
                info
            }
            HudText::Health => format!("{}  {:.0} / {:.0}", me.name, me.health.max(0.0), max),
            HudText::Level => {
                if me.level >= MAX_LEVEL {
                    format!("Level {} (max)", me.level)
                } else {
                    format!(
                        "Level {}  -  {} / {} XP",
                        me.level,
                        me.xp,
                        xp_to_next(me.level)
                    )
                }
            }
            HudText::Perks => {
                let perks: Vec<&str> = Perk::ALL
                    .iter()
                    .filter(|p| has_perk(me.perks, **p))
                    .map(|p| p.name())
                    .collect();
                let mut lines = Vec::new();
                if !perks.is_empty() {
                    lines.push(format!("Perks: {}", perks.join(", ")));
                }
                let guns: Vec<&str> = elements_in(me.gun_elements).map(|e| e.name()).collect();
                if !guns.is_empty() {
                    lines.push(format!("Gun elements: {}", guns.join(", ")));
                }
                let abil: Vec<&str> = elements_in(me.ability_elements).map(|e| e.name()).collect();
                if !abil.is_empty() {
                    lines.push(format!("Ability elements: {}", abil.join(", ")));
                }
                lines.join("\n")
            }
            HudText::Points => format!("{} pts", me.points),
            HudText::Gun => loadout
                .current()
                .map(|g| {
                    let parts = g.attach.names();
                    let extra = if parts.is_empty() {
                        String::new()
                    } else {
                        format!("\n{}", parts.join(" + "))
                    };
                    let alt = alt_fire(g.id);
                    let alt_line = match alt {
                        AltFire::Grenade => {
                            let cd = loadout.grenade_cd.max(me.grenade_cd);
                            if cd > 0.0 {
                                format!("\n[RMB] {} ({cd:.1}s)", alt.describe())
                            } else {
                                format!("\n[RMB] {} (ready)", alt.describe())
                            }
                        }
                        a => format!("\n[RMB] {}", a.describe()),
                    };
                    format!(
                        "{}{}  ({}){extra}{alt_line}",
                        gun_def(g.id).name,
                        tier_name(g.tier),
                        skin_def(me.skin_for(g.id)).name
                    )
                })
                .unwrap_or_default(),
            HudText::Ammo => match loadout.current() {
                Some(_) if me.overdrive > 0.0 || me.gun_buff().free_ammo => "INFINITE".to_string(),
                Some(_) if loadout.reload > 0.0 => "Reloading...".to_string(),
                Some(g) => format!("{} / {}", g.mag, g.reserve),
                None => String::new(),
            },
            HudText::OtherGun => loadout.slots[1 - loadout.active]
                .map(|g| {
                    format!(
                        "[{}] {}",
                        key_name(settings.key(Action::SwapWeapon)),
                        gun_def(g.id).name
                    )
                })
                .unwrap_or_default(),
            HudText::Ability(i) if i >= 3 => {
                let w = weapon[i - 3];
                let status = if running(i) {
                    format!("ACTIVE {:.1}s", me.buff_time)
                } else if ready(i) {
                    "READY".to_string()
                } else {
                    format!("{:.1}s", me.weapon_cd[i - 3])
                };
                format!("{}\n[{}] {}", w.name(), key_name(settings.key(keys[i])), status)
            }
            HudText::Ability(i) => {
                let status = if i == 2 {
                    if ready(2) {
                        "READY".to_string()
                    } else {
                        format!("{:.0}%", me.ult_charge)
                    }
                } else if me.max_charges(i) > 1 {
                    let mut t = format!("{}/{}", me.charges[i], me.max_charges(i));
                    if me.cooldowns[i] > 0.0 {
                        t += &format!("  {:.1}s", me.cooldowns[i]);
                    }
                    t
                } else if ready(i) {
                    "READY".to_string()
                } else {
                    format!("{:.1}s", me.cooldowns[i])
                };
                let roman = ["I", "II", "III", "IV", "V", "VI"][(me.tiers[i] as usize).min(5)];
                let copies = me.copies(i);
                let many = if copies > 1 { format!(" x{copies}") } else { String::new() };
                format!(
                    "{} {roman}\n[{}] {}{many}",
                    abilities[i].name(),
                    key_name(settings.key(keys[i])),
                    status
                )
            }
        };
        set(&mut t, value);
    }

    for (kind, mut node, mut bg) in &mut fills {
        match *kind {
            HudFill::Health => {
                node.width = Val::Percent((me.health / max * 100.0).clamp(0.0, 100.0));
            }
            HudFill::Xp => {
                node.width = Val::Percent(if me.level >= MAX_LEVEL {
                    100.0
                } else {
                    (me.xp as f32 / xp_to_next(me.level) as f32 * 100.0).clamp(0.0, 100.0)
                });
            }
            HudFill::Ability(i) if i >= 3 => {
                let w = weapon[i - 3];
                let frac = if running(i) {
                    me.buff_time / w.def().duration
                } else {
                    1.0 - me.weapon_cd[i - 3] / w.cooldown()
                };
                node.height = Val::Percent(frac.clamp(0.0, 1.0) * 100.0);
                bg.0 = if running(i) {
                    w.color().with_alpha(0.45)
                } else if ready(i) {
                    Color::srgba(0.3, 0.9, 0.5, 0.35)
                } else {
                    Color::srgba(1.0, 0.6, 0.25, 0.3)
                };
            }
            HudFill::Ability(i) => {
                let frac = if i == 2 {
                    me.ult_charge / 100.0
                } else {
                    if me.cooldowns[i] <= 0.0 {
                        1.0
                    } else {
                        1.0 - me.cooldowns[i] / me.ability_cooldown(i)
                    }
                };
                node.height = Val::Percent(frac.clamp(0.0, 1.0) * 100.0);
                bg.0 = if ready(i) {
                    Color::srgba(0.3, 0.9, 0.5, 0.35)
                } else {
                    Color::srgba(0.3, 0.6, 1.0, 0.3)
                };
            }
        }
    }
    for (AbilityBox(i), mut border) in &mut ability_box {
        border.0 = if running(*i) {
            weapon[*i - 3].color()
        } else if ready(*i) {
            Color::srgba(0.4, 1.0, 0.6, 0.8)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.25)
        };
    }

    // White on a hit, orange on a headshot, red and bigger on a kill; it
    // pops out and shrinks back.
    let (mut vis, mut tf) = hitmarker.into_inner();
    let kill = loadout.kill_marker > 0.0;
    let showing = loadout.hitmarker > 0.0 || kill;
    *vis = if showing {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let pop = if kill {
        1.0 + loadout.kill_marker * 1.6
    } else {
        1.0 + loadout.hitmarker.max(0.0) * 2.5
    };
    tf.scale = Vec3::splat(pop);
    let color = if kill {
        Color::srgb(1.0, 0.15, 0.1)
    } else if loadout.headshot {
        Color::srgb(1.0, 0.6, 0.15)
    } else {
        Color::WHITE
    };
    for mut bar in &mut hitmarker_bars {
        bar.0 = color;
    }

    if me.health < *last_health - 0.5 {
        *flash = 0.35;
    }
    *last_health = me.health;
    *flash = (*flash - time.delta_secs()).max(0.0);
    let downed = if me.alive { 0.0 } else { 0.25 };
    hurt.0 = Color::srgba(0.8, 0.0, 0.0, (*flash).max(downed));
}

fn update_prompt(
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    settings: Res<Settings>,
    map: Option<Res<CurrentMap>>,
    overlay: Res<Overlay>,
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
    mut prompt: Single<&mut Text, With<PromptText>>,
) {
    let (Some(me), Some(map)) = (roster.me(&session), map) else {
        return;
    };
    let key = key_name(settings.key(Action::Interact));
    let mut msg = String::new();
    if match_ended(&state) {
        // The end screen says it all.
    } else if *overlay == Overlay::None && !crate::cursor_locked(&window) {
        msg = if session.role == crate::Role::Solo {
            "Paused - click to play".into()
        } else {
            "Click to play".into()
        };
    } else if !me.alive {
        msg = "You are down! You'll get back up next round if a teammate survives.".into();
    } else {
        let feet = me.feet().with_y(0.0);
        let box_pos = map.0.box_spots[(state.box_spot as usize).min(4)];
        if feet.distance(box_pos) < INTERACT_RANGE {
            msg = match state.box_state {
                BoxState::Idle => {
                    if me.points >= BOX_COST {
                        format!("[{key}] Armory - new attachments for the gun in your hands ({BOX_COST} pts)")
                    } else {
                        format!("Armory - need {BOX_COST} pts")
                    }
                }
                BoxState::Rolling { player, .. } => {
                    if player == me.id {
                        "Rolling...".into()
                    } else {
                        "Someone is using the Armory".into()
                    }
                }
                BoxState::Offer {
                    player,
                    gun,
                    attach,
                    ..
                } => {
                    let def = gun_def(gun);
                    let rare = if def.rare { "RARE! " } else { "" };
                    let parts = attach.names();
                    let kit = if parts.is_empty() {
                        String::new()
                    } else {
                        format!("\nwith {}", parts.join(", "))
                    };
                    let held = me.guns.contains(&Some(gun));
                    match (player == me.id, held) {
                        (true, true) => format!(
                            "[{key}] Fit to your {}{kit}",
                            def.name
                        ),
                        (true, false) => {
                            let cur = me.guns[me.active_slot as usize]
                                .map(|g| gun_def(g).name)
                                .unwrap_or("");
                            format!("[{key}] Take {rare}{} (replacing your {cur}){kit}", def.name)
                        }
                        _ => format!("{rare}{} - not yours{kit}", def.name),
                    }
                }
                BoxState::Moving { .. } => "The Armory moved! Find where it went.".into(),
            };
        }
        for (spot, perk) in map.0.perk_spots.iter().zip(Perk::ALL) {
            if feet.distance(*spot) < INTERACT_RANGE {
                msg = if has_perk(me.perks, perk) {
                    format!("{} - you already have it", perk.name())
                } else if me.points >= perk.cost() {
                    format!(
                        "[{key}] Buy {} ({} pts)\n{}",
                        perk.name(),
                        perk.cost(),
                        perk.description()
                    )
                } else {
                    format!(
                        "{} - need {} pts\n{}",
                        perk.name(),
                        perk.cost(),
                        perk.description()
                    )
                };
            }
        }
        let feet3 = me.feet();
        if map.0.wall_buys.iter().any(|w| w.near(feet3)) {
            msg = if me.points >= AMMO_COST {
                format!("[{key}] Ammo cache - refill both guns ({AMMO_COST} pts)")
            } else {
                format!("Ammo cache - need {AMMO_COST} pts")
            };
        }
        if msg.is_empty() && !me.choices.is_empty() {
            msg = format!(
                "LEVEL UP! Press [{}] to pick an upgrade",
                key_name(settings.key(Action::Upgrades))
            );
        }
    }
    set(&mut prompt, msg);
}

fn update_banner(
    time: Res<Time>,
    state: Res<MatchState>,
    session: Res<Session>,
    roster: Res<Roster>,
    map: Option<Res<CurrentMap>>,
    banner: Single<(&mut Text, &mut TextColor), With<BannerText>>,
    mut last_seq: Local<Option<u32>>,
    mut show: Local<f32>,
) {
    let (mut text, mut color) = banner.into_inner();
    if *last_seq != Some(state.powerup_seq) {
        if last_seq.is_some() && state.last_powerup.is_some() {
            *show = 2.5;
        }
        *last_seq = Some(state.powerup_seq);
    }
    *show -= time.delta_secs();

    if match_ended(&state) {
        set(&mut text, String::new());
        return;
    }
    if *show > 0.0 {
        if let Some(p) = state.last_powerup {
            set(&mut text, format!("{}!", p.name().to_uppercase()));
            color.0 = p.color();
            return;
        }
    }
    if state.teleport {
        let dist = match (roster.me(&session), &map) {
            (Some(me), Some(map)) => me.feet().with_y(0.0).distance(map.0.extraction),
            _ => 0.0,
        };
        let hold = if state.teleport_hold > 0.0 {
            format!("\nTeleporting... {:.1} / {:.1}", state.teleport_hold, TELEPORT_HOLD)
        } else {
            String::new()
        };
        set(
            &mut text,
            format!(
                "BOSS DOWN! Teleporter open ({:.0}m)\nWhole team into the purple light{hold}",
                dist
            ),
        );
        color.0 = Color::srgb(0.75, 0.55, 1.0);
        return;
    }
    if state.sandbox.on && !state.sandbox.waves {
        set(&mut text, "Sandbox - F1 for tools".to_string());
        color.0 = DIM_HINT;
        return;
    }
    if state.stage_round == 0 {
        let lead = if state.sandbox.on {
            String::new()
        } else {
            format!("Map {} of {}: {}\n", state.stage + 1, STAGES, map_name(state.map))
        };
        set(
            &mut text,
            format!("{lead}Get ready... {:.0}", state.intermission.max(0.0).ceil()),
        );
        color.0 = ACCENT;
        return;
    }
    if state.to_spawn == 0 && state.intermission > 0.0 && state.intermission < 7.5 {
        let boss_next = !state.sandbox.on && state.stage_round == ROUNDS_PER_STAGE;
        if boss_next {
            let name = boss_name(state.map, state.stage + 1 >= STAGES);
            set(
                &mut text,
                format!(
                    "Round {} cleared!\n{} is coming... {:.0}",
                    state.round,
                    name,
                    state.intermission.ceil()
                ),
            );
            color.0 = Color::srgb(1.0, 0.45, 0.3);
        } else if state.intermission < 4.5 {
            set(&mut text, format!("Round {} cleared!", state.round));
            color.0 = ACCENT;
        } else {
            set(&mut text, String::new());
        }
        return;
    }
    set(&mut text, String::new());
}

fn update_scoreboard(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    state: Res<MatchState>,
    roster: Res<Roster>,
    mut board: Single<&mut Visibility, With<Scoreboard>>,
    mut text: Single<&mut Text, With<ScoreboardText>>,
) {
    let show = keys.pressed(settings.key(Action::Scoreboard)) && !match_ended(&state);
    **board = if show {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if !show {
        return;
    }
    let mut s = format!(
        "{:<16} {:<8} {:>5} {:>6} {:>7} {:>7}\n",
        "Player", "Class", "Level", "Kills", "Score", "Points"
    );
    let mut players: Vec<_> = roster.0.values().collect();
    players.sort_by_key(|p| std::cmp::Reverse(p.score));
    for p in players {
        let down = if p.alive { "" } else { " (down)" };
        s += &format!(
            "{:<16} {:<8} {:>5} {:>6} {:>7} {:>7}{down}\n",
            p.name,
            p.character.name(),
            p.level,
            p.kills,
            p.score,
            p.points
        );
    }
    set(&mut text, s);
}

/// The level-up picker: three choices as buttons. Rebuilt when they change.
fn upgrade_panel(
    mut commands: Commands,
    overlay: Res<Overlay>,
    session: Res<Session>,
    roster: Res<Roster>,
    panels: Query<Entity, With<UpgradePanel>>,
    mut shown: Local<Option<Vec<crate::data::Upgrade>>>,
) {
    let me = roster.me(&session);
    let want = (*overlay == Overlay::Upgrades)
        .then(|| me.map(|m| m.choices.clone()))
        .flatten()
        .filter(|c| !c.is_empty());
    if *shown == want {
        return;
    }
    *shown = want.clone();
    for e in &panels {
        commands.entity(e).despawn();
    }
    let (Some(choices), Some(me)) = (want, me) else {
        return;
    };
    commands
        .spawn((
            InGameEntity,
            UpgradePanel,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(24.0)),
                    min_width: Val::Px(520.0),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|p| {
                p.spawn(text(
                    format!("LEVEL {} - PICK AN UPGRADE", me.level),
                    28.0,
                    ACCENT,
                ));
                let left = if me.pending_picks > 1 {
                    format!("{} picks waiting", me.pending_picks)
                } else {
                    "Every 5 levels you get a pick. Levels also add damage.".into()
                };
                p.spawn(text(left, 16.0, Color::srgb(0.7, 0.75, 0.85)));
                for (i, c) in choices.iter().enumerate() {
                    button(
                        p,
                        c.label(me),
                        UiAction::ChooseUpgrade(i as u8),
                    );
                }
                p.spawn(text(
                    "Esc or B to close (you can pick later)",
                    14.0,
                    Color::srgb(0.6, 0.6, 0.7),
                ));
            });
        });
}

fn end_screen(
    mut commands: Commands,
    state: Res<MatchState>,
    session: Res<Session>,
    roster: Res<Roster>,
    result: Res<MatchResult>,
    profile: Res<crate::config::Profile>,
    panels: Query<Entity, With<EndPanel>>,
    mut shown: Local<bool>,
) {
    let want = match_ended(&state) && result.awarded;
    if want == *shown {
        return;
    }
    *shown = want;
    for e in &panels {
        commands.entity(e).despawn();
    }
    if !want {
        return;
    }
    let me = roster.me(&session);
    let (title, color) = if state.won {
        ("RUN COMPLETE!", Color::srgb(0.75, 0.55, 1.0))
    } else {
        ("GAME OVER", Color::srgb(0.95, 0.2, 0.15))
    };
    let how_far = if state.won {
        format!("All {STAGES} maps cleared and {FINAL_BOSS_NAME} defeated")
    } else if state.sandbox.on {
        String::new()
    } else {
        format!("Fell on map {} of {} ({})", state.stage + 1, STAGES, map_name(state.map))
    };
    commands
        .spawn((
            InGameEntity,
            EndPanel,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(28.0)),
                    min_width: Val::Px(460.0),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|p| {
                p.spawn(text(title, 54.0, color));
                if !how_far.is_empty() {
                    p.spawn(text(how_far.clone(), 20.0, Color::srgb(0.85, 0.85, 0.95)));
                }
                p.spawn(text(
                    format!("Rounds survived: {}", result.round),
                    22.0,
                    Color::WHITE,
                ));
                if let Some(me) = me {
                    p.spawn(text(
                        format!(
                            "Kills {}   Score {}   Level {}",
                            me.kills, me.score, me.level
                        ),
                        18.0,
                        Color::srgb(0.8, 0.82, 0.9),
                    ));
                }
                if result.new_best {
                    p.spawn(text("New personal best!", 18.0, ACCENT));
                }
                let spins = if result.spins > 0 {
                    format!("+{} gacha spins", result.spins)
                } else {
                    "No spins this time - reach round 5 to earn spins".into()
                };
                p.spawn(text(spins, 22.0, Color::srgb(1.0, 0.8, 0.3)));
                if result.premium_quarters > 0 {
                    p.spawn(text(
                        format!("+{} premium spin", crate::config::quarters_text(result.premium_quarters)),
                        20.0,
                        Color::srgb(1.0, 0.55, 0.9),
                    ));
                }
                p.spawn(text(format!("+{} career XP", result.xp), 20.0, Color::srgb(0.55, 0.85, 1.0)));
                let (before, after) = result.levels;
                if after > before {
                    let unlocked: Vec<String> = (before + 1..=after)
                        .filter_map(|l| crate::progression::UNLOCKS.get(l as usize - 2))
                        .map(|u| match *u {
                            crate::progression::Unlock::Gun(g) => gun_def(g).name.to_string(),
                            crate::progression::Unlock::Attachment(i) => crate::data::ATTACHMENTS[i].name.to_string(),
                        })
                        .collect();
                    p.spawn(text(
                        format!("Career level {after}! Unlocked for your loadout: {}", unlocked.join(", ")),
                        18.0,
                        ACCENT,
                    ));
                }
                let (before, after) = result.char_levels;
                if after > before {
                    let c = result.character;
                    let new: Vec<&str> = c
                        .pool()
                        .iter()
                        .filter(|a| a.def().unlock > before && a.def().unlock <= after)
                        .map(|a| a.name())
                        .collect();
                    let msg = if new.is_empty() {
                        format!("{} reached level {after}!", c.name())
                    } else {
                        format!(
                            "{} reached level {after}! New abilities to pick in Characters: {}",
                            c.name(),
                            new.join(", ")
                        )
                    };
                    p.spawn(text(msg, 18.0, ACCENT));
                }
                p.spawn(text(
                    format!(
                        "You have {} spins and {} premium spins. Open them in Gun Crates on the main menu.",
                        profile.spins,
                        crate::config::quarters_text(profile.premium_quarters)
                    ),
                    15.0,
                    Color::srgb(0.7, 0.72, 0.8),
                ));
                if session.is_authority() {
                    button(p, "Play again", UiAction::BackToLobby);
                } else {
                    p.spawn(text("Waiting for the host to start again...", 16.0, Color::srgb(0.7, 0.75, 0.85)));
                }
                button(p, "Quit to main menu", UiAction::Leave);
            });
        });
}

/// Shows the boss's name and health while one is out.
fn boss_bar(
    state: Res<MatchState>,
    mut bar: Single<&mut Visibility, With<BossBar>>,
    mut name: Single<&mut Text, With<BossName>>,
    mut fill: Single<&mut Node, With<BossFill>>,
) {
    let show = state.boss != 0 && !match_ended(&state);
    let want = if show {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if **bar != want {
        **bar = want;
    }
    if !show {
        return;
    }
    set(&mut name, boss_name(state.map, state.boss == 2).to_uppercase());
    fill.width = Val::Percent(state.boss_hp * 100.0);
}
