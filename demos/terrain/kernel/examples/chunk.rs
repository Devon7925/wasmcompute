use std::{env,fs,time::Instant};
fn main(){
    let a:Vec<_>=env::args().collect();
    let seed=a[1].parse::<i64>().expect("seed") as u64;
    let x=a[2].parse().expect("chunk x");let z=a[3].parse().expect("chunk z");
    let start=Instant::now();let generator=terrain_kernel::generator::Generator::new(seed);let setup=start.elapsed();
    let start=Instant::now();let chunk=if a.get(5).is_some_and(|s|s=="surface"){generator.surface_chunk(x,z)}else{generator.chunk(x,z)};let elapsed=start.elapsed();
    let bytes:Vec<_>=chunk.iter().flat_map(|b|b.0.to_le_bytes()).collect();fs::write(&a[4],bytes).expect("output");
    println!("seed={seed} chunk={x},{z} setup_ms={} chunk_ms={}",setup.as_secs_f64()*1000.,elapsed.as_secs_f64()*1000.);
}
