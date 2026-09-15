"""Build actual source packs in both modes and verify their public ABI and fallback metadata."""
import argparse
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", type=Path, required=True)
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    compiler = args.compiler.resolve()
    with tempfile.TemporaryDirectory(prefix="wc-ci-") as directory:
        hashes = {}
        for example, entry in [("hello", "hello:platform"), ("source-pack", "arena:config/roundtrip")]:
            for mode in ["fast", "accurate"]:
                pack = Path(directory) / f"{example}-{mode}"
                command = [str(compiler), "build", str(root / "examples" / example), "-o", str(pack)]
                if mode == "accurate":
                    command.append("--accurate")
                if args.offline:
                    command.append("--offline")
                result = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                if result.returncode:
                    raise RuntimeError(result.stdout + result.stderr)
                meta = json.loads((pack / "wasmcompute-build.json").read_text())
                assert meta["vanilla_semantics"] == mode and entry in meta["entries"]
                accel = json.loads((pack / "wasmcompute-accelerator.json").read_text())
                payload = pack / accel["module"]
                actual = hashlib.sha256(payload.read_bytes()).hexdigest()
                assert actual == accel["sha256"]
                for function, data in accel["entries"].items():
                    namespace, name = function.split(":", 1)
                    path = pack / f"data/{namespace}/function/{name}.mcfunction"
                    assert hashlib.sha256(path.read_bytes()).hexdigest() == data["sha256"]
                if mode == "accurate":
                    assert actual == hashes[example], "Backends must retain identical original Wasm"
                    assert len(list((pack / "licenses").glob("*"))) >= 2
                hashes[example] = actual
                if example == "hello":
                    commands = "\n".join(p.read_text() for p in pack.rglob("*.mcfunction"))
                    assert "Building a platform from Rust!" in commands and "minecraft:sea_lantern strict" in commands
                else:
                    assert (pack / "data/arena/advancement/start.json").read_bytes() == (root / "examples/source-pack/data/arena/advancement/start.json").read_bytes()
                print(f"PASS {example}/{mode}: public functions, resources, payload and accelerator hashes")


if __name__ == "__main__":
    main()
