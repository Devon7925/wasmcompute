#pragma once
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef void FILE;
#define stderr ((FILE*)0)
int printf(const char*,...);
int fprintf(FILE*,const char*,...);
int snprintf(char*,size_t,const char*,...);
#ifdef __cplusplus
}
#endif
