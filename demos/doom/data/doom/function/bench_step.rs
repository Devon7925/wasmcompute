pub fn main() -> i32 {
    let input = minecraft::command!("scoreboard players get #input demo_bench");
    doom_demo::doom_set_input(input as u32);
    doom_demo::step()
}
