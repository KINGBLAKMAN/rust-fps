//! Sound: every sound is synthesised at start-up (see `synth`), then played
//! from a queue any system can push to. Sounds in the world are positional
//! (louder near you, panned left/right); each belongs to a volume group the
//! player can turn up or down in Settings > Audio.

mod synth;

use bevy::audio::{PlaybackMode, SpatialScale, Volume};
use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;

use synth::*;

use crate::config::Settings;
use crate::data::{gun_def, GunClass};
use crate::fx::{Fx, FxQueue};
use crate::player::LocalPlayer;
use crate::weapons::Loadout;
use crate::{
    AppState, BoxState, Enemy, EnemyStatus, MatchState, NetKind, Phase, Replicated, Roster, Session,
};

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundQueue>()
            .add_systems(PreStartup, build_bank)
            .add_systems(
                Update,
                (
                    (
                        fx_sounds.before(crate::fx::play),
                        gun_sounds,
                        movement_sounds,
                        enemy_sounds,
                        match_sounds,
                    )
                        .in_set(Phase::Present)
                        .run_if(in_state(AppState::InGame)),
                    ui_sounds,
                ),
            )
            .add_systems(PostUpdate, play_queue);
    }
}

/// Volume groups (each has its own slider).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Guns,
    Enemies,
    Movement,
    Effects,
    Interface,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Snd {
    ShotPistol,
    ShotMagnum,
    ShotSmg,
    ShotRifle,
    ShotShotgun,
    ShotLmg,
    ShotSniper,
    ShotRay,
    ShotThunder,
    ShotSuppressed,
    ShotTurret,
    DryFire,
    MagOut,
    MagIn,
    Bolt,
    Shell,
    Swap,
    Step,
    StepSoft,
    Jump,
    Land,
    Slide,
    HitTick,
    HitHead,
    Kill,
    Groan,
    BruteRoar,
    ShooterHiss,
    CrawlerRasp,
    ZombieAttack,
    ZombieHurt,
    ZombieDeath,
    Spit,
    PlayerHurt,
    Explosion,
    BigExplosion,
    Fire,
    Ice,
    Heal,
    Zap,
    Slash,
    Whoosh,
    Orbital,
    BladeStorm,
    BoxJingle,
    BoxReady,
    BoxFly,
    Buy,
    Deny,
    Door,
    Perk,
    PowerUp,
    RoundStart,
    RoundEnd,
    LevelUp,
    GameOver,
    Extract,
    UiClick,
    CrateTick,
    CrateReveal,
    CrateRare,
    Ping,
    MeleeSwing,
    MeleeHit,
    Throw,
}

impl Snd {
    pub fn group(self) -> Group {
        use Snd::*;
        match self {
            ShotPistol | ShotMagnum | ShotSmg | ShotRifle | ShotShotgun | ShotLmg | ShotSniper
            | ShotRay | ShotThunder | ShotSuppressed | ShotTurret | DryFire | MagOut | MagIn
            | Bolt | Shell | Swap | HitTick | HitHead | Kill | MeleeSwing | MeleeHit => Group::Guns,
            Groan | BruteRoar | ShooterHiss | CrawlerRasp | ZombieAttack | ZombieHurt
            | ZombieDeath | Spit => Group::Enemies,
            Step | StepSoft | Jump | Land | Slide | PlayerHurt => Group::Movement,
            Explosion | BigExplosion | Fire | Ice | Heal | Zap | Slash | Whoosh | Orbital
            | BladeStorm | Throw => Group::Effects,
            _ => Group::Interface,
        }
    }

    /// Base loudness and how far it carries (metres to half volume).
    fn mix(self) -> (f32, f32) {
        use Snd::*;
        match self {
            ShotSniper | ShotShotgun | ShotMagnum | ShotThunder => (0.9, 30.0),
            ShotPistol | ShotSmg | ShotRifle | ShotLmg | ShotRay => (0.75, 25.0),
            ShotSuppressed | ShotTurret => (0.5, 10.0),
            Explosion | Orbital => (1.0, 40.0),
            BigExplosion => (1.0, 60.0),
            Groan | ShooterHiss | CrawlerRasp => (0.35, 8.0),
            BruteRoar => (0.6, 18.0),
            ZombieAttack | ZombieHurt | ZombieDeath | Spit => (0.5, 10.0),
            Step | StepSoft => (0.35, 6.0),
            Door => (0.8, 20.0),
            BoxJingle | BoxReady | BoxFly => (0.6, 14.0),
            HitTick | HitHead | Kill => (0.55, 100.0),
            UiClick | CrateTick => (0.4, 100.0),
            _ => (0.65, 15.0),
        }
    }
}

