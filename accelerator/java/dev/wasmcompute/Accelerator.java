package dev.wasmcompute;
import dev.wasmcompute.mixin.CommandsAccess;
import java.util.*;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import com.google.gson.*;
import net.minecraft.server.packs.PackResources;
import net.minecraft.server.packs.resources.Resource;
import net.minecraft.resources.Identifier;
import net.minecraft.commands.*;
import net.minecraft.commands.functions.*;
import net.minecraft.world.scores.*;
import net.minecraft.network.chat.Component;
import org.slf4j.LoggerFactory;

public final class Accelerator {
    private static final Map<PackResources,Optional<Pack>> PACKS=Collections.synchronizedMap(new WeakHashMap<>());
    private static final Set<Pack> LIVE=new HashSet<>(); // server thread only
    private static String sha(byte[] bytes)throws Exception{return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));}
    private static byte[] root(PackResources pack,String path)throws Exception{
        if(path.contains("..")||path.contains("\\")||path.startsWith("/"))throw new IllegalArgumentException("invalid payload path");
        var supplier=pack.getRootResource(path.split("/"));if(supplier==null)throw new IllegalArgumentException("missing "+path);
        try(var in=supplier.get()){return in.readAllBytes();}
    }
    private static Pack read(PackResources pack)throws Exception{
        if(pack.getRootResource("wasmcompute-accelerator.json")==null)return null;
        var meta=JsonParser.parseString(new String(root(pack,"wasmcompute-accelerator.json"),StandardCharsets.UTF_8)).getAsJsonObject();
        if(meta.get("schema").getAsInt()!=1||!meta.get("target").getAsString().equals("26.3-rc-1"))throw new IllegalArgumentException("unsupported accelerator metadata");
        byte[] wasm=root(pack,meta.get("module").getAsString());
        if(!sha(wasm).equals(meta.get("sha256").getAsString()))throw new IllegalArgumentException("Wasm integrity mismatch");
        return new Pack(wasm,meta);
    }
    public static CommandFunction<CommandSourceStack> load(Resource resource,Identifier id){
        try {
            Pack pack;
            synchronized(PACKS){var value=PACKS.get(resource.source());if(value==null){value=Optional.ofNullable(read(resource.source()));PACKS.put(resource.source(),value);}pack=value.orElse(null);}
            if(pack==null)return null;
            var entry=pack.meta.getAsJsonObject("entries").getAsJsonObject(id.toString());if(entry==null)return null;
            try(var in=resource.open()){if(!sha(in.readAllBytes()).equals(entry.get("sha256").getAsString()))return null;}
            return new PlainTextFunction<>(id,List.of((source,context,frame)->{
                context.incrementCost();
                try {
                    if(entry.has("init")){pack.reset(source);frame.returnSuccess(0);}
                    else {
                        long[] result=pack.invoke(source,entry);
                        if(result[0]==0)frame.returnSuccess(result.length>2?(int)result[2]:0);
                        else {
                            pack.command("data modify storage "+pack.ns+":diagnostics last_trap set value {function:\""+id+"\",code:"+result[0]+"}",source);
                            frame.returnFailure();
                        }
                    }
                }catch(Throwable e){source.sendFailure(Component.literal("Wasm accelerator "+id+": "+e.getMessage()));frame.returnFailure();}
                frame.discard();
            }));
        }catch(Exception e){LoggerFactory.getLogger("wasmcompute").warn("Acceleration disabled for {}: {}",id,e.toString());return null;}
    }
    public static void closeInstances(){for(var pack:LIVE){WasmRuntime.close(pack.handle);pack.handle=0;}LIVE.clear();}
    public static final class Pack implements WasmRuntime.CommandHost {
        final byte[] wasm;final JsonObject meta;final String ns;long handle;CommandSourceStack current;int depth;
        Pack(byte[] wasm,JsonObject meta){this.wasm=wasm;this.meta=meta;this.ns=meta.get("namespace").getAsString();}
        void ensure(CommandSourceStack source){if(handle==0){handle=WasmRuntime.create(this,wasm);LIVE.add(this);command("scoreboard objectives add "+ns+" dummy",source);}}
        void reset(CommandSourceStack source){if(depth!=0)throw new IllegalStateException("cannot reset an active module");if(handle!=0)WasmRuntime.close(handle);handle=0;ensure(source);score(source,"#accelerated_calls",0);}
        int score(CommandSourceStack source,String holder){var board=source.getServer().getScoreboard();var obj=board.getObjective(ns);var s=obj==null?null:board.getPlayerScoreInfo(ScoreHolder.forNameOnly(holder),obj);return s==null?0:s.value();}
        void score(CommandSourceStack source,String holder,int value){var board=source.getServer().getScoreboard();board.getOrCreatePlayerScore(ScoreHolder.forNameOnly(holder),board.getObjective(ns)).set(value);}
        long[] invoke(CommandSourceStack source,JsonObject entry){
            if(depth>=64)throw new IllegalStateException("callback depth limit");ensure(source);var previous=current;current=source;depth++;
            try {
                score(source,"#accelerated_calls",score(source,"#accelerated_calls")+1);
                var params=entry.has("params")?entry.getAsJsonArray("params"):new JsonArray();long[] args=new long[params.size()];
                for(int i=0;i<args.length;i++){args[i]=score(source,"#arg"+i);if(params.get(i).getAsString().endsWith("64"))args[i]=(args[i]&0xffffffffL)|((long)score(source,"#arg"+i+"h")<<32);}
                long[] r=WasmRuntime.invoke(handle,entry.get("export").getAsString(),args);
                score(source,"#trap",(int)r[0]);score(source,"#public_trap",(int)r[0]);score(source,"#host_error",(int)r[1]);
                if(r.length>2){score(source,"#ret0",(int)r[2]);score(source,"#ret0h",(int)(r[2]>>>32));score(source,"#public_result",(int)r[2]);}
                return r;
            }finally{depth--;current=previous;}
        }
        public long command(String text){return command(text,current);}
        long command(String text,CommandSourceStack source){
            // Vanilla normally queues nested commands. Host imports require their
            // result now, so isolate and drain a child execution context, restoring
            // the enclosing one in finally. Sources retain all context/permissions.
            var local=CommandsAccess.wasmcompute$context();var previous=local.get();local.remove();
            final long[] outcome={0};
            try{
                var commands=source.getServer().getCommands();
                var executionSource=source.withSuppressedOutput().withCallback((success,result)->outcome[0]=(success?1L<<32:0)|(result&0xffffffffL));
                // Function command text is parsed verbatim. The player/RCON
                // convenience API strips '/', which would change macro behavior.
                commands.performCommand(commands.getDispatcher().parse(text,executionSource),text);
                return outcome[0];
            }
            finally{if(previous==null)local.remove();else local.set(previous);}
        }
    }
}
