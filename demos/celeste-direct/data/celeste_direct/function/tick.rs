use celeste_direct as demo;
pub fn main() -> i32 {
    match demo::state::get().phase {
        demo::state::Phase::Unprepared => {
            demo::lifecycle::prepare::main();
            return 0;
        }
        demo::state::Phase::WaitingForChunks => return 0,
        demo::state::Phase::Ready => {}
    }
    if minecraft::command!("execute if entity @a[tag=!celeste_player]") != 0 {
        demo::lifecycle::join::main();
    }
    // Minecraft /trigger counters are external player inputs, not internal state.
    minecraft::commands! {
     "scoreboard players enable @a celeste_reset";
     "scoreboard players enable @a celeste_pause";
    }
    if minecraft::command!("execute if entity @a[scores={celeste_reset=1..}]") != 0 {
        demo::reset();
    }
    if minecraft::command!("execute if entity @a[scores={celeste_pause=1..}]") != 0 {
        demo::lifecycle::toggle::main();
    }
    minecraft::commands! {
     "scoreboard players set @a[scores={celeste_reset=1..}] celeste_reset 0";
     "scoreboard players set @a[scores={celeste_pause=1..}] celeste_pause 0";
    }
    if demo::state::get().running && minecraft::command!("execute if entity @a") != 0 {
        return demo::lifecycle::frame::main();
    }
    0
}