/// One sound to play.
pub struct SoundReq {
    pub snd: Snd,
    /// Where in the world (None plays in your head: your own gun, the UI).
    pub pos: Option<Vec3>,
    pub gain: f32,
    pub pitch: f32,
}

/// Sounds requested this frame.
#[derive(Resource, Default)]
pub struct SoundQueue(pub Vec<SoundReq>);

impl SoundQueue {
    pub fn at(&mut self, snd: Snd, pos: Vec3) {
        self.0.push(SoundReq {
            snd,
            pos: Some(pos),
            gain: 1.0,
            pitch: 1.0,
        });
    }
    pub fn here(&mut self, snd: Snd) {
        self.0.push(SoundReq {
            snd,
            pos: None,
            gain: 1.0,
            pitch: 1.0,
        });
    }
    pub fn push(&mut self, snd: Snd, pos: Option<Vec3>, gain: f32, pitch: f32) {
        self.0.push(SoundReq {
            snd,
            pos,
            gain,
            pitch,
        });
    }
}

#[derive(Resource)]
struct Bank(HashMap<Snd, Vec<Handle<AudioSource>>>);

/// The gun sound for a gun, depending on its class (and a suppressor).
pub fn shot_sound(gun: u8, suppressed: bool) -> Snd {
    let d = gun_def(gun);
    if suppressed && d.class != GunClass::Wonder {
        return Snd::ShotSuppressed;
    }
    match d.class {
        GunClass::Pistol if d.damage > 80.0 => Snd::ShotMagnum,
        GunClass::Pistol => Snd::ShotPistol,
        GunClass::Smg => Snd::ShotSmg,
        GunClass::Rifle => Snd::ShotRifle,
        GunClass::Shotgun => Snd::ShotShotgun,
        GunClass::Lmg => Snd::ShotLmg,
        GunClass::Sniper => Snd::ShotSniper,
        GunClass::Wonder if gun == 21 => Snd::ShotRay,
        GunClass::Wonder => Snd::ShotThunder,
    }
}

