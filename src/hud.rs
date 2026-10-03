//! Health, ammo, score, scoreboard, messages and the game-over screen.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::player::{Weapon, MAG_SIZE};
use crate::{cursor_locked, MatchState, Phase, Role, Roster, Session};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .add_systems(Startup, setup_hud)
            .add_systems(Update, update_hud.in_set(Phase::Present));
    }
}

#[derive(Resource, Default)]
struct HudState {
    last_health: f32,
    damage_flash: f32,
    last_wave: u32,
    banner_timer: f32,
}

#[derive(Component)]
struct HealthText;
#[derive(Component)]
struct AmmoText;
#[derive(Component)]
struct ScoreText;
#[derive(Component)]
struct BoardText;
#[derive(Component)]
struct StatusText;
#[derive(Component)]
struct CenterText;
#[derive(Component)]
struct DamageOverlay;
#[derive(Component)]
struct GameOverScreen;
#[derive(Component)]
struct GameOverText;

fn font(size: f32) -> TextFont {
    TextFont {
        font_size: size,
        ..default()
    }
}

fn absolute() -> Node {
    Node {
        position_type: PositionType::Absolute,
        ..default()
    }
}

fn setup_hud(mut commands: Commands) {
    // Red damage vignette (full screen)
    commands.spawn((
        DamageOverlay,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.8, 0.0, 0.0, 0.0)),
    ));

    // Crosshair
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|p| {
            for (w, h) in [(18.0, 2.0), (2.0, 18.0)] {
                p.spawn((
                    Node {
                        width: Val::Px(w),
                        height: Val::Px(h),
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    BackgroundColor(Color::WHITE),
                ));
            }
        });

    commands.spawn((
        HealthText,
        Text::new(""),
        font(32.0),
        TextColor(Color::srgb(0.4, 1.0, 0.4)),
        Node {
            left: Val::Px(20.0),
            bottom: Val::Px(16.0),
            ..absolute()
        },
    ));
    commands.spawn((
        AmmoText,
        Text::new(""),
        font(32.0),
        TextColor(Color::WHITE),
        Node {
            right: Val::Px(20.0),
            bottom: Val::Px(16.0),
            ..absolute()
        },
    ));
    commands.spawn((
        ScoreText,
        Text::new(""),
        font(26.0),
        TextColor(Color::WHITE),
        Node {
            left: Val::Px(20.0),
            top: Val::Px(14.0),
            ..absolute()
        },
    ));
    commands.spawn((
        BoardText,
        Text::new(""),
        font(18.0),
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
        TextLayout::new_with_justify(JustifyText::Right),
        Node {
            right: Val::Px(20.0),
            top: Val::Px(14.0),
            ..absolute()
        },
    ));
    commands.spawn((
        StatusText,
        Text::new(""),
        font(16.0),
        TextColor(Color::srgb(0.85, 0.9, 1.0)),
        Node {
            left: Val::Px(20.0),
            top: Val::Px(48.0),
            ..absolute()
        },
    ));
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            top: Val::Percent(30.0),
            justify_content: JustifyContent::Center,
            ..absolute()
        })
        .with_children(|p| {
            p.spawn((
                CenterText,
                Text::new(""),
                font(40.0),
                TextColor(Color::srgb(1.0, 0.9, 0.4)),
                TextLayout::new_with_justify(JustifyText::Center),
            ));
        });

    commands
        .spawn((
            GameOverScreen,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("GAME OVER"),
                font(72.0),
                TextColor(Color::srgb(0.9, 0.15, 0.15)),
            ));
            p.spawn((
                GameOverText,
                Text::new(""),
                font(28.0),
                TextLayout::new_with_justify(JustifyText::Center),
            ));
        });
}

