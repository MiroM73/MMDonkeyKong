//! Chip-tune synthesis: square / triangle / noise tones rendered into sample buffers.
//! Pure functions, no I/O - the backends only play the buffers.

use crate::game::Event;

/// Sample rate of every buffer (mono, `f32` in -1..1).
pub const RATE: u32 = 22_050;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wave {
    Square,
    Triangle,
    /// Pseudo-random samples, held for one period of the frequency.
    Noise,
}

/// One sound segment: the frequency slides linearly from `from` to `to` Hz while the volume
/// decays; `vol` 0 is a rest.
#[derive(Clone, Copy, Debug)]
pub struct Tone {
    pub wave: Wave,
    pub from: f32,
    pub to: f32,
    pub ms: u32,
    pub vol: f32,
}

const fn tone(wave: Wave, from: f32, to: f32, ms: u32, vol: f32) -> Tone {
    Tone {
        wave,
        from,
        to,
        ms,
        vol,
    }
}

const fn sq(freq: f32, ms: u32) -> Tone {
    tone(Wave::Square, freq, freq, ms, 0.25)
}

/// Renders the tones one after another.
pub fn render(tones: &[Tone]) -> Vec<f32> {
    let mut out = Vec::new();
    let mut noise = 0x1234_5678_u32;
    let mut held = 0.0;
    for t in tones {
        let n = (t.ms as usize * RATE as usize / 1000).max(1);
        let mut phase = 0.0_f32;
        let mut noise_phase = 1.0_f32;
        for i in 0..n {
            let x = i as f32 / n as f32;
            let freq = t.from + (t.to - t.from) * x;
            phase = (phase + freq / RATE as f32).fract();
            let raw = match t.wave {
                Wave::Square => {
                    if phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
                Wave::Noise => {
                    noise_phase += freq / RATE as f32;
                    if noise_phase >= 1.0 {
                        noise_phase -= 1.0;
                        noise ^= noise << 13;
                        noise ^= noise >> 17;
                        noise ^= noise << 5;
                        held = (noise % 2001) as f32 / 1000.0 - 1.0;
                    }
                    held
                }
            };
            // A 2 ms attack avoids a click, then a decay to silence.
            let attack = (i as f32 / (RATE as f32 * 0.002)).min(1.0);
            let env = attack * (1.0 - x).powf(0.7);
            out.push(raw * t.vol * env);
        }
    }
    out
}

/// The tones of the sound effect of `event`.
fn effect_tones(event: Event) -> Vec<Tone> {
    use Wave::*;
    match event {
        Event::Jump => vec![tone(Square, 260.0, 620.0, 110, 0.25)],
        Event::Land => vec![tone(Noise, 2500.0, 900.0, 45, 0.3)],
        Event::Died => vec![
            tone(Square, 600.0, 120.0, 520, 0.28),
            tone(Noise, 700.0, 200.0, 160, 0.25),
        ],
        Event::TimeUp => vec![sq(880.0, 90), sq(660.0, 90), sq(880.0, 90), sq(660.0, 140)],
        Event::Respawn => vec![sq(392.0, 70), sq(523.0, 70), sq(659.0, 110)],
        Event::HammerPickup => vec![sq(523.0, 55), sq(659.0, 55), sq(784.0, 55), sq(1047.0, 120)],
        Event::Smash => vec![
            tone(Noise, 3500.0, 600.0, 120, 0.4),
            tone(Square, 220.0, 70.0, 140, 0.3),
        ],
        Event::JumpedBarrel => vec![sq(880.0, 45), sq(1319.0, 90)],
        Event::ExtraLife => vec![
            sq(523.0, 80),
            sq(659.0, 80),
            sq(784.0, 80),
            sq(1047.0, 80),
            sq(784.0, 80),
            sq(1047.0, 220),
        ],
        Event::Rivet => vec![tone(Triangle, 900.0, 500.0, 90, 0.5)],
        Event::LevelComplete => vec![
            sq(523.0, 110),
            sq(659.0, 110),
            sq(784.0, 110),
            tone(Square, 0.0, 0.0, 40, 0.0),
            sq(784.0, 110),
            sq(1047.0, 350),
        ],
        Event::GameOver => vec![
            sq(392.0, 220),
            sq(330.0, 220),
            sq(262.0, 220),
            tone(Square, 220.0, 90.0, 600, 0.25),
        ],
    }
}

/// The sound effect of `event`.
pub fn effect(event: Event) -> Vec<f32> {
    render(&effect_tones(event))
}

/// Notes (eighths) in one music loop: 4 bars of 8 eighth notes.
const LOOP_EIGHTHS: usize = 32;

/// Seven-note scales as semitone offsets from the root (all minor-ish, one family).
const SCALES: [[i32; 7]; 4] = [
    [0, 2, 3, 5, 7, 8, 10], // natural minor
    [0, 2, 3, 5, 7, 9, 10], // dorian
    [0, 1, 3, 5, 7, 8, 10], // phrygian
    [0, 2, 3, 5, 7, 8, 11], // harmonic minor
];

/// Chord roots (scale degrees) of the four bars; the first is always the tonic.
const PROGRESSIONS: [[i32; 4]; 4] = [
    [0, 5, 2, 6], // i - VI - III - VII
    [0, 3, 4, 0], // i - iv - v - i
    [0, 6, 5, 6], // i - VII - VI - VII
    [0, 2, 5, 4], // i - III - VI - v
];

/// Bass patterns: four quarter notes per bar as (scale-degree offset from the chord root, octave).
const BASS_PATTERNS: [[(i32, i32); 4]; 3] = [
    [(0, 0), (0, 1), (0, 0), (0, 1)],
    [(0, 0), (0, 0), (4, 0), (0, 0)],
    [(0, 0), (4, 0), (0, 1), (4, 0)],
];

/// Everything that makes one level's tune: derived from the level number only.
#[derive(Clone, Debug, PartialEq)]
struct MusicSpec {
    /// Semitones from A4 of the tonic (lead octave).
    root: i32,
    scale: [i32; 7],
    progression: [i32; 4],
    bass: usize,
    /// Length of one eighth note in milliseconds.
    eighth_ms: u32,
    lead_wave: Wave,
    /// Seeds the melody.
    seed: u32,
}

/// Small xorshift generator (deterministic on every platform).
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        let mut r = Self(seed.wrapping_mul(0x9E37_79B9) ^ 0x5bd1_e995);
        if r.0 == 0 {
            r.0 = 1;
        }
        for _ in 0..4 {
            r.next();
        }
        r
    }

    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
}

