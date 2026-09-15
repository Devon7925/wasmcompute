# Feature and semantics contract

This document describes the default **fast mode**. Both optimization levels use this contract unless `--accurate` is selected. See the [accurate-mode contract](accurate.md) for software floats, checked memory and division traps. Neither mode claims support for every WebAssembly proposal.

## Implemented

- i32/i64 arithmetic, signed and unsigned division/remainders, comparisons, bitwise operations, shifts, rotates, clz/ctz/popcount, integer extension/wrapping and sign extension.
- f32/f64 arithmetic, comparisons, rounding, square root, min/max, absolute value, sign copying, integer conversions and bit reinterpretation, subject to the limits below.
- Direct calls, indirect calls through the initial funcref table, recursion, structured block/loop/if, branches, branch tables, return, select, mutable globals and multiple typed arguments/results.
- A single 32-bit linear memory with active data segments. Byte/halfword/word/wide loads and stores, unaligned accesses, memory size/grow, memory copy/fill. Memory defaults to a maximum of 256 pages when no maximum is declared; explicit maxima are capped at 32,767 pages.
- Function imports for the documented command ABI, configured command templates and supported math functions. Explicit initialization invokes a module start function when present.

Operations/features not implemented fail compilation when encountered; unknown imports are not silently stubbed. The baseline benchmark adapters contain narrowly defined libc shims for their own workloads, not a general libc supplied by the compiler.

## Floating-point tradeoffs

All floating arithmetic uses Minecraft's native **f32** providers, including f64 inputs and operations. Constant folds also round results to f32. F64 loads/reinterpretation reduce their bits to this representation; stores/reinterpretation expand the available f32 value into a double layout. Do not use this for numerical algorithms requiring double precision.

Finite, normal, moderate-magnitude values are the useful domain. NaN constants map to zero; infinite and out-of-f32-range constants saturate to the largest finite f32 magnitude, with a compiler warning. This can change algorithms that use NaN sentinels. NaN propagation, infinities produced at runtime, subnormal encodings, signed zero, overflow/underflow, exact conversion traps and saturating conversions do not have a conforming IEEE/Wasm guarantee. `nearest` uses Minecraft's tie behavior toward positive infinity, not ties to even. Provider math, including sine, can differ from host libm. Out-of-range float-to-integer conversions are not checked as WebAssembly traps.

Ordinary positive/negative f32 arithmetic and representative exact f64 values, wide conversions and reinterpretation were tested against CPU Wasmtime. This does not establish all-bit-pattern conformance. Floating Celeste's first-frame state differs slightly: one inspected coordinate changes by about 0.0000801. Bitwise state hashes therefore differ. This is explicitly retained in the validation evidence.

## Other relaxed behavior and limits

- Memory accesses are not bounds-checked. Negative/out-of-range addresses are outside the supported contract; they are not guaranteed to trap. Sparse storage is not a memory-isolation mechanism.
- Division by zero and signed division overflow are not guaranteed to behave like Wasm traps. Valid ordinary arithmetic uses wrapping integer storage and truncation toward zero as appropriate.
- Memory/table imports, multiple memories/tables, memory64, shared memory, threads/atomics, SIMD, reference values as ordinary Wasm values, dynamic table operations, passive data/element segments, exception handling, GC, WASI and component-model modules are not implemented.
- The input table is static. An invalid indirect call sets trap 2; `unreachable` sets trap 1. Caller code checks these trap flags to unwind. Other errors can be Minecraft command failures rather than Wasm traps.
- One namespace is one shared instance. Standalone exports do not provide a host reentry boundary. Source-pack public functions support synchronous callbacks by saving caller scratch state and restoring the Rust ABI stack pointer; guest memory/global mutations remain visible. External edits of internal memory/storage are unsupported. Aliasing guarantees apply to writes emitted through the compiler's runtime; editing the `namespace_mem` scoreboard or optional `namespace:mem` storage directly bypasses float-cache invalidation.
- Aligned float stores may exist only in typed storage until a Wasm integer/byte read needs their IEEE bits. The raw integer backing words (scoreboard or optional NBT) are not a coherent external view while dirty float words exist. Integer/byte Wasm accesses materialize pending floats automatically. The optional integer word buffer flushes on export return.
- Execution is synchronous. There is no fuel mechanism, automatic tick yielding or general infinite-loop protection beyond Minecraft's configured command limits. Long-running workloads should expose bounded steps.
- `-O0` changes optimization, not these semantics. Select `--accurate` independently to use the accurate scalar contract.

These restrictions make the compiler appropriate for trusted, bounded game logic and application kernels that accept fast math. Compiler errors, `wasmcompute.json` warnings and the real-server regression suite make limitations visible instead of claiming full Wasm compliance.