fn build_bank(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    use Snd::*;
    let mut bank: HashMap<Snd, Vec<Buf>> = HashMap::new();
    let mut add = |s: Snd, b: Buf| bank.entry(s).or_default().push(b);
    let shot = |body, tail, bright, echo| ShotRecipe {
        body,
        tail,
        bright,
        echo,
    };
    for i in 0..3 {
        let seed = 100 + i * 17;
        add(ShotPistol, gunshot(&shot(190.0, 0.05, 5000.0, 0.25), seed));
        add(
            ShotMagnum,
            gunshot(&shot(120.0, 0.09, 3500.0, 0.4), seed + 1),
        );
        add(ShotSmg, gunshot(&shot(230.0, 0.04, 6000.0, 0.2), seed + 2));
        add(
            ShotRifle,
            gunshot(&shot(140.0, 0.07, 4500.0, 0.35), seed + 3),
        );
        add(
            ShotShotgun,
            gunshot(&shot(75.0, 0.14, 2600.0, 0.45), seed + 4),
        );
        add(ShotLmg, gunshot(&shot(110.0, 0.08, 3800.0, 0.35), seed + 5));
        add(
            ShotSniper,
            gunshot(&shot(90.0, 0.16, 3000.0, 0.55), seed + 6),
        );
        add(ShotSuppressed, suppressed(160.0, seed + 7));
        add(
            ShotTurret,
            gunshot(&shot(300.0, 0.03, 7000.0, 0.1), seed + 8),
        );
        add(ShotRay, laser(seed + 9));
        add(ShotThunder, thunder(seed + 10));
    }
    add(DryFire, click(2800.0, 5, 0.06));
    for i in 0..2 {
        add(MagOut, mag_out(200 + i));
        add(MagIn, mag_in(210 + i));
        add(Bolt, bolt(220 + i));
        add(Shell, shell(230 + i));
    }
    add(
        Swap,
        whoosh(0.18, 600.0, 1400.0, 9)
            .mix(&click(2400.0, 10, 0.05), 0.12, 0.8)
            .normalize(0.5),
    );
    for i in 0..5 {
        add(Step, step(300 + i, false));
        add(StepSoft, step(320 + i, true));
    }
    add(
        Jump,
        whoosh(0.2, 300.0, 900.0, 11)
            .mix(&step(12, false), 0.0, 0.6)
            .normalize(0.45),
    );
    add(
        Land,
        thud(70.0, 0.25, 13)
            .mix(&step(14, false), 0.0, 1.0)
            .normalize(0.7),
    );
    add(Slide, whoosh(0.6, 500.0, 1800.0, 15));
    add(
        HitTick,
        bell(&[(2600.0, 1.0, 0.025), (3900.0, 0.4, 0.015)], 0.08),
    );
    add(
        HitHead,
        bell(
            &[
                (1760.0, 1.0, 0.18),
                (2640.0, 0.6, 0.12),
                (4400.0, 0.3, 0.06),
            ],
            0.4,
        ),
    );
    add(
        Kill,
        thud(140.0, 0.15, 16)
            .mix(&bell(&[(1200.0, 0.7, 0.05)], 0.1), 0.0, 0.6)
            .normalize(0.6),
    );

    // Zombie voices: pitch, formants and rasp vary per variant.
    for i in 0..6u32 {
        let f = i as f32;
        add(
            Groan,
            voice(
                &Voice {
                    pitch: 75.0 + f * 9.0,
                    pitch_end: 62.0 + f * 7.0,
                    formants: (480.0 + f * 40.0, 900.0),
                    formants_end: (650.0, 1100.0 - f * 30.0),
                    rasp: 0.25 + 0.05 * f,
                    len: 1.1 + 0.15 * f,
                    attack: 0.25,
                },
                400 + i,
            ),
        );
    }
    for i in 0..2u32 {
        add(
            BruteRoar,
            voice(
                &Voice {
                    pitch: 58.0,
                    pitch_end: 46.0,
                    formants: (420.0, 760.0),
                    formants_end: (520.0, 880.0),
                    rasp: 0.45,
                    len: 1.5,
                    attack: 0.2,
                },
                420 + i,
            )
            .mix(&thud(50.0, 0.4, 421), 0.0, 0.5)
            .normalize(0.9),
        );
        add(
            ShooterHiss,
            voice(
                &Voice {
                    pitch: 140.0,
                    pitch_end: 110.0,
                    formants: (1800.0, 3200.0),
                    formants_end: (1500.0, 2800.0),
                    rasp: 0.75,
                    len: 0.9,
                    attack: 0.3,
                },
                430 + i,
            ),
        );
        add(
            CrawlerRasp,
            voice(
                &Voice {
                    pitch: 120.0,
                    pitch_end: 90.0,
                    formants: (700.0, 1500.0),
                    formants_end: (550.0, 1200.0),
                    rasp: 0.6,
                    len: 0.8,
                    attack: 0.15,
                },
                440 + i,
            ),
        );
    }
    for i in 0..3u32 {
        add(
            ZombieHurt,
            voice(
                &Voice {
                    pitch: 110.0 + i as f32 * 15.0,
                    pitch_end: 80.0,
                    formants: (700.0, 1200.0),
                    formants_end: (500.0, 900.0),
                    rasp: 0.4,
                    len: 0.28,
                    attack: 0.1,
                },
                450 + i,
            ),
        );
        add(
            ZombieDeath,
            voice(
                &Voice {
                    pitch: 95.0 + i as f32 * 10.0,
                    pitch_end: 45.0,
                    formants: (650.0, 1100.0),
                    formants_end: (350.0, 700.0),
                    rasp: 0.35,
                    len: 0.9,
                    attack: 0.08,
                },
                460 + i,
            )
            .mix(&thud(60.0, 0.3, 461 + i), 0.6, 0.6)
            .normalize(0.8),
        );
        add(
            ZombieAttack,
            whoosh(0.25, 700.0, 2200.0, 470 + i)
                .mix(
                    &voice(
                        &Voice {
                            pitch: 130.0,
                            pitch_end: 100.0,
                            formants: (800.0, 1400.0),
                            formants_end: (700.0, 1200.0),
                            rasp: 0.5,
                            len: 0.35,
                            attack: 0.1,
                        },
                        475 + i,
                    ),
                    0.0,
                    0.8,
                )
                .normalize(0.75),
        );
    }
    add(
        Spit,
        whoosh(0.35, 400.0, 1200.0, 480)
            .mix(&fire(481, 0.4), 0.05, 0.5)
            .normalize(0.6),
    );
    for i in 0..2u32 {
        add(
            PlayerHurt,
            voice(
                &Voice {
                    pitch: 150.0 + i as f32 * 20.0,
                    pitch_end: 120.0,
                    formants: (600.0, 1000.0),
                    formants_end: (500.0, 900.0),
                    rasp: 0.2,
                    len: 0.22,
                    attack: 0.08,
                },
                490 + i,
            )
            .mix(&thud(90.0, 0.15, 492), 0.0, 0.6)
            .normalize(0.7),
        );
    }
    add(Explosion, explosion(500, 1.0));
    add(Explosion, explosion(501, 1.0));
    add(BigExplosion, explosion(502, 1.8));
    add(Fire, fire(510, 1.0));
    add(Ice, ice(520));
    add(Heal, chime_up(523.0, &[1.0, 1.26, 1.5, 2.0, 2.52], 0.07));
    add(Zap, zap(530, 0.45));
    add(
        Slash,
        whoosh(0.22, 1500.0, 4500.0, 540)
            .mix(
                &bell(&[(3200.0, 0.5, 0.12), (4800.0, 0.3, 0.08)], 0.3),
                0.08,
                0.6,
            )
            .normalize(0.7),
    );
    add(Whoosh, whoosh(0.35, 400.0, 1600.0, 550));
    add(Throw, whoosh(0.25, 500.0, 1300.0, 551));
    add(
        Orbital,
        render(1.0, {
            let mut o = Osc::new();
            move |t| o.sine(300.0 + 900.0 * t) * t * 0.6
        })
        .mix(&explosion(560, 1.6), 0.9, 1.0)
        .normalize(1.0),
    );
    add(
        BladeStorm,
        whoosh(1.2, 800.0, 3000.0, 570)
            .mix(&bell(&[(2900.0, 0.4, 0.2)], 0.6), 0.2, 0.4)
            .normalize(0.7),
    );

    // A music-box tune while the box spins.
    let tune = [
        659.0, 784.0, 988.0, 784.0, 880.0, 659.0, 740.0, 988.0, 1175.0, 988.0, 880.0, 784.0,
    ];
    let notes: Vec<(f32, f32)> = tune
        .iter()
        .enumerate()
        .map(|(i, f)| (*f, i as f32 * 0.23))
        .collect();
    add(BoxJingle, melody(&notes, 0.6));
    add(
        BoxReady,
        bell(
            &[(1318.0, 1.0, 0.5), (1975.0, 0.6, 0.4), (2637.0, 0.3, 0.3)],
            1.0,
        ),
    );
    add(BoxFly, chime_up(880.0, &[1.0, 0.89, 0.75, 0.67, 0.5], 0.12));
    add(
        Buy,
        bell(&[(2093.0, 1.0, 0.15), (2637.0, 0.8, 0.2)], 0.4)
            .mix(&click(3000.0, 600, 0.05), 0.0, 0.5)
            .normalize(0.6),
    );
    add(
        Deny,
        render(0.25, {
            let mut o = Osc::new();
            move |t| o.square(110.0) * env(t, 0.005, 0.12) * 0.5
        })
        .normalize(0.5),
    );
    add(
        Door,
        rumble(1.5, 610)
            .mix(&thud(60.0, 0.4, 611), 1.3, 0.7)
            .normalize(0.8),
    );
    add(
        Perk,
        thud(200.0, 0.1, 620)
            .mix(&thud(180.0, 0.1, 621), 0.18, 1.0)
            .mix(&chime_up(784.0, &[1.0, 1.5, 2.0], 0.08), 0.35, 0.8)
            .normalize(0.7),
    );
    add(
        PowerUp,
        chime_up(659.0, &[1.0, 1.26, 1.5, 2.0, 2.52, 3.0], 0.06),
    );
    add(
        RoundStart,
        bell(
            &[
                (110.0, 1.0, 2.0),
                (220.0, 0.6, 1.4),
                (331.0, 0.4, 1.0),
                (462.0, 0.25, 0.6),
            ],
            3.0,
        ),
    );
    add(
        RoundEnd,
        bell(
            &[(147.0, 1.0, 1.6), (294.0, 0.5, 1.0), (441.0, 0.3, 0.6)],
            2.4,
        ),
    );
    add(LevelUp, chime_up(784.0, &[1.0, 1.26, 1.5, 2.0], 0.09));
    add(
        GameOver,
        chime_up(392.0, &[1.0, 0.94, 0.84, 0.75, 0.5], 0.3),
    );
    add(
        Extract,
        chime_up(523.0, &[1.0, 1.26, 1.5, 2.0, 2.52, 3.0, 4.0], 0.11),
    );
    add(UiClick, click(3500.0, 700, 0.04));
    add(CrateTick, click(1900.0, 710, 0.05));
    add(CrateReveal, chime_up(1047.0, &[1.0, 1.5, 2.0], 0.06));
    add(
        CrateRare,
        chime_up(784.0, &[1.0, 1.26, 1.5, 2.0, 2.52, 3.0, 4.0], 0.07)
            .mix(&bell(&[(3136.0, 0.6, 0.6)], 1.0), 0.5, 0.6)
            .normalize(0.7),
    );
    add(
        Ping,
        bell(&[(1568.0, 1.0, 0.08)], 0.1)
            .mix(&bell(&[(2093.0, 1.0, 0.12)], 0.2), 0.09, 1.0)
            .normalize(0.5),
    );
    add(MeleeSwing, whoosh(0.22, 600.0, 2000.0, 720));
    add(
        MeleeHit,
        thud(110.0, 0.2, 721)
            .mix(&click(900.0, 722, 0.08), 0.0, 0.6)
            .normalize(0.85),
    );

    let bank = bank
        .into_iter()
        .map(|(s, bufs)| {
            let handles = bufs
                .into_iter()
                .map(|b| {
                    sources.add(AudioSource {
                        bytes: Arc::from(b.wav()),
                    })
                })
                .collect();
            (s, handles)
        })
        .collect();
    commands.insert_resource(Bank(bank));
}

