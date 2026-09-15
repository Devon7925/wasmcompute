use terrain_kernel::{BlockStateId,generator::{Generator,ChunkJob}};
pub use terrain_kernel::generator::STAGES;
mod blocks {include!(concat!(env!("OUT_DIR"),"/blocks.rs"));}
const NORMAL_SEED:i64=42;
const ALTERNATE_SEED:i64=1337;
static mut LAST:Option<Vec<BlockStateId>>=None;
static mut JOB:Option<ChunkJob>=None;
static mut ORIGIN:(i32,i32)=(0,0);
static mut INSTANT:bool=false;
/// Regenerate exactly one chunk. All coordinates are absolute, including seed noise.
pub fn generate(seed:i64,cx:i32,cz:i32)->i32 {
    let terrain=Generator::new(seed as u64).surface_chunk(cx,cz);
    write_chunk(terrain,cx,cz)
}
fn write_chunk(terrain:Vec<BlockStateId>,cx:i32,cz:i32)->i32 {
    let mut commands=0;
    for x in 0..16 {for z in 0..16 {
        let column=&terrain[(x*16+z)*384..(x*16+z+1)*384];let mut lo=0;
        while lo<384 {
            let mut hi=lo+1;while hi<384 && column[hi]==column[lo]{hi+=1;}
            blocks::fill(column[lo].0,cx*16+x as i32,cz*16+z as i32,lo as i32-64,hi as i32-65);commands+=1;lo=hi;
        }
    }}
    // One immutable result retained for parity checks outside the timed operation.
    unsafe{LAST=Some(terrain);}commands
}
pub fn checksum()->i32 {
    let mut hash=2166136261u32;
    // The single-threaded guest never aliases this state with a host callback.
    unsafe{if let Some(blocks)=&*core::ptr::addr_of!(LAST){for block in blocks {for b in block.0.to_le_bytes(){hash=(hash^b as u32).wrapping_mul(16777619);}}}}
    hash as i32
}
pub fn normal()->i32{generate(NORMAL_SEED,0,0)}
pub fn alternate()->i32{generate(ALTERNATE_SEED,0,0)}
/// Start an incremental job. Public command calls are single threaded.
pub fn begin(seed:i64,cx:i32,cz:i32)->i32 {
    unsafe{JOB=Some(ChunkJob::new(seed as u64,cx,cz));ORIGIN=(cx,cz);}
    if instant(){minecraft::command!("schedule function #terrain:finish 1t replace");}0
}
/// Positive values are completed compute stages; a negative result is the final
/// number of block-write commands. No partially generated chunk is published.
pub fn step()->i32 {
    unsafe{
        let slot=&mut *core::ptr::addr_of_mut!(JOB);
        let Some(job)=slot.as_mut() else{return 0;};
        if job.step(){let blocks=slot.take().unwrap().into_blocks();-write_chunk(blocks,ORIGIN.0,ORIGIN.1)}
        else{job.phase() as i32}
    }
}
pub fn busy()->bool{unsafe{(&*core::ptr::addr_of!(JOB)).is_some()}}
/// Scheduling policy only: both policies execute the same bounded guest stages.
pub fn set_instant(){unsafe{INSTANT=true;}}
pub fn instant()->bool{unsafe{INSTANT}}
/// One scheduler entry stays below the accelerator's per-export fuel limit.
pub fn finish_batch()->i32 {
    for _ in 0..32 {
        let result=step();
        if result<0 {minecraft::command!("tellraw @a {text:'Terrain replacement complete.',color:'green'}");return result;}
        if !busy(){break;}
    }
    0
}
