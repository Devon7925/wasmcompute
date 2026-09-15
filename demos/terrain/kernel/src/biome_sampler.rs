use crate::climate::{TargetPoint,quantize_coord};
use crate::density_functions::overworld::{self,OverworldNoises,OverworldColumnCache};
use crate::surface::SurfaceBiomeProvider;
use rustc_hash::FxHashMap;
use sha2::{Digest,Sha256};

pub struct BiomeState {
    cache:OverworldColumnCache,
    values:FxHashMap<(i32,i32,i32),u16>,
    zoom:i64,
}
pub struct BiomeSampler<'a> { noises:&'a OverworldNoises, state:Box<BiomeState> }
impl<'a> std::ops::Deref for BiomeSampler<'a>{type Target=BiomeState;fn deref(&self)->&BiomeState{&self.state}}
impl<'a> std::ops::DerefMut for BiomeSampler<'a>{fn deref_mut(&mut self)->&mut BiomeState{&mut self.state}}
impl<'a> BiomeSampler<'a> {
    pub fn new(noises:&'a OverworldNoises,seed:u64)->Self {
        let hash=Sha256::digest(seed.to_le_bytes());let mut bytes=[0;8];bytes.copy_from_slice(&hash[..8]);
        Self{noises,state:Box::new(BiomeState{cache:Default::default(),values:Default::default(),zoom:i64::from_le_bytes(bytes)})}
    }
    pub fn from_state(noises:&'a OverworldNoises,state:Box<BiomeState>)->Self{Self{noises,state}}
    pub fn into_state(self)->Box<BiomeState>{self.state}
    pub fn quart(&mut self,x:i32,y:i32,z:i32)->u16 {
        // Biome palettes clamp vertical queries at the world bounds.
        let y=y.clamp(-16,79);let key=(x,y,z);
        if let Some(&id)=self.values.get(&key){return id;}
        self.state.cache.ensure(x*4,z*4,self.noises);
        let (x,y,z)=((x*4) as f64,(y*4) as f64,(z*4) as f64);
        let q=|v:f64|quantize_coord(v as f32 as f64);
        let target=TargetPoint::new(
            q(overworld::router_temperature(self.noises,&self.cache,x,y,z)),
            q(overworld::router_vegetation(self.noises,&self.cache,x,y,z)),
            q(overworld::router_continentalness(self.noises,&self.cache,x,y,z)),
            q(overworld::router_erosion(self.noises,&self.cache,x,y,z)),
            q(overworld::router_depth(self.noises,&self.cache,x,y,z)),
            q(overworld::router_ridges(self.noises,&self.cache,x,y,z)));
        let id=crate::multi_noise::get_overworld_biome(&target).id as u16;self.values.insert(key,id);id
    }
    pub fn block(&mut self,x:i32,y:i32,z:i32)->u16 {
        let a=[x-2,y-2,z-2];let parent=a.map(|v|v>>2);let frac=a.map(|v|(v&3) as f64/4.);
        let mut nearest=[0;3];let mut distance=f64::INFINITY;
        for i in 0..8 {
            let delta=[(i>>2)&1,(i>>1)&1,i&1];let p=[parent[0]+delta[0],parent[1]+delta[1],parent[2]+delta[2]];
            let mut state=self.zoom;
            for k in [0,1,2,0,1,2]{state=lcg(state,p[k] as i64);}
            let fx=fiddle(state);state=lcg(state,self.zoom);let fy=fiddle(state);state=lcg(state,self.zoom);let fz=fiddle(state);
            let dx=frac[0]-delta[0] as f64+fx;let dy=frac[1]-delta[1] as f64+fy;let dz=frac[2]-delta[2] as f64+fz;
            let d=dz*dz+dy*dy+dx*dx;
            if d<distance{distance=d;nearest=p;}
        }
        self.quart(nearest[0],nearest[1],nearest[2])
    }
}
fn lcg(v:i64,c:i64)->i64 {v.wrapping_mul(v.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)).wrapping_add(c)}
fn fiddle(v:i64)->f64{(((v>>24).rem_euclid(1024)) as f64/1024.-0.5)*0.9}
pub struct ColumnBiome<'a,'b>{pub sampler:&'a mut BiomeSampler<'b>,pub x:i32,pub z:i32}
impl SurfaceBiomeProvider for ColumnBiome<'_,'_>{fn biome_id(&mut self,y:i32)->u16{self.sampler.block(self.x,y,self.z)}}
