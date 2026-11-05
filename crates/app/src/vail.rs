//! The Vail adapter over MIDI: list the outputs and set its keyer type, so nobody needs
//! `amidi`. Program change 0 is passthrough (the adapter only reports the paddles, this
//! program keys), 8 is its own iambic B. The adapter keeps the type in its EEPROM.
//! See docs/MIDI_INTEGRATION_SPEC.md of github.com/Vail-CW/vail-adapter.

use midir::MidiOutput;

/// Program change values of the adapter.
pub const PASSTHROUGH: u8 = 0;
pub const IAMBIC_B: u8 = 8;

const CLIENT: &str = "morsehunter";

/// The names of all MIDI outputs, the likely adapter first.
pub fn ports() -> Result<Vec<String>, String> {
    let out = MidiOutput::new(CLIENT).map_err(|e| e.to_string())?;
    let mut names: Vec<String> = out.ports().iter().filter_map(|p| out.port_name(p).ok()).filter(|n| !n.contains("Midi Through")).collect();
    names.sort_by_key(|n| !likely(n));
    Ok(names)
}

/// What the adapter calls itself, by the boards it is built on.
pub fn likely(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["vail", "qt py", "xiao", "trinkey", "arduino micro"].iter().any(|b| n.contains(b))
}

/// Keyboard mode (dit on left Ctrl, dah on right Ctrl), then the keyer type.
pub fn set_keyer(port: &str, keyer: u8) -> Result<(), String> {
    let out = MidiOutput::new(CLIENT).map_err(|e| e.to_string())?;
    let found = out.ports().into_iter().find(|p| out.port_name(p).is_ok_and(|n| n == port)).ok_or_else(|| format!("{port}?"))?;
    let mut conn = out.connect(&found, "vail").map_err(|e| e.to_string())?;
    conn.send(&[0xB0, 0x00, 0x7F]).map_err(|e| e.to_string())?;
    conn.send(&[0xC0, keyer & 0x7F]).map_err(|e| e.to_string())?;
    conn.close();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_boards_of_the_adapter_are_found() {
        assert!(likely("QT Py M0:QT Py M0 MIDI 1 24:0"));
        assert!(likely("Seeed XIAO M0"));
        assert!(likely("TRRS Trinkey M0"));
        assert!(!likely("Scarlett 2i2 USB"));
    }

    /// Against a real receiver: `aseqdump` in one shell, then
    /// `VAIL_PORT=aseqdump cargo test -p morsehunter -- --ignored sends`.
    #[test]
    #[ignore]
    fn sends_keyboard_mode_and_the_keyer() {
        let want = std::env::var("VAIL_PORT").unwrap_or("aseqdump".into());
        let port = ports().unwrap().into_iter().find(|p| p.contains(&want)).expect("no such port");
        set_keyer(&port, PASSTHROUGH).unwrap();
        set_keyer(&port, IAMBIC_B).unwrap();
    }
}
