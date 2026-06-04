/* -*- coding: utf-8 -*- */
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

#include <fontforge-config.h>

#include "autohint.h"
#include "baseviews.h"
#include "formatstubs.h"
#include "bitmapchar.h"
#include "encoding.h"
#include "ffglib_compat.h"
#include "ffprocess.h"
#include "fontforge.h"
#include "fvcomposite.h"
#include "fvfonts.h"
#include "gfile.h"
#include "groups.h"
#include "namelist.h"
#include "psfont.h"
#include "pua.h"
#include "sfd.h"
#include "spiro.h"
#include "splineoverlap.h"
#include "splinesaveafm.h"
#include "splineutil.h"
#include "splineutil2.h"
#include "ustring.h"
#include "utype.h"

#include <math.h>
#include "ffunistd.h"

static FontViewBase *fv_list=NULL;

extern int onlycopydisplayed;
float joinsnap=0;

void TransHints(StemInfo *stem,real mul1, real off1, real mul2, real off2, int round_to_int ) {
    HintInstance *hi;

    for ( ; stem!=NULL; stem=stem->next ) {
	stem->start = stem->start*mul1 + off1;
	stem->width *= mul1;
	if ( round_to_int ) {
	    stem->start = rint(stem->start);
	    stem->width = rint(stem->width);
	}
	if ( mul1<0 ) {
	    stem->start += stem->width;
	    stem->width = -stem->width;
	}
	for ( hi=stem->where; hi!=NULL; hi=hi->next ) {
	    hi->begin = hi->begin*mul2 + off2;
	    hi->end = hi->end*mul2 + off2;
	    if ( round_to_int ) {
		hi->begin = rint(hi->begin);
		hi->end = rint(hi->end);
	    }
	    if ( mul2<0 ) {
		double temp = hi->begin;
		hi->begin = hi->end;
		hi->end = temp;
	    }
	}
    }
}

/* added by akryukov 01/01/2008 to enable resizing and flipping DStem hints */
void TransDStemHints( DStemInfo *ds,real xmul,real xoff,real ymul,real yoff,int round_to_int ) {
    HintInstance *hi;
    double dmul, temp;

    for ( ; ds!=NULL; ds=ds->next ) {
	ds->left.x = xmul*ds->left.x + xoff;
	ds->left.y = ymul*ds->left.y + yoff;
	ds->right.x = xmul*ds->right.x + xoff;
	ds->right.y = ymul*ds->right.y + yoff;
	if ( round_to_int ) {
	    ds->left.x = rint( ds->left.x );
            ds->left.y = rint( ds->left.y );
	    ds->right.x = rint( ds->right.x );
            ds->right.y = rint( ds->right.y );
	}

	if (( xmul < 0 && ymul > 0 ) || ( xmul > 0 && ymul < 0 ))
	    ds->unit.y = -ds->unit.y;
        ds->unit.x *= fabs( xmul ); ds->unit.y *= fabs( ymul );
        dmul = sqrt( pow( ds->unit.x,2 ) + pow( ds->unit.y,2 ));
        ds->unit.x /= dmul; ds->unit.y /= dmul;
        if ( xmul < 0 ) dmul = -dmul;
        
	for ( hi=ds->where; hi!=NULL; hi=hi->next ) {
	    if ( dmul > 0 ) {
	        hi->begin = hi->begin * dmul;
	        hi->end = hi->end * dmul;
            } else {
                temp = hi->begin;
	        hi->begin = hi->end * dmul;
	        hi->end = temp * dmul;
            }
	}
    }
}

void VrTrans(struct vr *vr,real transform[6]) {
    /* I'm interested in scaling and skewing. I think translation should */
    /*  not affect these guys (they are offsets, so offsets should be */
    /*  unchanged by translation */
    double x,y;

    x = vr->xoff; y=vr->yoff;
    vr->xoff = rint(transform[0]*x + transform[1]*y);
    vr->yoff = rint(transform[2]*x + transform[3]*y);
    x = vr->h_adv_off; y=vr->v_adv_off;
    vr->h_adv_off = rint(transform[0]*x + transform[1]*y);
    vr->v_adv_off = rint(transform[2]*x + transform[3]*y);
}

