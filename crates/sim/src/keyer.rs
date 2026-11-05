//! Paddle keyer. One tick per audio sample in the program, one per ms in the tests:
//! all lengths are in ticks and come from `rate`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Straight,
    IambicA,
    IambicB,
    Bug,
    Ultimatic,
}

pub const DIT: u8 = 1;
pub const DAH: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub mode: Mode,
    pub wpm: u32,
    /// 50 = normal; each mark gets 2 * (weight - 50) % of a dit more, the space as much less.
    pub weight: u32,
    /// dah = dit * ratio / 10, 30 = 3:1.
    pub ratio: u32,
    /// Pause > 1 dit after an element: wait for a full character space.
    pub autospace: bool,
    pub swap: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { mode: Mode::IambicB, wpm: 20, weight: 50, ratio: 30, autospace: false, swap: false }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Idle,
    Mark,
    Space,
    Gap,
}

pub struct Keyer {
    cfg: Config,
    rate: u32,
    dit: u32,
    mark_dit: u32,
    mark_dah: u32,
    space: u32,
    state: State,
    cur: u8,
    mem: u8,
    last: u8,
    prev_pad: u8,
    t: u32,
}

impl Keyer {
    /// `rate`: ticks per second.
    pub fn new(cfg: Config, rate: u32) -> Self {
        let mut k = Keyer {
            cfg,
            rate,
            dit: 1,
            mark_dit: 1,
            mark_dah: 1,
            space: 1,
            state: State::Idle,
            cur: 0,
            mem: 0,
            last: 0,
            prev_pad: 0,
            t: 0,
        };
        k.configure(cfg);
        k
    }

    /// Takes effect at once; an element in progress keeps its remaining length.
    pub fn configure(&mut self, cfg: Config) {
        self.cfg = cfg;
        let dit = (self.rate as u64 * 6 / (5 * cfg.wpm.max(1) as u64)) as i64;
        let weight = dit * (cfg.weight as i64 - 50) / 50;
        self.dit = dit.max(1) as u32;
        self.mark_dit = (dit + weight).max(1) as u32;
        self.mark_dah = (dit * cfg.ratio as i64 / 10 + weight).max(1) as u32;
        self.space = (dit - weight).max(1) as u32;
    }

    pub fn config(&self) -> Config {
        self.cfg
    }

    fn start(&mut self, e: u8) {
        self.cur = e;
        self.mem &= !e;
        self.t = if e == DIT { self.mark_dit } else { self.mark_dah };
        self.state = State::Mark;
    }

    /// Next element from the paddles and the memory, 0 = none.
    fn next(&self, pad: u8) -> u8 {
        let mut p = pad;
        if self.cfg.mode != Mode::IambicA {
            p |= self.mem;
        }
        if p == DIT | DAH {
            if self.cfg.mode == Mode::Ultimatic {
                return self.last;
            }
            return if self.cur == DIT { DAH } else { DIT }; // squeeze: alternate
        }
        p
    }

