# Benchmarks

The [README table](../README.md#performance) compares completed demo workloads.
All measurements used CPU-only, headless servers on a shared Windows machine
with an Intel i7-13700F. Existing training was left running. No rendering client
or GPU was used. Results describe these runs, not a universal FPS guarantee.

## What the timers include

| Workload | Timer | Included | Excluded |
|---|---|---|---|
| Current Celeste and Doom | Server stopwatch around each public function | Game update, rasterizer, command dispatch, block writes | Reset, client transport, validation, idle time between calls |
| Published wasmcraft2 games | Local elapsed time between completed-frame counters | Game/rendering work and the VM's scheduled tick pauses | Initialization and time to first frame |
| Terrain | Sum of server execution times | Seed setup, generation, block writes | Gaps between stages, saves, validation |
| Serde, all four modes | Client elapsed time for 64 consecutive calls, divided by 64 | Serialization, SDK input, local command transport, trace/completion bookkeeping | Setup, reset, final output verification |

Serde's timer is consistent across its four columns. It measures the time a
local caller waits for serialization, rather than timing only the work inside
Minecraft. This overhead is included, not estimated and subtracted. The reported
value is the median of seven 64-object replays, following two warmup replays;
seeds are 42 through 105. It uses serde 1.0.162 and serde-json-core 0.6.0 for a
nested object containing arrays, booleans, signed coordinates and escaped text.
All backends matched the checked lengths, byte checksums and JSON content.

The current game/terrain stopwatches have millisecond resolution. Asking for
microsecond-scaled scores does not give them microsecond accuracy. Compiling,
server startup and JIT compilation are not separate workload benchmarks here;
there is no additional discarded warmup replay for the fresh Celeste runs.

## Celeste Classic: corrected palette

Fresh September 13, 2026 runs use the fixed-point game and cached block renderer,
including the hair-color remapping and peach skin material correction. Each
current mode executes the public `celeste_direct:bench_step` entry 180 times with
`[2,2,18,2,2,34,2,0]` repeated. That entry reads the supplied buttons into ordinary
Rust state before calling the unchanged game update/draw path. Fast, Accurate
and Accelerated retain identical original Wasm bytes.

| Measurement | Fast | Accurate | Accelerated |
|---|---:|---:|---:|
| Mean server execution per rendered frame | 44.54 ms | 45.64 ms | 12.72 ms |
| Mean client call elapsed time | 45.24 ms | 46.33 ms | 13.12 ms |
| Reset, measured separately | 3,110 ms | 3,447 ms | 552 ms |

All 180 returned draw counts matched the CPU oracle in all three modes. The
CPU replay matched all 180 corrected-palette reference frames; each saved
Minecraft screen matched all 16,384 final reference pixels after block-palette
mapping. This validates the benchmark input path as well as its rendered output.
The old pre-palette-fix FPS measurement is not used in the public table.

The upstream baseline is the Celeste world distributed with SuperTails'
[published demo video](https://www.youtube.com/watch?v=wCHB1UgwM9o). Its original
128×128 block renderer, buffer copies and material palette are retained. Only
frame counters, deterministic input at the original block buttons, and a bounded
stop condition were added. The mean is **553.63 ms** over 180 intervals after the
first completed frame. Frame-counter differences include short frames that
complete between polls. This is a playable-demo baseline with a different
renderer and executable, not a byte-identical compiler comparison or a claim
that the upstream demo shared our former palette bug.

## Doom: published upstream demo and current port

The wasmcraft2 baseline now uses the actual Doom datapack distributed with the
same upstream video. It initialized successfully using its original 200-page
memory layout and produced six complete **320×200** frames. Initialization took
11.30 seconds; the first frame completed 205.56 seconds after starting the game.
The following five intervals were 178.81, 18.22, 17.08, 11.50 and 10.25 seconds,
for a **47,173.54 ms** mean. All five intervals are included.

Its original arguments are `doom -iwad DOOM1.WAD -nosound -turbo 400`, with no
level warp or player input. The measurement covers the **title/attract startup**,
not an E1M1 gameplay replay. The original per-row scheduled pauses remain. Two
frame counters were the only changes to its functions. The published demo runs
successfully; the table no longer substitutes an initialization failure from
our larger, different Doom port for this baseline.

Our current port renders **128×80** blocks and runs 72 deterministic frames,
with `[4,4,20,20,5,5,36,0]` repeated. Its 1,221.74 / 1,627.57 / 8.32 ms means
include the opening wipe and gameplay. All three modes use identical Wasm,
match all 10,240 final screen blocks and return framebuffer checksum
`1171215790`. The final eight gameplay frames alone average 2,228.25 / 3,018.88 /
8.12 ms. Reset is separate: 34,480 / 43,493 / 64 ms.

These Doom implementations render different scenes at different resolutions.
Their absolute results demonstrate working demos; dividing their timings would
not establish a controlled compiler speedup.

## Terrain

Each measurement completes a 16×16×384 chunk: 98,304 blocks. Accurate and
Accelerated matched every tested block for seeds 42 and 1337. Fast matched seed
42 exactly and differed in 45 blocks for seed 1337. Neighboring chunks remained
unchanged. wasmcraft2 cannot compile the required scalar floating-point workload;
a minimal `f64.sqrt` probe also fails. Unsupported is not a numerical speedup.

Vanilla totals sum 6,834 completed generation stages plus initialization. The
accelerator uses the playable pack's 214 batch entries, each running up to 32 of
the same stages, to avoid stopwatch rounding on tiny individual calls. Checks,
saves and scheduled gaps are outside the timer. Player-visible vanilla generation
therefore takes longer than these already substantial execution totals.

The reference is a controlled Minecraft **26.2** generation subset, then upgraded
to the tested command platform. Structures, placed features, legacy carvers and
eroded-badlands pillars are excluded. Noise caves, aquifers, ore veins and other
surface rules remain. Replacement writes block states, not biome metadata.
These checks do not establish all-seed, all-biome or newer-version conformity.

## Pure-Java migration

The Accelerated column now reports Chicory, not the previous Rust/Wasmtime
backend. The Windows server replay uses identical retained Wasm and inputs,
checks traps and completion, and includes actual Minecraft command effects.
The earlier Wasmtime measurements are retained here for comparison:

| Workload | Previous Wasmtime | Current pure Java |
|---|---:|---:|
| Celeste, 180 frames, mean | 12.45 ms/frame | 12.72 ms/frame |
| Doom, 72 frames, mean | 15.97 ms/frame | 8.32 ms/frame |
| Terrain seed 42, complete chunk | 70 ms | 260 ms |
| Terrain seed 1337, complete chunk | 80 ms | 206 ms |
| Serde, client timer, median | 0.141 ms/object | 0.184 ms/object |

These server runs were collected separately on the same shared machine; small
differences are not statistically established regressions. Both game replays
match all returned draw counts and final screen blocks. Both terrain seeds
match all 98,304 Accurate block states. Serde matches exact JSON, length and
checksum for six verification inputs.

An additional paired engine test used identical Wasm with a mock command host
that returned fixed results and hashed command strings. It excludes Minecraft
writes, module compilation and initialization. Seven measured replays followed
two warmups in separate JVMs (180 Celeste frames, 72 Doom frames, 64 Serde objects).
Median per-operation times were:

| Engine work only | Wasmtime | Java |
|---|---:|---:|
| Celeste | 0.158 ms/frame | 0.103 ms/frame |
| Doom | 6.405 ms/frame | 1.057 ms/frame |
| Serde | 0.0030 ms/object | 0.0155 ms/object |

Every replay's result trace and command-string hash/count matched. A separate
terrain engine run used five chunk replays per seed, discarding the first:
14.40 vs 142.90 ms for seed 42 and 17.50 vs 123.02 ms for seed 1337.
This isolates a substantial terrain computation regression from block-write
cost. Pure Java was selected for its single-jar portability after checking this
tradeoff; it is not faster than Wasmtime on every workload. It remains much
faster than generated Fast mcfunctions for the measured demos.

## Environment and reproduction scope

Current measurements use Minecraft 26.3-rc-1 (datapack format 121), Java 25 and,
where accelerated, Fabric Loader 0.19.5 and the pure-Java Chicory 1.7.5 backend. Upstream runs use Minecraft 1.19.4 and
Java 21. JVMs use two active processors and below-normal priority. Fresh Celeste
runs cap the heap at 4 GiB; published Doom uses 6 GiB. Serde used a smaller
256–1536 MiB heap configuration for earlier backends; the Java accelerator
remeasurement used the same 4 GiB server as the other current Java demos. The machine is shared, so scheduling and JIT
warmup affect absolute numbers.

The published upstream games use a 40,000-command VM yield threshold and a
10,000,000-command sequence ceiling. Vanilla Celeste uses 4,194,304 commands per
frame and a separate 268,435,456 setup ceiling. The Java benchmark used the
268,435,456 test ceiling for its bounded runs; limits were restored afterward.
The Java accelerator uses 100 million execution checks per outer invocation,
shared by nested calls; these are different units from the former Wasmtime fuel.

To measure a new build, use fresh output directories, verify matching retained
Wasm hashes across current modes, and replay identical inputs through public
functions. Report setup separately, state the timer and sample count, and check
completion, traps and output before reporting performance. Keep the server
headless and run only one timed JVM at a time. The public source tree contains
maintained guides and build tools; local historical reports, server worlds and
research utilities are not part of the release.
