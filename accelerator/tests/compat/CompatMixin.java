package dev.wasmcompute.compat.mixin;
import net.minecraft.commands.Commands;
import net.minecraft.commands.CommandBuildContext;
import net.minecraft.world.scores.ScoreHolder;
import com.mojang.brigadier.arguments.IntegerArgumentType;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(Commands.class)
public abstract class CompatMixin {
    @Inject(method="<init>",at=@At("TAIL"))
    private void register(Commands.CommandSelection selection,CommandBuildContext context,CallbackInfo ci){
        ((Commands)(Object)this).getDispatcher().register(Commands.literal("compat_probe")
            .requires(Commands.hasPermission(Commands.LEVEL_GAMEMASTERS))
            .then(Commands.argument("value",IntegerArgumentType.integer()).executes(c->{
                int value=IntegerArgumentType.getInteger(c,"value");
                var board=c.getSource().getServer().getScoreboard();
                var score=board.getOrCreatePlayerScore(ScoreHolder.forNameOnly("#compat_effects"),board.getObjective("sdktest"));
                score.set(score.get()+1);return value;
            })));
    }
}
