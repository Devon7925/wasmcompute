# Finite-area terrain regeneration

This Rust source pack regenerates the chunk at X/Z `0..15`, Y `-64..319`, from a
seed. It uses the actual noise, aquifer, ore-vein and surface algorithms from
SteelMC's Minecraft 26.2 implementation. Generation uses absolute world
coordinates; the result is computed from the requested seed, not selected from
saved terrain images.

## Reference scope

The reference is **Minecraft 26.2 terrain**, run in an upgraded 26.3-rc-1 world so
the generated datapack can use snapshot command features. Minecraft 26.3 changed
terrain generation substantially; this demo does not claim to reproduce 26.3's
new algorithm.

Structures, placed features and legacy cave carvers are excluded from both the
controlled reference world and the demo comparison. Noise caves, aquifers, ore
veins and surface rules remain. Eroded-badlands pillar extensions are currently
omitted; frozen-ocean icebergs are implemented. These are explicit scope limits,
not a promise of exact generation for every biome and seed.

The checked four chunks matched all 393,216 block states against the original
Minecraft 26.2 noise/surface implementation. A normal generated reference world
also matched its entire 98,304-block target chunk. Accelerated same-seed,
alternate-seed and restore operations preserved all eight neighboring chunks.
Replacement writes block states; biome metadata stays with the underlying
normal world, including after an alternate-seed replacement.
Fast matched seed 42 exactly; seed 1337 differed in 45 of 98,304 blocks in the
measured build. These were surface decisions involving air, red sand and
terracotta. See the [four-backend measurements](../../docs/benchmarks.md).

## Build

First import the external Minecraft inputs using the [demo guide](../../docs/demos.md#terrain).

This demo uses Rust's standard library for allocation and portable SIMD in the
source kernel. SIMD is scalarized for the current Wasm target. The pinned
toolchain used here is `nightly-2026-08-21`.
The included `rust-toolchain.toml` selects it when the source-pack builder runs
Cargo in the demo directory.

```powershell
rustup target add wasm32-unknown-unknown --toolchain nightly-2026-08-21
$env:RUSTUP_TOOLCHAIN = 'nightly-2026-08-21'
cargo run --release -- build demos/terrain -o generated/terrain-fast
cargo run --release -- build demos/terrain -o generated/terrain-accurate --accurate
```

The source pack opts into portable Rust std with
`[package.metadata.wasmcompute] std = true`. No WASI, OS randomness, network,
thread pool or native world-generation service runs inside the guest.

## Controls and execution

- `/trigger terrain_normal`: regenerate seed 42.
- `/trigger terrain_alternate`: regenerate seed 1337.
- `/trigger terrain_seed set <signed integer>`, then `/trigger terrain_generate`:
  regenerate a custom seed. The kernel itself accepts 64-bit seeds; this command
  trigger exposes Minecraft's 32-bit score range.

The Rust job state holds seed initialization, 25 noise columns, 64 block groups
and 256 surface columns. Aquifer setup scans one quart row per step; each noise
column is split into four vertical ranges, and interpolation buffers are cached once per group of four Z columns. Block evaluation resumes in 32-block vertical ranges. Surface columns preserve their scan state and condition cache across 32-block vertical slices. The job has 6834 stages. The tick
function advances the job and reports progress.
The final block writes are published only after computation completes. This
keeps intermediate terrain out of the world, but individual vanilla steps can
still take seconds. The demo needs an increased command budget in its isolated
world; vanilla generation is not currently real time.

For accelerated execution, run `/function terrain:instant_mode`. It schedules a function
tag for the following tick, whose distinct Rust entry points each process a
bounded batch of stages. Minecraft deduplicates tag entries, so these names must
be distinct. This scheduling policy preserves the same computation and avoids
exceeding the accelerator's per-export fuel allowance. All authored functions,
including these small scheduler entries, are Rust.

See [portable installation instructions](../../docs/demos.md) for a fresh server.

The seed-independent climate search tree is built by Cargo from the real biome
parameters. Its node and tie order come from the same reference construction
code used at runtime before this optimization. Noise tables and terrain still
depend on the requested seed. Incremental and synchronous generation share the
same arithmetic kernels and pass full-chunk equality checks.

## Source and license

`kernel/` contains the portable subset of SteelMC and steel-math with their
original license notices (AGPL-3.0). Registry and worldgen inputs are extracted
from Minecraft 26.2. Build scripts regenerate the derived Rust tables. Do not
edit generated files manually.
