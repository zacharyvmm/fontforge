/* Copyright (C) 2026 FontForge contributors
 *
 * TTF round-trip test for libfontforge.so.
 * Tests: font creation, TTF save, TTF re-load, glyph count verification,
 * font name preservation.
 */

#include <fontforge-config.h>

#include "encoding.h"
#include "fontforge.h"
#include "fvfonts.h"
#include "parsettf.h"
#include "splinefont.h"
#include "splineutil.h"
#include "splineutil2.h"
#include "start.h"
#include "tottf.h"
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

    /* Set predictable font name (PostScript name — used by TTF) */
    free(sf->fontname);
    sf->fontname = strdup("TestFontTTF");
    sf->onlybitmaps = false;

    /* Set SFD family name as well (some TTF code paths use this) */
    sf->familyname = strdup("TestFontTTF");
    sf->fullname = strdup("TestFontTTF");

    /* Set units-per-em (required for TTF output) */
    sf->ascent = 800;
    sf->descent = 200;
    sf->design_size = 1000;

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

        /* Create the SplinePointList for the foreground layer */
        SplinePointList *spl = calloc(1, sizeof(SplinePointList));
        spl->first = sp[0];
        spl->last = sp[0];  /* Closed contour */
        sc->layers[ly_fore].splines = spl;

        SPLCategorizePoints(spl);
    }

    /* Create a 1:1 encoding map and save to TTF */
    EncMap *map = EncMap1to1(sf->glyphcnt);
    char tmpfile[256];
    snprintf(tmpfile, sizeof(tmpfile), "/tmp/fontforge_ttf_test_%d.ttf", getpid());

    int ok = WriteTTFFont(tmpfile, sf, ff_ttf, NULL, bf_none,
                          ttf_flag_nohints | ttf_flag_dummyDSIG | ttf_flag_shortps,
                          map, ly_fore);
    if (!ok) {
        fprintf(stderr, "FAIL: WriteTTFFont() returned false for %s\n", tmpfile);
        EncMapFree(map);
        SplineFontFree(sf);
        unlink(tmpfile);
        return 1;
    }

    /* Free original font and map */
    EncMapFree(map);
    SplineFontFree(sf);

    /* Re-load from TTF */
    SplineFont *sf2 = SFReadTTF(tmpfile, 0, (enum openflags)0);
    if (sf2 == NULL) {
        fprintf(stderr, "FAIL: SFReadTTF() returned NULL for %s\n", tmpfile);
        unlink(tmpfile);
        return 1;
    }

    /* Verify results */
    int ret = 0;

    /* TTF may add extra glyphs (.null, nonmarkingreturn, etc.) beyond our 4.
     * Check that we have at least our 4 original glyphs. */
    if (sf2->glyphcnt < nglyphs) {
        fprintf(stderr, "FAIL: expected at least %d glyphs, got %d\n", nglyphs, sf2->glyphcnt);
        ret = 1;
    }

    /* Check font name */
    if (sf2->fontname == NULL || strcmp(sf2->fontname, "TestFontTTF") != 0) {
        fprintf(stderr, "FAIL: font name mismatch: expected 'TestFontTTF', got '%s'\n",
                sf2->fontname ? sf2->fontname : "(null)");
        ret = 1;
    }

    /* Look up each test glyph by name and verify it has spline data */
    for (i = 0; i < nglyphs; i++) {
        SplineChar *sc = SFGetChar(sf2, glyphs[i].uni, glyphs[i].name);
        if (sc == NULL) {
            fprintf(stderr, "FAIL: glyph '%s' (U+%04X) not found after TTF reload\n",
                    glyphs[i].name, glyphs[i].uni > 0 ? glyphs[i].uni : 0);
            ret = 1;
            continue;
        }
        if (sc->layers[ly_fore].splines == NULL) {
            fprintf(stderr, "FAIL: glyph '%s' has no spline data after TTF reload\n",
                    sc->name ? sc->name : "(null)");
            ret = 1;
        }
    }

    /* Verify encoding was preserved: A, B, C should map to their codepoints */
    int unis[] = { 0x0041, 0x0042, 0x0043 };
    const char *names[] = { "A", "B", "C" };
    for (i = 0; i < 3; i++) {
        SplineChar *sc = SFGetChar(sf2, unis[i], names[i]);
        if (sc != NULL && sc->unicodeenc != unis[i]) {
            fprintf(stderr, "FAIL: glyph '%s' has unicodeenc %d, expected %d\n",
                    names[i], sc->unicodeenc, unis[i]);
            ret = 1;
        }
    }

    /* Clean up */
    SplineFontFree(sf2);
    unlink(tmpfile);

    if (ret == 0) {
        printf("PASS: TTF font round-trip test passed\n");
    } else {
        printf("FAIL: some checks failed\n");
    }

    return ret;
}
