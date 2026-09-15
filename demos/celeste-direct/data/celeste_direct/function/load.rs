use celeste_direct as demo;
pub fn main() -> i32 {
    demo::state::clear();
    minecraft::commands! {
     "scoreboard objectives add celeste_reset trigger";
     "scoreboard objectives add celeste_pause trigger";
     "tag @a remove celeste_player";
    }
    0
}
