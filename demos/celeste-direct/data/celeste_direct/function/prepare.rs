use celeste_direct as demo;

pub fn main() -> i32 {
    if demo::BLOCKS {
        minecraft::commands! {
            r##"forceload add 0 64 127 64"##;
            "forceload add 54 152 74 170";
            "forceload add 256 64 383 87";
            "forceload add 512 64 639 66";
            r##"setworldspawn 64 142 161 180 -0.9"##;
        }
    } else {
        minecraft::commands! {
            r##"forceload add -16 -16 16 32"##;
            r##"setworldspawn 0 69 14 180 0.9"##;
        }
    }
    minecraft::commands! {
        r##"gamerule minecraft:advance_time false"##;
        r##"gamerule minecraft:advance_weather false"##;
        r##"gamerule minecraft:spawn_mobs false"##;
        r##"gamerule minecraft:send_command_feedback false"##;
        r##"gamerule minecraft:command_block_output false"##;
        r##"gamerule minecraft:fall_damage false"##;
        r##"gamerule minecraft:max_command_sequence_length 100000000"##;
        r##"gamerule minecraft:max_command_forks 1000000"##;
        r##"time set noon"##;
        r##"weather clear"##;
    }
    demo::state::update(|s| s.phase = demo::state::Phase::WaitingForChunks);
    if demo::IS_DIRECT {
        minecraft::command!(r##"schedule function celeste_direct:ready 2t replace"##);
    } else {
        minecraft::command!(r##"schedule function celeste:ready 2t replace"##);
    }
    0
}