void BackgroundImageTransform(SplineChar *UNUSED(sc), ImageList *img, real transform[6]) {
    /* GImage removed — images never populated. Simplified to no-op. */
    (void)img; (void)transform;
}

static void GV_Trans(struct glyphvariants *gv,real transform[6], int is_v) {
    int i;

    if ( gv==NULL )
return;
    gv->italic_correction = rint(gv->italic_correction*transform[0]);
    is_v = 3*is_v;
    for ( i=0; i<gv->part_cnt; ++i ) {
	gv->parts[i].startConnectorLength = rint( gv->parts[i].startConnectorLength*transform[is_v] );
	gv->parts[i].endConnectorLength = rint( gv->parts[i].endConnectorLength*transform[is_v] );
	gv->parts[i].fullAdvance = rint( gv->parts[i].fullAdvance*transform[is_v] );
    }
}

static void MKV_Trans(struct mathkernvertex *mkv,real transform[6]) {
    int i;

    for ( i=0; i<mkv->cnt; ++i ) {
	mkv->mkd[i].kern  = rint( mkv->mkd[i].kern  *transform[0]);
	mkv->mkd[i].height= rint( mkv->mkd[i].height*transform[0]);
    }
}

static void MK_Trans(struct mathkern *mk,real transform[6]) {
    if ( mk==NULL )
return;
    MKV_Trans(&mk->top_right,transform);
    MKV_Trans(&mk->top_left ,transform);
    MKV_Trans(&mk->bottom_right,transform);
    MKV_Trans(&mk->bottom_left ,transform);
}

