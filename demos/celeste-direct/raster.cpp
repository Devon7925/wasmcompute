// Platform shim around the upstream mcmain.c rasterizer. build.rs retains the
// reference path and optionally batches rectangles, sprites and map layers.
// The game is unchanged. There is no destination framebuffer.
#include <stddef.h>
#include <stdint.h>
#include <stdarg.h>
#include <string.h>
#include <stdio.h>
extern "C" __attribute__((import_module("env"),import_name("sinf"))) float host_sinf(float);
extern "C" __attribute__((import_module("env"),import_name("fmodf"))) float host_fmodf(float,float);
extern "C" float sinf(float x){return host_sinf(x);}
extern "C" float fmodf(float x,float y){return host_fmodf(x,y);}
#include CELESTE_SOURCE
#undef float
#undef bool
#include CELESTE_TILEMAP
#include "assets.h"
#include "libc.inc"
typedef uint8_t Uint8;typedef uint16_t Uint16;typedef uint32_t Uint32;
struct SDL_Rect {int x,y,w,h;};
struct SDL_Color {uint8_t r,g,b;};
struct SDL_PixelFormat {uint8_t BitsPerPixel,BytesPerPixel;};
struct SDL_Surface {SDL_PixelFormat*format;int w,h,pitch;void*pixels;int locked;SDL_Rect clip_rect;};
static SDL_PixelFormat format={8,1};
static SDL_Surface gfx_surface={&format,128,64,128,(void*)gfx_pixels,0,{0,0,128,64}};
static SDL_Surface font_surface={&format,128,64,128,(void*)font_pixels,0,{0,0,128,64}};
static SDL_Surface*gfx=&gfx_surface,*font=&font_surface;
#define PICO8_W 128
#define PICO8_H 128
#define SCREEN_W 128
#define SCREEN_H 128
static const int scale=1;
static Uint16 buttons_state;
static int enable_screenshake=1;
// Output boundary: use the source palette's RGB values rather than block IDs.
static const SDL_Color base_palette[16]={{0,0,0},{29,43,83},{126,37,83},{0,135,81},{171,82,54},{95,87,79},{194,195,199},{255,241,232},{255,0,77},{255,163,0},{255,236,39},{0,228,54},{41,173,255},{131,118,156},{255,119,168},{255,204,170}};
static SDL_Color palette[16];
static const int base_rgb[16]={0x000000,0x1d2b53,0x7e2553,0x008751,0xab5236,0x5f574f,0xc2c3c7,0xfff1e8,0xff004d,0xffa300,0xffec27,0x00e436,0x29adff,0x83769c,0xff77a8,0xffccaa};
static int palette_colors[16];
extern "C" void direct_palette(int,int);
extern "C" void direct_palette_reset();
static void SetPalette(int a,int b){palette[a]=base_palette[b];palette_colors[a]=base_rgb[b];direct_palette(a,b);}
static void ResetPalette(){memcpy(palette,base_palette,sizeof palette);memcpy(palette_colors,base_rgb,sizeof palette_colors);direct_palette_reset();}
static int abs(int x){return x<0?-x:x;}
static void print(int){} // Upstream diagnostic output is not part of the image.
static void mc_sleep(){} // The host invokes exactly one update/draw per function.
extern "C" void direct_pixel(int x,int y,int rgb);
static int tx=128,ty=128,last_color=0,copied_color=0;
static void turtle_x(int x){tx=x;}
static void turtle_y(int y){ty=y;}
static void turtle_z(int){}
static void turtle_set(int rgb){last_color=rgb;direct_pixel(128-tx,128-ty,rgb);}
static void turtle_copy(){copied_color=last_color;}
static void turtle_paste(){turtle_set(copied_color);}
#include "upstream-primitives.inc"
#include "upstream-raster.inc"
extern "C" {
void play_reset(){buttons_state=0;camera_x=camera_y=0;tx=ty=128;last_color=copied_color=0;ResetPalette();
    Celeste_P8_set_call_func(pico8emu);Celeste_P8_set_rndseed(0x1234);Celeste_P8_init();begin_game();Celeste_P8_draw();}
void play_frame(unsigned buttons){buttons_state=buttons;Celeste_P8_update();Celeste_P8_draw();}
}

#ifdef CELESTE_RASTER_TEST
extern "C" __attribute__((export_name("raster_palette"))) void raster_palette(int a,int b){SetPalette(a,b);}
extern "C" __attribute__((export_name("raster_palette_reset"))) void raster_palette_reset(){ResetPalette();}
// Optional verification entry; absent from released demo modules.
extern "C" __attribute__((export_name("raster_test"))) void raster_test(int kind,int sx,int sy,int x,int y,int w,int h,int color,int flip) {
    if(kind==0){SDL_Rect r={x,y,w,h};SDL_FillRect_screen_palette(&r,color);}
    else if(kind==3){pico8emu(CELESTE_P8_MAP,sx,sy,x,y,w,h,color);}
    else {SDL_Rect src={sx,sy,w,h},dst={x,y,w,h};Xblit(kind==1?gfx:font,&src,&dst,color,flip,0);}
}
#endif