#[derive(Component)]
struct Voice3d;

/// Plays everything queued this frame.
fn play_queue(
    mut commands: Commands,
    mut queue: ResMut<SoundQueue>,
    bank: Option<Res<Bank>>,
    settings: Res<Settings>,
    listener: Query<&GlobalTransform, With<LocalPlayer>>,
    playing: Query<(), With<Voice3d>>,
) {
    let Some(bank) = bank else { return };
    let ear = listener
        .iter()
        .next()
        .map_or(Vec3::ZERO, |g| g.translation());
    let mut active = playing.iter().count();
    let mut this_frame: HashMap<Snd, u32> = HashMap::new();
    let mut rng = rand::thread_rng();
    for req in queue.0.drain(..) {
        let group = match req.snd.group() {
            Group::Guns => settings.vol_guns,
            Group::Enemies => settings.vol_enemies,
            Group::Movement => settings.vol_movement,
            Group::Effects => settings.vol_effects,
            Group::Interface => settings.vol_interface,
        };
        let (base, reach) = req.snd.mix();
        let falloff = req.pos.map_or(1.0, |p| {
            let d = p.distance(ear);
            1.0 / (1.0 + (d / reach).powi(2))
        });
        let vol = settings.volume / 100.0 * group / 100.0 * base * req.gain * falloff;
        if vol < 0.01 {
            continue;
        }
        // Don't stack dozens of the same sound in one frame, or swamp the mixer.
        let n = this_frame.entry(req.snd).or_default();
        *n += 1;
        if *n > 3 || active > 48 {
            continue;
        }
        let Some(list) = bank.0.get(&req.snd) else {
            continue;
        };
        let handle = list[rng.gen_range(0..list.len())].clone();
        let pitch = req.pitch * rng.gen_range(0.96..1.04);
        let settings = PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::Linear(vol),
            speed: pitch,
            spatial: req.pos.is_some(),
            // Shrink the world for the panner so it only pans (our own
            // falloff above handles distance).
            spatial_scale: Some(SpatialScale::new(0.02)),
            ..PlaybackSettings::DESPAWN
        };
        commands.spawn((
            AudioPlayer(handle),
            settings,
            Transform::from_translation(req.pos.unwrap_or(ear)),
            Voice3d,
            crate::InGameEntity,
        ));
        active += 1;
    }
}

