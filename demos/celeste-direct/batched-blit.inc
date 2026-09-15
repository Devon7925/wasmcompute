// Cached texture copies avoid decoding the same immutable pixels every frame.
// Unusual source regions and the display-entity backend use the reference path.
extern "C" int direct_blit(int,int,int,int,int,int,int,int,int);
void _blitter(uint8_t *src,int sx,int sy,int pitch,SDL_Rect *dst,int dw,int w,int h,int flip) {
    if(direct_blit(src==gfx_pixels?0:1,sx,sy,dst->x,dst->y,w,h,0,flip))return;
    reference_blitter(src,sx,sy,pitch,dst,dw,w,h,flip);
}
void _blitter_masked(uint8_t *src,int sx,int sy,int pitch,SDL_Rect *dst,int dw,int w,int h,int color,int flip) {
    if(direct_blit(src==gfx_pixels?0:1,sx,sy,dst->x,dst->y,w,h,color,flip))return;
    reference_blitter_masked(src,sx,sy,pitch,dst,dw,w,h,color,flip);
}
