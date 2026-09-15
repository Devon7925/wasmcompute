use terrain_kernel::generator::{ChunkJob,Generator};
fn main(){
    for (seed,cx,cz) in [(42u64,0,0),(1337,0,0),(u64::MAX,-2,3),(12345,7,-11)]{
        let expected=Generator::new(seed).surface_chunk(cx,cz);
        let mut job=ChunkJob::new(seed,cx,cz);let mut phases=0;
        loop {phases+=1;if job.step(){break;}}
        let actual=job.into_blocks();assert_eq!(actual,expected);
        if let Some(directory)=std::env::args().nth(1){
            let path=std::path::Path::new(&directory).join(format!("{}_{}_{}.vanilla.bin",seed as i64,cx,cz));
            let reference=std::fs::read(path).expect("independent Minecraft reference");
            let bytes:Vec<_>=actual.iter().flat_map(|b|b.0.to_le_bytes()).collect();
            assert_eq!(bytes,reference,"static climate index must retain vanilla terrain");
        }
        println!("seed={seed} chunk={cx},{cz} blocks={} phases={phases} exact=true",actual.len());
    }
}