// ---------------------------------------------------------------------------
// What makes sounds
// ---------------------------------------------------------------------------

/// Effects from abilities, explosions, other players' shots and pings.
fn fx_sounds(
    queue: Res<FxQueue>,
    session: Res<Session>,
    roster: Res<Roster>,
    mut sounds: ResMut<SoundQueue>,
    mut seen: Local<Vec<u8>>,
) {
    seen.clear();
    for fx in &queue.0 {
        match *fx {
            Fx::Tracer { shooter, a, .. } => {
                if shooter == session.my_id || seen.contains(&shooter) {
                    continue;
                }
                seen.push(shooter);
                let snd = match roster.0.get(&shooter) {
                    Some(p) => {
                        let slot = p.active_slot as usize;
                        match p.guns[slot] {
                            Some(g) => shot_sound(g, p.attach[slot].handling(g).quiet),
                            None => Snd::ShotPistol,
                        }
                    }
                    None => Snd::ShotTurret,
                };
                sounds.at(snd, Vec3::from_array(a));
            }
            Fx::Explosion { pos, radius, .. } => sounds.at(
                if radius > 7.0 {
                    Snd::BigExplosion
                } else {
                    Snd::Explosion
                },
                Vec3::from_array(pos),
            ),
            Fx::Lightning { b, .. } => sounds.at(Snd::Zap, Vec3::from_array(b)),
            Fx::Spear { a, b } => {
                sounds.at(Snd::Throw, Vec3::from_array(a));
                sounds.at(Snd::Zap, Vec3::from_array(b));
            }
            Fx::Slam { pos, .. } => sounds.at(Snd::ShotThunder, Vec3::from_array(pos)),
            Fx::Beam { pos, .. } => sounds.at(Snd::Orbital, Vec3::from_array(pos)),
            Fx::Heal { pos, .. } => sounds.at(Snd::Heal, Vec3::from_array(pos)),
            Fx::Nova { pos, .. } => sounds.at(Snd::Ice, Vec3::from_array(pos)),
            Fx::Dash { a, .. } => sounds.at(Snd::Whoosh, Vec3::from_array(a)),
            Fx::Slash { pos, .. } => sounds.at(Snd::Slash, Vec3::from_array(pos)),
            Fx::Cone { pos, .. } => sounds.at(Snd::Fire, Vec3::from_array(pos)),
            Fx::Zone { pos, kind, .. } => sounds.at(
                match kind {
                    0 => Snd::BladeStorm,
                    1 => Snd::Zap,
                    4 => Snd::ShotThunder,
                    _ => Snd::Fire,
                },
                Vec3::from_array(pos),
            ),
            Fx::Ping { pos, .. } => sounds.push(Snd::Ping, Some(Vec3::from_array(pos)), 1.0, 1.0),
            Fx::Cast { .. } => {}
            Fx::Ring { pos, .. } => sounds.push(Snd::Whoosh, Some(Vec3::from_array(pos)), 0.5, 1.2),
        }
    }
}

