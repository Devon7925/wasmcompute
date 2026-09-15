package dev.wasmcompute.mixin;
import dev.wasmcompute.Accelerator;
import java.util.Map;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
import net.minecraft.server.ServerFunctionLibrary;
import net.minecraft.server.packs.resources.Resource;
import net.minecraft.resources.Identifier;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.commands.functions.CommandFunction;
@Mixin(ServerFunctionLibrary.class)
public abstract class LibraryMixin {
    @Inject(method="lambda$reload$3",at=@At("HEAD"),cancellable=true)
    private void wasmcompute$load(Map.Entry<Identifier,Resource> entry,Identifier id,CommandSourceStack source,CallbackInfoReturnable<CommandFunction<CommandSourceStack>> cir) {
        var accelerated=Accelerator.load(entry.getValue(),id);
        if(accelerated!=null)cir.setReturnValue(accelerated);
    }
}