static void MATH_Trans(struct MATH *math,real transform[6]) {
    if ( math==NULL )
return;
    math->DelimitedSubFormulaMinHeight = rint( math->DelimitedSubFormulaMinHeight*transform[3]);
    math->DisplayOperatorMinHeight = rint( math->DisplayOperatorMinHeight*transform[3]);
    math->MathLeading = rint( math->MathLeading*transform[3]);
    math->AxisHeight = rint( math->AxisHeight*transform[3]);
    math->AccentBaseHeight = rint( math->AccentBaseHeight*transform[3]);
    math->FlattenedAccentBaseHeight = rint( math->FlattenedAccentBaseHeight*transform[3]);
    math->SubscriptShiftDown = rint( math->SubscriptShiftDown*transform[3]);
    math->SubscriptTopMax = rint( math->SubscriptTopMax*transform[3]);
    math->SubscriptBaselineDropMin = rint( math->SubscriptBaselineDropMin*transform[3]);
    math->SuperscriptShiftUp = rint( math->SuperscriptShiftUp*transform[3]);
    math->SuperscriptShiftUpCramped = rint( math->SuperscriptShiftUpCramped*transform[3]);
    math->SuperscriptBottomMin = rint( math->SuperscriptBottomMin*transform[3]);
    math->SuperscriptBaselineDropMax = rint( math->SuperscriptBaselineDropMax*transform[3]);
    math->SubSuperscriptGapMin = rint( math->SubSuperscriptGapMin*transform[3]);
    math->SuperscriptBottomMaxWithSubscript = rint( math->SuperscriptBottomMaxWithSubscript*transform[3]);
    /* SpaceAfterScript is horizontal and is below */
    math->UpperLimitGapMin = rint( math->UpperLimitGapMin*transform[3]);
    math->UpperLimitBaselineRiseMin = rint( math->UpperLimitBaselineRiseMin*transform[3]);
    math->LowerLimitGapMin = rint( math->LowerLimitGapMin*transform[3]);
    math->LowerLimitBaselineDropMin = rint( math->LowerLimitBaselineDropMin*transform[3]);
    math->StackTopShiftUp = rint( math->StackTopShiftUp*transform[3]);
    math->StackTopDisplayStyleShiftUp = rint( math->StackTopDisplayStyleShiftUp*transform[3]);
    math->StackBottomShiftDown = rint( math->StackBottomShiftDown*transform[3]);
    math->StackBottomDisplayStyleShiftDown = rint( math->StackBottomDisplayStyleShiftDown*transform[3]);
    math->StackGapMin = rint( math->StackGapMin*transform[3]);
    math->StackDisplayStyleGapMin = rint( math->StackDisplayStyleGapMin*transform[3]);
    math->StretchStackTopShiftUp = rint( math->StretchStackTopShiftUp*transform[3]);
    math->StretchStackBottomShiftDown = rint( math->StretchStackBottomShiftDown*transform[3]);
    math->StretchStackGapAboveMin = rint( math->StretchStackGapAboveMin*transform[3]);
    math->StretchStackGapBelowMin = rint( math->StretchStackGapBelowMin*transform[3]);
    math->FractionNumeratorShiftUp = rint( math->FractionNumeratorShiftUp*transform[3]);
    math->FractionNumeratorDisplayStyleShiftUp = rint( math->FractionNumeratorDisplayStyleShiftUp*transform[3]);
    math->FractionDenominatorShiftDown = rint( math->FractionDenominatorShiftDown*transform[3]);
    math->FractionDenominatorDisplayStyleShiftDown = rint( math->FractionDenominatorDisplayStyleShiftDown*transform[3]);
    math->FractionNumeratorGapMin = rint( math->FractionNumeratorGapMin*transform[3]);
    math->FractionNumeratorDisplayStyleGapMin = rint( math->FractionNumeratorDisplayStyleGapMin*transform[3]);
    math->FractionRuleThickness = rint( math->FractionRuleThickness*transform[3]);
    math->FractionDenominatorGapMin = rint( math->FractionDenominatorGapMin*transform[3]);
    math->FractionDenominatorDisplayStyleGapMin = rint( math->FractionDenominatorDisplayStyleGapMin*transform[3]);
    /* SkewedFractionHorizontalGap is horizontal and is below */
    math->SkewedFractionVerticalGap = rint( math->SkewedFractionVerticalGap*transform[3]);
    math->OverbarVerticalGap = rint( math->OverbarVerticalGap*transform[3]);
    math->OverbarRuleThickness = rint( math->OverbarRuleThickness*transform[3]);
    math->OverbarExtraAscender = rint( math->OverbarExtraAscender*transform[3]);
    math->UnderbarVerticalGap = rint( math->UnderbarVerticalGap*transform[3]);
    math->UnderbarRuleThickness = rint( math->UnderbarRuleThickness*transform[3]);
    math->UnderbarExtraDescender = rint( math->UnderbarExtraDescender*transform[3]);
    math->RadicalVerticalGap = rint( math->RadicalVerticalGap*transform[3]);
    math->RadicalDisplayStyleVerticalGap = rint( math->RadicalDisplayStyleVerticalGap*transform[3]);
    math->RadicalRuleThickness = rint( math->RadicalRuleThickness*transform[3]);
    math->RadicalExtraAscender = rint( math->RadicalExtraAscender*transform[3]);
    math->RadicalDegreeBottomRaisePercent = rint( math->RadicalDegreeBottomRaisePercent*transform[3]);

    /* Horizontals */
    math->SpaceAfterScript = rint( math->SpaceAfterScript*transform[0]);
    math->SkewedFractionHorizontalGap = rint( math->SkewedFractionHorizontalGap*transform[0]);
    math->RadicalKernBeforeDegree = rint( math->RadicalKernBeforeDegree*transform[0]);
    math->RadicalKernAfterDegree = rint( math->RadicalKernAfterDegree*transform[0]);

    /* This number is the same for both horizontal and vertical connections */
    /*  Use the vertical amount as a) will probably be the same and */
    /*   b) most are vertical anyway */
    math->RadicalKernAfterDegree = rint( math->RadicalKernAfterDegree*transform[0]);
}

