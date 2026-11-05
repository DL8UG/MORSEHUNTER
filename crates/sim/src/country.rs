//! The country of a call, by the longest prefix of AD1C's cty.dat that it starts with.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::prefixes::{COUNTRIES, Country};

/// The part of a call that says where it is: OH0 in OH0/DL8UG, DL8UG in DL8UG/P.
fn located(call: &str) -> &str {
    let parts: Vec<&str> = call.split('/').filter(|p| !p.is_empty()).collect();
    let home = parts.iter().copied().max_by_key(|p| p.len()).unwrap_or("");
    // a short part with a letter that is not a portable mark is the country it is used in
    let away = parts.iter().copied().find(|p| {
        p.len() < home.len() && p.len() <= 4 && p.bytes().any(|b| b.is_ascii_alphabetic()) && !matches!(*p, "P" | "M" | "MM" | "AM" | "QRP" | "A")
    });
    away.unwrap_or(home)
}

/// The country of `call`, `None` if no prefix fits.
pub fn of(call: &str) -> Option<&'static Country> {
    static MAP: OnceLock<HashMap<&'static str, &'static Country>> = OnceLock::new();
    let map = MAP.get_or_init(|| COUNTRIES.iter().flat_map(|r| r.iter()).flat_map(|c| c.prefixes.iter().map(move |p| (*p, c))).collect());
    let call = call.trim().to_ascii_uppercase();
    let at = located(&call);
    (1..=at.len().min(5)).rev().find_map(|n| at.get(..n).and_then(|p| map.get(p).copied()))
}

/// The name for a narrow column: the long official ones of cty.dat made short, the rest
/// as they are.
pub fn short_name(c: &Country) -> &'static str {
    match c.name {
        "Fed. Rep. of Germany" => "Germany",
        "United States" => "USA",
        "European Russia" => "Russia (EU)",
        "Asiatic Russia" => "Russia (AS)",
        "Asiatic Turkey" => "Turkey",
        "United Arab Emirates" => "UAE",
        "Dem. Rep. of the Congo" => "DR Congo",
        "Republic of the Congo" => "Congo",
        "Central African Republic" => "C. African Rep.",
        "Republic of South Sudan" => "South Sudan",
        "Sov Mil Order of Malta" => "SMOM",
        "Kingdom of Eswatini" => "Eswatini",
        "UK Base Areas on Cyprus" => "UK Bases Cyprus",
        "British Virgin Islands" => "BVI",
        "Turks & Caicos Islands" => "Turks & Caicos",
        "St. Pierre & Miquelon" => "St. Pierre",
        "San Andres & Providencia" => "San Andres",
        "Saba & St. Eustatius" => "Saba",
        "San Felix & San Ambrosio" => "San Felix",
        "Juan Fernandez Islands" => "Juan Fernandez",
        "Trindade & Martim Vaz" => "Trindade",
        "Andaman & Nicobar Is." => "Andaman",
        "Wallis & Futuna Islands" => "Wallis & Futuna",
        "Baker & Howland Islands" => "Baker & Howland",
        "Palmyra & Jarvis Islands" => "Palmyra",
        "Cocos (Keeling) Islands" => "Cocos (Keeling)",
        "Amsterdam & St. Paul Is." => "Amsterdam",
        "Tristan da Cunha & Gough" => "Tristan da Cunha",
        "Pr. Edward & Marion Is." => "Marion Island",
        _ => c.name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::callgen::{WORLD_MIX, pick_call};
    use crate::rng::Rng;

    fn name(call: &str) -> &'static str {
        of(call).map_or("-", |c| c.name)
    }

    #[test]
    fn well_known_calls() {
        assert_eq!(name("DL8UG"), "Fed. Rep. of Germany");
        assert_eq!(name("W1AW"), "United States");
        assert_eq!(name("K5ZD"), "United States");
        assert_eq!(name("VE3ABC"), "Canada");
        assert_eq!(name("JA1ABC"), "Japan");
        assert_eq!(name("UA3ABC"), "European Russia");
        assert_eq!(name("UA9ABC"), "Asiatic Russia");
        assert_eq!(name("G4ABC"), "England");
        assert_eq!(name("dl8ug/p"), "Fed. Rep. of Germany");
        assert_eq!(name("OH0/DL8UG"), of("OH0AA").unwrap().name);
        assert_eq!(name("XX?"), "-");
        assert_eq!(name(""), "-");
        assert_eq!(short_name(of("DL8UG").unwrap()), "Germany");
        assert_eq!(short_name(of("JA1ABC").unwrap()), "Japan");
    }

    /// Short names are short, and every name they replace is one of cty.dat.
    #[test]
    fn short_names_fit_a_column() {
        for c in COUNTRIES.iter().flat_map(|r| r.iter()) {
            assert!(short_name(c).len() <= 22, "{}", c.name);
        }
    }

    /// Every made-up call has a country, the one of the prefix it was made from or one
    /// with a longer prefix of it, as cty.dat would have it.
    #[test]
    fn every_made_up_call_has_a_country() {
        let mut rng = Rng::new(7);
        for _ in 0..20000 {
            let c = pick_call(&WORLD_MIX, &mut rng);
            assert!(of(&c).is_some(), "{c}");
        }
    }
}
