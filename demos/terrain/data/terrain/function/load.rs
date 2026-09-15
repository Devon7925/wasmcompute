pub fn main()->i32 {
    minecraft::commands! {
        "scoreboard objectives add terrain_normal trigger";
        "scoreboard objectives add terrain_alternate trigger";
        "scoreboard objectives add terrain_seed trigger";
        "scoreboard objectives add terrain_generate trigger";
        "forceload add 0 0 15 15";
        "tellraw @a {text:'Terrain demo: /trigger terrain_normal restores seed 42; /trigger terrain_alternate generates seed 1337. Area: x/z 0..15, y -64..319.',color:'gold'}";
    }0
}