/// Your own gun: shots, reloads, empty clicks, hit confirmations.
fn gun_sounds(
    loadout: Res<Loadout>,
    session: Res<Session>,
    roster: Res<Roster>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut sounds: ResMut<SoundQueue>,
    mut last: Local<(u32, f32, usize, u32, bool, bool)>,
) {
    let (last_shots, last_reload, last_slot, last_kills, was_hit, swinging) = &mut *last;
    if let Some(g) = loadout.current() {
        if loadout.shots != *last_shots {
            sounds.here(shot_sound(g.id, g.attach.handling(g.id).quiet));
        }
        // Reload sounds at points through the reload.
        if loadout.reload > 0.0 && loadout.reload_total > 0.0 {
            let k = 1.0 - loadout.reload / loadout.reload_total;
            let before = 1.0 - *last_reload / loadout.reload_total;
            let shells = matches!(g.id, 10 | 11 | 12 | 18);
            let marks: &[(f32, Snd)] = if shells {
                &[
                    (0.2, Snd::Shell),
                    (0.45, Snd::Shell),
                    (0.7, Snd::Shell),
                    (0.92, Snd::Bolt),
                ]
            } else {
                &[(0.12, Snd::MagOut), (0.6, Snd::MagIn), (0.88, Snd::Bolt)]
            };
            for (at, snd) in marks {
                if before < *at && k >= *at {
                    sounds.here(*snd);
                }
            }
        }
        if g.mag == 0 && g.reserve == 0 && mouse.just_pressed(MouseButton::Left) {
            sounds.here(Snd::DryFire);
        }
    }
    if loadout.active != *last_slot {
        sounds.here(Snd::Swap);
    }
    if loadout.melee.is_some_and(|t| t < 0.03) && !*swinging {
        sounds.here(Snd::MeleeSwing);
    }
    *swinging = loadout.melee.is_some();
    let hit = loadout.hitmarker > 0.0;
    if hit && (!*was_hit || loadout.shots != *last_shots) {
        sounds.here(if loadout.melee.is_some() {
            Snd::MeleeHit
        } else if loadout.headshot {
            Snd::HitHead
        } else {
            Snd::HitTick
        });
    }
    let kills = roster.me(&session).map_or(0, |m| m.kills);
    if kills > *last_kills {
        sounds.here(Snd::Kill);
    }
    *last_shots = loadout.shots;
    *last_reload = loadout.reload;
    *last_slot = loadout.active;
    *last_kills = kills;
    *was_hit = hit;
}

