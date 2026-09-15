"""Build a deterministic compiler + SDK/source zip from a locally built binary."""
import argparse
import hashlib
import zipfile
from pathlib import Path
from export_source import ROOT, public_bytes, source_files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit("Build the compiler first; --binary must name its executable")
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        raise SystemExit("Archive already exists; choose a fresh output path")
    files = {p.relative_to(ROOT).as_posix(): public_bytes(p) for p in source_files()}
    files["bin/wasmcompute" + (".exe" if binary.suffix == ".exe" else "")] = binary.read_bytes()
    hashes = "".join(f"{hashlib.sha256(data).hexdigest()}  {name}\n" for name, data in sorted(files.items()))
    files["SHA256SUMS.txt"] = hashes.encode()
    with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, data in sorted(files.items()):
            info = zipfile.ZipInfo("wasmcompute/" + name, (2026, 9, 13, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = (0o100755 if name.startswith("bin/") else 0o100644) << 16
            archive.writestr(info, data)
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    output.with_suffix(output.suffix + ".sha256").write_text(f"{digest}  {output.name}\n")
    print(f"{output.name}: {len(files)} files, SHA-256 {digest}")


if __name__ == "__main__":
    main()
