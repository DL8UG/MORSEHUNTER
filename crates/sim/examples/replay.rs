//! Runs a key log (`morsehunter --keylog FILE`) through the decoder again, as the program
//! would, and shows the overs it read then and now. For tuning the decoder on a real hand.
//!
//!     cargo run --release -p morse-runner-sim --example replay -- FILE
use mr_sim::rxtext::RxText;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let rate = 48000.0;
    let mut edges = vec![];
    let mut old = vec![];
    for l in std::fs::read_to_string(path).unwrap().lines() {
        let p: Vec<&str> = l.split_whitespace().collect();
        match p.first() {
            Some(&"EDGE") => edges.push((p[1].parse::<u64>().unwrap(), p[2] == "1", p[4].parse::<u32>().unwrap())),
            Some(&"OVER") => old.push(p[1..].join(" ")),
            Some(&"START") => edges.push((0, false, 0)),
            _ => {}
        }
    }
    let mut rx = RxText::new(1.2 / 15.0 * rate, false);
    let mut new = vec![];
    let mut last_t = 0u64;
    let flush = |rx: &mut RxText, new: &mut Vec<String>, t: u64| {
        rx.poll(t);
        let s = rx.take_text();
        if !s.trim().is_empty() { new.push(format!("{}   [word gap {:.0} ms, dit {:.0} ms]", s.trim(), rx.word_gap() / 48.0, rx.unit() / 48.0)); }
    };
    for (t, down, wpm) in edges {
        if wpm == 0 { flush(&mut rx, &mut new, last_t + 10_000_000); last_t = 0; continue; }
        // the over ends if the silence before this edge was long enough
        if down && let Some(up) = rx.last_key() {
            let wait = rx.over_gap(rate) as u64;
            if t.saturating_sub(up) >= wait { flush(&mut rx, &mut new, up + wait); }
        }
        rx.set_unit(1.2 / wpm as f64 * rate, false);
        rx.key(t, down);
        last_t = t;
    }
    flush(&mut rx, &mut new, last_t + 10_000_000);
    println!("read then ({} overs):", old.len());
    for o in &old { println!("   {o}"); }
    println!("read now ({} overs):", new.len());
    for o in &new { println!("   {o}"); }
}
