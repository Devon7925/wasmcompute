# Building and installing demos

Run these commands from the repository root. Build the compiler and install
Rust's Wasm target as described in the [README](../README.md). Examples below
use a POSIX shell; on PowerShell set environment variables with
`$env:NAME = 'value'` before invoking Cargo. Use absolute external-input paths.
Clang and LLVM's `llvm-ar` must support wasm32.

## Celeste Classic

Fetch the original source and artwork separately:

```sh
git clone https://github.com/SuperTails/cceleste-in-mc vendor/cceleste
git -C vendor/cceleste checkout 35488069f0a2b04ddfd2d663d31387bf1b1bdbbc
export CELESTE_SOURCE="$PWD/vendor/cceleste"
export CELESTE_FIXED=1
export CELESTE_BLOCKS=1
export CELESTE_RASTER=cached
export CLANGXX=clang++
export LLVM_AR=llvm-ar
cargo run --release -- build demos/celeste-direct -o generated/celeste-fast --copy-profile examples/copy-profiles/celeste-direct-fixed.json
```

`CELESTE_FIXED=1` selects the port's fixed-point implementation. Use `0` for
floating point. `CELESTE_BLOCKS=0` selects the slower exact-RGB display-entity
renderer. Neither uses a resource pack. The game source stays unchanged.
`--accurate` can be added to the build command.

Controls: W/A/S/D directions, Space jump, Shift dash;
`/trigger celeste_pause` and `/trigger celeste_reset`. The screen reserves
x=0..127, y=80..207, z=64, plus atlas/map-cache areas described in the
[demo guide](../demos/celeste-direct/README.md). Use `/tick rate 30`.

## Doom

Provide a legitimately obtained **shareware doom1.wad**. The build embeds it;
the repository and release source archive intentionally do not include it.

```sh
export DOOM_WAD=/absolute/path/to/doom1.wad
export CLANG=clang
export LLVM_AR=llvm-ar
cargo run --release -- build demos/doom -o generated/doom-fast
cargo run --release -- build demos/doom -o generated/doom-accurate --accurate
```

Controls: W/S move, A/D turn, Space fire, Shift use doors;
`/trigger doom_weapon set 1` through `7`, `/trigger doom_pause`,
`/trigger doom_reset`. Run `/function doom:prepare` after installation and use
`/tick rate 35`. The demo's Rust tick lifecycle joins the player at the screen.
Vanilla cannot sustain 35 FPS in the measured build.

## Terrain

The portable kernel is adapted from SteelMC (AGPL-3.0-or-later). Minecraft's
extracted data is kept outside the public source package. Prepare a matching
SteelMC checkout and its Minecraft 26.2 generated/extracted resources using
that project's build instructions at revision
`5fbba4982efced6d7da606cba9fa0f96eec8b188`. Then import the required inputs:

```sh
python tools/import_terrain_assets.py --steel /absolute/path/to/SteelMC
rustup toolchain install nightly-2026-08-21 --profile minimal --target wasm32-unknown-unknown
cargo build --release
# The demo's rust-toolchain.toml selects nightly for its guest build.
target/release/wasmcompute build demos/terrain -o generated/terrain-fast
target/release/wasmcompute build demos/terrain -o generated/terrain-accurate --accurate
```

On Windows use `target/release/wasmcompute.exe`. The importer verifies the exact
SHA-256 inputs before writing and fails on missing/mismatched data. It does not
download Minecraft, accept an EULA, or import decompiled Java code.

Use a **26.2 reference world** generated without structures, placed features or
legacy cave carvers and upgrade a copy to 26.3-rc-1. An ordinary unmodified
26.3 world is not the matching reference. The portable subset and exclusions
are described in the [terrain guide](../demos/terrain/README.md).

The demo edits only x/z=0..15, y=-64..319. `/trigger terrain_normal` generates
seed 42; `/trigger terrain_alternate` generates 1337. For another 32-bit seed,
use `/trigger terrain_seed set 123`, then `/trigger terrain_generate`.
With the accelerator, run `/function terrain:instant_mode` to enable bounded
batch scheduling. Vanilla uses smaller steps and can take hours.

## Install into your own test server

1. Create a separate Minecraft 26.3-rc-1 server/world with Java 25. Review and
   accept the [Minecraft EULA](https://aka.ms/MinecraftEULA) yourself. Start it
   normally with `java -Xmx4G -jar server.jar nogui` and stop it before copying packs.
2. For Accelerated, install Fabric Loader 0.19.5 and the built accelerator jar
   on Windows x64. See [the mod build guide](accelerator.md). For vanilla use the
   normal server. Clients need neither the accelerator nor a resource pack.
3. Before installing a large demo, run
   `/gamerule minecraft:max_command_sequence_length 268435456` in that test world.
   This is the measured setup budget, substantially above the default. Long
   vanilla calls can exceed the server watchdog: only in this isolated server,
   set `max-tick-time=-1` in `server.properties` if necessary.
4. Stop the server, copy one complete generated pack into its world's
   `datapacks` directory, restart and run `/reload`. Allow initialization to
   finish. For Doom, run `/function doom:prepare`; for terrain choose a trigger.
5. From the server console, run `op <your-player-name>` for full command access.
   Connect using your own Minecraft client. A client uses graphics normally;
   builds and headless validation do not need the GPU.

Keep the world's original command budget recorded; restore it when removing
the demo (normally 65536). Do not use the research workspace's old launchers as
public installers: they assume pre-created local worlds and client profiles.
The source release provides reproducible pack builds and manual installation;
it does not distribute a Minecraft server, game assets or an authenticated client.
