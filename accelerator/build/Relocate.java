import java.nio.file.*;
import java.util.*;
import java.util.zip.*;
import java.io.*;
import org.objectweb.asm.*;
import org.objectweb.asm.commons.*;

/** Keep the mod's pinned Chicory and ASM isolated from Fabric and other mods. */
public final class Relocate {
    static String rename(String value){
        return value.replace("com/dylibso/chicory","dev/wasmcompute/shaded/chicory")
            .replace("com.dylibso.chicory","dev.wasmcompute.shaded.chicory")
            .replace("org/objectweb/asm","dev/wasmcompute/shaded/asm")
            .replace("org.objectweb.asm","dev.wasmcompute.shaded.asm");
    }
    public static void main(String[] args)throws Exception{
        var entries=new TreeMap<String,byte[]>();
        for(int i=1;i<args.length;i++)try(var zip=new ZipFile(args[i])){
            var iter=zip.entries();while(iter.hasMoreElements()){
                var entry=iter.nextElement();String name=entry.getName();
                if(entry.isDirectory()||!name.endsWith(".class")||name.endsWith("module-info.class"))continue;
                if(!name.startsWith("dev/wasmcompute/")&&!name.startsWith("com/dylibso/chicory/")&&!name.startsWith("org/objectweb/asm/"))throw new IllegalStateException("Unexpected class "+name);
                byte[] bytes=zip.getInputStream(entry).readAllBytes();var writer=new ClassWriter(0);
                var remapper=new Remapper(Opcodes.ASM9){
                    @Override public String map(String name){return rename(name);}
                    @Override public Object mapValue(Object value){return value instanceof String?rename((String)value):super.mapValue(value);}
                };
                new ClassReader(bytes).accept(new ClassRemapper(writer,remapper),0);
                byte[] previous=entries.put(rename(name),writer.toByteArray());
                if(previous!=null)throw new IllegalStateException("Duplicate class "+name);
            }
        }
        try(var out=new ZipOutputStream(Files.newOutputStream(Path.of(args[0])))){
            for(var entry:entries.entrySet()){var z=new ZipEntry(entry.getKey());z.setTime(0);out.putNextEntry(z);out.write(entry.getValue());out.closeEntry();}
        }
    }
}