static void KCTrans(KernClass *kc,double scale) {
    /* Again these are offsets, so I don't apply translation */
    int i;

    for ( i=kc->first_cnt*kc->second_cnt-1; i>=0; --i )
	kc->offsets[i] = rint(scale*kc->offsets[i]);
}

static void SCTransLayer(FontViewBase *fv, SplineChar *sc, int flags, int i, real transform[6], uint8_t *sel) {
    int j;
    RefChar *refs;
    real t[6];
    ImageList *img;

    SplinePointListTransform(sc->layers[i].splines,transform,tpt_AllPoints);
    for ( refs = sc->layers[i].refs; refs!=NULL; refs=refs->next ) {
	if ( (sel!=NULL && sel[fv->map->backmap[refs->sc->orig_pos]]) ||
		(flags&fvt_partialreftrans)) {
	    /* if the character referred to is selected then it's going to */
	    /*  be scaled too (or will have been) so we don't want to scale */
	    /*  it twice */
	    t[4] = refs->transform[4]*transform[0] +
			refs->transform[5]*transform[2] +
			/*transform[4]*/0;
	    t[5] = refs->transform[4]*transform[1] +
			refs->transform[5]*transform[3] +
			/*transform[5]*/0;
	    t[0] = refs->transform[4]; t[1] = refs->transform[5];
	    refs->transform[4] = t[4];
	    refs->transform[5] = t[5];
	    /* Now update the splines to match */
	    t[4] -= t[0]; t[5] -= t[1];
	    if ( t[4]!=0 || t[5]!=0 ) {
		t[0] = t[3] = 1; t[1] = t[2] = 0;
		for ( j=0; j<refs->layer_cnt; ++j )
		    SplinePointListTransform(refs->layers[j].splines,t,tpt_AllPoints);
	    }
	} else {
	    for ( j=0; j<refs->layer_cnt; ++j )
		SplinePointListTransform(refs->layers[j].splines,transform,tpt_AllPoints);
	    t[0] = refs->transform[0]*transform[0] +
			refs->transform[1]*transform[2];
	    t[1] = refs->transform[0]*transform[1] +
			refs->transform[1]*transform[3];
	    t[2] = refs->transform[2]*transform[0] +
			refs->transform[3]*transform[2];
	    t[3] = refs->transform[2]*transform[1] +
			refs->transform[3]*transform[3];
	    t[4] = refs->transform[4]*transform[0] +
			refs->transform[5]*transform[2] +
			transform[4];
	    t[5] = refs->transform[4]*transform[1] +
			refs->transform[5]*transform[3] +
			transform[5];
	    memcpy(refs->transform,t,sizeof(t));
	}
	RefCharFindBounds(refs);
    }
    for ( img = sc->layers[i].images; img!=NULL; img=img->next )
	BackgroundImageTransform(sc, img, transform);
}

