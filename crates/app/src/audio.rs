//! Output stream. Receiver audio comes through a ring from the simulation, the keyer runs
//! right here on the sample clock, so a paddle is heard after one audio buffer.

use std::f32::consts::{PI, TAU};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering::Relaxed};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use mr_sim::keyer::{self, Keyer};
use mr_sim::resample::Resampler;
use mr_sim::{BLOCK, RATE};
use rtrb::{Consumer, Producer, RingBuffer};

const BUFFER_FRAMES: u32 = 128;
/// Room for this many samples of the simulation: the delay of a message from the keyboard.
const RING: usize = 3 * BLOCK;
const SIDETONE_LEVEL: f32 = 0.5;
const EDGE_S: f32 = 0.005;
const BEEP_S: f32 = 0.12;
const BEEP_HZ: f32 = 880.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioCfg {
    pub paddle: bool,
    pub keyer: keyer::Config,
    pub pitch: u32,
    pub mon_db: f32,
    pub qsk: bool,
}

pub struct Shared {
    pub paddles: AtomicU8,
    /// Samples played so far.
    pub clock: AtomicU64,
    pub underruns: AtomicU32,
    pub beep: AtomicBool,
    pub quit: AtomicBool,
}

impl Shared {
    pub fn new() -> Self {
        Shared {
            paddles: AtomicU8::new(0),
            clock: AtomicU64::new(0),
            underruns: AtomicU32::new(0),
            beep: AtomicBool::new(false),
            quit: AtomicBool::new(false),
        }
    }

    pub fn set_paddle(&self, bit: u8, down: bool) {
        if down {
            self.paddles.fetch_or(bit, Relaxed);
        } else {
            self.paddles.fetch_and(!bit, Relaxed);
        }
    }

    pub fn release_all(&self) {
        self.paddles.store(0, Relaxed);
    }
}

pub struct Audio {
    pub stream: cpal::Stream,
    pub rate: u32,
    pub samples: Producer<f32>,
    pub config: Producer<AudioCfg>,
    pub keys: Consumer<(u64, bool)>,
}

pub fn start(shared: Arc<Shared>, first: AudioCfg) -> Result<Audio, String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no audio output device")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let rate = supported.sample_rate();
    let channels = supported.channels() as usize;
    let buffer_size = match supported.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => cpal::BufferSize::Fixed(BUFFER_FRAMES.clamp(*min, *max)),
        _ => cpal::BufferSize::Default,
    };
    let config = cpal::StreamConfig { channels: channels as u16, sample_rate: rate, buffer_size };

    let (samples, mut sample_rx) = RingBuffer::<f32>::new(RING);
    let (config_tx, mut config_rx) = RingBuffer::<AudioCfg>::new(64);
    let (mut key_tx, keys) = RingBuffer::<(u64, bool)>::new(4096);

    let mut cfg = first;
    let mut keyer = Keyer::new(first.keyer, rate);
    let mut resampler = Resampler::new(RATE, rate);
    let fs = rate as f32;
    let edge = 1.0 / (EDGE_S * fs);
    // the receiver comes back the way the original does it per sample at its own rate
    let recover = 1.0 - 0.997f32.powf(RATE as f32 / fs);
    let (mut t, mut key, mut env, mut phase, mut rx_gain) = (0u64, false, 0f32, 0f32, 1f32);
    let (mut beep_left, mut beep_phase) = (0u32, 0f32);
    let beep_len = (BEEP_S * fs) as u32;

    let s = shared.clone();
    let on_error = {
        let s = shared.clone();
        move |e: cpal::Error| {
            if e.to_string().contains("underrun") {
                s.underruns.fetch_add(1, Relaxed);
            } else {
                eprintln!("audio: {e}");
            }
        }
    };
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                while let Ok(c) = config_rx.pop() {
                    if c.keyer != cfg.keyer {
                        keyer.configure(c.keyer);
                    }
                    cfg = c;
                }
                if s.beep.swap(false, Relaxed) {
                    beep_left = beep_len;
                }
                let pad = if cfg.paddle { s.paddles.load(Relaxed) } else { 0 };
                let side_level = (SIDETONE_LEVEL * 10f32.powf(cfg.mon_db / 20.0)).min(0.9);
                let step = TAU * cfg.pitch as f32 / fs;
                let mut short = false;
                for frame in data.chunks_mut(channels) {
                    let down = keyer.tick(pad);
                    if down != key {
                        key = down;
                        let _ = key_tx.push((t, down));
                    }
                    t += 1;
                    env = (env + if key { edge } else { -edge }).clamp(0.0, 1.0);
                    phase = (phase + step) % TAU;
                    let shaped = 0.5 - 0.5 * (PI * env).cos();
                    let side = side_level * shaped * phase.sin();
                    rx_gain = if 1.0 - shaped < rx_gain { 1.0 - shaped } else { rx_gain + recover * (1.0 - rx_gain) };
                    let rx_part = if cfg.qsk { rx_gain } else { 1.0 };
                    let rx = resampler.next(|| {
                        sample_rx.pop().unwrap_or_else(|_| {
                            short = true;
                            0.0
                        })
                    });
                    let mut v = rx * rx_part + side;
                    if beep_left > 0 {
                        beep_left -= 1;
                        beep_phase = (beep_phase + TAU * BEEP_HZ / fs) % TAU;
                        v += 0.25 * beep_phase.sin();
                    }
                    frame.fill(v);
                }
                if short {
                    s.underruns.fetch_add(1, Relaxed);
                }
                s.clock.store(t, Relaxed);
            },
            on_error,
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(Audio { stream, rate, samples, config: config_tx, keys })
}
