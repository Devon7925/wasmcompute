package dev.wasmcompute;

import com.dylibso.chicory.compiler.InterpreterFallback;
import com.dylibso.chicory.compiler.internal.ClassLoadingCollector;
import com.dylibso.chicory.compiler.internal.Compiler;
import com.dylibso.chicory.runtime.Instance;
import com.dylibso.chicory.runtime.Machine;
import java.util.Set;
import org.objectweb.asm.*;

/** Small adapter against the pinned Chicory compiler; upstream jars stay untouched. */
public final class BytecodeCompiler {
    private static final Set<String> OPS=Set.of("F32_MIN","F32_MAX","F64_MIN","F64_MAX","F64_CEIL","F64_FLOOR","F64_TRUNC","I32_DIV_S","I64_DIV_S");
    private BytecodeCompiler(){}
    public static Machine compile(Instance instance){
        java.util.function.Supplier<com.dylibso.chicory.compiler.internal.ClassCollector> factory=()->new ClassLoadingCollector(){
            @Override public void put(String name,byte[] bytes){super.put(name,adapt(bytes));}
            @Override public void putMainClass(String name,byte[] bytes){super.putMainClass(name,adapt(bytes));}
        };
        var result=Compiler.builder(instance.module()).withInterpreterFallback(InterpreterFallback.FAIL)
            .withClassCollectorFactory(factory).build().compile();
        return ((ClassLoadingCollector)result.collector()).machineFactory().apply(instance);
    }
    private static byte[] adapt(byte[] bytes){
        var writer=new ClassWriter(0);
        new ClassReader(bytes).accept(new ClassVisitor(Opcodes.ASM9,writer){
            @Override public MethodVisitor visitMethod(int access,String name,String desc,String signature,String[] exceptions){
                return new MethodVisitor(Opcodes.ASM9,super.visitMethod(access,name,desc,signature,exceptions)){
                    @Override public void visitMethodInsn(int opcode,String owner,String method,String descriptor,boolean itf){
                        if(owner.equals("com/dylibso/chicory/runtime/OpcodeImpl")&&OPS.contains(method))owner="dev/wasmcompute/RuntimeOps";
                        if(method.equals("checkInterruption")&&descriptor.equals("()V")){owner="dev/wasmcompute/ExecutionBudget";method="check";}
                        super.visitMethodInsn(opcode,owner,method,descriptor,itf);
                    }
                };
            }
        },0);
        return writer.toByteArray();
    }
}
