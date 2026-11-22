//! Windows backend: a mixer thread feeding a `waveOut` device (winmm, no extra crate).
//! Commands go to the thread through a channel; dropping the `WinAudio` ends the thread.

use std::ffi::c_void;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::Duration;

use super::Audio;
use super::mixer::{Library, Mixer};
use super::synth::RATE;
use crate::game::Event;

const BUFFERS: usize = 4;
/// Samples per buffer (about 23 ms).
const BLOCK: usize = 512;

#[repr(C)]
struct WaveFormatEx {
    format_tag: u16,
    channels: u16,
    samples_per_sec: u32,
    avg_bytes_per_sec: u32,
    block_align: u16,
    bits_per_sample: u16,
    cb_size: u16,
}

#[repr(C)]
struct WaveHdr {
    data: *mut u8,
    buffer_length: u32,
    bytes_recorded: u32,
    user: usize,
    flags: u32,
    loops: u32,
    next: *mut WaveHdr,
    reserved: usize,
}

type HWaveOut = *mut c_void;

#[link(name = "winmm")]
unsafe extern "system" {
    fn waveOutOpen(
        out: *mut HWaveOut,
        device: u32,
        format: *const WaveFormatEx,
        callback: usize,
        instance: usize,
        flags: u32,
    ) -> u32;
    fn waveOutPrepareHeader(hwo: HWaveOut, hdr: *mut WaveHdr, size: u32) -> u32;
    fn waveOutUnprepareHeader(hwo: HWaveOut, hdr: *mut WaveHdr, size: u32) -> u32;
    fn waveOutWrite(hwo: HWaveOut, hdr: *mut WaveHdr, size: u32) -> u32;
    fn waveOutReset(hwo: HWaveOut) -> u32;
    fn waveOutClose(hwo: HWaveOut) -> u32;
}

const WAVE_MAPPER: u32 = 0xFFFF_FFFF;
const WAVE_FORMAT_PCM: u16 = 1;
const WHDR_DONE: u32 = 1;

enum Cmd {
    Play(Arc<[f32]>),
    Volume(f32),
    Music(Option<Arc<[f32]>>),
}

pub struct WinAudio {
    tx: Sender<Cmd>,
    lib: Library,
    music_on: bool,
}

impl WinAudio {
    /// Starts the mixer thread; `None` when the thread cannot be created.
    pub fn new() -> Option<Self> {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("audio".into())
            .spawn(move || run(rx))
            .ok()?;
        Some(Self {
            tx,
            lib: Library::new(),
            music_on: false,
        })
    }
}

impl Audio for WinAudio {
    fn play(&mut self, event: Event) {
        if let Some(data) = self.lib.effect(event) {
            let _ = self.tx.send(Cmd::Play(data));
        }
    }

    fn set_volume(&mut self, volume: f32) {
        let _ = self.tx.send(Cmd::Volume(volume));
    }

    fn set_music(&mut self, on: bool) {
        if on != self.music_on {
            self.music_on = on;
            let _ = self.tx.send(Cmd::Music(on.then(|| self.lib.music())));
        }
    }
}

/// One output buffer: its samples and the header `waveOut` knows it by.
struct Slot {
    pcm: Box<[i16; BLOCK]>,
    hdr: Box<WaveHdr>,
    queued: bool,
}

fn run(rx: Receiver<Cmd>) {
    let format = WaveFormatEx {
        format_tag: WAVE_FORMAT_PCM,
        channels: 1,
        samples_per_sec: RATE,
        avg_bytes_per_sec: RATE * 2,
        block_align: 2,
        bits_per_sample: 16,
        cb_size: 0,
    };
    let mut hwo: HWaveOut = std::ptr::null_mut();
    // SAFETY: `hwo` and `format` are valid for the call; CALLBACK_NULL (0) needs no callback.
    if unsafe { waveOutOpen(&mut hwo, WAVE_MAPPER, &format, 0, 0, 0) } != 0 {
        return; // no sound device: stay silent
    }
    let hdr_size = std::mem::size_of::<WaveHdr>() as u32;
    let mut slots: Vec<Slot> = (0..BUFFERS)
        .map(|_| {
            let mut pcm = Box::new([0i16; BLOCK]);
            let hdr = Box::new(WaveHdr {
                data: pcm.as_mut_ptr().cast(),
                buffer_length: (BLOCK * 2) as u32,
                bytes_recorded: 0,
                user: 0,
                flags: 0,
                loops: 0,
                next: std::ptr::null_mut(),
                reserved: 0,
            });
            Slot {
                pcm,
                hdr,
                queued: false,
            }
        })
        .collect();
    for s in &mut slots {
        // SAFETY: the header points to the boxed buffer, both stay alive and unmoved (boxed)
        // until they are unprepared below.
        unsafe { waveOutPrepareHeader(hwo, &mut *s.hdr, hdr_size) };
    }

    let mut mixer = Mixer::default();
    let mut volume = 0.0_f32;
    let mut block = [0.0_f32; BLOCK];
    'main: loop {
        // Commands: block while nothing plays and every buffer is played out.
        let all_done = slots
            .iter()
            .all(|s| !s.queued || s.hdr.flags & WHDR_DONE != 0);
        let cmd = if mixer.is_idle() && all_done {
            match rx.recv() {
                Ok(c) => Some(c),
                Err(_) => break 'main,
            }
        } else {
            match rx.try_recv() {
                Ok(c) => Some(c),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => break 'main,
            }
        };
        let mut next = cmd;
        while let Some(c) = next {
            match c {
                Cmd::Play(d) => mixer.play(d),
                Cmd::Volume(v) => volume = v.clamp(0.0, 1.0),
                Cmd::Music(m) => mixer.set_music(m),
            }
            next = rx.try_recv().ok();
        }
        // Refill every buffer that finished playing.
        let mut wrote = false;
        for s in &mut slots {
            if s.queued && s.hdr.flags & WHDR_DONE == 0 {
                continue;
            }
            if mixer.is_idle() && s.queued {
                s.queued = false; // nothing to play: let the device run dry
                continue;
            }
            if mixer.is_idle() {
                continue;
            }
            mixer.fill(&mut block, volume);
            for (o, v) in s.pcm.iter_mut().zip(block) {
                *o = (v * 32767.0) as i16;
            }
            // SAFETY: the header was prepared above and its previous play is done.
            unsafe { waveOutWrite(hwo, &mut *s.hdr, hdr_size) };
            s.queued = true;
            wrote = true;
        }
        if !wrote {
            std::thread::sleep(Duration::from_millis(4));
        } else {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    // SAFETY: reset returns every buffer, then the headers are unprepared before the close.
    unsafe {
        waveOutReset(hwo);
        for s in &mut slots {
            waveOutUnprepareHeader(hwo, &mut *s.hdr, hdr_size);
        }
        waveOutClose(hwo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plays real sound on the default device (run by hand: `cargo test -- --ignored win`).
    #[test]
    #[ignore = "plays sound on the sound device"]
    fn plays_on_the_default_device_and_shuts_down() {
        let mut a = WinAudio::new().unwrap();
        a.set_volume(0.2);
        a.play(Event::LevelComplete);
        a.set_music(true);
        std::thread::sleep(Duration::from_millis(1200));
        a.set_music(false);
        a.play(Event::Smash);
        std::thread::sleep(Duration::from_millis(400));
        drop(a);
        std::thread::sleep(Duration::from_millis(100));
    }
}
