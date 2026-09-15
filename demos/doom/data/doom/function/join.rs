pub fn main()->i32 {
 minecraft::commands! {"execute as @a[tag=!doom_player] run gamemode adventure @s";"execute as @a[tag=!doom_player] run tp @s 64 120 161 180 0";"execute as @a[tag=!doom_player] run attribute @s minecraft:movement_speed base set 0";"execute as @a[tag=!doom_player] run attribute @s minecraft:jump_strength base set 0";"execute as @a[tag=!doom_player] run attribute @s minecraft:gravity base set 0";"execute as @a[tag=!doom_player] run effect give @s minecraft:night_vision infinite 0 true";
 r##"tellraw @a[tag=!doom_player] {text:"DOOM: W/S move, A/D turn, Space fires, Shift opens doors. /trigger doom_weapon set 1..7, /trigger doom_reset, /trigger doom_pause.",color:"red"}"##;
 "tag @a[tag=!doom_player] add doom_player";}
 0
}
