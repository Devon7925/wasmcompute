package dev.wasmcompute;

/** The numerical and trap choices shared with Accurate mcfunctions. */
public final class RuntimeOps {
    private RuntimeOps() {}
    public static float F32_MIN(float a,float b){return Float.isNaN(a)||Float.isNaN(b)?Float.NaN:Math.min(a,b);}
    public static float F32_MAX(float a,float b){return Float.isNaN(a)||Float.isNaN(b)?Float.NaN:Math.max(a,b);}
    public static double F64_MIN(double a,double b){return Double.isNaN(a)||Double.isNaN(b)?Double.NaN:Math.min(a,b);}
    public static double F64_MAX(double a,double b){return Double.isNaN(a)||Double.isNaN(b)?Double.NaN:Math.max(a,b);}
    private static double quiet(double a){return Double.longBitsToDouble(Double.doubleToRawLongBits(a)|0x0008000000000000L);}
    public static double F64_CEIL(double a){return Double.isNaN(a)?quiet(a):Math.ceil(a);}
    public static double F64_FLOOR(double a){return Double.isNaN(a)?quiet(a):Math.floor(a);}
    public static double F64_TRUNC(double a){return Double.isNaN(a)?quiet(a):(a<0?Math.ceil(a):Math.floor(a));}
    public static int I32_DIV_S(int a,int b){if(b==0||(a==Integer.MIN_VALUE&&b==-1))throw new Trap(4);return a/b;}
    public static long I64_DIV_S(long a,long b){if(b==0||(a==Long.MIN_VALUE&&b==-1))throw new Trap(4);return a/b;}
    public static final class Trap extends RuntimeException {
        public final int code;
        public Trap(int code){super("Wasm trap "+code);this.code=code;}
    }
}
