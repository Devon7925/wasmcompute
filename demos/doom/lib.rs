#![no_std]
extern crate self as doom_demo;
pub mod lifecycle;
mod renderer;
static WAD:&[u8]=include_bytes!(concat!(env!("OUT_DIR"),"/doom.wad"));
extern "C" {
 fn doom_bind_wad(data:*const u8,len:i32);
 fn wasmdoom_argv_ptr()->*mut u8;
 fn wasmdoom_init();fn wasmdoom_tick();
 fn wasmdoom_keydown(key:i32);fn wasmdoom_keyup(key:i32);
 fn wasmdoom_get_framebuffer()->*const u8;fn wasmdoom_get_palette()->*const u8;
 fn wasmdoom_events_clear();
 fn G_DeferedInitNew(skill:i32,episode:i32,map:i32);
}
#[derive(Clone,Copy)]
pub struct State {pub prepared:bool,pub ready:bool,pub running:bool,pub initialized:bool,pub buttons:u32,pub previous:u32,pub frames:u32,pub present:bool,pub weapon_key:i32}
const INITIAL:State=State{prepared:false,ready:false,running:true,initialized:false,buttons:0,previous:0,frames:0,present:false,weapon_key:0};
static mut STATE:State=INITIAL;
pub fn state()->State {unsafe{STATE}}
pub fn update(f:impl FnOnce(&mut State)){unsafe{let mut next=STATE;f(&mut next);STATE=next;}}
pub fn clear(){unsafe{STATE=INITIAL;}}
pub fn reset()->i32 {
 if !state().initialized {
  unsafe {
   doom_bind_wad(WAD.as_ptr(),WAD.len() as i32);
   let args=b"-iwad\0doom1.wad\0-warp\x001\x001\0-skill\x003\0\0";
   core::ptr::copy_nonoverlapping(args.as_ptr(),wasmdoom_argv_ptr(),args.len());
   wasmdoom_init();
  }
  update(|s|s.initialized=true);
 } else {unsafe{G_DeferedInitNew(2,1,1);}}
 update(|s|{s.buttons=0;s.previous=0;s.frames=0;s.running=true;});step()
}
#[no_mangle]
pub extern "C" fn doom_set_input(buttons:u32){update(|s|s.buttons=buttons);}
pub fn step()->i32 {
 let s=state();
 for (bit,key) in [(0,0xac),(1,0xae),(2,0xad),(3,0xaf),(4,0x9d),(5,32)] {
  if (s.buttons^s.previous)&(1<<bit)!=0 {unsafe{if s.buttons&(1<<bit)!=0 {wasmdoom_keydown(key)} else {wasmdoom_keyup(key)}}}
 }
 unsafe {wasmdoom_events_clear();wasmdoom_tick();}
 // A weapon key must survive one engine tick before its release is queued.
 if s.weapon_key!=0 {unsafe{wasmdoom_keyup(s.weapon_key);}update(|s|s.weapon_key=0);}
 update(|s|{s.previous=s.buttons;s.frames=s.frames.wrapping_add(1);});
 unsafe{renderer::draw(wasmdoom_get_framebuffer(),wasmdoom_get_palette())}
}
pub fn weapon(number:i32){
 if (1..=7).contains(&number){
  let old=state().weapon_key;
  unsafe{if old!=0{wasmdoom_keyup(old);}wasmdoom_keydown(48+number);}
  update(|s|s.weapon_key=48+number);
 }
}
#[no_mangle]
pub extern "C" fn doom_frame_hash()->u32 {
 let mut h=2166136261u32;unsafe{let p=wasmdoom_get_framebuffer();for i in 0..64000{h=(h^*p.add(i) as u32).wrapping_mul(16777619);}}h
}
