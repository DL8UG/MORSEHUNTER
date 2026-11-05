//! Made-up calls: a country prefix from AD1C's cty.dat, its numeral and 2 or 3 letters.
//! Nobody's real call is looked up, a generated one can still exist by chance.

use serde::{Deserialize, Serialize};

use crate::prefixes::{COUNTRIES, Country};
use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Region {
    NorthAmerica,
    SouthAmerica,
    Europe,
    Asia,
    Oceania,
    Africa,
}

impl Region {
    pub const ALL: [Region; 6] = [
        Region::NorthAmerica,
        Region::SouthAmerica,
        Region::Europe,
        Region::Asia,
        Region::Oceania,
        Region::Africa,
    ];
}

/// Share of callers per region, in the order of `Region::ALL`: a world-wide contest.
pub const WORLD_MIX: [f64; 6] = [30.0, 6.0, 48.0, 10.0, 3.0, 3.0];

/// The countries that are busy in a contest, by their usual prefix. They get
/// `ACTIVE_WEIGHT`: nobody expects Bouvet in a pileup.
const ACTIVE: [&str; 36] = [
    "VE", "XE", "PY", "LU", "CE", "HK", "DL", "G", "GM", "GW", "GI", "F", "EA", "I", "ON", "PA", "OK", "SP", "SM",
    "OH", "LA", "OZ", "HB", "OE", "S5", "9A", "EI", "JA", "HL", "BV", "VU", "VK", "ZL", "ZS", "5Z", "CN",
];
const ACTIVE_WEIGHT: usize = 12;

const US: &str = "K";

fn letters(rng: &mut Rng, n: usize) -> String {
    (0..n).map(|_| (b'A' + rng.below(26) as u8) as char).collect()
}

/// US calls keep their real formats.
fn us_call(rng: &mut Rng) -> String {
    let d = rng.below(10);
    let form = rng.f64();
    let (pre, n): (&[&str], usize) = if form < 0.25 {
        (&["K", "W", "N"], 3)
    } else if form < 0.65 {
        (&["KB", "KC", "KD", "KE", "KF", "KI", "KO", "WA", "WB", "N"], 3)
    } else if form < 0.9 {
        (&["K", "W", "N"], 2)
    } else {
        (&["AA", "AB", "AC", "AD", "AE", "AF", "AG", "AI", "AJ", "AK"], 2)
    };
    let pre = *rng.pick(pre);
    format!("{pre}{d}{}", letters(rng, n))
}

/// Does the prefix bring the numeral of the call along? HB9, 3D2 and CE0X do. S5 and 9A
/// do not: there the digit is part of the country, the call goes on to S51AB and 9A2XY.
fn has_numeral(prefix: &str) -> bool {
    prefix.len() >= 3 && prefix[1..].contains(|c: char| c.is_ascii_digit())
}

/// A country of the region, then one of its prefixes.
fn pick_prefix(region: Region, rng: &mut Rng) -> (&'static str, &'static Country) {
    let weight = |c: &Country| if ACTIVE.contains(&c.main) { ACTIVE_WEIGHT } else { 1 };
    let countries = || COUNTRIES[region as usize].iter().filter(|c| c.main != US);
    let mut i = rng.below(countries().map(weight).sum());
    for c in countries() {
        if i < weight(c) {
            return (rng.pick(c.prefixes), c);
        }
        i -= weight(c);
    }
    unreachable!("the index is below the sum of the weights")
}

fn call_in(region: Region, rng: &mut Rng) -> String {
    if region == Region::NorthAmerica && rng.chance(0.85) {
        return us_call(rng);
    }
    let (prefix, country) = pick_prefix(region, rng);
    let numeral = has_numeral(prefix);
    // letters behind the numeral of the prefix, as in CE0X, are part of the suffix
    let given = if numeral { prefix.bytes().rev().take_while(u8::is_ascii_alphabetic).count() } else { 0 };
    let n = (2 + rng.below(2)).saturating_sub(given).max(1);
    let suffix = letters(rng, n);
    // Canadian call areas are 1 to 7
    let digit = if country.main == "VE" && !numeral { 1 + rng.below(7) } else { rng.below(10) };
    if numeral { format!("{prefix}{suffix}") } else { format!("{prefix}{digit}{suffix}") }
}

fn pick_region(mix: &[f64; 6], rng: &mut Rng) -> Region {
    let mut r = rng.f64() * mix.iter().sum::<f64>().max(1e-9);
    for (i, m) in mix.iter().enumerate() {
        r -= m;
        if r < 0.0 {
            return Region::ALL[i];
        }
    }
    Region::ALL[0]
}

pub fn pick_call(mix: &[f64; 6], rng: &mut Rng) -> String {
    let region = pick_region(mix, rng);
    call_in(region, rng)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> impl Iterator<Item = &'static Country> {
        COUNTRIES.iter().flat_map(|r| r.iter())
    }

    #[test]
    fn calls_have_prefix_numeral_and_suffix() {
        let mut rng = Rng::new(1);
        for _ in 0..5000 {
            let c = pick_call(&WORLD_MIX, &mut rng);
            assert!(c.len() >= 3 && c.len() <= 9, "{c}");
            assert!(c.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()), "{c}");
            assert!(c[1..].bytes().any(|b| b.is_ascii_digit()), "{c}");
            assert!(c.as_bytes().last().unwrap().is_ascii_uppercase(), "{c}");
        }
    }

    #[test]
    fn prefix_table_is_clean() {
        for c in all() {
            assert!(!c.prefixes.is_empty(), "{}", c.name);
            for p in c.prefixes {
                assert!(p.bytes().all(|b| b.is_ascii_alphanumeric()), "{p}");
            }
        }
    }

    #[test]
    fn numeral_rule() {
        for p in ["HB9", "3D2", "CE0X"] {
            assert!(has_numeral(p), "{p}");
        }
        for p in ["S5", "9A", "DL", "W"] {
            assert!(!has_numeral(p), "{p}");
        }
    }

    #[test]
    fn mix_is_respected() {
        let mut rng = Rng::new(2);
        let only_eu = [0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        for _ in 0..200 {
            assert_eq!(pick_region(&only_eu, &mut rng), Region::Europe);
        }
    }

    #[test]
    fn active_countries_exist() {
        for a in ACTIVE {
            assert!(all().any(|c| c.main == a), "{a}");
        }
    }
}
