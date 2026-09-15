# Doom source pack

The original Linux Doom engine runs inside Wasm. Rust owns the datapack lifecycle,
movement input, palette quantization and the 128 by 80 block screen. All authored
Minecraft functions are Rust files in `data/doom/function`. No resource pack is used.

## Build

From the wasmcompute directory, with Rust's `wasm32-unknown-unknown` target and
Clang/LLVM installed:

```powershell
$env:DOOM_WAD = 'C:/path/to/doom1.wad'
$env:CLANG = 'C:/Program Files/LLVM/bin/clang.exe'
$env:LLVM_AR = 'C:/Program Files/LLVM/bin/llvm-ar.exe'
cargo run --release -- build demos/doom -o generated/doom-fast
cargo run --release -- build demos/doom -o generated/doom-accurate --accurate
```

The build embeds the supplied shareware IWAD. The source distribution does not
include the IWAD. The Fabric accelerator can execute either generated pack's
retained original Wasm with accurate semantics.

## Play

The viewing platform is at `(64,120,161)`, facing the screen at Z=64. Player
movement provides game input: W/S move, A/D turn, Space fires, Shift uses doors.
Use `/trigger doom_weapon set 1` through `7` to select a weapon,
`/trigger doom_pause` to pause and `/trigger doom_reset` to restart E1M1.
Grant your own player operator access from the server console.

See [portable installation instructions](../../docs/demos.md) for a fresh server.

Vanilla initialization and gameplay require a larger command budget than the
server default. The demo source does not silently change that server rule.
Set the budget as shown in the installation guide before initialization.

## Provenance and checks

`engine/` comes from the `theMagicalKarp/wasmdoom` freestanding Linux Doom port.
`ENGINE-LICENSE` retains its GPL notice. `build.rs` adds a zero-copy binding for
the embedded IWAD. `FixedDiv2` uses widened integer multiplication/division;
180 input-driven frames matched the original port's complete 64,000-byte
framebuffer and player snapshot after this change.

The measured screens and deterministic inputs are described in the
[current benchmark guide](../../docs/benchmarks.md).

Weapon requests remain held for one engine tick before release so that Doom
can consume the request while constructing its next input command.
