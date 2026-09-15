/// One deterministic update/draw for headless measurements and replay.
pub fn main() -> i32 {
    let buttons = minecraft::command!("scoreboard players get #input demo_bench");
    celeste_direct::state::update(|state| state.input = buttons as u32);
    celeste_direct::step()
}
