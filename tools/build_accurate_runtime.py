"""Rebuild the embedded integer-ABI SoftFloat/musl helper module (CPU only)."""
import argparse,concurrent.futures,hashlib,json,re,subprocess
from pathlib import Path

def run(a):
    root=Path(__file__).resolve().parents[1];source=root/'runtime/accurate';out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=True)
    sf=source/'vendor/softfloat';musl=source/'vendor/musl'
    c=['#include "softfloat.h"','#include <stdint.h>','#include "math.h"','#define EXPORT(n) __attribute__((export_name(n)))']
    for bits in [32,64]:
        t=f'uint{bits}_t';ft=f'float{bits}_t';prefix=f'f{bits}'
        for op in ['add','sub','mul','div']:
            c.append(f'EXPORT("{prefix}_{op}") {t} wc_{prefix}_{op}({t} a,{t} b){{return {prefix}_{op}(({ft}){{a}},({ft}){{b}}).v;}}')
        c.append(f'EXPORT("{prefix}_sqrt") {t} wc_{prefix}_sqrt({t} a){{return {prefix}_sqrt(({ft}){{a}}).v;}}')
        for op,mode in [('ceil',3),('floor',2),('trunc',1),('nearest',0)]:
            c.append(f'EXPORT("{prefix}_{op}") {t} wc_{prefix}_{op}({t} a){{return {prefix}_roundToInt(({ft}){{a}},{mode},false).v;}}')
        sign=f'UINT{bits}_C(0x{1<<(bits-1):x})';mask=f'UINT{bits}_C(0x{(1<<(bits-1))-1:x})';inf=f'UINT{bits}_C(0x{(0x7f800000 if bits==32 else 0x7ff0000000000000):x})';nan=f'UINT{bits}_C(0x{(0x7fc00000 if bits==32 else 0x7ff8000000000000):x})'
        for op,expr in [('abs',f'a&{mask}'),('neg',f'a^{sign}')]:c.append(f'EXPORT("{prefix}_{op}") {t} wc_{prefix}_{op}({t} a){{return {expr};}}')
        c.append(f'EXPORT("{prefix}_copysign") {t} wc_{prefix}_copysign({t} a,{t} b){{return (a&{mask})|(b&{sign});}}')
        for op,expr in [('eq',f'{prefix}_eq(({ft}){{a}},({ft}){{b}})'),('ne',f'!{prefix}_eq(({ft}){{a}},({ft}){{b}})'),('lt',f'{prefix}_lt_quiet(({ft}){{a}},({ft}){{b}})'),('le',f'{prefix}_le_quiet(({ft}){{a}},({ft}){{b}})'),('gt',f'{prefix}_lt_quiet(({ft}){{b}},({ft}){{a}})'),('ge',f'{prefix}_le_quiet(({ft}){{b}},({ft}){{a}})')]:c.append(f'EXPORT("{prefix}_{op}") int wc_{prefix}_{op}({t} a,{t} b){{return {expr};}}')
        for op in ['min','max']:
            eqzero='a|b' if op=='min' else 'a&b';comparison=f'{prefix}_lt_quiet(({ft}){{a}},({ft}){{b}})'
            result='a:b' if op=='min' else 'b:a'
            c.append(f'EXPORT("{prefix}_{op}") {t} wc_{prefix}_{op}({t} a,{t} b){{if((a&{mask})>{inf} || (b&{mask})>{inf})return {nan};if(((a|b)&{mask})==0)return {eqzero};return {comparison}?{result};}}')
        for integer in [32,64]:
            for signed in [True,False]:
                ct=f'{"int" if signed else "uint"}{integer}_t';code=f'{"i" if signed else "ui"}{integer}';s='s' if signed else 'u'
                c.append(f'EXPORT("{prefix}_convert_i{integer}_{s}") {t} wc_{prefix}_convert_i{integer}_{s}({ct} a){{return {code}_to_{prefix}(a).v;}}')
                for sat in [False,True]:
                    name=f'i{integer}_trunc_{"sat_" if sat else ""}{prefix}_{s}'
                    maximum=f'UINT{integer}_MAX' if not signed else f'INT{integer}_MAX';minimum=f'INT{integer}_MIN' if signed else '0'
                    failure=f'if((a&{mask})>{inf})return 0;return (a&{sign})?{minimum}:{maximum};' if sat else '__builtin_trap();'
                    c.append(f'EXPORT("{name}") {ct} wc_{name}({t} a){{softfloat_exceptionFlags=0;{ct} r={prefix}_to_{code}_r_minMag(({ft}){{a}},false);if(softfloat_exceptionFlags & softfloat_flag_invalid){{{failure}}}return r;}}')
    c += ['EXPORT("f32_demote_f64") uint32_t wc_demote(uint64_t a){return f64_to_f32((float64_t){a}).v;}','EXPORT("f64_promote_f32") uint64_t wc_promote(uint32_t a){return f32_to_f64((float32_t){a}).v;}',
          'EXPORT("host_sinf") uint32_t wc_sinf(uint32_t a){union{float f;uint32_t i;}u={.i=a};u.f=sinf(u.f);return u.i;}',
          'EXPORT("host_fmodf") uint32_t wc_fmodf(uint32_t a,uint32_t b){union{float f;uint32_t i;}u={.i=a},v={.i=b};u.f=fmodf(u.f,v.f);return u.i;}',
          'EXPORT("host_fminf") uint32_t wc_fminf(uint32_t a,uint32_t b){if((a&0x7fffffff)>0x7f800000)return b;if((b&0x7fffffff)>0x7f800000)return a;return wc_f32_min(a,b);}',
          'EXPORT("host_fmaxf") uint32_t wc_fmaxf(uint32_t a,uint32_t b){if((a&0x7fffffff)>0x7f800000)return b;if((b&0x7fffffff)>0x7f800000)return a;return wc_f32_max(a,b);}']
    bridge=out/'bridge.c';bridge.write_text('\n'.join(c)+'\n')
    names=re.findall(r'^\s+(\w+)\$\(OBJ\)',(source/'SoftFloat-Makefile').read_text(),re.M)
    names=[n for n in names if not any(x in n.lower() for x in ['bf16','f16','extf80','f128'])]
    sources=[(sf/(n+'.c') if (sf/(n+'.c')).exists() else sf/'8086-SSE'/(n+'.c')) for n in dict.fromkeys(names)]+list(musl.glob('*.c'))+[bridge]
    flags=['--target=wasm32','-O2','-flto','-ffp-contract=off','-fno-builtin','-nostdlib','-DSOFTFLOAT_FAST_INT64','-DINLINE_LEVEL=5','-DSOFTFLOAT_ROUND_ODD','-I'+str(source),'-I'+str(sf/'include'),'-I'+str(sf/'8086-SSE')]
    def compile(item):
        i,path=item;obj=out/f'{i}.o'
        result=subprocess.run([a.clang,*flags,'-c',str(path),'-o',str(obj)],capture_output=True,text=True)
        if result.returncode:raise RuntimeError(str(path)+'\n'+result.stderr)
        return str(obj)
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:objects=list(pool.map(compile,enumerate(sources)))
    dest=root/'src/accurate_runtime.wasm'
    subprocess.run([a.clang,'--target=wasm32','-nostdlib','-O2','-flto',*objects,'-Wl,--no-entry,--export-memory,--global-base=2147418112,-z,stack-size=16384,--initial-memory=2147483648','-o',str(dest)],check=True)
    recorded=[flag if not flag.startswith('-I') else '-I'+str(Path(flag[2:]).relative_to(source)).replace('\\','/') for flag in flags]
    version=subprocess.check_output([a.clang,'--version'],text=True).splitlines()[0]
    (source/'BUILD.json').write_text(json.dumps(dict(clang=version,flags=recorded,helper_base=2147418112,helper_stack_bytes=16384,wasm_sha256=hashlib.sha256(dest.read_bytes()).hexdigest()),indent=2)+'\n')
    math=root/'accelerator/runtime/math.wasm'
    math.parent.mkdir(parents=True,exist_ok=True)
    subprocess.run([a.clang,'--target=wasm32','-nostdlib','-O2','-flto',*objects,'-Wl,--no-entry,--export-memory,--global-base=1024,-z,stack-size=16384,--initial-memory=65536','-o',str(math)],check=True)
    if a.oracle:
        Path(a.oracle).write_bytes(math.read_bytes())
    print(json.dumps(dict(wasm=str(dest),bytes=dest.stat().st_size)))

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--clang',default='clang');p.add_argument('--output',required=True);p.add_argument('--oracle',help='Optional low-memory CPU oracle module output');run(p.parse_args())
