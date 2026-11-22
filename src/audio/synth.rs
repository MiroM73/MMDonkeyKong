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

/// Length of one eighth note of the music in milliseconds.
const EIGHTH_MS: u32 = 140;

/// The lead melody: semitones from A4 (`None` is a rest), one entry per eighth note.
const LEAD: [Option<i32>; 32] = [
    Some(0),
    Some(3),
    Some(7),
    Some(3),
    Some(12),
    Some(7),
    Some(3),
    Some(7),
    Some(5),
    Some(8),
    Some(12),
    Some(8),
    Some(14),
    Some(12),
    Some(8),
    Some(12),
    Some(7),
    Some(11),
    Some(14),
    Some(11),
    Some(19),
    Some(14),
    Some(11),
    Some(14),
    Some(12),
    Some(10),
    Some(7),
    Some(3),
    Some(0),
    Some(3),
    Some(7),
    None,
];

/// The bass: one note per quarter note, semitones from A4.
const BASS: [i32; 16] = [
    -24, -12, -24, -12, -19, -7, -19, -7, -17, -5, -17, -5, -24, -12, -24, -12,
];

fn hz(semitones: i32) -> f32 {
    440.0 * 2f32.powf(semitones as f32 / 12.0)
}

/// Eight seconds-ish music loop (4 bars in A minor, lead + bass), seamless when repeated.
pub fn music() -> Vec<f32> {
    let mut lead = Vec::new();
    for note in LEAD {
        match note {
            Some(n) => {
                let f = hz(n);
                lead.push(tone(Wave::Square, f, f, EIGHTH_MS * 85 / 100, 0.13));
                lead.push(tone(Wave::Square, f, f, EIGHTH_MS * 15 / 100, 0.0));
            }
            None => lead.push(tone(Wave::Square, 0.0, 0.0, EIGHTH_MS, 0.0)),
        }
    }
    let mut bass = Vec::new();
    for n in BASS {
        let f = hz(n);
        bass.push(tone(Wave::Triangle, f, f, EIGHTH_MS * 2 * 85 / 100, 0.3));
        bass.push(tone(Wave::Triangle, f, f, EIGHTH_MS * 2 * 15 / 100, 0.0));
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

    const ALL: [Event; 11] = [
        Event::Jump,
        Event::Land,
        Event::Died,
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

    #[test]
    fn the_music_loop_is_in_range_and_the_expected_length() {
        let m = music();
        let expect = 32 * EIGHTH_MS as usize * RATE as usize / 1000;
        assert!(m.len().abs_diff(expect) < 64, "{} vs {expect}", m.len());
        let peak = m.iter().fold(0.0_f32, |p, v| p.max(v.abs()));
        assert!(peak > 0.1 && peak <= 1.0, "{peak}");
    }
}
