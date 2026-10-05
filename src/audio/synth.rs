//! A tiny synthesiser: every sound in the game is generated here at start-up
//! (noise bursts, swept tones, filtered formants), so the game ships without
//! any audio files.

use std::f32::consts::TAU;

pub const RATE: u32 = 44_100;

/// A mono buffer of samples in -1..1.
pub struct Buf(pub Vec<f32>);

/// Deterministic white noise.
pub struct Noise(u32);

impl Noise {
    pub fn new(seed: u32) -> Self {
        Self(seed.wrapping_mul(2_654_435_761).max(1))
    }
    pub fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// One-pole low-pass filter.
pub struct Lp {
    y: f32,
}

impl Lp {
    pub fn new() -> Self {
        Self { y: 0.0 }
    }
    pub fn run(&mut self, x: f32, cutoff: f32) -> f32 {
        let a = 1.0 - (-TAU * cutoff.max(10.0) / RATE as f32).exp();
        self.y += a * (x - self.y);
        self.y
    }
}

/// Resonant band-pass (biquad, constant peak gain).
pub struct Bp {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Bp {
    pub fn new() -> Self {
        Self {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
    pub fn run(&mut self, x: f32, freq: f32, q: f32) -> f32 {
        let w = TAU * freq.clamp(20.0, RATE as f32 * 0.45) / RATE as f32;
        let alpha = w.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        let b0 = alpha / a0;
        let b2 = -alpha / a0;
        let a1 = -2.0 * w.cos() / a0;
        let a2 = (1.0 - alpha) / a0;
        let y = b0 * x + b2 * self.x2 - a1 * self.y1 - a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Fills `secs` of audio from `f(t)`.
pub fn render(secs: f32, mut f: impl FnMut(f32) -> f32) -> Buf {
    let n = (secs * RATE as f32) as usize;
    Buf((0..n).map(|i| f(i as f32 / RATE as f32)).collect())
}

/// Exponential decay with time constant `tau`.
pub fn decay(t: f32, tau: f32) -> f32 {
    (-t / tau).exp()
}

/// Rises over `a` seconds, then decays with `tau`.
pub fn env(t: f32, a: f32, tau: f32) -> f32 {
    if t < a {
        t / a
    } else {
        decay(t - a, tau)
    }
}

/// A sine oscillator that can change pitch smoothly.
pub struct Osc {
    phase: f32,
}

impl Osc {
    pub fn new() -> Self {
        Self { phase: 0.0 }
    }
    pub fn sine(&mut self, freq: f32) -> f32 {
        self.phase = (self.phase + freq / RATE as f32).fract();
        (self.phase * TAU).sin()
    }
    /// Band-limited-ish sawtooth (a few harmonics), good for voices.
    pub fn saw(&mut self, freq: f32) -> f32 {
        self.phase = (self.phase + freq / RATE as f32).fract();
        let p = self.phase * TAU;
        let mut s = 0.0;
        let harmonics = ((RATE as f32 * 0.4) / freq.max(20.0)).min(24.0) as i32;
        for k in 1..=harmonics.max(1) {
            s += (p * k as f32).sin() / k as f32;
        }
        s * 0.6
    }
    pub fn square(&mut self, freq: f32) -> f32 {
        self.phase = (self.phase + freq / RATE as f32).fract();
        if self.phase < 0.5 {
            1.0
        } else {
            -1.0
        }
    }
}

impl Buf {
    /// Scales so the loudest sample hits `peak`, with a soft clip.
    pub fn normalize(mut self, peak: f32) -> Self {
        let max = self.0.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-6);
        for s in &mut self.0 {
            *s = (*s / max * peak * 1.2).tanh() / 1.2f32.tanh();
        }
        self.fade()
    }

    /// Short fades at both ends so nothing clicks.
    pub fn fade(mut self) -> Self {
        let n = self.0.len();
        let f = 64.min(n / 2);
        for i in 0..f {
            let g = i as f32 / f as f32;
            self.0[i] *= g;
            self.0[n - 1 - i] *= g;
        }
        self
    }

    /// Mixes `other` in, starting `at` seconds in.
    pub fn mix(mut self, other: &Buf, at: f32, gain: f32) -> Self {
        let start = (at * RATE as f32) as usize;
        if self.0.len() < start + other.0.len() {
            self.0.resize(start + other.0.len(), 0.0);
        }
        for (i, s) in other.0.iter().enumerate() {
            self.0[start + i] += s * gain;
        }
        self
    }

    /// A cheap room echo: a few delayed, darker copies.
    pub fn echo(mut self, delay: f32, feedback: f32, taps: usize) -> Self {
        let d = (delay * RATE as f32) as usize;
        let extra = d * taps;
        let n = self.0.len();
        self.0.resize(n + extra, 0.0);
        let mut lp = Lp::new();
        for i in d..self.0.len() {
            let wet = lp.run(self.0[i - d], 2500.0) * feedback;
            self.0[i] += wet;
        }
        self
    }

    /// 16-bit mono WAV file bytes.
    pub fn wav(&self) -> Vec<u8> {
        let data_len = (self.0.len() * 2) as u32;
        let mut out = Vec::with_capacity(44 + data_len as usize);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&RATE.to_le_bytes());
        out.extend_from_slice(&(RATE * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for s in &self.0 {
            out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32_000.0) as i16).to_le_bytes());
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Sound recipes
// ---------------------------------------------------------------------------

/// What a gunshot sounds like.
pub struct ShotRecipe {
    /// Low "body" thump frequency.
    pub body: f32,
    /// How long the blast rings.
    pub tail: f32,
    /// Brightness of the blast noise.
    pub bright: f32,
    /// Echo off the surroundings.
    pub echo: f32,
}

pub fn gunshot(r: &ShotRecipe, seed: u32) -> Buf {
    let mut n = Noise::new(seed);
    let mut lp = Lp::new();
    let mut lp2 = Lp::new();
    let mut o = Osc::new();
    let len = r.tail * 4.0 + 0.05;
    let b = render(len, |t| {
        let x = n.next();
        let crack = x * decay(t, 0.0025);
        let blast = lp.run(x, r.bright * (0.35 + 0.65 * decay(t, 0.03))) * decay(t, r.tail);
        let thump = o.sine(r.body * (1.0 + 2.5 * decay(t, 0.012))) * decay(t, r.tail * 0.9);
        let rumble = lp2.run(x, 300.0) * decay(t, r.tail * 2.5) * 0.6;
        crack * 0.8 + blast * 1.3 + thump * 0.9 + rumble
    });
    b.echo(0.09, r.echo, 3).normalize(0.95)
}

/// A gun fitted with a suppressor: a dull "thup" and a mechanical clack.
pub fn suppressed(body: f32, seed: u32) -> Buf {
    let mut n = Noise::new(seed);
    let mut lp = Lp::new();
    let mut o = Osc::new();
    let b = render(0.18, |t| {
        let x = n.next();
        lp.run(x, 900.0 * decay(t, 0.03) + 200.0) * decay(t, 0.03) * 1.5
            + o.sine(body * 1.4) * decay(t, 0.025) * 0.5
            + x * decay((t - 0.012).abs(), 0.0015) * 0.25
    });
    b.normalize(0.6)
}

pub fn laser(seed: u32) -> Buf {
    let mut o = Osc::new();
    let mut o2 = Osc::new();
    let mut n = Noise::new(seed);
    render(0.35, |t| {
        let f = 2400.0 * decay(t, 0.06) + 260.0;
        (o.sine(f) + 0.4 * o2.square(f * 0.5 + 30.0 * (t * 60.0).sin())) * env(t, 0.004, 0.09)
            + n.next() * decay(t, 0.01) * 0.3
    })
    .normalize(0.8)
}

pub fn thunder(seed: u32) -> Buf {
    let mut n = Noise::new(seed);
    let mut lp = Lp::new();
    let mut o = Osc::new();
    let mut spark = 0.0f32;
    render(0.9, |t| {
        let x = n.next();
        if n.next() > 0.995 {
            spark = 1.0;
        }
        spark *= 0.993;
        lp.run(x, 1800.0 * decay(t, 0.1) + 120.0) * env(t, 0.003, 0.25) * 1.2
            + x * spark * decay(t, 0.4) * 0.6
            + o.sine(48.0) * decay(t, 0.3) * 0.8
    })
    .normalize(0.95)
}

/// Small mechanical click (bolts, magazines, triggers).
pub fn click(freq: f32, seed: u32, len: f32) -> Buf {
    let mut n = Noise::new(seed);
    let mut bp = Bp::new();
    let mut o = Osc::new();
    render(len, |t| {
        bp.run(n.next(), freq, 4.0) * decay(t, 0.004) * 3.0
            + o.sine(freq * 0.5) * decay(t, 0.008) * 0.3
    })
    .normalize(0.7)
}

/// A slide of metal on metal.
pub fn slide(freq: f32, len: f32, seed: u32) -> Buf {
    let mut n = Noise::new(seed);
    let mut bp = Bp::new();
    render(len, |t| {
        let k = t / len;
        bp.run(n.next(), freq * (0.8 + 0.4 * k), 3.0) * (k * (1.0 - k) * 4.0).min(1.0)
    })
    .normalize(0.45)
}

pub fn mag_out(seed: u32) -> Buf {
    click(2200.0, seed, 0.05)
        .mix(&slide(1800.0, 0.12, seed + 1), 0.02, 0.8)
        .normalize(0.55)
}

pub fn mag_in(seed: u32) -> Buf {
    slide(1500.0, 0.08, seed)
        .mix(&click(2600.0, seed + 1, 0.05), 0.07, 1.0)
        .mix(&click(3200.0, seed + 2, 0.04), 0.1, 0.6)
        .normalize(0.65)
}

pub fn bolt(seed: u32) -> Buf {
    click(2000.0, seed, 0.04)
        .mix(&slide(2400.0, 0.1, seed + 1), 0.03, 0.9)
        .mix(&click(3000.0, seed + 2, 0.05), 0.14, 1.1)
        .normalize(0.7)
}

pub fn shell(seed: u32) -> Buf {
    slide(1200.0, 0.06, seed)
        .mix(&click(1700.0, seed + 1, 0.05), 0.05, 1.0)
        .normalize(0.55)
}

/// Feet on the ground. `soft` is grass (duller, longer).
pub fn step(seed: u32, soft: bool) -> Buf {
    let mut n = Noise::new(seed);
    let mut lp = Lp::new();
    let mut bp = Bp::new();
    let (cut, tau) = if soft { (700.0, 0.05) } else { (1400.0, 0.025) };
    render(0.16, |t| {
        let x = n.next();
        let heel = lp.run(x, cut) * decay(t, tau) * 1.6;
        let toe = if t > 0.04 {
            lp.run(x, cut) * decay(t - 0.04, tau * 0.8) * 0.8
        } else {
            0.0
        };
        let grit = bp.run(x, if soft { 2200.0 } else { 4000.0 }, 1.0) * decay(t, 0.04) * 0.25;
        heel + toe + grit
    })
    .normalize(0.5)
}

pub fn whoosh(len: f32, lo: f32, hi: f32, seed: u32) -> Buf {
    let mut n = Noise::new(seed);
    let mut bp = Bp::new();
    render(len, |t| {
        let k = t / len;
        let f = lo + (hi - lo) * (k * std::f32::consts::PI).sin();
        bp.run(n.next(), f, 1.2) * (k * std::f32::consts::PI).sin().powf(1.5)
    })
    .normalize(0.6)
}

pub fn thud(freq: f32, len: f32, seed: u32) -> Buf {
    let mut n = Noise::new(seed);
    let mut lp = Lp::new();
    let mut o = Osc::new();
    render(len, |t| {
        lp.run(n.next(), 600.0) * decay(t, len * 0.2) * 1.2
            + o.sine(freq * (1.0 + decay(t, 0.02))) * decay(t, len * 0.3)
    })
    .normalize(0.8)
}

/// A voice-like sound: a buzzy source through two formant filters.
pub struct Voice {
    pub pitch: f32,
    pub pitch_end: f32,
    pub formants: (f32, f32),
    pub formants_end: (f32, f32),
    pub rasp: f32,
    pub len: f32,
    pub attack: f32,
}

pub fn voice(v: &Voice, seed: u32) -> Buf {
    let mut o = Osc::new();
    let mut n = Noise::new(seed);
    let (mut f1, mut f2, mut lp) = (Bp::new(), Bp::new(), Lp::new());
    let wobble = 3.0 + (seed % 5) as f32;
    render(v.len, |t| {
        let k = t / v.len;
        let pitch =
            (v.pitch + (v.pitch_end - v.pitch) * k) * (1.0 + 0.04 * (t * wobble * TAU).sin());
        let src = o.saw(pitch) * (1.0 - v.rasp) + n.next() * v.rasp;
        let fa = v.formants.0 + (v.formants_end.0 - v.formants.0) * k;
        let fb = v.formants.1 + (v.formants_end.1 - v.formants.1) * k;
        let s = f1.run(src, fa, 5.0) + 0.6 * f2.run(src, fb, 6.0);
        let shape = if k < v.attack {
            k / v.attack
        } else {
            ((1.0 - k) / (1.0 - v.attack)).powf(0.7)
        };
        lp.run(s, 3000.0) * shape
    })
    .normalize(0.8)
}

/// Bright decaying partials (bells, chimes, music-box tines).
pub fn bell(partials: &[(f32, f32, f32)], len: f32) -> Buf {
    let mut oscs: Vec<Osc> = partials.iter().map(|_| Osc::new()).collect();
    render(len, |t| {
        partials
            .iter()
            .zip(oscs.iter_mut())
            .map(|((f, amp, tau), o)| o.sine(*f) * amp * env(t, 0.002, *tau))
            .sum::<f32>()
    })
    .normalize(0.7)
}

/// A sequence of tine notes (frequency, start time).
pub fn melody(notes: &[(f32, f32)], note_len: f32) -> Buf {
    let mut out = Buf(Vec::new());
    for (f, at) in notes {
        let tine = bell(
            &[
                (*f, 1.0, 0.25),
                (f * 2.0, 0.35, 0.12),
                (f * 3.01, 0.15, 0.06),
            ],
            note_len,
        );
        out = out.mix(&tine, *at, 1.0);
    }
    out.normalize(0.6)
}

pub fn explosion(seed: u32, size: f32) -> Buf {
    let mut n = Noise::new(seed);
    let mut lp = Lp::new();
    let mut lp2 = Lp::new();
    let mut o = Osc::new();
    let mut crackle = 0.0f32;
    let len = 1.2 * size;
    render(len, |t| {
        let x = n.next();
        if n.next() > 0.997 {
            crackle = 1.0;
        }
        crackle *= 0.99;
        let boom = lp.run(x, 3500.0 * decay(t, 0.05) + 150.0) * env(t, 0.002, 0.25 * size) * 1.4;
        let sub = o.sine(42.0 + 30.0 * decay(t, 0.05)) * decay(t, 0.35 * size) * 1.2;
        let debris = lp2.run(x, 2500.0) * crackle * decay(t, 0.6 * size) * 0.5;
        boom + sub + debris
    })
    .normalize(1.0)
}

pub fn zap(seed: u32, len: f32) -> Buf {
    let mut n = Noise::new(seed);
    let mut o = Osc::new();
    let mut bp = Bp::new();
    render(len, |t| {
        let buzz = o.square(110.0 + 40.0 * (t * 37.0).sin());
        let fizz = bp.run(n.next(), 3500.0, 2.0) * if n.next() > 0.6 { 1.0 } else { 0.2 };
        (buzz * 0.35 + fizz) * env(t, 0.005, len * 0.4)
    })
    .normalize(0.7)
}

pub fn fire(seed: u32, len: f32) -> Buf {
    let mut n = Noise::new(seed);
    let mut lp = Lp::new();
    let mut bp = Bp::new();
    render(len, |t| {
        let k = t / len;
        let x = n.next();
        let roar = lp.run(x, 600.0 + 1500.0 * (k * 3.0).min(1.0)) * 1.4;
        let hiss = bp.run(x, 5000.0, 0.8) * 0.3;
        let pop = if n.next() > 0.998 { x * 4.0 } else { 0.0 };
        (roar + hiss + pop) * env(t, len * 0.15, len * 0.35)
    })
    .normalize(0.75)
}

pub fn ice(seed: u32) -> Buf {
    let mut n = Noise::new(seed);
    let mut bp = Bp::new();
    let crack = render(0.4, |t| {
        bp.run(n.next(), 4500.0, 1.5) * decay(t, 0.03) * 2.0
    });
    bell(
        &[
            (2320.0, 0.6, 0.35),
            (3150.0, 0.5, 0.3),
            (4710.0, 0.4, 0.2),
            (6230.0, 0.25, 0.15),
        ],
        0.9,
    )
    .mix(&crack, 0.0, 1.0)
    .normalize(0.7)
}

pub fn chime_up(base: f32, steps: &[f32], gap: f32) -> Buf {
    let notes: Vec<(f32, f32)> = steps
        .iter()
        .enumerate()
        .map(|(i, s)| (base * s, i as f32 * gap))
        .collect();
    melody(&notes, 0.6)
}

pub fn rumble(len: f32, seed: u32) -> Buf {
    let mut n = Noise::new(seed);
    let mut lp = Lp::new();
    render(len, |t| {
        let rattle = 0.6 + 0.4 * (t * 38.0 * TAU).sin();
        lp.run(n.next(), 400.0) * rattle * env(t, 0.1, len * 0.5) * 2.0
    })
    .normalize(0.7)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(b: &Buf) {
        assert!(!b.0.is_empty());
        assert!(b.0.iter().all(|s| s.is_finite()));
        let peak = b.0.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.1 && peak <= 1.0, "peak {peak}");
    }

    #[test]
    fn sounds_are_sane() {
        ok(&gunshot(
            &ShotRecipe {
                body: 140.0,
                tail: 0.07,
                bright: 4500.0,
                echo: 0.35,
            },
            1,
        ));
        ok(&suppressed(160.0, 2));
        ok(&laser(3));
        ok(&thunder(4));
        ok(&mag_in(5));
        ok(&step(6, true));
        ok(&voice(
            &Voice {
                pitch: 80.0,
                pitch_end: 60.0,
                formants: (500.0, 900.0),
                formants_end: (600.0, 1000.0),
                rasp: 0.3,
                len: 1.0,
                attack: 0.2,
            },
            7,
        ));
        ok(&explosion(8, 1.0));
        ok(&ice(9));
        ok(&melody(&[(659.0, 0.0), (784.0, 0.2)], 0.5));
        ok(&rumble(1.0, 10));
        ok(&zap(11, 0.4));
        ok(&fire(12, 1.0));
        let w = gunshot(
            &ShotRecipe {
                body: 140.0,
                tail: 0.07,
                bright: 4500.0,
                echo: 0.35,
            },
            1,
        )
        .wav();
        if let Ok(dir) = std::env::var("FPS_WAV_DIR") {
            std::fs::write(format!("{dir}/rifle.wav"), &w).unwrap();
            std::fs::write(
                format!("{dir}/groan.wav"),
                voice(
                    &Voice {
                        pitch: 80.0,
                        pitch_end: 60.0,
                        formants: (500.0, 900.0),
                        formants_end: (600.0, 1000.0),
                        rasp: 0.3,
                        len: 1.0,
                        attack: 0.2,
                    },
                    7,
                )
                .wav(),
            )
            .unwrap();
        }
        assert_eq!(&w[0..4], b"RIFF");
    }
}

/// Rattling links: a run of small metal clinks.
pub fn chain(len: f32, seed: u32) -> Buf {
    let mut out = render(len + 0.1, |_| 0.0);
    let mut n = Noise::new(seed);
    let mut t = 0.0;
    let mut i = 0;
    while t < len {
        let f = 2400.0 + 900.0 * n.next();
        let clink = bell(&[(f, 1.0, 0.03), (f * 1.47, 0.5, 0.02)], 0.08)
            .mix(&click(f * 0.8, seed + i, 0.03), 0.0, 0.5);
        out = out.mix(&clink, t, 0.6 * (1.0 - t / len * 0.5));
        t += 0.025 + 0.02 * n.next().abs();
        i += 1;
    }
    out.normalize(0.7)
}

/// A plucked string (bowstrings).
pub fn twang(freq: f32, seed: u32) -> Buf {
    let mut o = Osc::new();
    let mut o2 = Osc::new();
    let mut n = Noise::new(seed);
    let mut lp = Lp::new();
    render(0.45, |t| {
        let f = freq * (1.0 + 0.3 * decay(t, 0.01));
        let pluck = lp.run(n.next(), 3000.0) * decay(t, 0.006);
        (o.saw(f) * 0.6 + o2.sine(f * 2.01) * 0.3) * decay(t, 0.09) + pluck
    })
    .normalize(0.7)
}

/// Steel jaws slamming shut.
pub fn snap(seed: u32) -> Buf {
    click(1800.0, seed, 0.06)
        .mix(&thud(160.0, 0.18, seed + 1), 0.0, 0.8)
        .mix(&bell(&[(1250.0, 0.5, 0.15), (1930.0, 0.35, 0.1)], 0.35), 0.01, 0.6)
        .normalize(0.85)
}

/// Glass or stone breaking into pieces.
pub fn shatter(seed: u32) -> Buf {
    let mut n = Noise::new(seed);
    let mut bp = Bp::new();
    let mut tinkle = render(0.7, |t| {
        let x = n.next();
        let spark = if n.next() > 0.995 { 3.0 } else { 0.0 };
        bp.run(x, 5200.0, 1.2) * (decay(t, 0.05) * 2.0 + spark * decay(t, 0.25))
    });
    tinkle = tinkle.mix(&thud(220.0, 0.15, seed + 1), 0.0, 0.5);
    tinkle.normalize(0.75)
}

/// A long hiss of gas or spray.
pub fn hiss(len: f32, seed: u32) -> Buf {
    let mut n = Noise::new(seed);
    let mut bp = Bp::new();
    render(len, |t| {
        let k = t / len;
        bp.run(n.next(), 4500.0 - 1500.0 * k, 0.9) * env(t, 0.05, len * 0.4)
    })
    .normalize(0.6)
}

/// A hollow ghostly moan.
pub fn wail(len: f32, seed: u32) -> Buf {
    let mut o = Osc::new();
    let mut o2 = Osc::new();
    let mut n = Noise::new(seed);
    let mut bp = Bp::new();
    render(len, |t| {
        let k = t / len;
        let f = 330.0 + 160.0 * (k * std::f32::consts::PI).sin() + 12.0 * (t * 5.0 * TAU).sin();
        let tone = o.sine(f) * 0.6 + o2.sine(f * 1.5) * 0.25;
        let air = bp.run(n.next(), f * 2.0, 3.0) * 0.5;
        (tone + air) * (k * std::f32::consts::PI).sin()
    })
    .normalize(0.6)
}
