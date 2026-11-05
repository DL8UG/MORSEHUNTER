//! Keeps the ring to the sound card full. The contest sits behind a mutex that is held for
//! one block at a time, so the window can call it directly, as in a program with one thread.

use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use mr_sim::BLOCK;
use mr_sim::contest::{Contest, RunMode};
use mr_sim::settings::Settings;

use crate::audio::{Audio, AudioCfg, Shared};

pub fn audio_cfg(s: &Settings) -> AudioCfg {
    let mut keyer = s.keyer;
    keyer.wpm = s.wpm;
    AudioCfg { paddle: s.paddle, keyer, pitch: s.pitch, mon_db: s.mon_db, qsk: s.qsk }
}

type Wav = hound::WavWriter<std::io::BufWriter<std::fs::File>>;

fn open_wav(path: &PathBuf) -> Option<Wav> {
    let spec = hound::WavSpec { channels: 1, sample_rate: mr_sim::RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    hound::WavWriter::create(path, spec).map_err(|e| eprintln!("recording: {e}")).ok()
}

/// One line per station and log entry, for test drivers that can't listen.
fn trace_state(c: &Contest) -> String {
    let mut out = format!("{:?} {} {}", c.mode, c.me.st.nr, c.seconds() as u64);
    for s in &c.stations {
        if let Some(d) = s.dx() {
            out += &format!("\nDX {} {:?} {:?} {} {}", s.st.my_call, d.op.state, s.st.state, s.st.nr, s.st.rst);
        }
    }
    for q in &c.log.qsos {
        out += &format!("\nLOG {} {:?}", q.call, q.err);
    }
    let e = &c.entry;
    out += &format!(
        "\nME {:?} {:?} {}\nENTRY {:?} {} {} {} {} {}\nOVER {}\nEND\n",
        c.me.st.state,
        c.me.st.msg,
        c.me.st.his_call,
        e.focus,
        e.call.text,
        e.rst.text,
        e.nr.text,
        e.call_sent,
        e.nr_sent,
        c.paddle.last_over
    );
    out
}

pub fn spawn(
    contest: Arc<Mutex<Contest>>,
    shared: Arc<Shared>,
    audio: Audio,
    wav_path: Option<PathBuf>,
    trace: Option<PathBuf>,
    keylog: Option<PathBuf>,
) -> cpal::Stream {
    let Audio { stream, samples: mut ring, config: mut to_audio, keys: mut key_rx, rate } = audio;
    // every key edge with its time, and what was read from each over: to see how a hand keys
    let mut keylog = keylog.and_then(|p| {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&p).map_err(|e| eprintln!("keylog: {e}")).ok()?;
        let _ = writeln!(f, "START rate {rate}");
        Some(f)
    });
    thread::spawn(move || {
        let mut block = [0f32; BLOCK];
        let mut cfg = None;
        let mut wav: Option<Wav> = None;
        let mut running = false;
        let mut traced = String::new();
        while !shared.quit.load(Relaxed) {
            let now = shared.clock.load(Relaxed);
            {
                let mut c = contest.lock().unwrap();
                let over_before = c.paddle.last_over.clone();
                while let Ok((t, down)) = key_rx.pop() {
                    if let Some(f) = &mut keylog {
                        use std::io::Write;
                        let s = &c.settings;
                        let wall = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_micros());
                        let _ = writeln!(f, "EDGE {t} {} wpm {} {:?} wall {wall} clock {now}", u8::from(down), s.wpm, s.keyer.mode);
                    }
                    c.paddle_edge(t, down);
                }
                c.paddle_poll(now);
                if let Some(f) = &mut keylog
                    && c.paddle.last_over != over_before
                {
                    use std::io::Write;
                    let _ = writeln!(f, "OVER {}", c.paddle.last_over);
                }
                if let Some(p) = &trace {
                    let now = trace_state(&c);
                    if now != traced {
                        let _ = std::fs::write(p, &now);
                        traced = now;
                    }
                }
                let wanted = audio_cfg(&c.settings);
                if cfg != Some(wanted) && to_audio.push(wanted).is_ok() {
                    cfg = Some(wanted);
                }
            }
            while ring.slots() >= BLOCK {
                let (mode, save) = {
                    let mut c = contest.lock().unwrap();
                    c.block(&mut block);
                    (c.mode, c.settings.save_wav)
                };
                let now_running = mode != RunMode::Stop;
                if now_running != running {
                    running = now_running;
                    if let Some(w) = wav.take() {
                        let _ = w.finalize();
                    }
                    if running && save && let Some(p) = &wav_path {
                        wav = open_wav(p);
                    }
                }
                if let Some(w) = &mut wav {
                    for v in block {
                        let _ = w.write_sample((v.clamp(-1.0, 1.0) * 32767.0) as i16);
                    }
                }
                for v in block {
                    let _ = ring.push(v);
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
        if let Some(w) = wav.take() {
            let _ = w.finalize();
        }
    });
    stream
}
