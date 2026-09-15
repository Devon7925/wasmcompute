// Freestanding C library subset for the Celeste platform shim.
extern "C" {

size_t strlen(const char*s){size_t n=0;while(s[n])n++;return n;}
void*memcpy(void*d,const void*s,size_t n){unsigned char*a=(unsigned char*)d;const unsigned char*b=(const unsigned char*)s;for(size_t i=0;i<n;i++)a[i]=b[i];return d;}
void*memmove(void*d,const void*s,size_t n){unsigned char*a=(unsigned char*)d;const unsigned char*b=(const unsigned char*)s;if(a<b){for(size_t i=0;i<n;i++)a[i]=b[i];}else{while(n){--n;a[n]=b[n];}}return d;}
void*memset(void*d,int v,size_t n){unsigned char*a=(unsigned char*)d;for(size_t i=0;i<n;i++)a[i]=(unsigned char)v;return d;}
int printf(const char*,...){return 0;}
int fprintf(FILE*,const char*,...){return 0;}
int snprintf(char*out,size_t cap,const char*fmt,...){
    va_list ap;va_start(ap,fmt);size_t pos=0;
    while(*fmt){if(*fmt!='%'){if(pos+1<cap)out[pos]=*fmt;pos++;fmt++;continue;}
        fmt++;int width=0;if(*fmt=='.'){fmt++;while(*fmt>='0'&&*fmt<='9')width=width*10+(*fmt++-'0');}
        if(*fmt=='i'||*fmt=='d'){int v=va_arg(ap,int);unsigned u=v<0?0u-(unsigned)v:(unsigned)v;char buf[16];int n=0;do{buf[n++]=(char)('0'+u%10);u/=10;}while(u);while(n<width)buf[n++]='0';if(v<0)buf[n++]='-';while(n){if(pos+1<cap)out[pos]=buf[n-1];pos++;n--;}}
        if(*fmt)fmt++;
    }va_end(ap);if(cap)out[pos<cap?pos:cap-1]=0;return (int)pos;
}

}