/// The spec of the tune of the level with index `level` (0-based).
fn music_spec(level: usize) -> MusicSpec {
    let mut r = Rng::new(level as u32 + 1);
    MusicSpec {
        root: r.below(8) as i32 - 5,
        scale: SCALES[r.below(SCALES.len() as u32) as usize],
        progression: PROGRESSIONS[r.below(PROGRESSIONS.len() as u32) as usize],
        bass: r.below(BASS_PATTERNS.len() as u32) as usize,
        eighth_ms: 110 + 10 * r.below(5),
        lead_wave: if r.below(3) == 0 {
            Wave::Triangle
        } else {
            Wave::Square
        },
        seed: r.next(),
    }
}

/// Semitones of scale degree `degree` (any integer; every 7 degrees are an octave) from the root.
fn degree_semitones(scale: &[i32; 7], degree: i32) -> i32 {
    scale[degree.rem_euclid(7) as usize] + 12 * degree.div_euclid(7)
}

fn hz(semitones: i32) -> f32 {
    440.0 * 2f32.powf(semitones as f32 / 12.0)
}

/// The lead melody as scale degrees (one per eighth): chord tones on the strong beats, a random
/// walk between them; it starts on the tonic and ends a step beside it, so the loop resolves.
fn lead_degrees(spec: &MusicSpec) -> [i32; LOOP_EIGHTHS] {
    let mut r = Rng::new(spec.seed);
    let mut out = [0; LOOP_EIGHTHS];
    let mut prev = 0;
    for (i, slot) in out.iter_mut().enumerate() {
        let chord = spec.progression[i / 8];
        let beat = i % 8;
        let d = if i == 0 {
            0
        } else if i == LOOP_EIGHTHS - 1 {
            if r.below(2) == 0 { 1 } else { -1 }
        } else if beat % 4 == 0 {
            chord + 2 * r.below(3) as i32 + 7 * (r.below(2) as i32)
        } else {
            (prev + r.below(5) as i32 - 2).clamp(-2, 9)
        };
        *slot = d;
        prev = d;
    }
    out
}

