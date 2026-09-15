// Public Minecraft identity: arena:load.
use minecraft::command;

pub fn main() -> i32 {
    // Adding an existing objective can fail normally; the next command still runs.
    command!("scoreboard objectives add arena dummy");
    command!("scoreboard players set #ticks arena 0")
}