    /// `pad`: DIT | DAH pressed, before the swap. Returns whether the key is down.
    pub fn tick(&mut self, pad: u8) -> bool {
        let straight = pad;
        let mut pad = if self.cfg.swap { (pad & DIT) << 1 | (pad & DAH) >> 1 } else { pad };
        let pressed = pad & !self.prev_pad;
        self.prev_pad = pad;
        if pressed & DAH != 0 {
            self.last = DAH;
        }
        if pressed & DIT != 0 {
            self.last = DIT; // both in the same tick: dit
        }

        if self.cfg.mode == Mode::Straight {
            self.state = State::Idle; // nothing left over for a later paddle mode
            return straight != 0;
        }

        let mut manual = false;
        if self.cfg.mode == Mode::Bug {
            manual = pad & DAH != 0; // dah side is a straight key
            pad &= DIT; // dit side gives automatic dits
        }

        // remember the other paddle during an element: iambic B while it is held,
        // ultimatic and bug only a new press, iambic A not at all
        match self.state {
            State::Mark | State::Space => {
                if self.cfg.mode == Mode::IambicB {
                    self.mem |= pad & !self.cur;
                } else if self.cfg.mode != Mode::IambicA {
                    self.mem |= pressed & pad & !self.cur;
                }
            }
            State::Gap => self.mem |= pad,
            State::Idle => {}
        }

        let mut out = false;
        if self.state == State::Idle {
            self.cur = 0;
            self.mem = 0;
            if pad != 0 {
                let e = if pad == DIT | DAH {
                    if self.cfg.mode == Mode::Ultimatic { self.last } else { DIT }
                } else {
                    pad
                };
                self.start(e);
            }
        }
        match self.state {
            State::Idle => {}
            State::Mark => {
                out = true;
                self.t -= 1;
                if self.t == 0 {
                    self.t = self.space;
                    self.state = State::Space;
                }
            }
            State::Space => {
                self.t -= 1;
                if self.t == 0 {
                    let e = self.next(pad);
                    if e != 0 {
                        self.start(e);
                    } else if self.cfg.autospace {
                        self.t = 2 * self.dit;
                        self.state = State::Gap;
                    } else {
                        self.state = State::Idle;
                    }
                }
            }
            State::Gap => {
                self.t -= 1;
                if self.t == 0 {
                    let e = if self.mem == DIT | DAH { DIT } else { self.mem };
                    if e != 0 {
                        self.start(e);
                    } else {
                        self.state = State::Idle;
                    }
                }
            }
        }
        out || manual
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Paddle script: pressed intervals (from_ms, to_ms, paddles).
    type Press = (u32, u32, u8);

    struct Run {
        out: String,
        marks: Vec<u32>,
        spaces: Vec<u32>,
    }

    fn run(cfg: Config, presses: &[Press], ms: u32) -> Run {
        let mut k = Keyer::new(cfg, 1000);
        let dit = 1200 / cfg.wpm;
        let mut r = Run { out: String::new(), marks: vec![], spaces: vec![] };
        let (mut key, mut since) = (false, 0);
        for t in 0..ms {
            let pad = presses
                .iter()
                .filter(|(a, b, _)| t >= *a && t < *b)
                .fold(0, |p, (_, _, x)| p | x);
            let d = k.tick(pad);
            if d != key {
                if key {
                    r.marks.push(since);
                    r.out.push(if since > 2 * dit { '-' } else { '.' });
                } else if !r.marks.is_empty() {
                    r.spaces.push(since);
                }
                key = d;
                since = 0;
            }
            since += 1;
        }
        r
    }

    fn cfg(mode: Mode) -> Config {
        Config { mode, ..Config::default() }
    }

    #[test]
    fn timing() {
        let c = cfg(Mode::IambicB);
        let r = run(c, &[(0, 290, DIT)], 1000);
        assert_eq!(r.out, "...");
        assert_eq!((r.marks[0], r.spaces[0], r.marks[2]), (60, 60, 60));
        let r = run(c, &[(0, 400, DAH)], 1000);
        assert_eq!(r.out, "--");
        assert_eq!((r.marks[0], r.spaces[0]), (180, 60));
        let r = run(Config { weight: 60, ..c }, &[(0, 290, DIT)], 1000);
        assert_eq!((r.marks[0], r.spaces[0]), (72, 48));
        let r = run(Config { ratio: 35, ..c }, &[(0, 400, DAH)], 400);
        assert_eq!(r.marks[0], 210);
    }

    #[test]
    fn iambic() {
        let b = cfg(Mode::IambicB);
        let a = cfg(Mode::IambicA);
        assert_eq!(run(b, &[(0, 500, DIT), (0, 500, DAH)], 1500).out, ".-.-.");
        // released during the first dah: B adds a dit, A stops
        let sq = [(0, 130, DIT), (0, 130, DAH)];
        assert_eq!(run(b, &sq, 1500).out, ".-.");
        assert_eq!(run(a, &sq, 1500).out, ".-");
        // squeeze released during the first dit: B sends one more dah, A does not
        let short = [(0, 30, DIT), (10, 30, DAH)];
        assert_eq!(run(b, &short, 1000).out, ".-");
        assert_eq!(run(a, &short, 1000).out, ".");
        // "C" with a squeeze: dah first, dit added
        assert_eq!(run(b, &[(0, 600, DAH), (100, 560, DIT)], 1500).out, "-.-.");
        // dah tapped during a dit
        assert_eq!(run(b, &[(0, 50, DIT), (20, 35, DAH)], 1000).out, ".-");
    }

    #[test]
    fn ultimatic() {
        let c = cfg(Mode::Ultimatic);
        assert_eq!(run(c, &[(0, 900, DIT), (100, 600, DAH)], 1500).out, ".---.");
        assert!(run(c, &[(0, 400, DIT), (100, 400, DAH)], 1500).out.ends_with('-'));
    }

    #[test]
    fn bug() {
        let r = run(cfg(Mode::Bug), &[(0, 170, DIT), (400, 650, DAH)], 1000);
        assert_eq!(r.out, "..-");
        assert_eq!(r.marks[2], 250); // manual dah as long as held
    }

    #[test]
    fn straight() {
        let r = run(cfg(Mode::Straight), &[(10, 133, DIT), (300, 320, DAH)], 500);
        assert_eq!(r.marks, vec![123, 20]);
    }

    #[test]
    fn swap() {
        let c = Config { swap: true, ..cfg(Mode::IambicB) };
        assert_eq!(run(c, &[(0, 100, DIT)], 1000).out, "-");
    }

    #[test]
    fn autospace() {
        let c = Config { autospace: true, ..cfg(Mode::IambicB) };
        let p = [(0, 30, DIT), (130, 250, DAH)];
        let r = run(c, &p, 1000);
        assert_eq!(r.out, ".-");
        assert_eq!(r.spaces[0], 180);
        assert_eq!(run(Config { autospace: false, ..c }, &p, 1000).spaces[0], 70);
    }

    #[test]
    fn dit_length_follows_rate() {
        let mut k = Keyer::new(Config::default(), 48000);
        let n = (0..48000).take_while(|_| k.tick(DIT) || false).count();
        assert_eq!(n, 2880); // 60 ms at 20 wpm
    }
}
