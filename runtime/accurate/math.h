#ifndef WC_MATH_H
#define WC_MATH_H
typedef double double_t;
typedef float float_t;
float sinf(float);
float fmodf(float,float);
double floor(double);
double scalbn(double,int);
#define M_PI_2 1.57079632679489661923
#define isnan(x) __builtin_isnan(x)
#endif
