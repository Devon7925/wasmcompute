//! Quantize the original framebuffer and issue horizontal block spans.
static mut LUT:[u8;256]=[0;256];
static mut LAST_PALETTE:[u8;768]=[0;768];
static mut PALETTE_READY:bool=false;
const COLORS:[u32;16]=[592656,2895503,11087776,4807460,6306848,3553597,8224115,13620694,9314336,14704896,15773461,6269209,2329030,6561692,13985166,13742753];
pub unsafe fn draw(pixels:*const u8,palette:*const u8)->i32 {
 let mut changed=!PALETTE_READY;
 for i in 0..768 {if LAST_PALETTE[i]!=*palette.add(i){changed=true;break;}}
 if changed {
  for i in 0..256 {
   let r=*palette.add(i*3) as i32;let g=*palette.add(i*3+1) as i32;let b=*palette.add(i*3+2) as i32;
   let mut best=i32::MAX;let mut index=0;
   for (j,&rgb) in COLORS.iter().enumerate(){let dr=r-(rgb>>16) as i32;let dg=g-((rgb>>8)&255) as i32;let db=b-(rgb&255) as i32;let d=dr*dr+dg*dg+db*db;if d<best{best=d;index=j as u8;}}
   LUT[i]=index;
  }
  core::ptr::copy_nonoverlapping(palette,core::ptr::addr_of_mut!(LAST_PALETTE).cast::<u8>(),768);PALETTE_READY=true;
 }
 let mut commands=0;
 for y in 0..80 {
  let sy=y*200/80;let mut x=0;
  while x<128 {
   let start=x;let color=LUT[*pixels.add(sy*320+x*320/128) as usize];x+=1;
   while x<128 && LUT[*pixels.add(sy*320+x*320/128) as usize]==color{x+=1;}
   span(start as i32,x as i32-1,159-y as i32,color);commands+=1;
  }
 }
 commands
}
fn span(x:i32,end:i32,y:i32,color:u8){match color {
0=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:black_concrete strict",x=x,y=y,end=end);},
1=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:blue_concrete strict",x=x,y=y,end=end);},
2=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:magenta_concrete strict",x=x,y=y,end=end);},
3=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:green_concrete strict",x=x,y=y,end=end);},
4=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:brown_concrete strict",x=x,y=y,end=end);},
5=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:gray_concrete strict",x=x,y=y,end=end);},
6=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:light_gray_concrete strict",x=x,y=y,end=end);},
7=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:white_concrete strict",x=x,y=y,end=end);},
8=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:red_concrete strict",x=x,y=y,end=end);},
9=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:orange_concrete strict",x=x,y=y,end=end);},
10=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:yellow_concrete strict",x=x,y=y,end=end);},
11=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:lime_concrete strict",x=x,y=y,end=end);},
12=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:light_blue_concrete strict",x=x,y=y,end=end);},
13=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:purple_concrete strict",x=x,y=y,end=end);},
14=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:pink_concrete strict",x=x,y=y,end=end);},
15=>{minecraft::command!("fill $(x) $(y) 64 $(end) $(y) 64 minecraft:white_terracotta strict",x=x,y=y,end=end);},
 _=>{}
}}
