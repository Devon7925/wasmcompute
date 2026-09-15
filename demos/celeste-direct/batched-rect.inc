// Draw a clipped solid rectangle as one output primitive, not a pixel loop.
extern "C" void direct_rect(int,int,int,int,int);
void SDL_FillRect_screen_palette(SDL_Rect *rect, char palette_idx) {
    int x=rect?rect->x:0, y=rect?rect->y:0;
    int w=rect?rect->w:128, h=rect?rect->h:128;
    if(x<0){w+=x;x=0;} if(y<0){h+=y;y=0;}
    if(x+w>128)w=128-x; if(y+h>128)h=128-y;
    if(w<=0||h<=0)return;
    int rgb=palette_colors[(unsigned char)palette_idx%16];
    direct_rect(x,y,w,h,rgb);
    // Preserve the turtle state observable by subsequent drawing primitives.
    tx=129-x-w;ty=129-y-h;last_color=copied_color=rgb;
}