/// Pushes a note of `ms` milliseconds: 85 % sounding, the rest a gap (it separates the notes and
/// brings the signal to zero).
fn note(tones: &mut Vec<Tone>, wave: Wave, semitones: i32, ms: u32, vol: f32) {
    let f = hz(semitones);
    let on = ms * 85 / 100;
    tones.push(tone(wave, f, f, on, vol));
    tones.push(tone(wave, f, f, ms - on, 0.0));
}

/// The looping background music of the level with index `level` (0-based): 4 bars, lead + bass,
/// seamless when repeated. The same level always gives the same buffer.
pub fn music(level: usize) -> Vec<f32> {
    let spec = music_spec(level);
    let mut lead = Vec::new();
    for d in lead_degrees(&spec) {
        note(
            &mut lead,
            spec.lead_wave,
            spec.root + degree_semitones(&spec.scale, d),
            spec.eighth_ms,
            0.13,
        );
    }
    let mut bass = Vec::new();
    for chord in spec.progression {
        for (offset, octave) in BASS_PATTERNS[spec.bass] {
            let semis = degree_semitones(&spec.scale, chord + offset) + 12 * octave;
            note(
                &mut bass,
                Wave::Triangle,
                spec.root - 24 + semis,
                spec.eighth_ms * 2,
                0.3,
            );
        }
    }
    let (lead, bass) = (render(&lead), render(&bass));
    let len = lead.len().max(bass.len());
    (0..len)
        .map(|i| lead.get(i).copied().unwrap_or(0.0) + bass.get(i).copied().unwrap_or(0.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Event; 12] = [
        Event::Jump,
        Event::Land,
        Event::Died,
        Event::TimeUp,
        Event::Respawn,
        Event::HammerPickup,
        Event::Smash,
        Event::JumpedBarrel,
        Event::ExtraLife,
        Event::Rivet,
        Event::LevelComplete,
        Event::GameOver,
    ];

    #[test]
    fn tone_has_the_requested_length() {
        let s = render(&[sq(440.0, 100)]);
        assert_eq!(s.len(), RATE as usize / 10);
    }

    #[test]
    fn every_effect_is_audible_and_within_range() {
        for e in ALL {
            let s = effect(e);
            assert!(!s.is_empty(), "{e:?}");
            let peak = s.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.05, "{e:?} is silent");
            assert!(peak <= 1.0, "{e:?} clips: {peak}");
            assert!(s.iter().all(|v| v.is_finite()), "{e:?}");
        }
    }

    #[test]
    fn a_rest_is_silent() {
        let s = render(&[tone(Wave::Square, 440.0, 440.0, 50, 0.0)]);
        assert!(s.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn rendering_is_deterministic() {
        assert_eq!(effect(Event::Smash), effect(Event::Smash));
    }

    const LEVELS: usize = 12;

    #[test]
    fn every_level_loop_is_in_range_seamless_and_the_expected_length() {
        for level in 0..LEVELS {
            let m = music(level);
            let expect = LOOP_EIGHTHS * music_spec(level).eighth_ms as usize * RATE as usize / 1000;
            assert!(
                m.len().abs_diff(expect) < 128,
                "{level}: {} vs {expect}",
                m.len()
            );
            let peak = m.iter().fold(0.0_f32, |p, v| p.max(v.abs()));
            assert!(peak > 0.1 && peak <= 1.0, "{level}: {peak}");
            assert!(m.iter().all(|v| v.is_finite()));
            assert!(m[0].abs() < 0.05 && m[m.len() - 1].abs() < 0.05, "{level}");
        }
    }

    #[test]
    fn music_is_deterministic_and_differs_per_level() {
        assert_eq!(music(3), music(3));
        for a in 0..LEVELS {
            for b in a + 1..LEVELS {
                assert_ne!(music(a), music(b), "levels {a} and {b} sound the same");
            }
        }
    }

    #[test]
    fn the_melody_starts_on_the_tonic_and_ends_beside_it() {
        for level in 0..LEVELS {
            let l = lead_degrees(&music_spec(level));
            assert_eq!(l[0], 0);
            assert_eq!(l[LOOP_EIGHTHS - 1].abs(), 1);
        }
    }
}
