//! Sound output. The game produces `Event`s; an `Audio` implementation turns them into sound.
//! The sounds are synthesised (`synth`: square / triangle / noise chip-tune tones), `mixer` adds
//! voices together for the native backend (`win`: a `waveOut` thread on Windows), `web` uses
//! Web Audio in the browser. Other systems stay silent (`NullAudio`).

// Only the Windows and web backends use the synthesiser and the mixer (the web one no mixer).
#[cfg_attr(not(windows), allow(dead_code))]
mod mixer;
#[cfg_attr(not(any(windows, target_arch = "wasm32")), allow(dead_code))]
mod synth;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(all(windows, not(target_arch = "wasm32")))]
mod win;

use crate::game::Event;

pub trait Audio {
    /// Plays the sound effect of `event` (nothing while the volume is 0).
    fn play(&mut self, event: Event);
    /// The master volume, 0..1 (0 is mute).
    fn set_volume(&mut self, volume: f32);
    /// Starts or stops the looping music.
    fn set_music(&mut self, on: bool);
}

/// Silent output (tests, systems without a backend, no sound device).
#[allow(dead_code)] // not used on the web
#[derive(Default)]
pub struct NullAudio;

impl Audio for NullAudio {
    fn play(&mut self, _event: Event) {}
    fn set_volume(&mut self, _volume: f32) {}
    fn set_music(&mut self, _on: bool) {}
}

/// The sound output of this platform.
pub fn platform_audio() -> Box<dyn Audio> {
    #[cfg(target_arch = "wasm32")]
    return Box::new(web::WebAudio::new());
    #[cfg(all(windows, not(target_arch = "wasm32")))]
    return match win::WinAudio::new() {
        Some(a) => Box::new(a),
        None => Box::new(NullAudio),
    };
    #[cfg(not(any(windows, target_arch = "wasm32")))]
    Box::new(NullAudio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_audio_accepts_everything() {
        let mut a = NullAudio;
        a.play(Event::Jump);
        a.set_volume(0.5);
        a.set_music(true);
    }
}
