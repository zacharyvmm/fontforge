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

/*
 * AUTO-HINTING STUBS — All auto-hinting functions are no-ops.
 *
 * RALPH-025 decision: STUB auto-hinting. The auto-hinting engine (~3,448 LOC)
 * and stem database (~6,088 LOC, stemdb.c) are removed. Total savings: ~9,336
 * LOC. Hint data structures (StemInfo, DStemInfo, HintMask) and their
 * manipulation (copy, free, transform in splineutil.c) are preserved for SFD
 * round-trip compatibility. TrueType instruction opcodes (ttfinstrs.h) and
 * instruction bytecode storage (SplineChar.ttf_instrs) are also preserved.
 *
 * Rationale: auto-hinting is deeply integrated into the SplineChar data model,
 * but the hint generation itself can be disabled. Modern font renderers
 * (FreeType, CoreText, DirectWrite) do not rely on hints for acceptable
 * rendering on high-DPI displays. Fonts saved through the stubbed pipeline
 * will lack auto-generated hints but will preserve any manually-specified
 * hints and existing instruction bytecode.
 *
 * This is consistent with the "smallest viable C library" goal. HarfBuzz
 * provides rasterization-time auto-hinting for consumers that need it.
 */

#include <fontforge-config.h>

#include "autohint.h"

#include "edgelist.h"
#include "splinefont.h"

#include <string.h>

/* ─── Edge-list functions ─────────────────────────────────────────── */

void ELFindEdges(SplineChar *sc, EIList *el) {
    el->cnt = 0;
}

void ElFreeEI(EIList *el) {
    (void)el;
}

void ELOrder(EIList *el, int major) {
    (void)el;
    (void)major;
}

EI *EIActiveEdgesFindStem(EI *apt, real i, int major) {
    (void)apt;
    (void)i;
    (void)major;
    return NULL;
}

EI *EIActiveEdgesRefigure(EIList *el, EI *active, real i, int major, int *_change) {
    (void)el;
    (void)active;
    (void)i;
    (void)major;
    if (_change) *_change = 0;
    return NULL;
}

EI *EIActiveListReorder(EI *active, int *change) {
    (void)active;
    if (change) *change = 0;
    return NULL;
}

int EISameLine(EI *e, EI *n, real i, int major) {
    (void)e;
    (void)n;
    (void)i;
    (void)major;
    return 0;
}

int EISkipExtremum(EI *e, real i, int major) {
    (void)e;
    (void)i;
    (void)major;
    return 0;
}

real EITOfNextMajor(EI *e, EIList *el, real sought_m) {
    (void)e;
    (void)el;
    (void)sought_m;
    return 0.0;
}

/* ─── Hint instance utilities ─────────────────────────────────────── */

HintInstance *HICopyTrans(HintInstance *hi, real mul, real offset) {
    (void)hi;
    (void)mul;
    (void)offset;
    return NULL;
}

real HIlen(StemInfo *stems) {
    (void)stems;
    return 0.0;
}

real HIoverlap(HintInstance *mhi, HintInstance *thi) {
    (void)mhi;
    (void)thi;
    return 0.0;
}

StemInfo *HintCleanup(StemInfo *stem, int dosort, int instance_count) {
    (void)dosort;
    (void)instance_count;
    return stem; /* pass-through: preserves existing hints */
}

int MergeDStemInfo(SplineFont *sf, DStemInfo **ds, DStemInfo *test) {
    (void)sf;
    (void)ds;
    (void)test;
    return 0;
}

int StemInfoAnyOverlaps(StemInfo *stems) {
    (void)stems;
    return 0;
}

int StemListAnyConflicts(StemInfo *stems) {
    (void)stems;
    return 0;
}

/* ─── Font-level hint data extraction ─────────────────────────────── */

void FindBlues(SplineFont *sf, int layer, real blues[14], real otherblues[10]) {
    (void)sf;
    (void)layer;
    memset(blues, 0, 14 * sizeof(real));
    memset(otherblues, 0, 10 * sizeof(real));
}

