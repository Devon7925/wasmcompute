# Celeste Classic with batched pixel rendering

This pack links the pinned, unchanged `celeste.c` game from
`SuperTails/cceleste-in-mc` at `35488069f0a2b04ddfd2d663d31387bf1b1bdbbc`.
The renderer starts with `mcmain.c` and `mc_sdl_compat.h`, retains a reference
path, and replaces selected drawing algorithms with batched output. No game
logic or source expressions are changed to influence LLVM optimization.

The default block renderer uses a 128 by 128 block screen with a one-to-one
palette mapping. Solid rectangles become `fill ... strict`; sprites and font
glyphs become transparent `clone ... strict masked force` copies from immutable
textures. Three static tile layers are cached per room, with empty layers skipped
and copy bounds clipped. Other drawing operations retain the reference path.
Both the block screen and optional exact-RGB display entities need no resource
pack. Block materials approximate the RGB colors; palette 15 uses white
terracotta for the original peach skin color.

Palette changes now affect both solid hair primitives and sprite texels. The
atlas contains red, blue, white and green hair variants, including flipped
sprites. Other palette changes use the reference blitter. The original Celeste
game code is unchanged.

There is no destination framebuffer, previous-frame buffer, or matching-pixel
merger. Overdraw remains, but native Minecraft operations process it in bulk:
the 600-frame replay falls from about 31,977 point commands to 62 draw commands
per frame. Texture arrays remain in Wasm for setup and fallback rendering. The
shim omits sleep, diagnostics and audio; each step runs one game update and draw.
Reset uses seed `0x1234` and enters the game directly. Tests preserve upstream
raster quirks, including clipped flips and the masked font's top-left pixel.

## Build

See [portable build and installation commands](../../docs/demos.md#celeste-classic).

`CELESTE_RASTER` accepts `reference`, `rects`, `atlas`, and `cached` (default).
These progressively enable solid fills, sprite textures, and static map caching.
Display entities fall back to point rendering. `CELESTE_RASTER_TEST=1` adds a
test-only primitive export; leave it unset or `0` for normal builds.

The optional copy profile specializes observed game object copies in mcfunction
with exact source/destination/length guards and a general fallback. It does not
change the Wasm payload. It was collected for the included fixed block build;
source changes can make it miss and lose speed without weakening copy semantics.
See the build guide for the optional copy-profile argument.

## Play

See the [installation and controls guide](../../docs/demos.md). Grant your own
player operator access from the server console. Install only in a separate demo
world: setup changes world rules and player movement attributes.

See [current benchmarks](../../docs/benchmarks.md) for the corrected-palette
build and comparison with the published wasmcraft2 demo.

## Rust datapack implementation

Every authored function in `data/celeste_direct/function/` is Rust, including
world setup, screen creation, player joining, input, pause and tick scheduling.
There are no authored mcfunctions or separate movement predicate files. The
function files contain the implementation; `lifecycle.rs` lets them call one
another as Rust modules. The game remains C, while Rust implements batching and
the C platform shim retains the reference rasterizer.

`state.rs` owns readiness, running state, frame count, screen initialization and
the input bitmask. `frame.rs` builds the bitmask using inline player predicates.
Only Minecraft's native `/trigger` inputs require authored scoreboards. State
resets on datapack reload. The exported `celeste_set_input` hook accepts a button
bitmask for deterministic headless replays; normal play reads player movement.

Setup uses `minecraft::commands! { "..."; "...", x = value; }` for command
sequences. Rust branches, loops and variables handle control flow. Chunk loading
is checked before building the viewing platform and screen.

The block viewing platform is near 64, 142, 161. The demo reserves and forceloads
the screen at x=0..127, y=80..207, z=64; texture layers at x=256..383,
y=80..143, z=64..87; and map caches at x=512..639, y=80..207, z=64..66.
These are immutable rendering resources rebuilt on load or room-key changes,
not a previous-frame buffer. Setup waits for every required chunk. Install in
the isolated demo world because these areas are overwritten.
Use a ten-chunk view distance so the full screen is visible. Validation used only a headless
CPU server and CPU Wasmtime. Launching the client later uses normal graphics.
