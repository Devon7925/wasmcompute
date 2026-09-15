//! Vanilla noise stage, with structures explicitly excluded.
use crate::{BlockStateId, vanilla_blocks};
use crate::density::{ColumnCache,DimensionNoises};
use crate::density_functions::overworld::OverworldNoises;
use crate::noise::{NoiseChunk,Aquifer,AquiferResult,OreVeinifier};
use crate::random::{Random,RandomSplitter,xoroshiro::Xoroshiro};

pub struct Generator {
    noises: Box<OverworldNoises>,
    splitter: RandomSplitter,
    ore: OreVeinifier,
    seed:u64,
    surface:crate::surface_system::SurfaceSystem,
}
impl Generator {
    pub fn new(seed:u64)->Self {
        let splitter=Xoroshiro::from_seed(seed).next_positional();
        let params=crate::noise_parameters::get_noise_parameters();
        let surface=crate::surface_system::SurfaceSystem::new(&splitter,&params,OverworldNoises::surface_noise_ids(),OverworldNoises::surface_gradient_ids(),vanilla_blocks::STONE.default_state(),63);
        Self{noises:Box::new(OverworldNoises::create(seed,&splitter,&params)),ore:OreVeinifier::new(&splitter),splitter,seed,surface}
    }
    /// Dense x,z,y layout, world height -64 through 319, no placed features/carvers.
    pub fn chunk(&self,chunk_x:i32,chunk_z:i32)->Vec<BlockStateId> {
        let x=chunk_x*16;let z=chunk_z*16;
        let mut grid=NoiseChunk::<OverworldNoises>::new(x,z);
        let mut cache=<OverworldNoises as DimensionNoises>::ColumnCache::default();
        cache.init_grid(x,z,&self.noises);
        let mut aquifer=Aquifer::<OverworldNoises>::new(x,z,-64,384,&self.splitter,&self.noises,cache.clone());
        let mut blocks=vec![vanilla_blocks::AIR.default_state();16*16*384];
        grid.fill(&self.noises,&mut cache,|lx,y,lz,density,interpolated,cache| {
            let wx=x+lx as i32;let wz=z+lz as i32;
            let state=match aquifer.compute_substance(&self.noises,wx,y,wz,density) {
                AquiferResult::Solid=>self.ore.compute_interpolated(&*self.noises,cache,interpolated,wx,y,wz).unwrap_or(vanilla_blocks::STONE.default_state()),
                AquiferResult::Fluid(id)=>id,
                AquiferResult::Air=>vanilla_blocks::AIR.default_state(),
            };
            blocks[(lx*16+lz)*384+(y+64) as usize]=state;
        });
        blocks
    }
    pub fn surface_chunk(&self,cx:i32,cz:i32)->Vec<BlockStateId> {
        use crate::surface::{SurfaceConditionNoiseCache,SurfaceRuleContext};
        use crate::biome_sampler::{BiomeSampler,ColumnBiome};
        use std::cell::Cell;
        let mut blocks=self.chunk(cx,cz);
        let state=self.surface_start(cx,cz,&blocks);
        self.surface_range(cx,cz,&mut blocks,&state,0..256,None);
        blocks
    }
    fn surface_start(&self,cx:i32,cz:i32,blocks:&[BlockStateId])->SurfaceState {
        let bx=cx*16;let bz=cz*16;

        let mut cache=<OverworldNoises as DimensionNoises>::ColumnCache::default();
        let corners=[(bx,bz),(bx+16,bz),(bx,bz+16),(bx+16,bz+16)].map(|(x,z)|crate::noise::preliminary_surface_level(&*self.noises,&mut cache,x,z));
        let heights:Vec<i32>=(0..256).map(|c|(0..384).rev().find(|&y|!blocks[c*384+y].is_air()).map_or(-64,|y|y as i32-63)).collect();
        SurfaceState{corners,heights}
    }
    fn surface_range(&self,cx:i32,cz:i32,blocks:&mut [BlockStateId],state:&SurfaceState,range:std::ops::Range<usize>,biome_state:Option<Box<crate::biome_sampler::BiomeState>>)->Box<crate::biome_sampler::BiomeState> {
        use crate::surface::{SurfaceConditionNoiseCache,SurfaceRuleContext};
        use crate::biome_sampler::{BiomeSampler,ColumnBiome};
        use std::cell::Cell;
        let bx=cx*16;let bz=cz*16;let corners=&state.corners;let heights=&state.heights;
        let values:Vec<_>=OverworldNoises::surface_noise_ids().iter().map(|_|Cell::new(0.)).collect();
        let initialized:Vec<_>=values.iter().map(|_|Cell::new(false)).collect();
        let conditions=SurfaceConditionNoiseCache::new(&values,&initialized);
        let mut biomes=match biome_state {Some(s)=>BiomeSampler::from_state(&self.noises,s),None=>BiomeSampler::new(&self.noises,self.seed)};
        let fluid=|b:BlockStateId|b==vanilla_blocks::WATER.default_state()||b==vanilla_blocks::LAVA.default_state();
        for column in range {let x=column/16;let z=column%16;
            let column=x*16+z;let wx=bx+x as i32;let wz=bz+z as i32;
            let snapshot=blocks[column*384..(column+1)*384].to_vec();
            let depth=self.surface.get_surface_depth(wx,wz);let secondary=self.surface.get_surface_secondary(wx,wz);
            let min_surface=steel_math::lerp2(x as f64/16.,z as f64/16.,corners[0] as f64,corners[1] as f64,corners[2] as f64,corners[3] as f64).floor() as i32+depth-8;
            let steep=heights[x*16+(z+1).min(15)]>=heights[x*16+z.saturating_sub(1)]+4||heights[x.saturating_sub(1)*16+z]>=heights[(x+1).min(15)*16+z]+4;
            let height=heights[column].min(319);let mut above=0;let mut water=i32::MIN;let mut ceiling=i32::MAX;
            conditions.reset();
            for y in (-64..=height).rev() {
                let iy=(y+64) as usize;let state=snapshot[iy];
                if state.is_air(){above=0;water=i32::MIN;continue;}
                if fluid(state){if water==i32::MIN{water=y+1;}continue;}
                if ceiling>=y {
                    ceiling=-64;
                    for la in (-64..y).rev(){let b=snapshot[(la+64) as usize];if b.is_air()||fluid(b){ceiling=la+1;break;}}
                }
                above+=1;
                if state==vanilla_blocks::STONE.default_state(){
                    let mut provider=ColumnBiome{sampler:&mut biomes,x:wx,z:wz};
                    let mut context=SurfaceRuleContext::new(wx,wz,depth,secondary,min_surface,steep,y,above,y-ceiling+1,water,None,Some(&mut provider),&self.surface,&conditions,OverworldNoises::surface_rule_block_states());
                    if let Some(b)=OverworldNoises::try_apply_surface_rule(&mut context){blocks[column*384+iy]=b;}
                }
            }
            let biome=biomes.block(wx,height,wz);
            if biome==crate::vanilla_biomes::FROZEN_OCEAN.id as u16||biome==crate::vanilla_biomes::DEEP_FROZEN_OCEAN.id as u16 {
                let mut writes=Vec::new();self.surface.collect_frozen_ocean_extension_writes(biome,wx,wz,height,min_surface,-64,&blocks[column*384..(column+1)*384],&mut writes);
                for (y,b) in writes{if y<384{blocks[column*384+y]=b;}}
            }
        }
        biomes.into_state()
    }
}

