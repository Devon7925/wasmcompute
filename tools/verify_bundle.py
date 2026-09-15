"""Verify archive hashes and build a hello pack using the relocated bundled SDK."""
import argparse
import hashlib
import json
import os
import subprocess
import tempfile
import tomllib
import zipfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="wc-bundle-") as temporary:
        root = Path(temporary).resolve()
        with zipfile.ZipFile(args.archive) as archive:
            for info in archive.infolist():
                path = (root / info.filename).resolve()
                if not path.is_relative_to(root):
                    raise ValueError("Archive path escapes extraction directory")
                path.parent.mkdir(parents=True, exist_ok=True)
                if not info.is_dir():
                    path.write_bytes(archive.read(info))
                    if info.filename == "wasmcompute/bin/wasmcompute":
                        path.chmod(0o755)
        bundle = root / "wasmcompute"
        records = (bundle / "SHA256SUMS.txt").read_text().splitlines()
        for record in records:
            expected, name = record.split("  ", 1)
            path = (bundle / name).resolve()
            assert path.is_relative_to(bundle.resolve())
            assert hashlib.sha256(path.read_bytes()).hexdigest() == expected, name
        binary = bundle / "bin" / ("wasmcompute.exe" if os.name == "nt" else "wasmcompute")
        output = root / "hello-pack"
        command = [str(binary), "build", str(bundle / "examples/hello"), "-o", str(output)]
        if args.offline:
            command.append("--offline")
        result = subprocess.run(command, capture_output=True, text=True)
        if result.returncode:
            raise RuntimeError(result.stdout + result.stderr)
        entry = bundle / "examples/hello/target/wasmcompute/entry/Cargo.toml"
        sdk = Path(tomllib.loads(entry.read_text())["dependencies"]["minecraft"]["path"])
        assert sdk.resolve() == (bundle / "sdk/rust/minecraft").resolve(), "Used checkout SDK instead of bundled SDK"
        meta = json.loads((output / "wasmcompute-build.json").read_text())
        assert "hello:platform" in meta["entries"]
        print(f"Bundle verified: {len(records)} hashes; relocated executable used its own SDK and built hello.")


if __name__ == "__main__":
    main()
