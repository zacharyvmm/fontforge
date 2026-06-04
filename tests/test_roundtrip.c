/* Copyright (C) 2024 FontForge contributors
 *
 * Minimal C test harness for libfontforge.so.
 * Tests: font creation, SFD save, SFD re-load, glyph count verification,
 * font name preservation, and spline data integrity.
 */

#include <fontforge-config.h>

#include "encoding.h"
#include "fontforge.h"
#include "fvfonts.h"
#include "sfd.h"
#include "splinefont.h"
#include "splineutil.h"
#include "splineutil2.h"
#include "start.h"
#include "ustring.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int main(int argc, char *argv[]) {
    (void)argc;
    (void)argv;

    /* Initialize FontForge */
    doinitFontForgeMain();

    /* Create a test font */
    SplineFont *sf = SplineFontNew();
    if (sf == NULL) {
        fprintf(stderr, "FAIL: SplineFontNew() returned NULL\n");
        return 1;
    }

    /* Set predictable font name */
    free(sf->fontname);
    sf->fontname = strdup("TestFont");
    sf->onlybitmaps = false;

    /* Create 4 glyphs: .notdef, A, B, C — each with a rectangular contour */
    struct { int uni; const char *name; } glyphs[] = {
        { -1, ".notdef" }, { 0x0041, "A" }, { 0x0042, "B" }, { 0x0043, "C" }
    };
    int nglyphs = (int)(sizeof(glyphs) / sizeof(glyphs[0]));
    int i;

    for (i = 0; i < nglyphs; i++) {
        SplineChar *sc = SFGetOrMakeChar(sf, glyphs[i].uni, glyphs[i].name);
        if (sc == NULL) {
            fprintf(stderr, "FAIL: SFGetOrMakeChar() returned NULL for glyph %d\n", i);
            SplineFontFree(sf);
            return 1;
        }

        /* Add a simple rectangular contour (500x700 units) */
        SplinePoint *sp[4];
        int j;
        for (j = 0; j < 4; j++) {
            real x = (j == 0 || j == 3) ? 0 : 500;
            real y = (j < 2) ? 0 : 700;
            sp[j] = SplinePointCreate(x, y);
        }
        /* Create cubic (order2=false) splines connecting the points */
        SplineMake(sp[0], sp[1], false);
        SplineMake(sp[1], sp[2], false);
        SplineMake(sp[2], sp[3], false);
        SplineMake(sp[3], sp[0], false);

        /* Create the SplinePointList for the foreground layer.
         * last == first signals a closed contour. */
        SplinePointList *spl = calloc(1, sizeof(SplinePointList));
        spl->first = sp[0];
        spl->last = sp[0];  /* Closed contour */
        sc->layers[ly_fore].splines = spl;

        SPLCategorizePoints(spl);
    }

    /* Create a 1:1 encoding map and save to SFD */
    EncMap *map = EncMap1to1(sf->glyphcnt);
    char tmpfile[256];
    snprintf(tmpfile, sizeof(tmpfile), "/tmp/fontforge_test_%d.sfd", getpid());
    int ok = SFDWrite(tmpfile, sf, map, map, false);
    if (!ok) {
        fprintf(stderr, "FAIL: SFDWrite() returned false\n");
        EncMapFree(map);
        SplineFontFree(sf);
        unlink(tmpfile);
        return 1;
    }

    /* Free original font and map */
    EncMapFree(map);
    SplineFontFree(sf);

    /* Re-load from SFD */
    SplineFont *sf2 = LoadSplineFont(tmpfile, 0);
    if (sf2 == NULL) {
        fprintf(stderr, "FAIL: LoadSplineFont() returned NULL\n");
        unlink(tmpfile);
        return 1;
    }

    /* Verify results */
    int ret = 0;

    /* Check glyph count */
    if (sf2->glyphcnt != nglyphs) {
        fprintf(stderr, "FAIL: expected %d glyphs, got %d\n", nglyphs, sf2->glyphcnt);
        ret = 1;
    }

    /* Check font name */
    if (sf2->fontname == NULL || strcmp(sf2->fontname, "TestFont") != 0) {
        fprintf(stderr, "FAIL: font name mismatch: expected 'TestFont', got '%s'\n",
                sf2->fontname ? sf2->fontname : "(null)");
        ret = 1;
    }

    /* Check all glyphs have spline data */
    for (i = 0; i < sf2->glyphcnt && i < nglyphs; i++) {
        SplineChar *sc = sf2->glyphs[i];
        if (sc == NULL || sc->layers[ly_fore].splines == NULL) {
            fprintf(stderr, "FAIL: glyph %d has no spline data after reload\n", i);
            ret = 1;
        }
    }

    /* Clean up */
    SplineFontFree(sf2);
    unlink(tmpfile);

    if (ret == 0) {
        printf("PASS: font round-trip test passed\n");
    } else {
        printf("FAIL: some checks failed\n");
    }

    return ret;
}