struct SurfaceState {corners:[i32;4],heights:Vec<i32>}
/// The descending surface scan owns its snapshot and memoized conditions across
/// yields. This is the same scan as surface_range, bounded by vertical slices.
struct SurfaceColumn {
    column:usize, wx:i32, wz:i32, snapshot:Vec<BlockStateId>,
    depth:i32, secondary:f64, min_surface:i32, steep:bool, height:i32,
    y:i32, above:i32, water:i32, ceiling:i32,
    values:Vec<std::cell::Cell<f64>>, initialized:Vec<std::cell::Cell<bool>>,
}
impl SurfaceColumn {
    fn new(g:&Generator,cx:i32,cz:i32,column:usize,blocks:&[BlockStateId],state:&SurfaceState)->Self {
        let x=column/16;let z=column%16;let wx=cx*16+x as i32;let wz=cz*16+z as i32;
        let depth=g.surface.get_surface_depth(wx,wz);
        let secondary=g.surface.get_surface_secondary(wx,wz);
        let c=&state.corners;let h=&state.heights;
        let min_surface=steel_math::lerp2(x as f64/16.,z as f64/16.,c[0] as f64,c[1] as f64,c[2] as f64,c[3] as f64).floor() as i32+depth-8;
        let steep=h[x*16+(z+1).min(15)]>=h[x*16+z.saturating_sub(1)]+4||h[x.saturating_sub(1)*16+z]>=h[(x+1).min(15)*16+z]+4;
        let height=h[column].min(319);
        let values:Vec<_>=OverworldNoises::surface_noise_ids().iter().map(|_|std::cell::Cell::new(0.)).collect();
        let initialized=values.iter().map(|_|std::cell::Cell::new(false)).collect();
        Self{column,wx,wz,snapshot:blocks[column*384..(column+1)*384].to_vec(),depth,secondary,min_surface,steep,height,y:height,above:0,water:i32::MIN,ceiling:i32::MAX,values,initialized}
    }
    fn slice(&mut self,g:&Generator,blocks:&mut [BlockStateId],biomes:&mut crate::biome_sampler::BiomeSampler<'_>) {
        use crate::surface::{SurfaceConditionNoiseCache,SurfaceRuleContext};
        let conditions=SurfaceConditionNoiseCache::new(&self.values,&self.initialized);
        let fluid=|b:BlockStateId|b==vanilla_blocks::WATER.default_state()||b==vanilla_blocks::LAVA.default_state();
        for _ in 0..32 {
            if self.y < -64 {break;}
            let y=self.y;self.y-=1;let iy=(y+64) as usize;let state=self.snapshot[iy];
            if state.is_air(){self.above=0;self.water=i32::MIN;continue;}
            if fluid(state){if self.water==i32::MIN{self.water=y+1;}continue;}
            if self.ceiling>=y {
                self.ceiling=-64;
                for la in (-64..y).rev(){let b=self.snapshot[(la+64) as usize];if b.is_air()||fluid(b){self.ceiling=la+1;break;}}
            }
            self.above+=1;
            if state==vanilla_blocks::STONE.default_state(){
                let mut provider=crate::biome_sampler::ColumnBiome{sampler:biomes,x:self.wx,z:self.wz};
                let mut context=SurfaceRuleContext::new(self.wx,self.wz,self.depth,self.secondary,self.min_surface,self.steep,y,self.above,y-self.ceiling+1,self.water,None,Some(&mut provider),&g.surface,&conditions,OverworldNoises::surface_rule_block_states());
                if let Some(b)=OverworldNoises::try_apply_surface_rule(&mut context){blocks[self.column*384+iy]=b;}
            }
        }
    }
    fn finish(&self,g:&Generator,blocks:&mut [BlockStateId],biomes:&mut crate::biome_sampler::BiomeSampler<'_>) {
        assert!(self.y < -64);
        let biome=biomes.block(self.wx,self.height,self.wz);
        if biome==crate::vanilla_biomes::FROZEN_OCEAN.id as u16||biome==crate::vanilla_biomes::DEEP_FROZEN_OCEAN.id as u16 {
            let mut writes=Vec::new();g.surface.collect_frozen_ocean_extension_writes(biome,self.wx,self.wz,self.height,self.min_surface,-64,&blocks[self.column*384..(self.column+1)*384],&mut writes);
            for (y,b) in writes{if y<384{blocks[self.column*384+y]=b;}}
        }
    }
}
/// Explicit state machine: setup, 25 noise columns, 64 block groups, 256 surface columns.
/// The synchronous and incremental paths share the same arithmetic kernels.
pub struct ChunkJob {
    generator:Generator,cx:i32,cz:i32,grid:NoiseChunk<OverworldNoises>,
    cache:<OverworldNoises as DimensionNoises>::ColumnCache,
    aquifer:Option<Aquifer<OverworldNoises>>,blocks:Vec<BlockStateId>,
    phase:usize,surface:Option<SurfaceState>,biomes:Option<Box<crate::biome_sampler::BiomeState>>,surface_column:Option<SurfaceColumn>,interpolation:Option<crate::noise::InterpolationColumn>,
}
impl ChunkJob {
    pub fn new(seed:u64,cx:i32,cz:i32)->Self{
        Self{generator:Generator::new(seed),cx,cz,grid:NoiseChunk::new(cx*16,cz*16),cache:Default::default(),aquifer:None,blocks:vec![vanilla_blocks::AIR.default_state();98304],phase:0,surface:None,biomes:None,surface_column:None,interpolation:None}
    }
    /// One bounded stage. Returns true only after every block is complete.
    pub fn step(&mut self)->bool{
        let x=self.cx*16;let z=self.cz*16;let g=&self.generator;
        match self.phase {
            0=>self.cache.init_grid(x,z,&g.noises),
            1=>self.aquifer=Some(Aquifer::new_sized_unprimed(x,z,16,16,-64,384,&g.splitter,self.cache.clone())),
            2..=12=>{let done=self.aquifer.as_mut().unwrap().prime_next(&g.noises);assert_eq!(done,self.phase==12);},
            13..=112=>{let part=self.phase-13;let i=part/4;let lo=(part%4)*12;let hi=if part%4==3{49}else{lo+12};self.grid.fill_partial_detail(&g.noises,&mut self.cache,i..i+1,lo..hi,0..0,0..0,|_,_,_,_,_,_|{});},
            113..=3248=>{
                let part=self.phase-113;let index=part/49;let slice=part%49;
                if slice==0 {self.interpolation=Some(self.grid.prepare_block_column(index));}
                else {
                    let part=slice-1;let z_in_cell=(part/12) as i32;let lo=(11-part%12)*32;
                    let aquifer=self.aquifer.as_mut().unwrap();let blocks=&mut self.blocks;
                    self.grid.fill_prepared(&g.noises,&mut self.cache,self.interpolation.as_ref().unwrap(),z_in_cell,lo..lo+32,|lx,y,lz,density,interpolated,cache|{
                        let wx=x+lx as i32;let wz=z+lz as i32;
                        blocks[(lx*16+lz)*384+(y+64) as usize]=match aquifer.compute_substance(&g.noises,wx,y,wz,density){
                            AquiferResult::Solid=>g.ore.compute_interpolated(&*g.noises,cache,interpolated,wx,y,wz).unwrap_or(vanilla_blocks::STONE.default_state()),
                            AquiferResult::Fluid(id)=>id,AquiferResult::Air=>vanilla_blocks::AIR.default_state(),
                        };
                    });
                }
            },
            3249=>self.surface=Some(g.surface_start(self.cx,self.cz,&self.blocks)),
            3250..=6833=>{
                let part=self.phase-3250;let column=part/14;let slice=part%14;
                if slice==0 {
                    self.surface_column=Some(SurfaceColumn::new(g,self.cx,self.cz,column,&self.blocks,self.surface.as_ref().unwrap()));
                    if self.biomes.is_none(){self.biomes=Some(crate::biome_sampler::BiomeSampler::new(&g.noises,g.seed).into_state());}
                } else {
                    let mut biomes=crate::biome_sampler::BiomeSampler::from_state(&g.noises,self.biomes.take().unwrap());
                    let surface=self.surface_column.as_mut().unwrap();
                    if slice==13 {surface.finish(g,&mut self.blocks,&mut biomes);}else{surface.slice(g,&mut self.blocks,&mut biomes);}
                    self.biomes=Some(biomes.into_state());
                    if slice==13{self.surface_column=None;}
                }
            },
            _=>return true,
        }
        self.phase+=1;self.phase>=STAGES
    }
    pub fn phase(&self)->usize{self.phase}
    pub fn into_blocks(self)->Vec<BlockStateId>{assert!(self.phase>=STAGES);self.blocks}
}
pub const STAGES:usize=6834;
