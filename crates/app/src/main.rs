// a release build on Windows opens no console window next to its own
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod audio;
mod gfx;
mod simthread;
mod store;
mod ui;
mod vail;

use std::process::ExitCode;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use audio::Shared;
use gfx::Gfx;
use mr_sim::BLOCK;
use mr_sim::bot::Bot;
use mr_sim::code::RunCode;
use mr_sim::contest::{Contest, RunMode};
use mr_sim::entry::Key as SimKey;
use mr_sim::settings::{CALLER_FAST_DEFAULT, CALLER_SLOW_DEFAULT, HEAD_START_MAX, Settings};
use mr_sim::RATE;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
use winit::window::{Window, WindowId};

const FRAME: Duration = Duration::from_millis(33);
const IDLE_FRAME: Duration = Duration::from_millis(250);

enum Action {
    Key(SimKey),
    Run(RunMode),
    Stop,
}

/// All keys of the operating position. Plain characters come as typed, so `+` and `;` work
/// on every keyboard layout.
fn action(e: &KeyEvent, mods: ModifiersState, ctrl_is_paddle: bool) -> Option<Action> {
    use SimKey::*;
    let ctrl = mods.control_key() && !ctrl_is_paddle;
    let (alt, shift) = (mods.alt_key(), mods.shift_key());
    let modifier = ctrl || alt;
    let key = |k| Some(Action::Key(k));
    if let PhysicalKey::Code(code) = e.physical_key {
        let f = match code {
            KeyCode::F1 => 1,
            KeyCode::F2 => 2,
            KeyCode::F3 => 3,
            KeyCode::F4 => 4,
            KeyCode::F5 => 5,
            KeyCode::F6 => 6,
            KeyCode::F7 => 7,
            KeyCode::F8 => 8,
            _ => 0,
        };
        if f != 0 {
            return if e.repeat { None } else { key(F(f)) };
        }
        match code {
            KeyCode::F9 if modifier => return key(Speed(-1)),
            KeyCode::F10 if modifier => return key(Speed(1)),
            KeyCode::F9 => return Some(Action::Run(RunMode::Pileup)),
            KeyCode::F10 => return Some(Action::Stop),
            // window managers like to keep F9 for themselves
            KeyCode::KeyR if alt => return Some(Action::Run(RunMode::Pileup)),
            KeyCode::F11 => return key(Wipe),
            KeyCode::KeyW if modifier => return key(Wipe),
            _ => {}
        }
    }
    match &e.logical_key {
        Key::Named(n) => match n {
            NamedKey::Enter if modifier || shift => key(SaveEnter),
            NamedKey::Enter => key(Enter),
            NamedKey::Tab if shift => key(BackTab),
            NamedKey::Tab => key(Tab),
            NamedKey::Escape => key(Esc),
            NamedKey::Backspace => key(Backspace),
            NamedKey::Delete => key(Delete),
            NamedKey::Insert => key(Insert),
            NamedKey::ArrowLeft => key(Left),
            NamedKey::ArrowRight => key(Right),
            NamedKey::Home => key(Home),
            NamedKey::End => key(End),
            NamedKey::ArrowUp if modifier => key(Bandwidth(1)),
            NamedKey::ArrowDown if modifier => key(Bandwidth(-1)),
            NamedKey::ArrowUp => key(Rit(1)),
            NamedKey::ArrowDown => key(Rit(-1)),
            NamedKey::PageUp => key(Speed(1)),
            NamedKey::PageDown => key(Speed(-1)),
            NamedKey::Space if !modifier => key(Char(' ')),
            _ => None,
        },
        Key::Character(s) if !modifier => s.chars().next().and_then(|c| key(Char(c))),
        _ => None,
    }
}

/// Wall clock in microseconds, to line up the window's key log with outside tools.
fn now_us() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_micros())
}

