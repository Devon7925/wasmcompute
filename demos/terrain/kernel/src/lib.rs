#![feature(portable_simd)]
#![feature(const_trait_impl, const_cmp)]
extern crate self as steel_worldgen;
extern crate self as steel_utils;
extern crate self as steel_registry;
pub mod random;
pub mod noise;
pub mod density;
pub mod surface;
pub mod generator;
pub mod biome_sampler;
#[path="generated/registry.rs"] mod registry;
pub use registry::*;
#[expect(warnings, reason="upstream generated noise expressions")]
#[path="generated/vanilla_noise_parameters.rs"] pub mod noise_parameters;
#[expect(warnings, reason="upstream generated density expressions")]
#[path="generated/vanilla_density_functions/mod.rs"] pub mod density_functions;

pub mod climate;
pub mod surface_system;
#[expect(warnings, reason="upstream generated biome table")]
#[path="generated/vanilla_multi_noise.rs"] pub mod multi_noise;
