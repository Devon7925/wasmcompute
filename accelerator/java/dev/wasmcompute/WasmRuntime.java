package dev.wasmcompute;

import com.dylibso.chicory.runtime.*;
import com.dylibso.chicory.wasm.*;
import com.dylibso.chicory.wasm.types.*;
import com.google.gson.*;
import java.nio.ByteBuffer;
import java.nio.charset.*;
import java.util.*;
import java.util.concurrent.atomic.AtomicLong;

/** Pure-Java scalar Wasm runtime. Minecraft calls stay on the calling server thread. */
public final class WasmRuntime {
    public interface CommandHost {long command(String text);}
    private static final AtomicLong NEXT=new AtomicLong(1);
    private static final ThreadLocal<Map<Long,Guest>> GUESTS=ThreadLocal.withInitial(HashMap::new);
    private WasmRuntime(){}
    public static long create(CommandHost host,byte[] wasm){
        ExecutionBudget.enter();
        try {var guest=new Guest(host,wasm);long id=NEXT.getAndIncrement();GUESTS.get().put(id,guest);return id;}
        finally {ExecutionBudget.leave();}
    }
    public static long[] invoke(long id,String name,long[] args){
        var guest=GUESTS.get().get(id);
        if(guest==null)throw new IllegalStateException("Unknown or closed Wasm instance");
        return guest.invoke(name,args);
    }
    public static void close(long id){var guests=GUESTS.get();guests.remove(id);if(guests.isEmpty())GUESTS.remove();}
    private record Entry(ExportFunction function,FunctionType type){}
    private static final class Guest {
        final Instance instance;
        final Map<String,Entry> entries=new HashMap<>();
        final GlobalInstance stack;
        int hostError;
        Guest(CommandHost host,byte[] bytes){
            var module=Parser.parse(bytes);
            var imports=ImportValues.builder();
            Instance math=null;
            for(int i=0;i<module.importSection().importCount();i++){
                if(!(module.importSection().getImport(i) instanceof FunctionImport imp))throw new IllegalArgumentException("Only function imports are supported");
                var type=module.typeSection().getType(imp.typeIndex());String name=imp.name();WasmFunctionHandle fn;
                if(name.startsWith("wc1_")){
                    require(imp.module().equals("minecraft"),"Wrong command import module");
                    var spec=JsonParser.parseString(new String(HexFormat.of().parseHex(name.substring(4)),StandardCharsets.UTF_8)).getAsJsonObject();
                    String template=spec.get("command").getAsString(),capture=spec.get("capture").getAsString();
                    require(type.params().stream().allMatch(t->t.equals(ValType.I32)||t.equals(ValType.I64))&&type.returns().size()==1&&
                        (capture.equals("outcome")?type.returns().getFirst().equals(ValType.I64):Set.of("result","success").contains(capture)&&type.returns().getFirst().equals(ValType.I32)),"Invalid command capture signature");
                    fn=(inst,args)->{
                        String command=template;
                        for(int j=0;j<args.length;j++)command=command.replace("$(a"+j+")",Long.toString(type.params().get(j).equals(ValType.I32)?(int)args[j]:args[j]));
                        long result=host.command(command);
                        return new long[]{capture.equals("outcome")?result:capture.equals("success")?(int)(result>>>32):(int)result};
                    };
                }else if(name.equals("mc_command_utf8_result")&&imp.module().equals("minecraft")){
                    signature(type,List.of(ValType.I32,ValType.I32),ValType.I32);
                    fn=(inst,args)->{
                        long ptr=Integer.toUnsignedLong((int)args[0]),len=Integer.toUnsignedLong((int)args[1]);
                        if(len>32767||ptr+len>(long)inst.memory().pages()*65536){hostError=1;return new long[]{0};}
                        try{
                            byte[] data=inst.memory().readBytes((int)ptr,(int)len);
                            for(byte b:data)if((b&255)<32){hostError=1;return new long[]{0};}
                            String text=StandardCharsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(data)).toString();
                            return new long[]{(int)host.command(text)};
                        }catch(CharacterCodingException e){hostError=1;return new long[]{0};}
                    };
                }else if(Set.of("sinf","fmodf","fminf","fmaxf").contains(name)){
                    signature(type,name.equals("sinf")?List.of(ValType.F32):List.of(ValType.F32,ValType.F32),ValType.F32);
                    if(math==null){
                        try(var stream=WasmRuntime.class.getResourceAsStream("/runtime/math.wasm")){
                            require(stream!=null,"Missing accurate math helper");
                            math=Instance.builder(Parser.parse(stream.readAllBytes())).withMachineFactory(BytecodeCompiler::compile).build();
                        }catch(java.io.IOException e){throw new IllegalStateException("Cannot load accurate math helper",e);}
                    }
                    var function=math.export("host_"+name);fn=(inst,args)->function.apply(args);
                }else if(name.equals("sqrtf")||name.equals("fabsf")){
                    signature(type,List.of(ValType.F32),ValType.F32);
                    fn=(inst,args)->new long[]{name.equals("fabsf")?args[0]&0x7fffffffL:Integer.toUnsignedLong(Float.floatToRawIntBits((float)Math.sqrt(Float.intBitsToFloat((int)args[0]))))};
                }else throw new IllegalArgumentException("Unsupported accurate host import "+imp.module()+"."+name);
                imports.addFunction(new HostFunction(imp.module(),name,type,fn));
            }
            var builder=Instance.builder(module).withImportValues(imports.build()).withMachineFactory(BytecodeCompiler::compile);
            module.memorySection().ifPresent(section->{
                require(section.memoryCount()<=1,"Multiple memories are unsupported");
                if(section.memoryCount()==1){var limits=section.getMemory(0).limits();require(!limits.shared()&&limits.initialPages()<=32767,"Unsupported memory limits");builder.withMemoryLimits(new MemoryLimits(limits.initialPages(),Math.min(32767,limits.maximumPages())));}
            });
            instance=builder.build();GlobalInstance foundStack=null;
            for(int i=0;i<module.exportSection().exportCount();i++){
                var e=module.exportSection().getExport(i);
                if(e.exportType()==ExternalType.FUNCTION){var type=instance.type(instance.functionType(e.index()));require(type.params().stream().allMatch(WasmRuntime::scalar)&&type.returns().stream().allMatch(WasmRuntime::scalar),"Non-scalar export");entries.put(e.name(),new Entry(instance.export(e.name()),type));}
                if(e.exportType()==ExternalType.GLOBAL&&e.name().equals("__stack_pointer"))foundStack=instance.global(e.index());
            }
            stack=foundStack;
        }
        long[] invoke(String name,long[] args){
            Entry entry=entries.get(name);require(entry!=null,"Unknown export: "+name);require(entry.type.params().size()==args.length,"Wrong argument count");
            ExecutionBudget.enter();long saved=stack==null?0:stack.getValue();hostError=0;
            try{
                long[] result=entry.function.apply(args),out=new long[2+result.length];out[1]=hostError;
                for(int i=0;i<result.length;i++)out[i+2]=entry.type.returns().get(i).equals(ValType.I32)?(int)result[i]:entry.type.returns().get(i).equals(ValType.F32)?Integer.toUnsignedLong((int)result[i]):result[i];
                return out;
            }catch(RuntimeOps.Trap e){return new long[]{e.code,hostError};}
            catch(StackOverflowError e){return new long[]{5,hostError};}
            catch(ChicoryException e){return new long[]{trapCode(e),hostError};}
            finally{if(stack!=null)stack.setValue(saved);ExecutionBudget.leave();}
        }
    }
    private static int trapCode(ChicoryException e){
        String message=String.valueOf(e.getMessage()).toLowerCase(Locale.ROOT);
        if(message.contains("out of bounds memory"))return 3;
        if(message.contains("integer divide by zero"))return 4;
        if(message.contains("call stack exhausted")||e instanceof ChicoryInterruptedException)return 5;
        if(message.contains("indirect call")||message.contains("table")||message.contains("undefined element")||message.contains("uninitialized element")||message.contains("null function")||message.contains("unknown function"))return 2;
        return 1;
    }
    private static boolean scalar(ValType type){return List.of(ValType.I32,ValType.I64,ValType.F32,ValType.F64).contains(type);}
    private static void signature(FunctionType type,List<ValType> params,ValType result){require(type.params().equals(params)&&type.returns().equals(List.of(result)),"Invalid host import signature");}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalArgumentException(message);}
}
