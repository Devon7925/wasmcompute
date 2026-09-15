pub fn main()->i32 {
 minecraft::commands! {"forceload add 0 64 127 64";"forceload add 54 152 74 170";"setworldspawn 64 120 161 180 0";"gamerule minecraft:advance_time false";"gamerule minecraft:advance_weather false";"gamerule minecraft:spawn_mobs false";"gamerule minecraft:send_command_feedback false";"gamerule minecraft:command_block_output false";"gamerule minecraft:fall_damage false";"time set noon";"weather clear";"schedule function doom:ready 2t replace";}
 doom_demo::update(|s|s.prepared=true);0
}

