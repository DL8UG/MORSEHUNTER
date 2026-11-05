//! Keying by a simulated human hand, through the real keyer and the paddle decoder, against
//! the decoder of the last commit. The hand listens to its own sidetone: it holds a paddle
//! for a run of equal elements and lets go once the last of them has begun, it presses the
//! other paddle while an element still sounds (the keyer remembers it) or now and then too
//! late, and it keeps its own pauses between letters and words, with some scatter. On a
//! straight key it makes the elements itself, at its own speed, which drifts.
//!
//!     cargo run --release -p morse-runner-sim --example handsim

mod oldrx;

use mr_sim::keyer::{Config, DAH, DIT, Keyer, Mode};
use mr_sim::morse;
use mr_sim::paddle::read_over;
use mr_sim::rng::Rng;
use mr_sim::rxtext::RxText;

/// Ticks per second for keyer and decoder.
const RATE: f64 = 8000.0;
/// The decoder is polled this often, as the sim thread does about every 2 ms.
const POLL: u64 = 16;

const MESSAGES: &[&str] = &[
    "CQ TEST DL8UG",
    "W1ABC 5NN 001",
    "TU DL8UG",
    "PA3XYZ",
    "5NN 123",
    "R 5NN 045",
    "AGN",
    "NR?",
    "XX?",
    "OK1ABC 5NN 012",
    "TU",
    "CQ CQ TEST DL8UG DL8UG",
    "JA1ABC 5NN 1234",
    "DE DL8UG",
];

fn ms(x: f64) -> f64 {
    x * RATE / 1000.0
}

fn unit_of(wpm: f64) -> f64 {
    1.2 / wpm * RATE
}

#[derive(Clone, Copy)]
enum Gap {
    /// In dits of the speed the hand keys at.
    Units(f64),
    /// In milliseconds, whatever the speed.
    Ms(f64),
}

#[derive(Clone)]
struct Hand {
    name: &'static str,
    mode: Mode,
    letter: Gap,
    word: Gap,
    /// Relative scatter of every pause and, on a straight key, of every element.
    jitter: f64,
    /// Chance to press the next paddle after the element space has run out.
    late: f64,
    /// Chance of a thinking pause of 0.8 to 1.4 s between two words.
    think: f64,
    /// Straight key: the hand's speed relative to the decoder's setting, and its drift over
    /// the session (end speed / start speed).
    hand_speed: f64,
    drift: f64,
    dah_ratio: f64,
}

/// One message as keyed and what each decoder made of it.
struct Result {
    sent: String,
    new: Vec<String>,
    old: Vec<String>,
}

/// The program's over logic around a decoder.
struct Overs<D> {
    rx: D,
    open: bool,
    out: Vec<(u64, String)>,
}

trait Decoder {
    fn key(&mut self, t: u64, down: bool);
    fn poll(&mut self, t: u64);
    fn busy(&self) -> bool;
    fn last_key(&self) -> Option<u64>;
    fn unit(&self) -> f64;
    fn take(&mut self) -> String;
    fn over_gap(&self) -> f64;
}

impl Decoder for RxText {
    fn key(&mut self, t: u64, down: bool) {
        RxText::key(self, t, down)
    }
    fn poll(&mut self, t: u64) {
        RxText::poll(self, t)
    }
    fn busy(&self) -> bool {
        RxText::busy(self)
    }
    fn last_key(&self) -> Option<u64> {
        RxText::last_key(self)
    }
    fn unit(&self) -> f64 {
        RxText::unit(self)
    }
    fn take(&mut self) -> String {
        self.take_text()
    }
    fn over_gap(&self) -> f64 {
        RxText::over_gap(self, RATE)
    }
}

impl Decoder for oldrx::RxText {
    fn key(&mut self, t: u64, down: bool) {
        oldrx::RxText::key(self, t, down)
    }
    fn poll(&mut self, t: u64) {
        oldrx::RxText::poll(self, t)
    }
    fn busy(&self) -> bool {
        oldrx::RxText::busy(self)
    }
    fn last_key(&self) -> Option<u64> {
        oldrx::RxText::last_key(self)
    }
    fn unit(&self) -> f64 {
        oldrx::RxText::unit(self)
    }
    fn take(&mut self) -> String {
        self.take_text()
    }
    fn over_gap(&self) -> f64 {
        (12.0 * self.unit()).max(0.35 * RATE)
    }
}

impl<D: Decoder> Overs<D> {
    fn wait(&self) -> u64 {
        self.rx.over_gap() as u64
    }

    fn edge(&mut self, t: u64, down: bool) {
        if down {
            self.open = true;
        }
        self.rx.key(t, down);
    }

