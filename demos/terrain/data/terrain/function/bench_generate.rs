pub fn main()->i32{
    let seed=minecraft::command!("scoreboard players get #input demo_bench");
    terrain_demo::generate(seed as i64,0,0)
}
