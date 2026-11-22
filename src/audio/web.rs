//! Browser backend: Web Audio. The `AudioContext` is created on the first sound (the browser only
//! lets it run after a user gesture, so it is also resumed on every play).

use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext, GainNode};

use super::Audio;
use super::mixer::Library;
use super::synth::RATE;
use crate::game::Event;

struct Engine {
    ctx: AudioContext,
    gain: GainNode,
}

pub struct WebAudio {
    engine: Option<Engine>,
    lib: Library,
    volume: f32,
    music_on: bool,
    music: Option<AudioBufferSourceNode>,
}

impl WebAudio {
    pub fn new() -> Self {
        Self {
            engine: None,
            lib: Library::new(),
            volume: 0.0,
            music_on: false,
            music: None,
        }
    }

    fn engine(&mut self) -> Option<&Engine> {
        if self.engine.is_none() {
            let ctx = AudioContext::new().ok()?;
            let gain = ctx.create_gain().ok()?;
            gain.gain().set_value(self.volume);
            gain.connect_with_audio_node(&ctx.destination()).ok()?;
            self.engine = Some(Engine { ctx, gain });
        }
        let e = self.engine.as_ref()?;
        let _ = e.ctx.resume();
        Some(e)
    }

    fn buffer(ctx: &AudioContext, data: &[f32]) -> Option<AudioBuffer> {
        let buffer = ctx
            .create_buffer(1, data.len().max(1) as u32, RATE as f32)
            .ok()?;
        buffer.copy_to_channel(data, 0).ok()?;
        Some(buffer)
    }

    fn start(&mut self, data: &[f32], looping: bool) -> Option<AudioBufferSourceNode> {
        let e = self.engine()?;
        let source = e.ctx.create_buffer_source().ok()?;
        source.set_buffer(Some(&Self::buffer(&e.ctx, data)?));
        source.set_loop(looping);
        source.connect_with_audio_node(&e.gain).ok()?;
        source.start().ok()?;
        Some(source)
    }
}

impl Audio for WebAudio {
    fn play(&mut self, event: Event) {
        if self.volume <= 0.0 {
            return;
        }
        if let Some(data) = self.lib.effect(event) {
            self.start(&data, false);
        }
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(e) = &self.engine {
            e.gain.gain().set_value(self.volume);
        }
    }

    fn set_music(&mut self, on: bool) {
        if on == self.music_on {
            return;
        }
        if on {
            let music = self.lib.music();
            self.music = self.start(&music, true);
            self.music_on = self.music.is_some();
        } else {
            self.music_on = false;
            if let Some(m) = self.music.take() {
                // web-sys marks both `stop` variants deprecated (the spec still has them).
                #[allow(deprecated)]
                let _ = m.stop();
            }
        }
    }
}
