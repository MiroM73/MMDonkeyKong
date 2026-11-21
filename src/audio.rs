//! Sound output. The game produces `Event`s; an `Audio` implementation turns them into sound:
//! native and Web Audio backends with synthesised chip-tune sounds follow in issue #16.

use crate::game::Event;

pub trait Audio {
    fn play(&mut self, event: Event);
}

/// Silent output (tests, muted game).
#[derive(Default)]
pub struct NullAudio;

impl Audio for NullAudio {
    fn play(&mut self, _event: Event) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_audio_accepts_events() {
        NullAudio.play(Event::Jump);
    }
}