    fn poll(&mut self, t: u64) {
        self.rx.poll(t);
        if self.open && !self.rx.busy() && self.rx.last_key().is_some_and(|up| t.saturating_sub(up) >= self.wait()) {
            let end = t + (20.0 * self.rx.unit()) as u64;
            self.rx.poll(end);
            let text = self.rx.take().trim().to_string();
            self.open = false;
            if !text.is_empty() {
                self.out.push((t, text));
            }
        }
    }
}

/// Runs of equal elements: "-.-." gives [(DAH,1),(DIT,1),(DAH,1),(DIT,1)].
fn runs(pattern: &str) -> Vec<(u8, usize)> {
    let mut r: Vec<(u8, usize)> = Vec::new();
    for c in pattern.chars() {
        let p = if c == '-' { DAH } else { DIT };
        match r.last_mut() {
            Some(last) if last.0 == p => last.1 += 1,
            _ => r.push((p, 1)),
        }
    }
    r
}

/// Keyer, both decoders and the hand's pending paddle releases, one tick at a time.
struct Sim {
    t: u64,
    key: bool,
    pad: u8,
    releases: Vec<(u64, u8)>,
    keyer: Keyer,
    new: Overs<RxText>,
    old: Overs<oldrx::RxText>,
}

impl Sim {
    /// One tick; returns a key edge of the keyer if there was one.
    fn tick(&mut self) -> Option<bool> {
        let t = self.t;
        let mut i = 0;
        while i < self.releases.len() {
            if self.releases[i].0 <= t {
                self.pad &= !self.releases[i].1;
                self.releases.swap_remove(i);
            } else {
                i += 1;
            }
        }
        let d = self.keyer.tick(self.pad);
        let mut edge = None;
        if d != self.key {
            self.key = d;
            self.new.edge(t, d);
            self.old.edge(t, d);
            edge = Some(d);
        }
        if t.is_multiple_of(POLL) {
            self.new.poll(t);
            self.old.poll(t);
        }
        self.t += 1;
        edge
    }

    fn wait(&mut self, ticks: f64) {
        let until = self.t + ticks.max(0.0) as u64;
        while self.t < until {
            self.tick();
        }
    }
}

/// The keyed signal of a session: the hand drives the keyer, both decoders listen.
fn session(hand: &Hand, wpm: u32, seed: u64, messages: &[&str]) -> (Vec<Result>, f64) {
    let mut rng = Rng::new(seed);
    let unit = unit_of(wpm as f64);
    let adaptive = matches!(hand.mode, Mode::Straight | Mode::Bug);
    let mut sim = Sim {
        t: 0,
        key: false,
        pad: 0,
        releases: Vec::new(),
        keyer: Keyer::new(Config { mode: hand.mode, wpm, ..Config::default() }, RATE as u32),
        new: Overs { rx: RxText::new(unit, adaptive), open: false, out: vec![] },
        old: Overs { rx: oldrx::RxText::new(unit, adaptive), open: false, out: vec![] },
    };
    let mut starts: Vec<u64> = Vec::new();
    let total_msgs = messages.len() as f64;
    let scatter = |rng: &mut Rng, x: f64| (x * (1.0 + hand.jitter * rng.gauss())).max(x * 0.3);
    let gap_len = |g: Gap, u: f64| match g {
        Gap::Units(n) => n * u,
        Gap::Ms(m) => ms(m),
    };

    for (mi, msg) in messages.iter().enumerate() {
        starts.push(sim.t);
        let progress = mi as f64 / (total_msgs - 1.0).max(1.0);
        // the hand's own dit on a straight key, drifting over the session
        let hand_unit = unit / hand.hand_speed / (1.0 + (hand.drift - 1.0) * progress);
        let elem_unit = if adaptive { hand_unit } else { unit };
        let mut first = true;
        let mut pending_word = false;
        for p in morse::patterns(msg) {
            let Some(p) = p else {
                pending_word = true;
                continue;
            };
            // the pause before this letter, from the end of the last element
            if !first {
                let mut g = if pending_word { gap_len(hand.word, elem_unit) } else { gap_len(hand.letter, elem_unit) };
                if pending_word && rng.chance(hand.think) {
                    g += ms(rng.range(800.0, 1400.0));
                }
                // a hand never comes back inside the element space, the letters would merge
                let g = scatter(&mut rng, g).max(1.6 * elem_unit);
                sim.wait(g);
            }
            first = false;
            pending_word = false;

            if adaptive {
                // straight key: the hand makes every element
                let n = p.len();
                for (i, e) in p.chars().enumerate() {
                    let len = if e == '-' { hand.dah_ratio * hand_unit } else { hand_unit };
                    let len = scatter(&mut rng, len);
                    sim.pad = DIT;
                    sim.wait(len);
                    sim.pad = 0;
                    if i + 1 < n {
                        let sp = scatter(&mut rng, hand_unit).max(0.4 * hand_unit);
                        sim.wait(sp);
                    }
                }
                continue;
            }

            // a paddle: one run of equal elements after the other
            let rs = runs(p);
            for (ri, &(paddle, count)) in rs.iter().enumerate() {
                sim.pad |= paddle;
                let elem = if paddle == DAH { 3.0 * unit } else { unit };
                let mut seen = 0;
                let mut next_at: Option<u64> = None;
                loop {
                    if next_at.is_some_and(|n| sim.t >= n) {
                        break;
                    }
                    let e = sim.tick();
                    if e == Some(true) {
                        seen += 1;
                        if seen == count {
                            // the last element of the run has begun: let go during it
                            // before the element space runs out, or the keyer adds one more
                            let frac = (0.5 + 0.2 * rng.gauss()).clamp(0.1, (elem + 0.8 * unit) / elem);
                            sim.releases.push((sim.t + (frac * elem) as u64, paddle));
                            if ri + 1 < rs.len() {
                                // the next paddle: while this one still sounds, or late
                                let at = if rng.chance(hand.late) {
                                    elem + unit + rng.range(0.1, 0.9) * unit
                                } else {
                                    (0.3 + 0.5 * rng.f64()) * elem
                                };
                                next_at = Some(sim.t + at as u64);
                            }
                        }
                    }
                    if e == Some(false) && seen >= count && ri + 1 == rs.len() {
                        break;
                    }
                }
            }
            // the letter is out once the paddles are free and the keyer is silent
            while sim.pad != 0 || sim.key {
                sim.tick();
            }
        }
        // listening to the other station before the next over
        let pause = ms(rng.range(3000.0, 4500.0));
        sim.wait(pause);
    }
    let learned = sim.new.rx.unit();
    let assign = |out: &Vec<(u64, String)>, i: usize| -> Vec<String> {
        let from = starts[i];
        let to = starts.get(i + 1).copied().unwrap_or(u64::MAX);
        out.iter().filter(|(t, _)| *t >= from && *t < to).map(|(_, s)| s.clone()).collect()
    };
    let results = messages
        .iter()
        .enumerate()
        .map(|(i, m)| Result { sent: m.to_string(), new: assign(&sim.new.out, i), old: assign(&sim.old.out, i) })
        .collect();
    (results, learned / unit)
}