/* If sel is specified then we decide how to transform references based on */
/*  whether the referred glyph is selected. (If we transform a reference that */
/*  is selected we are, in effect, transforming it twice -- since the glyph */
/*  itself will be transformed -- so instead we just transform the offsets */
/*  of the reference */
/* If sel is NULL then we transform the reference */
/* if flags&fvt_partialreftrans then we always just transform the offsets */
void FVTrans(FontViewBase *fv,SplineChar *sc,real transform[6], uint8_t *sel,
	enum fvtrans_flags flags) {
    AnchorPoint *ap;
    int i,first,last;
    KernPair *kp;
    PST *pst;

    if ( sc->blended ) {
	int j;
	MMSet *mm = sc->parent->mm;
	for ( j=0; j<mm->instance_count; ++j )
	    FVTrans(fv,mm->instances[j]->glyphs[sc->orig_pos],transform,sel,flags);
    }

    if ( !(flags&fvt_dontmovewidth) )
	if ( transform[0]>0 && transform[3]>0 && transform[1]==0 && transform[2]==0 ) {
	    int widthset = sc->widthset;
	    SCSynchronizeWidth(sc,sc->width*transform[0]+transform[4],sc->width,fv);
	    if ( !(flags&fvt_dontsetwidth) ) sc->widthset = widthset;
	    sc->vwidth = sc->vwidth*transform[3]+transform[5];
	}
    if ( flags & fvt_scalepstpos ) {
	for ( kp=sc->kerns; kp!=NULL; kp=kp->next )
	    kp->off = rint(kp->off*transform[0]);
	for ( kp=sc->vkerns; kp!=NULL; kp=kp->next )
	    kp->off = rint(kp->off*transform[3]);
	for ( pst = sc->possub; pst!=NULL; pst=pst->next ) {
	    if ( pst->type == pst_position )
		VrTrans(&pst->u.pos,transform);
	    else if ( pst->type==pst_pair ) {
		VrTrans(&pst->u.pair.vr[0],transform);
		VrTrans(&pst->u.pair.vr[1],transform);
	    } else if ( pst->type == pst_lcaret ) {
		int j;
		for ( j=0; j<pst->u.lcaret.cnt; ++j )
		    pst->u.lcaret.carets[j] = rint(pst->u.lcaret.carets[j]*transform[0]+transform[4]);
	    }
	}
    }

    if ( sc->tex_height!=TEX_UNDEF )
	sc->tex_height = rint(sc->tex_height*transform[3]);
    if ( sc->tex_depth !=TEX_UNDEF )
	sc->tex_depth  = rint(sc->tex_depth *transform[3]);
    if ( sc->italic_correction!=TEX_UNDEF )
	sc->italic_correction = rint(sc->italic_correction *transform[0]);
    if ( sc->top_accent_horiz !=TEX_UNDEF )
	sc->top_accent_horiz  = rint(sc->top_accent_horiz *transform[0]);
    GV_Trans(sc->vert_variants ,transform, true);
    GV_Trans(sc->horiz_variants,transform, false);
    MK_Trans(sc->mathkern,transform);

    for ( ap=sc->anchor; ap!=NULL; ap=ap->next )
	ApTransform(ap,transform);
    if ( flags&fvt_alllayers ) {
	first = 0;
	last = sc->layer_cnt-1;
    } else if ( sc->parent->multilayer ) {
	first = ly_fore;
	last = sc->layer_cnt-1;
    } else
	first = last = fv->active_layer;
    for ( i=first; i<=last; ++i )
	SCTransLayer(fv,sc,flags,i,transform,sel);
    if ( transform[1]==0 && transform[2]==0 ) {
	if ( transform[0]==1 && transform[3]==1 &&
		transform[5]==0 && transform[4]!=0 && 
		isalpha(sc->unicodeenc)) {
	    SCSynchronizeLBearing(sc,transform[4],fv->active_layer);	/* this moves the hints */
	} else {
	    TransHints(sc->hstem,transform[3],transform[5],transform[0],transform[4],flags&fvt_round_to_int);
	    TransHints(sc->vstem,transform[0],transform[4],transform[3],transform[5],flags&fvt_round_to_int);
	    TransDStemHints(sc->dstem,transform[0],transform[4],transform[3],transform[5],flags&fvt_round_to_int);
	}
    }
    /*if ( flags&fvt_round_to_int )*/
    if ( (flags&fvt_round_to_int) && (!sc->inspiro || !hasspiro())) {
    	/* Rounding the spiros might be a bad idea. */
	/* Not rounding the spiros is also a bad idea. */
	/* Not sure which is worse */
	/* Barry thinks rounding them is a bad idea. */
	SCRound2Int(sc,fv->active_layer,1.0);
    }
    if ( !(flags&fvt_noupdate) )
	SCCharChangedUpdate(sc,fv->active_layer);
}

