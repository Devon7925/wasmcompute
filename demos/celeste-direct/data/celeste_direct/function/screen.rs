use celeste_direct as demo;

pub fn main() -> i32 {
    minecraft::commands! {
        "kill @e[tag=celeste_pixel]";
        "kill @e[tag=celeste_direct_pixel]";
    }
    if demo::BLOCKS {
        if demo::ATLAS {
            demo::prepare_atlas();
        }
        minecraft::command!("fill 0 80 64 127 207 64 minecraft:black_concrete strict");
    } else {
        for y in 0..128i32 {
            for x in 0..128i32 {
                let px = -636 + 10 * x;
                let py = 640 + 127 - y;
                let mut decimal = 10000001 + y * 128 + x;
                let mut uuid = 0;
                let mut place = 1;
                while decimal != 0 {
                    uuid += (decimal % 10) * place;
                    decimal /= 10;
                    place *= 16;
                }
                minecraft::command!(
                    r##"summon minecraft:text_display $(px) $(py) 0 {UUID:[I;1128616004,1230128451,0,$(uuid)],Tags:["celeste_direct_pixel"],text:{text:" "},billboard:"fixed",brightness:{block:15,sky:15},shadow:false,default_background:false,line_width:100,view_range:2f,interpolation_duration:0,transformation:{translation:[0f,0f,0f],scale:[0.8f,0.4f,1f],left_rotation:[0f,0f,0f,1f],right_rotation:[0f,0f,0f,1f]},background:-16777216}"##,
                    px: f32 = px as f32 / 100.0,
                    py: f32 = py as f32 / 10.0,
                    uuid = uuid
                );
            }
        }
    }
    demo::state::update(|s| s.screen_ready = true);
    0
}