#[derive(Default, Clone, Copy)]
struct Score {
    n: usize,
    exact: usize,
    /// The stations understand the same: same messages, same call.
    meaning: usize,
    split: usize,
    char_err: usize,
    chars: usize,
    space_err: usize,
}

impl Score {
    fn add(&mut self, sent: &str, got: &[String]) {
        self.n += 1;
        let joined = got.join(" ");
        let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        if norm(&joined) == norm(sent) {
            self.exact += 1;
        }
        if got.len() > 1 {
            self.split += 1;
        }
        // what the stations would take from the over (the first one, as they react to it)
        let first = got.first().cloned().unwrap_or_default();
        if read_over(&first, "DL8UG") == read_over(sent, "DL8UG") {
            self.meaning += 1;
        }
        let a: String = sent.chars().filter(|c| *c != ' ').collect();
        let b: String = joined.chars().filter(|c| *c != ' ').collect();
        let ce = morse::levenshtein(&a, &b);
        self.char_err += ce;
        self.chars += a.len();
        if ce == 0 && norm(&joined) != norm(sent) {
            self.space_err += 1;
        }
    }

    fn line(&self) -> String {
        let pct = |x: usize| 100.0 * x as f64 / self.n.max(1) as f64;
        format!(
            "{:>5.1} % {:>5.1} % {:>5.1} % {:>5.1} % {:>5.1} %",
            pct(self.exact),
            pct(self.meaning),
            pct(self.space_err),
            pct(self.split),
            100.0 * self.char_err as f64 / self.chars.max(1) as f64
        )
    }
}

