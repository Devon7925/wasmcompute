#include <stdint.h>
#include <float.h>
#include "math.h"
#define GET_FLOAT_WORD(w,x) do {union {float f;uint32_t i;} u={x};(w)=u.i;}while(0)
/* Wasm has no observable floating exception flags. */
#define FORCE_EVAL(x) ((void)0)
#define predict_false(x) (x)
float __sindf(double);
float __cosdf(double);
int __rem_pio2f(float,double*);
int __rem_pio2_large(double*,double*,int,int,int);
