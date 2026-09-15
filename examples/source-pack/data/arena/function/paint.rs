use arena_helpers::Rules;
use minecraft::{command, fill};

pub fn main() -> i32 {
    let rules = Rules::default();
    command!("scoreboard players set #ticks arena 0");
    fill!(
        "~ ~ ~ ~7 ~$(height) ~7 minecraft:stone",
        height = rules.height
    )
}
