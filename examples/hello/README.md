# Hello datapack

Build from the repository root:

```sh
cargo run --release -- build examples/hello -o generated/hello
```

Copy `generated/hello` into a world's `datapacks` folder,
using datapack format 121. Run `/reload`, then `/function hello:platform`. This places a small stone
platform one block below the command position and a light above it.
Use a test area: the function replaces blocks at relative coordinates.

Append `--accurate` to compile the checked vanilla runtime. This source pack
also supports the Fabric accelerator without changing the Rust source.
