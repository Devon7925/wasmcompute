package dev.wasmcompute.mixin;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;
import net.minecraft.commands.Commands;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.commands.execution.ExecutionContext;
@Mixin(Commands.class)
public interface CommandsAccess {
    @Accessor("CURRENT_EXECUTION_CONTEXT")
    static ThreadLocal<ExecutionContext<CommandSourceStack>> wasmcompute$context() {throw new AssertionError();}
}