void FVTransFunc(void *_fv,real transform[6],int otype, BVTFunc *bvts,
	enum fvtrans_flags flags ) {
    FontViewBase *fv = _fv;
    real transx = transform[4], transy=transform[5];
    DBounds bb;
    BasePoint base;
    int i, cnt=0, gid;
    BDFFont *bdf;

    for ( i=0; i<fv->map->enccount; ++i )
	if ( fv->selected[i] && (gid = fv->map->map[i])!=-1 &&
		SCWorthOutputting(fv->sf->glyphs[gid]) )
	    ++cnt;


    ff_progress_start_indicator(10,_("Transforming..."),_("Transforming..."),0,cnt,1);

    SFUntickAll(fv->sf);
    for ( i=0; i<fv->map->enccount; ++i ) if ( fv->selected[i] &&
	    (gid = fv->map->map[i])!=-1 &&
	    SCWorthOutputting(fv->sf->glyphs[gid]) &&
	    !fv->sf->glyphs[gid]->ticked ) {
	SplineChar *sc = fv->sf->glyphs[gid];

	if ( onlycopydisplayed && fv->active_bitmap!=NULL ) {
	    if ( fv->active_bitmap->glyphs[gid]!=NULL )
		BCTrans(fv->active_bitmap,fv->active_bitmap->glyphs[gid],bvts,fv);
	} else {
	    if ( otype==1 ) {
		SplineCharFindBounds(sc,&bb);
		base.x = (bb.minx+bb.maxx)/2;
		base.y = (bb.miny+bb.maxy)/2;
		transform[4]=transx+base.x-
		    (transform[0]*base.x+transform[2]*base.y);
		transform[5]=transy+base.y-
		    (transform[1]*base.x+transform[3]*base.y);
	    }
	    FVTrans(fv,sc,transform,fv->selected,flags);
	    if ( !onlycopydisplayed ) {
		for ( bdf = fv->sf->bitmaps; bdf!=NULL; bdf=bdf->next )
		    if ( gid<bdf->glyphcnt && bdf->glyphs[gid]!=NULL )
			BCTrans(bdf,bdf->glyphs[gid],bvts,fv);
	    }
	}
	sc->ticked = true;
	if ( !ff_progress_next())
    break;
    }
    if ( flags&fvt_dogrid ) {
	SplinePointListTransform(fv->sf->grid.splines,transform,tpt_AllPoints);
    }
    ff_progress_end_indicator();

    if ( flags&fvt_scalekernclasses ) {
	KernClass *kc;
	SplineFont *sf = fv->cidmaster!=NULL ? fv->cidmaster : fv->sf;
	for ( kc=sf->kerns; kc!=NULL; kc=kc->next )
	    KCTrans(kc,transform[0]);
	for ( kc=sf->vkerns; kc!=NULL; kc=kc->next )
	    KCTrans(kc,transform[3]);
	if ( sf->MATH!=NULL )
	    MATH_Trans(sf->MATH,transform);
    }
}

/*                             FV Interface                                   */