/// Footsteps, jumps, landings and slides, for you and your teammates.
fn movement_sounds(
    time: Res<Time>,
    session: Res<Session>,
    roster: Res<Roster>,
    map: Option<Res<crate::maps::CurrentMap>>,
    player: Single<&LocalPlayer>,
    mut sounds: ResMut<SoundQueue>,
    mut me: Local<(f32, bool, f32, bool, f32)>,
    mut others: Local<HashMap<u8, (Vec3, f32)>>,
) {
    let soft = map.is_some_and(|m| m.0.ground_kind == crate::maps::Ground::Grass);
    let step = if soft { Snd::StepSoft } else { Snd::Step };
    let p = *player;
    let (walked, grounded, air, sliding, hurt) = &mut *me;
    let dt = time.delta_secs();
    if p.on_ground && p.sliding <= 0.0 {
        *walked += p.horizontal_speed() * dt;
        let stride = if p.sprinting {
            2.6
        } else if p.crouching {
            1.6
        } else {
            2.1
        };
        if *walked > stride {
            *walked = 0.0;
            let gain = if p.crouching { 0.4 } else { 1.0 };
            sounds.push(step, None, gain, 1.0);
        }
    }
    if !p.on_ground {
        *air += dt;
    }
    if *grounded && !p.on_ground && p.vel.y > 1.0 {
        sounds.here(Snd::Jump);
    }
    if !*grounded && p.on_ground && *air > 0.25 {
        sounds.push(Snd::Land, None, (*air * 1.5).min(1.0), 1.0);
    }
    if p.on_ground {
        *air = 0.0;
    }
    let slide = p.sliding > 0.0;
    if slide && !*sliding {
        sounds.here(Snd::Slide);
    }
    *grounded = p.on_ground;
    *sliding = slide;

    // Getting hurt.
    if let Some(m) = roster.me(&session) {
        if m.health < *hurt - 1.0 && m.alive {
            sounds.here(Snd::PlayerHurt);
        }
        *hurt = m.health;
    }

    // Teammates' footsteps.
    for (id, info) in &roster.0 {
        if *id == session.my_id || !info.alive {
            continue;
        }
        let feet = info.feet();
        let entry = others.entry(*id).or_insert((feet, 0.0));
        let moved = feet.with_y(0.0).distance(entry.0.with_y(0.0));
        if moved < 3.0 && feet.y < 0.3 {
            entry.1 += moved;
        }
        entry.0 = feet;
        if entry.1 > 2.2 {
            entry.1 = 0.0;
            sounds.at(step, feet);
        }
    }
}

