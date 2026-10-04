//! Opening a crate: a strip of skins slides past a marker, slows down and
//! stops on the one you won. Click to skip to the end.

use bevy::prelude::*;
use rand::Rng;

use crate::audio::{Snd, SoundQueue};
use crate::data::{roll_crate, skin_def, Rarity};

use super::SkinShop;

/// How long a spin takes, and which card in the strip is the prize.
const SPIN_TIME: f32 = 5.5;
const CARDS: usize = 52;
const PRIZE: usize = 45;
/// Card width plus the gap between cards, in pixels.
const STEP: f32 = 140.0;

/// The spin in progress, if any.
#[derive(Resource, Default)]
pub(super) struct CrateSpin(Option<Spin>);

struct Spin {
    cards: Vec<u8>,
    t: f32,
    /// Where in the prize card the marker stops (-0.4..0.4 of a card), so
    /// every spin lands a little differently.
    land: f32,
    prize: (u8, bool),
    last_card: i32,
    shown: bool,
}

impl CrateSpin {
    /// Starts a spin that lands on `prize` (already added to the profile).
    pub fn start(&mut self, crate_id: usize, prize: u8, new: bool) {
        let mut rng = rand::thread_rng();
        let cards = (0..CARDS)
            .map(|i| {
                if i == PRIZE {
                    prize
                } else {
                    roll_crate(crate_id, &mut rng)
                }
            })
            .collect();
        self.0 = Some(Spin {
            cards,
            t: 0.0,
            land: rng.gen_range(-0.4..0.4),
            prize: (prize, new),
            last_card: -1,
            shown: false,
        });
    }

    pub fn spinning(&self) -> bool {
        self.0.is_some()
    }
}

#[derive(Component)]
pub(super) struct CarouselRoot;

#[derive(Component)]
pub(super) struct Strip;

/// Slows down like a wheel losing speed.
fn ease_out(u: f32) -> f32 {
    1.0 - (1.0 - u.clamp(0.0, 1.0)).powi(4)
}

pub(super) fn carousel(
    mut commands: Commands,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut spin: ResMut<CrateSpin>,
    mut shop: ResMut<SkinShop>,
    mut sounds: ResMut<SoundQueue>,
    roots: Query<Entity, With<CarouselRoot>>,
    mut strip: Query<&mut Node, With<Strip>>,
) {
    let Some(s) = spin.0.as_mut() else {
        for e in &roots {
            commands.entity(e).despawn();
        }
        return;
    };
    if !s.shown {
        s.shown = true;
        spawn_overlay(&mut commands, &s.cards);
        return;
    }
    s.t += time.delta_secs();
    if mouse.just_pressed(MouseButton::Left) && s.t > 0.3 && s.t < SPIN_TIME {
        s.t = SPIN_TIME;
    }
    let pos = ease_out(s.t / SPIN_TIME) * (PRIZE as f32 + s.land);
    for mut node in &mut strip {
        node.left = Val::Px(-(pos + 0.5) * STEP);
    }
    // A click each time a card passes the marker.
    let card = (pos + 0.5).floor() as i32;
    if card != s.last_card && s.t < SPIN_TIME {
        s.last_card = card;
        sounds.here(Snd::CrateTick);
    }
    // Hold on the prize for a moment, then show it on the skins screen.
    if s.t >= SPIN_TIME && s.last_card != i32::MAX {
        s.last_card = i32::MAX;
        let rare = matches!(skin_def(s.prize.0).rarity, Rarity::Epic | Rarity::Legendary);
        sounds.here(if rare {
            Snd::CrateRare
        } else {
            Snd::CrateReveal
        });
    }
    if s.t >= SPIN_TIME + 1.4 {
        shop.last = Some(s.prize);
        shop.preview = Some(s.prize.0);
        shop.message = None;
        spin.0 = None;
    }
}

fn spawn_overlay(commands: &mut Commands, cards: &[u8]) {
    commands
        .spawn((
            CarouselRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 0.9)),
            GlobalZIndex(50),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("OPENING..."),
                TextFont {
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.8, 0.3)),
            ));
            // The window the cards slide through, with the marker on top.
            p.spawn((
                Node {
                    width: Val::Percent(92.0),
                    height: Val::Px(200.0),
                    overflow: Overflow::clip(),
                    border: UiRect::vertical(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.06, 0.06, 0.09)),
                BorderColor(Color::srgb(0.3, 0.3, 0.4)),
            ))
            .with_children(|w| {
                // An anchor at the centre of the window; the strip hangs off it.
                w.spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Px(15.0),
                    width: Val::Px(0.0),
                    height: Val::Px(170.0),
                    ..default()
                })
                .with_children(|a| {
                    a.spawn((
                        Strip,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(-STEP / 2.0),
                            top: Val::Px(0.0),
                            flex_direction: FlexDirection::Row,
                            ..default()
                        },
                    ))
                    .with_children(|s| {
                        for id in cards {
                            card(s, *id);
                        }
                    });
                });
                // The marker.
                w.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        margin: UiRect::left(Val::Px(-2.0)),
                        width: Val::Px(4.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(1.0, 0.8, 0.25)),
                ));
            });
            p.spawn((
                Text::new("Click to skip"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.65)),
            ));
        });
}

/// One skin card: a colour swatch in the skin's colours, its name and
/// rarity, edged in the rarity colour.
fn card(p: &mut ChildSpawnerCommands, id: u8) {
    let s = skin_def(id);
    let rarity = s.rarity.color();
    let col = |c: [f32; 3]| Color::srgb(c[0], c[1], c[2]);
    p.spawn((
        Node {
            width: Val::Px(STEP - 12.0),
            height: Val::Px(170.0),
            margin: UiRect::horizontal(Val::Px(6.0)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(6.0)),
            border: UiRect::bottom(Val::Px(6.0)),
            row_gap: Val::Px(4.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.13, 0.13, 0.17)),
        BorderColor(rarity),
        BorderRadius::all(Val::Px(6.0)),
    ))
    .with_children(|c| {
        c.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(84.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(col(s.color)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|sw| {
            sw.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(16.0),
                    ..default()
                },
                BackgroundColor(col(s.accent)),
            ));
        });
        c.spawn((
            Text::new(s.name),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::WHITE),
            TextLayout::new_with_justify(JustifyText::Center),
        ));
        c.spawn((
            Text::new(s.rarity.name()),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(rarity),
        ));
    });
}