static FontViewBase *_FontViewBaseCreate(SplineFont *sf) {
    FontViewBase *fv = calloc(1,sizeof(FontViewBase));
    int i;

    fv->nextsame = sf->fv;
    fv->active_layer = ly_fore;
    sf->fv = fv;
    if ( sf->mm!=NULL ) {
	sf->mm->normal->fv = fv;
	for ( i = 0; i<sf->mm->instance_count; ++i )
	    sf->mm->instances[i]->fv = fv;
    }
    if ( sf->subfontcnt==0 ) {
	fv->sf = sf;
	if ( fv->nextsame!=NULL ) {
	    fv->map = EncMapCopy(fv->nextsame->map);
	    fv->normal = fv->nextsame->normal==NULL ? NULL : EncMapCopy(fv->nextsame->normal);
	    fprintf(stderr, "There are two FontViews using the same SplineFont. Please report on the issue tracker or the mailing list how you reached this point.\n");
	} else if ( sf->compacted ) {
	    fv->normal = sf->map;
	    fv->map = CompactEncMap(EncMapCopy(sf->map),sf);
	    sf->map = fv->map;
	} else {
	    fv->map = sf->map;
	    fv->normal = NULL;
	}
    } else {
	fv->cidmaster = sf;
	for ( i=0; i<sf->subfontcnt; ++i )
	    sf->subfonts[i]->fv = fv;
	for ( i=0; i<sf->subfontcnt; ++i )	/* Search for a subfont that contains more than ".notdef" (most significant in .gai fonts) */
	    if ( sf->subfonts[i]->glyphcnt>1 ) {
		fv->sf = sf->subfonts[i];
	break;
	    }
	if ( fv->sf==NULL )
	    fv->sf = sf->subfonts[0];
	sf = fv->sf;
	if ( fv->nextsame==NULL ) { EncMapFree(sf->map); sf->map = NULL; }
	fv->map = EncMap1to1(sf->glyphcnt);
	if ( fv->nextsame==NULL ) { sf->map = fv->map; }
    }
    fv->selected = calloc(fv->map->enccount,sizeof(uint8_t));

#ifndef _NO_PYTHON
    PyFF_InitFontHook(fv);
#endif
return( fv );
}

static FontViewBase *FontViewBase_Create(SplineFont *sf,int UNUSED(hide)) {
    FontViewBase *fv = _FontViewBaseCreate(sf);
return( fv );
}

static FontViewBase *FontViewBase_Append(FontViewBase *fv)
{
    /* Normally fontviews get added to the fv list when their windows are */
    /*  created. but we don't create any windows here, so... */
    FontViewBase *test;

    if ( fv_list==NULL ) fv_list = fv;
    else {
	for ( test = fv_list; test->next!=NULL; test=test->next );
	test->next = fv;
    }
return( fv );
}

static void FontViewBase_Free(FontViewBase *fv) {
    int i;
    FontViewBase *prev;

   if ( fv->nextsame==NULL && fv->sf->fv==fv ) {
	EncMapFree(fv->map);
	if (fv->sf != NULL && fv->map == fv->sf->map) { fv->sf->map = NULL; }
	fv->map = NULL;
	SplineFontFree(fv->cidmaster?fv->cidmaster:fv->sf);
    } else {
	EncMapFree(fv->map);
	if (fv->sf != NULL && fv->map == fv->sf->map) { fv->sf->map = NULL; }
	fv->map = NULL;
	if ( fv->sf->fv==fv ) {
	    if ( fv->cidmaster==NULL )
		fv->sf->fv = fv->nextsame;
	    else {
		fv->cidmaster->fv = fv->nextsame;
		for ( i=0; i<fv->cidmaster->subfontcnt; ++i )
		    fv->cidmaster->subfonts[i]->fv = fv->nextsame;
	    }
	} else {
	    for ( prev = fv->sf->fv; prev->nextsame!=fv; prev=prev->nextsame );
	    prev->nextsame = fv->nextsame;
	}
    }
#ifndef _NO_FFSCRIPT
    DictionaryFree(fv->fontvars);
    free(fv->fontvars);
#endif
    free(fv->selected);
#ifndef _NO_PYTHON
    PyFF_FreeFV(fv);
#endif
    free(fv);
}

static int FontViewBaseWinInfo(FontViewBase *UNUSED(fv), int *cc, int *rc) {
    *cc = 16; *rc = 4;
return( -1 );
}

static void FontViewBaseSetTitle(FontViewBase *UNUSED(foo)) { }
static void FontViewBaseSetTitles(SplineFont *UNUSED(foo)) { }
static void FontViewBaseRefreshAll(SplineFont *UNUSED(foo)) { }
static void FontViewBaseReformatOne(FontViewBase *UNUSED(foo)) { }
static void FontViewBaseReformatAll(SplineFont *UNUSED(foo)) { }
static void FontViewBaseLayerChanged(FontViewBase *UNUSED(foo)) { }
static void FV_ToggleCharChanged(SplineChar *UNUSED(foo)) { }
static FontViewBase *FVAny(void) { return fv_list; }
static int  FontIsActive(SplineFont *sf) {
    FontViewBase *fv;

    for ( fv=fv_list; fv!=NULL; fv=fv->next )
	if ( fv->sf == sf )
return( true );

return( false );
}

