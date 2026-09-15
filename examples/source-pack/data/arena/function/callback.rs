use minecraft::command;
pub fn main() -> i32 {
    let before = arena_helpers::add_counter(10);
    let inner = command!("function arena:counter");
    before * 1000 + inner * 10 + arena_helpers::add_counter(0)
}
