"""Build one pure-Java Fabric accelerator jar for Windows and Linux."""
import argparse, hashlib, json, os, shutil, subprocess, urllib.request, zipfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
MINECRAFT = '26.3-rc-1'
FABRIC = '0.19.5'

def fetch(url, path, digest=None, algorithm='sha256'):
    if path.exists() and (digest is None or hashlib.new(algorithm, path.read_bytes()).hexdigest() == digest):
        return path
    path.parent.mkdir(parents=True, exist_ok=True)
    data = urllib.request.urlopen(url, timeout=120).read()
    if digest and hashlib.new(algorithm, data).hexdigest() != digest:
        raise RuntimeError(f'Hash mismatch: {url}')
    path.write_bytes(data)
    return path

def minecraft_inputs(build):
    manifest = json.loads(urllib.request.urlopen('https://piston-meta.mojang.com/mc/game/version_manifest_v2.json').read())
    version = next(v for v in manifest['versions'] if v['id'] == MINECRAFT)
    metadata = json.loads(fetch(version['url'], build / 'minecraft-version.json', version['sha1'], 'sha1').read_text())
    download = metadata['downloads']['server']
    bundle = fetch(download['url'], build / 'minecraft-bundle.jar', download['sha1'], 'sha1')
    libraries = build / 'minecraft-libraries'
    with zipfile.ZipFile(bundle) as archive:
        servers = [n for n in archive.namelist() if n.startswith('META-INF/versions/') and n.endswith('.jar')]
        assert len(servers) == 1
        server = build / 'minecraft-server.jar'
        server.write_bytes(archive.read(servers[0]))
        for name in archive.namelist():
            if name.startswith('META-INF/libraries/') and name.endswith('.jar'):
                target = libraries / name.removeprefix('META-INF/libraries/')
                assert target.resolve().is_relative_to(libraries.resolve())
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.read(name))
    fabric = fetch(f'https://meta.fabricmc.net/v2/versions/loader/{MINECRAFT}/{FABRIC}', build / 'fabric.json')
    return server, libraries, fabric

def javac(jdk, build, name, classpath, sources, classes):
    classes.mkdir(parents=True, exist_ok=True)
    arguments = ['-proc:none', '-classpath', classpath, '-d', str(classes), *map(str, sources)]
    argfile = build / f'{name}.args'
    argfile.write_text('\n'.join('"' + s.replace('\\', '/') + '"' for s in arguments), encoding='utf-8')
    subprocess.run([str(jdk / 'bin' / ('javac.exe' if os.name == 'nt' else 'javac')), '@' + str(argfile)], check=True)

def run(args):
    mod = ROOT / 'accelerator'
    build = args.build.resolve()
    build.mkdir(parents=True, exist_ok=True)
    jdk = args.jdk.resolve()
    supplied = [args.server_jar, args.libraries, args.fabric_metadata]
    if any(supplied) and not all(supplied):
        raise ValueError('Supply all three local Minecraft/Fabric paths, or omit all three to download build dependencies')
    server, libraries, fabric = supplied if all(supplied) else minecraft_inputs(build)
    metadata = json.loads(Path(fabric).read_text(encoding='utf-8-sig'))
    jars = []
    for lib in metadata['launcherMeta']['libraries']['common'] + [{'name': metadata['loader']['maven'], 'url': 'https://maven.fabricmc.net/'}]:
        group, name, version = lib['name'].split(':')
        filename = f'{name}-{version}.jar'
        jars.append(fetch(lib['url'] + group.replace('.', '/') + f'/{name}/{version}/{filename}', build / 'deps' / filename, lib.get('sha256')))
    runtime_jars = []
    for lib in json.loads((mod / 'java-dependencies.json').read_text()):
        group, name, version = lib['group'], lib['artifact'], lib['version']
        filename = f'{name}-{version}.jar'
        url = f'https://repo.maven.apache.org/maven2/{group.replace(chr(46), chr(47))}/{name}/{version}/{filename}'
        runtime_jars.append(fetch(url, build / 'deps' / filename, lib['sha256']))
    classpath = os.pathsep.join(map(str, [*runtime_jars, Path(server).resolve(), *Path(libraries).resolve().rglob('*.jar'), *jars]))
    classes = build / 'classes'
    if classes.exists():
        assert classes.resolve().is_relative_to(build) and classes.resolve() != build
        shutil.rmtree(classes)
    javac(jdk, build, 'bridge', classpath, sorted((mod / 'java').rglob('*.java')), classes)
    own = build / 'bridge.jar'
    with zipfile.ZipFile(own, 'w') as archive:
        for path in sorted(classes.rglob('*.class')):
            archive.write(path, path.relative_to(classes).as_posix())
    helpers = build / 'build-tools'
    javac(jdk, build, 'relocate', classpath, [mod / 'build/Relocate.java'], helpers)
    relocated = build / 'relocated.jar'
    subprocess.run([str(jdk / 'bin' / ('java.exe' if os.name == 'nt' else 'java')), '-cp', str(helpers) + os.pathsep + classpath,
                    'Relocate', str(relocated), str(own), *map(str, runtime_jars)], check=True)
    output = (args.output or ROOT / 'bin/wasmcompute-accelerator-0.1.0.jar').resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        def add(name, data):
            info = zipfile.ZipInfo(name, (2026, 9, 13, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, data)
        with zipfile.ZipFile(relocated) as compiled:
            for name in sorted(compiled.namelist()):
                assert name.startswith('dev/wasmcompute/') and name.endswith('.class')
                add(name, compiled.read(name))
        for path in sorted((mod / 'resources').rglob('*')):
            if path.is_file():
                add(path.relative_to(mod / 'resources').as_posix(), path.read_bytes())
        add('runtime/math.wasm', (mod / 'runtime/math.wasm').read_bytes())
        for path in sorted((mod / 'licenses').iterdir()):
            add('licenses/' + path.name, path.read_bytes())
        for name in ['softfloat', 'musl']:
            notice = ROOT / 'runtime/accurate/vendor' / name / ('COPYRIGHT' if name == 'musl' else 'LICENSE')
            add('licenses/' + name + '.txt', notice.read_bytes())
        add('licenses/wasmcompute.txt', (ROOT / 'LICENSE').read_bytes())
        add('licenses/java-dependencies.json', (mod / 'java-dependencies.json').read_bytes())
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    output.with_suffix('.jar.sha256').write_text(digest + '  ' + output.name + '\n')
    (build / 'test-classpath.txt').write_text(str(output) + os.pathsep + os.pathsep.join(map(str, Path(libraries).resolve().rglob('gson-*.jar'))))
    print(json.dumps({'jar': str(output), 'sha256': digest, 'runtime': 'Chicory 1.7.5', 'native_libraries': 0, 'gpu_used': False}))

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--jdk', type=Path, required=True)
    parser.add_argument('--build', type=Path, required=True)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--server-jar', type=Path)
    parser.add_argument('--libraries', type=Path)
    parser.add_argument('--fabric-metadata', type=Path)
    run(parser.parse_args())
