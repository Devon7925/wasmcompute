use arena_helpers::{export_command, Rules};
use minecraft::command;

pub fn main() -> i32 {
    let height = command!("data get storage arena:config rules.height");
    if !(1..=16).contains(&height) { return -1; }
    let mut out = [0u8; 256];
    let Some(n) = export_command(&Rules { height }, &mut out) else { return -2; };
    command(core::str::from_utf8(&out[..n]).unwrap())
}