struct WindowApp {
    /// With `--keylog`: when paddle keys arrive and how long drawing takes.
    ui_log: Option<std::fs::File>,
    /// Stops the sound when dropped.
    _audio: Option<cpal::Stream>,
    ui: ui::App,
    window: Option<Arc<Window>>,
    gfx: Option<Gfx>,
    egui_ctx: egui::Context,
    egui_state: Option<egui_winit::State>,
    mods: ModifiersState,
    last_frame: Instant,
    dirty: bool,
}

impl WindowApp {
    fn redraw(&mut self) {
        let (Some(window), Some(gfx), Some(state)) = (&self.window, &mut self.gfx, &mut self.egui_state) else {
            return;
        };
        let input = state.take_egui_input(window);
        let ui = &mut self.ui;
        let output = self.egui_ctx.run_ui(input, |root| {
            egui::CentralPanel::default().show(root, |panel| ui.ui(panel));
        });
        state.handle_platform_output(window, output.platform_output.clone());
        let ppp = output.pixels_per_point;
        let start = now_us();
        if !gfx.paint(window, &self.egui_ctx, output, ppp) {
            // the next frame comes within FRAME, not after the idle pause
            self.dirty = true;
        }
        if let Some(f) = &mut self.ui_log {
            use std::io::Write;
            let _ = writeln!(f, "PAINT {start} {}", now_us() - start);
        }
    }

    /// True if the key is used up and egui must not see it.
    fn key(&mut self, event: &KeyEvent) -> bool {
        let PhysicalKey::Code(code) = event.physical_key else { return false };
        let name = format!("{code:?}");
        let down = event.state == ElementState::Pressed;
        if self.ui.capture.is_some() {
            if down && !event.repeat {
                self.ui.captured(name);
            }
            return true;
        }
        if let Some(bit) = self.ui.paddle_of(&name) {
            if let Some(f) = &mut self.ui_log {
                use std::io::Write;
                let _ = writeln!(f, "KEY {} {bit} {} {}", now_us(), u8::from(down), u8::from(event.repeat));
            }
            // a character key belongs to the text box while one has the focus
            let modifier = ["Control", "Alt", "Shift", "Super"].iter().any(|m| name.starts_with(m));
            if down && !modifier && self.egui_ctx.egui_wants_keyboard_input() {
                return false;
            }
            if !event.repeat {
                self.ui.shared.set_paddle(bit, down);
            }
            return true;
        }
        if !down {
            return false;
        }
        // a text box of the options page has the keys
        if self.egui_ctx.egui_wants_keyboard_input() {
            return false;
        }
        let ctrl_paddle = self.ui.ctrl_is_paddle();
        match action(event, self.mods, ctrl_paddle) {
            Some(Action::Key(k)) => self.ui.key(k),
            Some(Action::Run(m)) => {
                if !self.ui.running() {
                    self.ui.start(m);
                }
            }
            Some(Action::Stop) => self.ui.stop(),
            None => return false,
        }
        true
    }
}