static SplineFont *FontOfFilename(const char *filename) {
    FontViewBase *fv;
    char *abspath = GFileGetAbsoluteName(filename);

    for ( fv=fv_list; fv!=NULL ; fv=fv->next ) {
	if ( (fv->sf->filename!=NULL && strcmp(fv->sf->filename,abspath)==0) ||
	     (fv->sf->origname!=NULL && strcmp(fv->sf->origname,abspath)==0) ) {
	    free(abspath);
	    return fv->sf;
	}
    }
    free(abspath);
return( NULL );
}

static void FVExtraEncSlots(FontViewBase *UNUSED(fv), int UNUSED(encmax)) {
}

static void FontViewBase_Close(FontViewBase *fv) {
    if ( fv_list==fv )
	fv_list = fv->next;
    else {
	FontViewBase *n;
	for ( n=fv_list; n->next!=fv; n=n->next );
	n->next = fv->next;
    }
    FontViewFree(fv);
}

static void FVB_ChangeDisplayBitmap(FontViewBase *fv, BDFFont *bdf) {
    fv->active_bitmap = bdf;
}

static void FVB_ShowFilled(FontViewBase *fv) {
    fv->active_bitmap = NULL;
}

static void FVB_ReattachCVs(SplineFont *UNUSED(old), SplineFont *UNUSED(new)) {
}

static void FVB_DeselectAll(FontViewBase *fv) {
    memset(fv->selected,0,fv->map->encmax);
}

static void FVB_DisplayChar(FontViewBase *UNUSED(fv),int UNUSED(gid)) {
}

static int SFB_CloseAllInstrs(SplineFont *UNUSED(sf)) {
return( true );
}

struct fv_interface noui_fv = {
    FontViewBase_Create,
    _FontViewBaseCreate,
    FontViewBase_Close,
    FontViewBase_Free,
    FontViewBaseSetTitle,
    FontViewBaseSetTitles,
    FontViewBaseRefreshAll,
    FontViewBaseReformatOne,
    FontViewBaseReformatAll,
    FontViewBaseLayerChanged,
    FV_ToggleCharChanged,
    FontViewBaseWinInfo,
    FontIsActive,
    FVAny,
    FontViewBase_Append,
    FontOfFilename,
    FVExtraEncSlots,
    FVExtraEncSlots,
    FVB_ChangeDisplayBitmap,
    FVB_ShowFilled,
    FVB_ReattachCVs,
    FVB_DeselectAll,
    FVB_DisplayChar,
    FVB_DisplayChar,
    FVB_DisplayChar,
    SFB_CloseAllInstrs
};

struct fv_interface *fv_interface = &noui_fv;

void FF_SetFVInterface(struct fv_interface *fvi) {
    fv_interface = fvi;
}


/******************************************************************************/
static int NoGlyphs(struct metricsview *UNUSED(mv)) {
return( 0 );
}

static SplineChar *Nothing(struct metricsview *UNUSED(mv), int UNUSED(i)) {
return( NULL );
}

static void NoReKern(struct splinefont *UNUSED(sf)) {
}

static void NoReFeature(struct splinefont *UNUSED(sf)) {
}

static void NoCloseAll(struct splinefont *UNUSED(sf)) {
}

struct mv_interface noui_mv = {
    NoGlyphs,
    Nothing,
    NoReKern,
    NoReFeature,
    NoCloseAll
};

struct mv_interface *mv_interface = &noui_mv;

void FF_SetMVInterface(struct mv_interface *mvi) {
    mv_interface = mvi;
}
