#ifndef WASMCOMPUTE_MINECRAFT_H
#define WASMCOMPUTE_MINECRAFT_H
#include <stdint.h>
#define MC_IMPORT(name) __attribute__((import_module("minecraft"), import_name(name)))
#ifdef __cplusplus
extern "C" {
#endif
// Runtime ASCII command bytes. Returns command result or success (0/1).
// UTF-8, quotes and escapes in immutable literals use the static API below.
MC_IMPORT("mc_command_result") int32_t mc_command_result(const char*,uint32_t);
MC_IMPORT("mc_command_success") int32_t mc_command_success(const char*,uint32_t);
// Compile-time command literals: pointer+length must be constants, and the data
// must remain immutable. No string construction or macro overhead at runtime.
MC_IMPORT("mc_command_static_result") int32_t mc_command_static_result(const char*,uint32_t);
MC_IMPORT("mc_command_static_success") int32_t mc_command_static_success(const char*,uint32_t);
#ifdef __cplusplus
}
#endif
#define MC_COMMAND(text) mc_command_static_result((text),sizeof(text)-1)
#define MC_COMMAND_OK(text) mc_command_static_success((text),sizeof(text)-1)
// Other imports may be bound to arbitrary commands using --bindings. Templates
// use $(a0), $(a1), ... for i32/f32 values. The current execute context is retained.
#endif
