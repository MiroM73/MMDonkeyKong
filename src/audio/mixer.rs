//! The sound library (every buffer rendered once) and the `Mixer` that adds playing voices
//! together. Used by the native backend; the web backend lets Web Audio do the mixing.

use std::sync::Arc;

use super::synth;
use crate::game::Event;

/// Every sound effect and the music, rendered once; the buffers are shared (`Arc`).
pub struct Library {
    effects: Vec<(Event, Arc<[f32]>)>,
    music: Arc<[f32]>,
}

impl Library {
    pub fn new() -> Self {
        let events = [
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
        Self {
            effects: events
                .into_iter()
                .map(|e| (e, Arc::from(synth::effect(e))))
                .collect(),
            music: Arc::from(synth::music()),
        }
    }

    pub fn effect(&self, event: Event) -> Option<Arc<[f32]>> {
        self.effects
            .iter()
            .find(|(e, _)| *e == event)
            .map(|(_, s)| s.clone())
    }

    pub fn music(&self) -> Arc<[f32]> {
        self.music.clone()
    }
}

/// At most this many sound effects play at once; a new one drops the oldest.
const MAX_VOICES: usize = 8;

struct Voice {
    data: Arc<[f32]>,
    pos: usize,
    looping: bool,
}

/// Adds the playing voices into output blocks.
#[derive(Default)]
pub struct Mixer {
    voices: Vec<Voice>,
}

impl Mixer {
    pub fn play(&mut self, data: Arc<[f32]>) {
        let effects = self.voices.iter().filter(|v| !v.looping).count();
        if effects >= MAX_VOICES
            && let Some(i) = self.voices.iter().position(|v| !v.looping)
        {
            self.voices.remove(i);
        }
        self.voices.push(Voice {
            data,
            pos: 0,
            looping: false,
        });
    }

    /// Starts (`Some`) or stops (`None`) the looping music.
    pub fn set_music(&mut self, music: Option<Arc<[f32]>>) {
        self.voices.retain(|v| !v.looping);
        if let Some(data) = music {
            self.voices.push(Voice {
                data,
                pos: 0,
                looping: true,
            });
        }
    }

    pub fn is_idle(&self) -> bool {
        self.voices.is_empty()
    }

    /// Overwrites `out` with the sum of the voices times `volume`, clipped to -1..1.
    pub fn fill(&mut self, out: &mut [f32], volume: f32) {
        out.fill(0.0);
        for v in &mut self.voices {
            if v.data.is_empty() {
                v.pos = usize::MAX;
                continue;
            }
            for o in out.iter_mut() {
                if v.pos >= v.data.len() {
                    if !v.looping {
                        break;
                    }
                    v.pos = 0;
                }
                *o += v.data[v.pos];
                v.pos += 1;
            }
        }
        self.voices.retain(|v| v.looping || v.pos < v.data.len());
        for o in out.iter_mut() {
            *o = (*o * volume).clamp(-1.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(v: &[f32]) -> Arc<[f32]> {
        Arc::from(v.to_vec())
    }

    #[test]
    fn a_voice_plays_once_and_ends() {
        let mut m = Mixer::default();
        m.play(buf(&[0.5, 0.25, 0.125]));
        let mut out = [9.0; 2];
        m.fill(&mut out, 1.0);
        assert_eq!(out, [0.5, 0.25]);
        assert!(!m.is_idle());
        m.fill(&mut out, 1.0);
        assert_eq!(out, [0.125, 0.0]);
        assert!(m.is_idle());
    }

    #[test]
    fn voices_add_and_the_volume_scales() {
        let mut m = Mixer::default();
        m.play(buf(&[0.2, 0.2]));
        m.play(buf(&[0.4, 0.4]));
        let mut out = [0.0; 2];
        m.fill(&mut out, 0.5);
        assert!((out[0] - 0.3).abs() < 1e-6);
    }

    #[test]
    fn the_sum_is_clipped() {
        let mut m = Mixer::default();
        m.play(buf(&[0.9]));
        m.play(buf(&[0.9]));
        let mut out = [0.0; 1];
        m.fill(&mut out, 1.0);
        assert_eq!(out[0], 1.0);
    }

    #[test]
    fn music_loops_until_stopped() {
        let mut m = Mixer::default();
        m.set_music(Some(buf(&[0.1, 0.2])));
        let mut out = [0.0; 5];
        m.fill(&mut out, 1.0);
        assert_eq!(out, [0.1, 0.2, 0.1, 0.2, 0.1]);
        assert!(!m.is_idle());
        m.set_music(None);
        assert!(m.is_idle());
    }

    #[test]
    fn too_many_effects_drop_the_oldest() {
        let mut m = Mixer::default();
        for _ in 0..MAX_VOICES + 3 {
            m.play(buf(&[0.1; 4]));
        }
        assert_eq!(m.voices.len(), MAX_VOICES);
    }

    #[test]
    fn the_library_has_every_effect() {
        let lib = Library::new();
        assert!(lib.effect(Event::Jump).is_some());
        assert!(lib.effect(Event::GameOver).is_some());
        assert!(!lib.music().is_empty());
    }
}
