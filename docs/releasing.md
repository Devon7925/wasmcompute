# Release procedure

The public repository root is this directory, not the surrounding benchmark
workspace. `tools/export_source.py` creates an empty-destination source tree
with the compiler, SDK, demo sources, docs, notices and CI. It omits build
caches, prebuilt binaries, generated packs, IWADs, extracted Minecraft data,
historical reports, research benchmarks, profiling utilities, client profiles
and workspace launchers. Documentation and build/release tools use explicit
allowlists; only the maintained guides and tools are exported.

```sh
python tools/check_release.py
python tools/export_source.py --output ../wasmcompute-public
```

Publish **that exported tree**. Its `SOURCE_SHA256SUMS.json` identifies the exact
export; it is distinct from historical benchmark payload hashes. This snapshot
manifest is ignored by Git so ordinary source edits do not require rewriting it. Keep the
original research workspace intact. Do not add all surrounding files to Git.

The CI workflow runs compiler/SDK tests and Fast/Accurate source-pack smoke
checks on Windows and Linux using Rust 1.92.0. A Java 25 job builds the
universal Fabric jar, then Windows/Linux jobs test that identical jar. Numeric
parity and resource limits run without launching Minecraft.
The workflow builds SDK-inclusive compiler zip artifacts for review. It grants
read-only repository access, does not publish releases, and needs no secrets.

```sh
python tools/package_release.py --binary target/release/wasmcompute --output dist/wasmcompute-linux-x64.zip
```

Use the `.exe` binary and an appropriate name on Windows. The bundle includes
the SDK, full accurate-runtime sources/notices, dependency notices, the hello
example and instructions. Users still install Rust's wasm32 target to compile
source packs. Neither game assets nor Minecraft are included.

Before tagging `v0.1.0`, inspect the clean export, run its quick start, review
Actions results on GitHub, enable private vulnerability reporting, and attach
only verified artifacts with their SHA-256 sidecars. No GitHub repository,
release or tag is created by these scripts. The universal Fabric jar is a separate artifact from the compiler bundle.
Its build uses official Minecraft dependencies only as compile inputs.
Review both accelerator test jobs before attaching that jar to a release. CI also extracts each compiler
bundle, verifies its hashes, and builds the hello pack using its relocated SDK.

The original project code is 0BSD. Keep all upstream notices and the separate
GPL Doom/AGPL terrain license boundaries. Do not attach generated game packs
without checking their external assets and corresponding-source obligations.
See THIRD_PARTY.md. A compiler's license does not relicense its inputs.