fn main() {
    let verbose = std::env::args().any(|a| a == "-v");
    let base = Hand {
        name: "",
        mode: Mode::IambicB,
        letter: Gap::Units(3.0),
        word: Gap::Units(7.0),
        jitter: 0.1,
        late: 0.1,
        think: 0.0,
        hand_speed: 1.0,
        drift: 1.0,
        dah_ratio: 3.0,
    };
    let hands = vec![
        Hand { name: "Lehrbuch, Iambic B", ..base.clone() },
        Hand { name: "eigener Rhythmus 300/650 ms, Iambic B", letter: Gap::Ms(300.0), word: Gap::Ms(650.0), jitter: 0.2, late: 0.3, ..base.clone() },
        Hand { name: "eigener Rhythmus 300/650 ms, Iambic A", mode: Mode::IambicA, letter: Gap::Ms(300.0), word: Gap::Ms(650.0), jitter: 0.2, late: 0.3, ..base.clone() },
        Hand { name: "gedrängt 2,4/5 Punkte", letter: Gap::Units(2.4), word: Gap::Units(5.0), jitter: 0.15, late: 0.2, ..base.clone() },
        Hand { name: "zögerlich 450/900 ms + Denkpausen", letter: Gap::Ms(450.0), word: Gap::Ms(900.0), jitter: 0.25, late: 0.4, think: 0.15, ..base.clone() },
        Hand { name: "unruhig 30 % Streuung", letter: Gap::Ms(280.0), word: Gap::Ms(600.0), jitter: 0.3, late: 0.4, ..base.clone() },
        Hand { name: "Handtaste, Tempo wie eingestellt", mode: Mode::Straight, jitter: 0.12, ..base.clone() },
        Hand { name: "Handtaste, 30 % langsamer", mode: Mode::Straight, jitter: 0.12, hand_speed: 0.7, ..base.clone() },
        Hand { name: "Handtaste, 40 % schneller", mode: Mode::Straight, jitter: 0.12, hand_speed: 1.4, ..base.clone() },
        Hand { name: "Handtaste, wird 30 % schneller", mode: Mode::Straight, jitter: 0.12, drift: 1.3, ..base.clone() },
        Hand { name: "Handtaste, Strich 2,5:1, unruhig", mode: Mode::Straight, jitter: 0.2, dah_ratio: 2.5, ..base.clone() },
        Hand { name: "Handtaste, eigener Rhythmus 350/800 ms", mode: Mode::Straight, letter: Gap::Ms(350.0), word: Gap::Ms(800.0), jitter: 0.15, ..base.clone() },
    ];
    let speeds = [15, 20, 25, 30, 35];
    let seeds: u64 = std::env::args().skip_while(|a| a != "-n").nth(1).and_then(|v| v.parse().ok()).unwrap_or(6);
    println!("Je Hand {} Tempi × {} Durchläufe × {} Durchgänge\n", speeds.len(), seeds, MESSAGES.len());
    println!("{:<40} {:<4} exakt   Sinn    Lücken  zerteilt Zeichenfehler   gelerntes Tempo", "Hand", "");
    for hand in &hands {
        let (mut new, mut old) = (Score::default(), Score::default());
        let mut learned = Vec::new();
        let mut shown = 0;
        // learning curve: exact reads of the new decoder by the place of the over in a session
        let mut by_place = [(0usize, 0usize); 4];
        for &wpm in &speeds {
            for s in 0..seeds {
                let (res, l) = session(hand, wpm, 1000 + s * 7 + wpm as u64, MESSAGES);
                learned.push(l);
                for (i, r) in res.iter().enumerate() {
                    let place = i.min(3);
                    by_place[place].1 += 1;
                    if r.new.join(" ").split_whitespace().collect::<Vec<_>>().join(" ") == r.sent {
                        by_place[place].0 += 1;
                    }
                    new.add(&r.sent, &r.new);
                    old.add(&r.sent, &r.old);
                    if verbose && shown < 6 && r.new.join(" ").split_whitespace().collect::<Vec<_>>().join(" ") != r.sent {
                        println!("    {wpm} wpm: {:<24} neu {:?}   alt {:?}", r.sent, r.new, r.old);
                        shown += 1;
                    }
                }
            }
        }
        let tempo = if matches!(hand.mode, Mode::Straight) {
            let want = hand.hand_speed;
            let mean = learned.iter().sum::<f64>() / learned.len() as f64;
            format!("Punkt ×{:.2} (Hand ×{:.2})", mean, 1.0 / (want * hand.drift))
        } else {
            String::new()
        };
        println!("{:<40} alt  {}", hand.name, old.line());
        println!("{:<40} neu  {}   {}", "", new.line(), tempo);
        if std::env::args().any(|a| a == "-l") {
            let p = |i: usize| 100.0 * by_place[i].0 as f64 / by_place[i].1.max(1) as f64;
            println!("{:<40}      Lernkurve exakt: 1. {:.0} %, 2. {:.0} %, 3. {:.0} %, ab 4. {:.0} %", "", p(0), p(1), p(2), p(3));
        }
    }
    println!("\nexakt: Text wie gemeint. Sinn: die Stationen verstehen dasselbe (erster Durchgang). Lücken: nur Wortlücken falsch.");
    println!("zerteilt: Durchgang in mehrere zerfallen. Zeichenfehler: Anteil falscher Zeichen ohne Leerzeichen.");
}