fn update_hud(
    time: Res<Time>,
    mut hud: ResMut<HudState>,
    session: Res<Session>,
    roster: Res<Roster>,
    state: Res<MatchState>,
    weapon: Res<Weapon>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut texts: ParamSet<(
        Single<(&mut Text, &mut TextColor), With<HealthText>>,
        Single<&mut Text, With<AmmoText>>,
        Single<&mut Text, With<ScoreText>>,
        Single<&mut Text, With<BoardText>>,
        Single<&mut Text, With<StatusText>>,
        Single<&mut Text, With<CenterText>>,
        Single<&mut Text, With<GameOverText>>,
    )>,
    mut overlay: Single<&mut BackgroundColor, With<DamageOverlay>>,
    mut game_over: Single<&mut Node, With<GameOverScreen>>,
) {
    let dt = time.delta_secs();
    let me = roster.me(&session);
    let hp = me.map(|m| m.health).unwrap_or(0.0);
    let alive = me.is_none_or(|m| m.alive);

    // Flash red whenever our health drops.
    if hp < hud.last_health {
        hud.damage_flash = 0.4;
    }
    hud.last_health = hp;
    hud.damage_flash = (hud.damage_flash - dt).max(0.0);
    overlay.0 = Color::srgba(0.8, 0.0, 0.0, hud.damage_flash * 0.8);

    if state.wave != hud.last_wave {
        hud.last_wave = state.wave;
        if state.wave > 0 {
            hud.banner_timer = 2.5;
        }
    }
    hud.banner_timer -= dt;

    {
        let mut health = texts.p0();
        let (text, color) = &mut *health;
        text.0 = format!("HP {:.0}", hp);
        color.0 = if hp > 60.0 {
            Color::srgb(0.4, 1.0, 0.4)
        } else if hp > 30.0 {
            Color::srgb(1.0, 0.85, 0.3)
        } else {
            Color::srgb(1.0, 0.3, 0.3)
        };
    }

    texts.p1().0 = if weapon.reload_timer > 0.0 {
        "RELOADING...".into()
    } else {
        format!("{} / {}", weapon.ammo, MAG_SIZE)
    };

    let (score, kills) = me.map(|m| (m.score, m.kills)).unwrap_or_default();
    texts.p2().0 = format!("Score {score}   Kills {kills}   Wave {}", state.wave);

    // Scoreboard, only interesting with company.
    texts.p3().0 = if roster.0.len() > 1 {
        let mut players: Vec<_> = roster.0.values().collect();
        players.sort_by(|a, b| b.score.cmp(&a.score));
        players
            .iter()
            .map(|p| {
                let hp = if p.alive {
                    format!("{:>3.0} HP", p.health)
                } else {
                    "DOWN".into()
                };
                let you = if p.id == session.my_id { " (you)" } else { "" };
                format!("{}{}  {}  {} kills  {}", p.name, you, hp, p.kills, p.score)
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        String::new()
    };

    texts.p4().0 = session.status.clone();

    texts.p5().0 = if state.game_over {
        String::new()
    } else if !session.connected {
        session.status.clone()
    } else if !cursor_locked(&window) {
        match (session.role, state.started) {
            (Role::Solo, true) => "PAUSED - click to play".into(),
            (Role::Host, false) => "Click to start\nFriends can join now".into(),
            _ => "Click to play".into(),
        }
    } else if !state.started {
        "Waiting for the host to start...".into()
    } else if !alive {
        "YOU'RE DOWN\nYou'll respawn next wave".into()
    } else if hud.banner_timer > 0.0 {
        format!("WAVE {}", state.wave)
    } else if weapon.ammo == 0 && weapon.reload_timer <= 0.0 {
        "Press R to reload".into()
    } else {
        String::new()
    };

    game_over.display = if state.game_over {
        Display::Flex
    } else {
        Display::None
    };
    if state.game_over {
        let mut players: Vec<_> = roster.0.values().collect();
        players.sort_by(|a, b| b.score.cmp(&a.score));
        let mut lines: Vec<String> = vec![format!("Your team reached wave {}", state.wave), String::new()];
        lines.extend(
            players
                .iter()
                .map(|p| format!("{}   {} kills   {} pts", p.name, p.kills, p.score)),
        );
        lines.push(String::new());
        lines.push(if session.is_authority() {
            "Press Enter to play again".into()
        } else {
            "Waiting for the host to restart...".into()
        });
        texts.p6().0 = lines.join("\n");
    }
}