/// Zombies groan now and then, grunt when shot, scream when they die.
#[allow(clippy::type_complexity)]
fn enemy_sounds(
    time: Res<Time>,
    enemies: Query<(&Replicated, &Transform, Option<&EnemyStatus>), With<Enemy>>,
    projectiles: Query<(&Replicated, &Transform), Without<Enemy>>,
    mut sounds: ResMut<SoundQueue>,
    mut known: Local<HashMap<u32, (Vec3, f32, bool, NetKind)>>,
    mut shots: Local<Vec<u32>>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    let mut alive: Vec<u32> = Vec::new();
    for (r, tf, status) in &enemies {
        alive.push(r.id);
        let pos = tf.translation + Vec3::Y * 1.5;
        let flash = status.is_some_and(|s| s.flash > 0.0);
        let crawler = tf.scale.y < 0.7 * tf.scale.x;
        let entry = known
            .entry(r.id)
            .or_insert_with(|| (pos, rng.gen_range(0.5..6.0), false, r.kind));
        entry.0 = pos;
        entry.1 -= dt;
        if entry.1 <= 0.0 {
            entry.1 = rng.gen_range(3.5..9.0);
            let snd = match r.kind {
                NetKind::Brute => Snd::BruteRoar,
                NetKind::Shooter => Snd::ShooterHiss,
                _ if crawler => Snd::CrawlerRasp,
                _ => Snd::Groan,
            };
            sounds.at(snd, pos);
        }
        if flash && !entry.2 && rng.gen_bool(0.35) {
            sounds.at(Snd::ZombieHurt, pos);
        }
        entry.2 = flash;
    }
    known.retain(|id, (pos, _, _, kind)| {
        let keep = alive.contains(id);
        if !keep {
            let pitch = if *kind == NetKind::Brute { 0.7 } else { 1.0 };
            sounds.push(Snd::ZombieDeath, Some(*pos), 1.0, pitch);
        }
        keep
    });
    // Fireballs spat by shooters.
    let mut now: Vec<u32> = Vec::new();
    for (r, tf) in &projectiles {
        if r.kind == NetKind::Fireball {
            now.push(r.id);
            if !shots.contains(&r.id) {
                sounds.at(Snd::Spit, tf.translation);
            }
        }
        if r.kind == NetKind::Grenade && !shots.contains(&r.id) {
            now.push(r.id);
            sounds.at(Snd::Throw, tf.translation);
        } else if r.kind == NetKind::Grenade {
            now.push(r.id);
        }
    }
    *shots = now;
}

/// Rounds, the box, doors, perks, power-ups, purchases.
#[allow(clippy::too_many_arguments)]
fn match_sounds(
    state: Res<MatchState>,
    session: Res<Session>,
    roster: Res<Roster>,
    map: Option<Res<crate::maps::CurrentMap>>,
    mut sounds: ResMut<SoundQueue>,
    mut last: Local<Option<MatchState>>,
    mut last_me: Local<Option<(u32, u8, u32, [Option<u8>; 2])>>,
) {
    let Some(map) = map else { return };
    if let Some(prev) = last.as_ref() {
        if state.round != prev.round && state.round > 0 {
            sounds.here(Snd::RoundStart);
        }
        if state.intermission > 0.0 && prev.intermission <= 0.0 && state.round > 0 {
            sounds.here(Snd::RoundEnd);
        }
        if state.game_over && !prev.game_over {
            sounds.here(Snd::GameOver);
        }
        if state.extracted && !prev.extracted {
            sounds.here(Snd::Extract);
        }
        let box_pos = map.0.box_spots[(state.box_spot as usize).min(4)];
        match (prev.box_state, state.box_state) {
            (BoxState::Idle, BoxState::Rolling { .. }) => sounds.at(Snd::BoxJingle, box_pos),
            (BoxState::Rolling { .. }, BoxState::Offer { .. }) => sounds.at(Snd::BoxReady, box_pos),
            (BoxState::Rolling { .. }, BoxState::Moving { .. }) => {
                let from = map.0.box_spots[(prev.box_spot as usize).min(4)];
                sounds.at(Snd::BoxFly, from)
            }
            _ => {}
        }
        for d in &map.0.doors {
            if d.locked(prev.doors) && !d.locked(state.doors) {
                sounds.at(Snd::Door, d.pos + Vec3::Y * 2.0);
            }
        }
        if state.powerup_seq != prev.powerup_seq {
            sounds.here(Snd::PowerUp);
        }
    }
    *last = Some(state.clone());

    if let Some(m) = roster.me(&session) {
        if let Some((points, perks, level, guns)) = *last_me {
            if m.perks != perks && m.perks != 0 {
                sounds.here(Snd::Perk);
            } else if m.points < points && m.guns != guns {
                sounds.here(Snd::Buy);
            } else if m.points + 10 < points {
                sounds.here(Snd::Buy);
            }
            if m.level > level {
                sounds.here(Snd::LevelUp);
            }
        }
        *last_me = Some((m.points, m.perks, m.level, m.guns));
    }
}

/// Clicks on menu buttons.
fn ui_sounds(
    buttons: Query<&Interaction, (Changed<Interaction>, With<Button>)>,
    mut sounds: ResMut<SoundQueue>,
) {
    for i in &buttons {
        if *i == Interaction::Pressed {
            sounds.here(Snd::UiClick);
        }
    }
}