impl ApplicationHandler for WindowApp {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes().with_title("MORSEHUNTER").with_inner_size(LogicalSize::new(1040.0, 680.0));
        // the app id, so the desktop entry and its icon belong to the window
        #[cfg(all(unix, not(target_os = "macos")))]
        let attrs = {
            use winit::platform::{wayland::WindowAttributesExtWayland, x11::WindowAttributesExtX11};
            let attrs = WindowAttributesExtWayland::with_name(attrs, "morsehunter", "morsehunter");
            WindowAttributesExtX11::with_name(attrs, "morsehunter", "morsehunter")
        };
        // the icon build.rs put into the exe, for the title bar and the taskbar
        #[cfg(windows)]
        let attrs = {
            use winit::platform::windows::{IconExtWindows, WindowAttributesExtWindows};
            use winit::window::Icon;
            match Icon::from_resource(1, None) {
                Ok(icon) => attrs.with_window_icon(Some(icon.clone())).with_taskbar_icon(Some(icon)),
                Err(_) => attrs,
            }
        };
        let window = Arc::new(el.create_window(attrs).expect("window"));
        let gfx = Gfx::new(window.clone(), el.owned_display_handle());
        self.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        ui::theme::apply(&self.egui_ctx);
        self.egui_state = Some(egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            None,
        ));
        self.gfx = Some(gfx);
        self.window = Some(window);
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.window.as_ref().is_none_or(|w| w.id() != id) {
            return;
        }
        match &event {
            WindowEvent::KeyboardInput { event: key, .. } => {
                let used = self.key(key);
                self.dirty = true;
                if used {
                    return;
                }
            }
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            WindowEvent::Focused(false) => self.ui.shared.release_all(),
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => {
                if let Some(g) = &mut self.gfx {
                    g.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
                return;
            }
            _ => {}
        }
        self.dirty = true;
        if let (Some(state), Some(window)) = (&mut self.egui_state, &self.window) {
            let _ = state.on_window_event(window, &event);
        }
    }

    /// The Wayland connection ends with `run_app`. The clipboard inside the egui state
    /// has a thread on it, so that state has to go while the connection is still there.
    fn exiting(&mut self, _: &ActiveEventLoop) {
        self.ui.save();
        self.ui.shared.quit.store(true, Relaxed);
        self.egui_state = None;
        self.gfx = None;
        self.window = None;
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        self.dirty |= self.ui.running();
        let now = Instant::now();
        let due = self.last_frame + if self.dirty { FRAME } else { IDLE_FRAME };
        if now >= due {
            self.dirty = false;
            self.last_frame = now;
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        el.set_control_flow(ControlFlow::WaitUntil(now + FRAME));
    }
}

fn run_window(args: &[String]) -> Result<(), String> {
    let (mut trace, mut autostart, mut code, mut keylog) = (None, false, None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--start" => autostart = true,
            "--code" => code = Some(RunCode::decode(it.next().ok_or(USAGE)?).map_err(code_error)?),
            "--trace" => trace = Some(std::path::PathBuf::from(it.next().ok_or(USAGE)?)),
            "--keylog" => keylog = Some(std::path::PathBuf::from(it.next().ok_or(USAGE)?)),
            _ => return Err(USAGE.into()),
        }
    }
    let (settings, first_run) = store::load_settings();
    let shared = Arc::new(Shared::new());
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    let first = simthread::audio_cfg(&settings);
    let contest = Arc::new(Mutex::new(Contest::new(settings, seed)));
    let (stream, error) = match audio::start(shared.clone(), first) {
        Ok(a) => {
            contest.lock().unwrap().paddle.set_rate(a.rate);
            (Some(simthread::spawn(contest.clone(), shared.clone(), a, store::wav_path(), trace, keylog.clone())), None)
        }
        Err(e) => (None, Some(e)),
    };
    let mut ui = ui::App::new(contest, shared, first_run, error);
    if let Some(c) = &code {
        ui.start_code(c);
    } else if autostart {
        ui.start(RunMode::Pileup);
    }
    let el = EventLoop::new().map_err(|e| e.to_string())?;
    let ui_log = keylog.map(|p| p.with_extension("ui.txt")).and_then(|p| std::fs::OpenOptions::new().create(true).append(true).open(p).ok());
    let mut app = WindowApp {
        ui_log,
        _audio: stream,
        ui,
        window: None,
        gfx: None,
        egui_ctx: egui::Context::default(),
        egui_state: None,
        mods: ModifiersState::empty(),
        last_frame: Instant::now(),
        dirty: true,
    };
    el.run_app(&mut app).map_err(|e| e.to_string())
}

const USAGE: &str = "usage: morsehunter [--start] [--code CODE] [--trace FILE] [--keylog FILE]    open the window
       morsehunter render <out.wav> [--mode pileup|single|wpx|hst] [--secs N] [--seed N] [--call CALL] [--wpm N]
                       [--minutes N] [--head-start HOURS] [--slow PCT] [--fast PCT] [--waves] [--code CODE]
                                    a session worked by a plain built-in operator, as an audio file";

fn code_error(e: mr_sim::code::CodeError) -> String {
    format!("run code: {e:?}")
}

