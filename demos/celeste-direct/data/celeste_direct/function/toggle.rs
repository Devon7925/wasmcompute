use celeste_direct as demo;
pub fn main() -> i32 {
    demo::state::update(|s| s.running = !s.running);
    0
}
