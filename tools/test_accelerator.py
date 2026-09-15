"""Test the shipped pure-Java accelerator jar on the current OS, without Minecraft."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--jdk', type=Path, required=True)
    parser.add_argument('--jar', type=Path, required=True)
    parser.add_argument('--build', type=Path, required=True)
    args = parser.parse_args()
    args.build.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(args.jar) as archive:
        names = archive.namelist()
        assert not any(n.lower().endswith(('.dll', '.so', '.dylib')) for n in names)
        assert all(n.startswith('dev/wasmcompute/') for n in names if n.endswith('.class'))
        for notice in ['CHICORY-LICENSE', 'ASM-LICENSE', 'softfloat.txt', 'musl.txt', 'wasmcompute.txt']:
            assert len(archive.read('licenses/' + notice)) > 500
    # Gson is supplied by Minecraft in production, and used to read test vectors here.
    gson = args.build / 'gson-2.11.0.jar'
    digest = '57928d6e5a6edeb2abd3770a8f95ba44dce45f3b23b7a9dc2b309c581552a78b'
    if not gson.exists():
        gson.write_bytes(urllib.request.urlopen('https://repo.maven.apache.org/maven2/com/google/code/gson/gson/2.11.0/gson-2.11.0.jar', timeout=120).read())
    assert hashlib.sha256(gson.read_bytes()).hexdigest() == digest, 'Gson hash mismatch'
    suffix = '.exe' if os.name == 'nt' else ''
    classes = args.build / 'classes'
    classes.mkdir(exist_ok=True)
    cp = os.pathsep.join(map(str, [args.jar.resolve(), gson.resolve()]))
    subprocess.run([str(args.jdk / 'bin' / ('javac' + suffix)), '-proc:none', '-cp', cp, '-d', str(classes), str(ROOT / 'accelerator/tests/RuntimeTest.java')], check=True)
    subprocess.run([str(args.jdk / 'bin' / ('java' + suffix)), '-Djava.awt.headless=true', '-XX:ActiveProcessorCount=2', '-Xmx1g', '-cp', str(classes.resolve()) + os.pathsep + cp, 'RuntimeTest', str(ROOT / 'accelerator/tests')], check=True, timeout=120)
    print(json.dumps({'jar_sha256': hashlib.sha256(args.jar.read_bytes()).hexdigest(), 'native_libraries': 0, 'gpu_used': False}))

if __name__ == '__main__':
    main()
