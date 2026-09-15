//! Rust-owned demo state. The server executes this Wasm instance serially.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Unprepared,
    WaitingForChunks,
    Ready,
}
#[derive(Clone, Copy)]
pub struct State {
    pub phase: Phase,
    pub running: bool,
    pub frames: u32,
    pub input: u32,
    pub screen_ready: bool,
}
const INITIAL: State = State {
    phase: Phase::Unprepared,
    running: false,
    frames: 0,
    input: 0,
    screen_ready: false,
};
static mut STATE: State = INITIAL;
pub fn get() -> State {
    unsafe { STATE }
}
pub fn update(f: impl FnOnce(&mut State)) {
    unsafe {
        let mut next = STATE;
        f(&mut next);
        STATE = next;
    }
}
pub fn clear() {
    unsafe {
        STATE = INITIAL;
    }
}
