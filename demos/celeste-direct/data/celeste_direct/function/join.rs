use celeste_direct as demo;

pub fn main() -> i32 {
    minecraft::command!(r##"execute as @a[tag=!celeste_player] run gamemode adventure @s"##);
    if demo::BLOCKS {
        minecraft::command!(
            r##"execute as @a[tag=!celeste_player] run tp @s 64 142 161 180 -0.9"##
        );
    } else {
        minecraft::command!(r##"execute as @a[tag=!celeste_player] run tp @s 0 69 14 180 0.9"##);
    }
    minecraft::commands! {
        r##"execute as @a[tag=!celeste_player] run attribute @s minecraft:movement_speed base set 0"##;
        r##"execute as @a[tag=!celeste_player] run attribute @s minecraft:jump_strength base set 0"##;
        r##"execute as @a[tag=!celeste_player] run attribute @s minecraft:gravity base set 0"##;
        r##"execute as @a[tag=!celeste_player] run effect give @s minecraft:night_vision infinite 0 true"##;
        r##"execute as @a[tag=!celeste_player] run title @s times 10 160 20"##;
        r##"execute as @a[tag=!celeste_player] run title @s title {text:"CELESTE CLASSIC",color:"aqua"}"##;
        r##"execute as @a[tag=!celeste_player] run title @s subtitle {text:"WASD: direction | Space: jump | Shift: dash",color:"white"}"##;
        r##"execute as @a[tag=!celeste_player] run tellraw @s {text:"Celeste: WASD direction, Space jump, Shift dash. /trigger celeste_reset restarts; /trigger celeste_pause pauses. No resource pack required.",color:"aqua"}"##;
        r##"tag @a[tag=!celeste_player] add celeste_player"##;
    }
    0
}
