#include "minecraft.h"
MC_IMPORT("set_health") int32_t set_health(float amount);
MC_IMPORT("fill_platform") int32_t fill_platform(int32_t size);
__attribute__((export_name("run"))) int run(void) {
    MC_COMMAND("scoreboard objectives add demo dummy");
    MC_COMMAND("scoreboard players set example demo 42");
    return MC_COMMAND("scoreboard players get example demo");
}
__attribute__((export_name("platform"))) int platform(int n){return fill_platform(n);}
__attribute__((export_name("health"))) int health(float n){return set_health(n);}