void FindHStems(SplineFont *sf, real snaps[12], real cnt[12]) {
    (void)sf;
    memset(snaps, 0, 12 * sizeof(real));
    memset(cnt, 0, 12 * sizeof(real));
}

void FindVStems(SplineFont *sf, real snaps[12], real cnt[12]) {
    (void)sf;
    memset(snaps, 0, 12 * sizeof(real));
    memset(cnt, 0, 12 * sizeof(real));
}

void QuickBlues(SplineFont *_sf, int layer, BlueData *bd) {
    (void)_sf;
    (void)layer;
    (void)bd;
}

/* ─── Flexibility checks ──────────────────────────────────────────── */

int SplineCharIsFlexible(SplineChar *sc, int layer) {
    (void)sc;
    (void)layer;
    return 0;
}

int SplineFontIsFlexible(SplineFont *sf, int layer, int flags) {
    (void)sf;
    (void)layer;
    (void)flags;
    return 0;
}

/* ─── Hint mask management ────────────────────────────────────────── */

void SCClearHintMasks(SplineChar *sc, int layer, int counterstoo) {
    (void)sc;
    (void)layer;
    (void)counterstoo;
}

void SCClearHints(SplineChar *sc) {
    (void)sc;
}

void SCFigureCounterMasks(SplineChar *sc) {
    (void)sc;
}

void SCFigureVerticalCounterMasks(SplineChar *sc) {
    (void)sc;
}

void SCFigureHintMasks(SplineChar *sc, int layer) {
    (void)sc;
    (void)layer;
}

void SCModifyHintMasksAdd(SplineChar *sc, int layer, StemInfo *stem) {
    (void)sc;
    (void)layer;
    (void)stem;
}

/* ─── Hint instance guessing ──────────────────────────────────────── */

void SCGuessDHintInstances(SplineChar *sc, int layer, DStemInfo *ds) {
    (void)sc;
    (void)layer;
    (void)ds;
}

void SCGuessHHintInstancesAndAdd(SplineChar *sc, int layer, StemInfo *stem, real guess1, real guess2) {
    (void)sc;
    (void)layer;
    (void)stem;
    (void)guess1;
    (void)guess2;
}

void SCGuessHHintInstancesList(SplineChar *sc, int layer) {
    (void)sc;
    (void)layer;
}

void SCGuessVHintInstancesAndAdd(SplineChar *sc, int layer, StemInfo *stem, real guess1, real guess2) {
    (void)sc;
    (void)layer;
    (void)stem;
    (void)guess1;
    (void)guess2;
}

void SCGuessVHintInstancesList(SplineChar *sc, int layer) {
    (void)sc;
    (void)layer;
}

void SCGuessHintInstancesList(SplineChar *sc, int layer, StemInfo *hstem, StemInfo *vstem, DStemInfo *dstem, int hvforce, int dforce) {
    (void)sc;
    (void)layer;
    (void)hstem;
    (void)vstem;
    (void)dstem;
    (void)hvforce;
    (void)dforce;
}

/* ─── Main auto-hinting entry points ──────────────────────────────── */

void SFSCAutoHint(SplineChar *sc, int layer, BlueData *bd) {
    (void)sc;
    (void)layer;
    (void)bd;
}

void SplineCharAutoHint(SplineChar *sc, int layer, BlueData *bd) {
    (void)sc;
    (void)layer;
    (void)bd;
}

void _SplineCharAutoHint(SplineChar *sc, int layer, BlueData *bd, struct glyphdata *gd2, int gen_undoes) {
    (void)sc;
    (void)layer;
    (void)bd;
    (void)gd2;
    (void)gen_undoes;
}

int SFNeedsAutoHint(SplineFont *_sf) {
    (void)_sf;
    return 0;
}

void SplineFontAutoHint(SplineFont *_sf, int layer) {
    (void)_sf;
    (void)layer;
}

void SplineFontAutoHintRefs(SplineFont *_sf, int layer) {
    (void)_sf;
    (void)layer;
}
