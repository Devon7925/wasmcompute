# Accurate vanilla compilation

Use `--accurate` with either frontend:

```powershell
wasmcompute program.wasm -o generated/accurate --namespace example --accurate
wasmcompute build my-source-pack -o generated/accurate --offline --accurate
```

Fast remains the default. Optimization level (`-O0`/`-O1` for standalone Wasm)
is independent of numeric semantics. Source packs retain the original Wasm
payload and use a different private namespace for accurate compilation.

Accurate mode implements the supported scalar Wasm instruction set using exact
integer representations. It does not expand support to SIMD, threads, memory64,
exceptions, dynamic tables, WASI, or other previously unsupported proposals.
This is a tested implementation, not a claim of WebAssembly certification.

## Numeric behavior

`f32` uses one score containing its 32 raw bits. `f64` uses two scores containing
64 raw bits. Arithmetic, square root, rounding, comparisons, conversions and
demotion/promotion call embedded Berkeley SoftFloat routines, which are themselves
compiled into ordinary mcfunctions. No mod, native execution, GPU or runtime
library download is required. Float loads/stores preserve bits, including NaN
payloads. Reinterpretation is a register move, and integer memory is always the
canonical representation.

Arithmetic uses nearest-even rounding, gradual underflow, infinities and signed
zero. Integral rounding instructions use their specified rounding directions.
Non-saturating out-of-range integer conversions trap; saturating conversions clamp
and convert NaN to zero. Arithmetic NaNs follow Wasm's permitted NaN outcomes;
an accelerator may produce a different permitted NaN payload. Bit identity of
all arithmetic NaNs is not part of the contract.

Signed/unsigned integer division by zero traps. Signed division overflow traps;
the corresponding remainder returns zero. Valid integer operations wrap at the
specified width. Indirect calls retain their original Wasm type identity even
though float and integer values use the same physical registers.

## Memory and traps

Guest loads, stores, memory.copy and memory.fill check the unsigned effective
range against the current memory size before touching memory. Bulk operations
check both ranges before writing; a zero-length range may point exactly to the
end of memory. Recognized byte-copy loops only use the bulk optimization when
the complete ranges are valid; otherwise the original loop preserves its
per-byte writes and trap order.

Guest memory is capped at 32,767 pages (2 GiB minus 64 KiB). The upper reserved
region holds private software-float data and its stack. These are sparse
scoreboard words, not a 2 GiB allocation on the Minecraft server. Checked guest
memory instructions cannot reach that region. Memory.grow returns -1 when a declared or implementation
limit prevents growth; successful growth exposes zero bytes. Without an explicit
maximum, accurate mode uses the implementation cap, rather than fast mode's
256-page default. Initialization rejects out-of-bounds active data segments.

Trap scores are 1 for unreachable/invalid float conversion, 2 for an invalid
indirect call, 3 for memory bounds, and 4 for integer division errors. Public
source functions expose the same diagnostics and stack restoration as fast
mode. The private software-float stack is restored after an export, including
when it traps. Minecraft's command limit is a separate execution limit; exhausting
it is not a successful or valid Wasm result. Exports remain synchronous.

## Host interface and standalone ABI

Integer command templates, static commands and runtime UTF-8 commands work as
before. Accurate core Wasm semantics do not change Minecraft command semantics.
Floating custom host imports and floating command-template arguments currently
fail compilation explicitly. They are not silently passed through fast math.

The current accurate math host surface includes `sinf`, `fmodf`, `fminf`, `fmaxf`,
`sqrtf` and `fabsf`. Sine and remainder use the bundled musl implementations,
lowered through the same software-float pipeline. These imports are an explicit
host contract, distinct from core Wasm instructions. Accelerated execution should
use the pinned host implementations to reproduce them; it can execute ordinary
Wasm float instructions directly with their normal semantics.

Standalone float arguments/results use raw bits in the integer ABI: f32 uses
`#arg0`/`#ret0`; f64 also uses `#arg0h`/`#ret0h`. The high and low scores are signed
i32 values containing the corresponding unsigned bit patterns. Source-pack
function IDs and Rust source signatures are unchanged.

## Provenance and validation

The runtime is built from [Berkeley SoftFloat](https://github.com/ucb-bar/berkeley-softfloat-3)
and [musl](https://git.musl-libc.org/cgit/musl/). Vendored licenses and exact input
hashes are under `runtime/accurate`; `tools/build_accurate_runtime.py` rebuilds the
embedded Wasm with a Wasm-capable LLVM installation. The compiler itself remains
Rust and consumes the prebuilt embedded library.

Compiler tests cover representative float bit patterns, traps and memory bounds.
The [benchmark guide](benchmarks.md) describes the measured workload boundaries.
