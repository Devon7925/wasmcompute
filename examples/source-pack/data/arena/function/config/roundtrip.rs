use arena_helpers::{export_command, parse_rules};
use minecraft::command;

pub fn main() -> i32 {
    // Bounded Serde deserialization, validation, then serialization. Reading an
    // arbitrary incoming storage string into Rust is a later SDK extension.
    let Some(rules) = parse_rules(br#"{"height":7}"#) else { return -1; };
    let mut out = [0u8; 256];
    let Some(n) = export_command(&rules, &mut out) else { return -2; };
    command(core::str::from_utf8(&out[..n]).unwrap())
}