fn render_wav(args: &[String]) -> Result<(), String> {
    let mut it = args.iter();
    let path = it.next().ok_or(USAGE)?.clone();
    let (mut secs, mut seed, mut mode, mut code) = (60.0, 1, RunMode::Pileup, None);
    let mut s = Settings { duration: 240, ..Settings::default() };
    while let Some(a) = it.next() {
        let mut value = || it.next().ok_or(USAGE);
        match a.as_str() {
            "--mode" => {
                mode = match value()?.as_str() {
                    "pileup" => RunMode::Pileup,
                    "single" => RunMode::Single,
                    "wpx" => RunMode::Wpx,
                    "hst" => RunMode::Hst,
                    _ => return Err(USAGE.into()),
                }
            }
            "--secs" => secs = value()?.parse().map_err(|_| USAGE)?,
            "--seed" => seed = value()?.parse().map_err(|_| USAGE)?,
            "--call" => s.call = value()?.to_uppercase(),
            "--wpm" => s.wpm = value()?.parse().map_err(|_| USAGE)?,
            "--minutes" => s.duration = value()?.parse().map_err(|_| USAGE)?,
            "--head-start" => s.head_start = value()?.parse().map_err(|_| USAGE)?,
            "--slow" => s.caller_slow = value()?.parse().map_err(|_| USAGE)?,
            "--fast" => s.caller_fast = value()?.parse().map_err(|_| USAGE)?,
            "--waves" => s.waves = true,
            "--code" => code = Some(RunCode::decode(value()?).map_err(code_error)?),
            _ => return Err(USAGE.into()),
        }
    }
    if s.head_start > HEAD_START_MAX {
        eprintln!("--head-start: {HEAD_START_MAX} h at most, taking {HEAD_START_MAX}");
    }
    let own = s.head_start > 0 || s.waves || (s.caller_slow, s.caller_fast) != (CALLER_SLOW_DEFAULT, CALLER_FAST_DEFAULT);
    if own && mode.competition() && code.is_none() {
        eprintln!("competitions keep the conditions of the original: no head start, no waves, callers at 50 to 100 %");
    }
    s.clamp();
    let mut c = Contest::new(s, seed);
    match &code {
        Some(code) => c.run_code(code),
        None => c.run(mode),
    }
    let mut bot = Bot::new(seed);
    let spec = hound::WavSpec { channels: 1, sample_rate: RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(&path, spec).map_err(|e| e.to_string())?;
    let mut b = [0f32; BLOCK];
    for _ in 0..mr_sim::seconds_to_blocks(secs) {
        c.block(&mut b);
        bot.step(&mut c);
        for v in b {
            w.write_sample((v.clamp(-1.0, 1.0) * 32767.0) as i16).map_err(|e| e.to_string())?;
        }
    }
    w.finalize().map_err(|e| e.to_string())?;
    let (raw, ver) = (c.log.raw(), c.log.verified());
    println!("{} QSOs, {} verified, score {}", raw.points, ver.points, ver.score);
    if let Some(code) = &c.code {
        println!("code {}", code.encode());
    }
    Ok(())
}

fn main() -> ExitCode {
    // Mesa says on every start that Vulkan on older Intel chips (Haswell, Ivy Bridge) is
    // incomplete. It says nothing about this program, which runs fine there.
    #[cfg(all(unix, not(target_os = "macos")))]
    if std::env::var_os("MESA_VK_IGNORE_CONFORMANCE_WARNING").is_none() {
        // SAFETY: the first thing in main, no other thread reads the environment yet
        unsafe { std::env::set_var("MESA_VK_IGNORE_CONFORMANCE_WARNING", "1") };
    }
    // without a console of its own, `render` writes into the one it was started from
    #[cfg(all(windows, not(debug_assertions)))]
    {
        use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
        // SAFETY: a plain call, fails harmlessly when started from the Explorer
        unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("render") => render_wav(&args[1..]),
        _ => run_window(&args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
