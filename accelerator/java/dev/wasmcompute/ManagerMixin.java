package dev.wasmcompute.mixin;
import dev.wasmcompute.Accelerator;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import net.minecraft.server.ServerFunctionManager;
import net.minecraft.server.ServerFunctionLibrary;
@Mixin(ServerFunctionManager.class)
public abstract class ManagerMixin {
    @Inject(method="replaceLibrary",at=@At("HEAD"))
    private void wasmcompute$reload(ServerFunctionLibrary library,CallbackInfo ci){Accelerator.closeInstances();}
}
