#![no_std]

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Rules {
    pub height: i32,
}

/// Serialize with the existing Serde ecosystem into a bounded command buffer.
/// No allocator is required. The second serialization quotes the JSON as an
/// NBT string; it preserves JSON quotes/backslashes through command execution.
pub fn export_command(rules: &Rules, out: &mut [u8]) -> Option<usize> {
    let mut json = [0u8; 128];
    let n = serde_json_core::to_slice(rules, &mut json).ok()?;
    let text = core::str::from_utf8(&json[..n]).ok()?;
    let prefix = b"data modify storage arena:config share set value ";
    out.get_mut(..prefix.len())?.copy_from_slice(prefix);
    let len = serde_json_core::to_slice(text, out.get_mut(prefix.len()..)?).ok()?;
    Some(prefix.len() + len)
}

/// Example validation: consume the whole JSON document before replacing state.
pub fn parse_rules(bytes: &[u8]) -> Option<Rules> {
    let (rules, used): (Rules, usize) = serde_json_core::from_slice(bytes).ok()?;
    if used != bytes.len() || !(1..=16).contains(&rules.height) { return None; }
    Some(rules)
}

// Shared guest state illustrates persistence across separate Rust entries and
// nested command callbacks. No references are held across a host command.
static mut COUNTER: i32 = 0;
pub fn add_counter(n: i32) -> i32 {
    unsafe { COUNTER = COUNTER.wrapping_add(n); COUNTER }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn serde_round_trip_and_validation() {
        let rules = parse_rules(br#"{"height":7}"#).unwrap();
        assert_eq!(rules.height,7);
        assert!(parse_rules(br#"{"height":999}"#).is_none());
        assert!(parse_rules(br#"{"height":7}garbage"#).is_none());
        let mut out = [0u8;256];
        let n = export_command(&rules,&mut out).unwrap();
        assert_eq!(core::str::from_utf8(&out[..n]).unwrap(),r#"data modify storage arena:config share set value "{\"height\":7}""#);
        assert!(export_command(&rules,&mut [0;10]).is_none());
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self { height: 3 }
    }
}
