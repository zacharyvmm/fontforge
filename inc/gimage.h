/* Copyright (C) 2000-2012 by George Williams */
/*
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:

 * Redistributions of source code must retain the above copyright notice, this
 * list of conditions and the following disclaimer.

 * Redistributions in binary form must reproduce the above copyright notice,
 * this list of conditions and the following disclaimer in the documentation
 * and/or other materials provided with the distribution.

 * The name of the author may not be used to endorse or promote products
 * derived from this software without specific prior written permission.

 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR IMPLIED
 * WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO
 * EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
 * OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
 * OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

#ifndef FONTFORGE_GIMAGE_H
#define FONTFORGE_GIMAGE_H

#include "basics.h"

typedef uint32_t Color;

#define COLOR_UNKNOWN		((Color) 0xffffffff)
#define COLOR_TRANSPARENT	((Color) 0xffffffff)
#define COLOR_DEFAULT		((Color) 0xfffffffe)
#define COLOR_WARNING		((Color) 0xfffffffd)
#define COLOR_WHITE		((Color) 0xffffff)
#define COLOR_CREATE(r,g,b)	(((r)<<16) | ((g)<<8) | (b))
#define COLOR_ALPHA(col)	(((col)>>24))
#define COLOR_RED(col)		(((col)>>16) & 0xff)
#define COLOR_GREEN(col)	(((col)>>8) & 0xff)
#define COLOR_BLUE(col)		((col)&0xff)

struct hslrgb {
    double h,s,l,v;
    double r,g,b;
    uint8_t rgb, hsl, hsv;
};

struct hslrgba {
    double h,s,l,v;
    double r,g,b;
    uint8_t rgb, hsl, hsv, has_alpha;
    double alpha;
};

typedef struct clut {
    int16_t clut_len;
    unsigned int is_grey: 1;
    uint32_t trans_index;
    Color clut[256];
} GClut;

#define GCLUT_CLUT_EMPTY \
{ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 \
}


typedef struct revcmap RevCMap;

enum image_type { it_mono, it_bitmap=it_mono, it_index, it_true, it_rgba };

/* GImage types retained as opaque definitions for FontData layout compatibility.
 * All GImage I/O functions have been removed. Do not instantiate. */
struct _GImage {
    enum image_type image_type: 2;
    int16_t delay;
    int32_t width, height;
    int32_t bytes_per_line;
    uint8_t *data;
    GClut *clut;
    Color trans;
};

typedef struct gimage {
    short list_len;
    union {
	struct _GImage *image;
    	struct _GImage **images;
    } u;
    void *userdata;
} GImage;

enum pastetrans_type { ptt_paste_trans_to_trans, ptt_old_shines_through};

typedef struct grect {
    int32_t x,y,width,height;
} GRect;

#define GRECT_EMPTY { 0, 0, 0, 0 }


typedef struct gpoint {
    int16_t x,y;
} GPoint;

#define GPOINT_EMPTY { 0, 0 }

#ifdef __cplusplus
extern "C" {
#endif

/* Color conversion functions (implemented in gutils/gcol.c) */
extern void gRGB2HSL(struct hslrgb *col);
extern void gHSL2RGB(struct hslrgb *col);
extern void gRGB2HSV(struct hslrgb *col);
extern void gHSV2RGB(struct hslrgb *col);
extern void gColor2Hslrgb(struct hslrgb *col,Color from);
extern void gColor2Hslrgba(struct hslrgba *col,Color from);
extern Color gHslrgb2Color(struct hslrgb *col);
extern Color gHslrgba2Color(struct hslrgba *col);

#ifdef __cplusplus
}
#endif

#endif /* FONTFORGE_GIMAGE_H */
