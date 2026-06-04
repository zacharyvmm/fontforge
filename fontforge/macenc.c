/* Copyright (C) 2003-2012 by George Williams */
/*
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * Redistributions of source code must retain the above copyright notice, this
 * list of conditions and the following disclaimer.
 *
 * Redistributions in binary form must reproduce the above copyright notice,
 * this list of conditions and the following disclaimer in the documentation
 * and/or other materials provided with the distribution.
 *
 * The name of the author may not be used to endorse or promote products
 * derived from this software without specific prior written permission.
 *
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
 * Stub — macenc.c reduced from 2,364 LOC to ~170 LOC (RALPH-024).
 *
 * Most Mac encoding tables, feature tables, and helper functions were removed.
 * Legacy Mac font support is minimal. Callers handle NULL/0 returns gracefully.
 *
 * Retained data:
 *   - MacRomanEnc[256] — required by tottf.c for Mac Roman cmap output
 *   - macfeat_otftag[] — Mac→OT feature tag map, used by tottfaat.c
 *
 * Symbols still provided for API compatibility:
 *   - MacStrToUtf8 / Utf8ToMacStr (return NULL — no encoding available)
 *   - MacEncFromMacLang (returns 0 — Mac Roman)
 *   - WinLangToMac / WinLangFromMac (return 0xffff / 0x409)
 *   - MacEncToUnicode (returns NULL — no mapping table)
 *   - FindMacFeature / FindMacSetting / FindMacSettingName (return NULL)
 *   - PickNameFromMacName / FindEnglishNameInMacName / MacNameCopy (return NULL)
 *   - MacLanguageFromCode / MacLangFromLocale / CanEncodingWinLangAsMac
 *   - UserFeaturesDiffer (returns 0 — no user features)
 */

#include "macenc.h"
#include "ttf.h" /* for struct macsettingname */

/* Mac Roman encoding table — maps byte 0–255 to Unicode. Retained because
 * tottf.c uses it for writing Mac Roman cmap subtables. */
unichar_t MacRomanEnc[256] = {
    0x0000, 0x0001, 0x0002, 0x0003, 0x0004, 0x0005, 0x0006, 0x0007,
    0x0008, 0x0009, 0x000a, 0x000b, 0x000c, 0x000d, 0x000e, 0x000f,
    0x0010, 0x0011, 0x0012, 0x0013, 0x0014, 0x0015, 0x0016, 0x0017,
    0x0018, 0x0019, 0x001a, 0x001b, 0x001c, 0x001d, 0x001e, 0x001f,
    0x0020, 0x0021, 0x0022, 0x0023, 0x0024, 0x0025, 0x0026, 0x0027,
    0x0028, 0x0029, 0x002a, 0x002b, 0x002c, 0x002d, 0x002e, 0x002f,
    0x0030, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037,
    0x0038, 0x0039, 0x003a, 0x003b, 0x003c, 0x003d, 0x003e, 0x003f,
    0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0045, 0x0046, 0x0047,
    0x0048, 0x0049, 0x004a, 0x004b, 0x004c, 0x004d, 0x004e, 0x004f,
    0x0050, 0x0051, 0x0052, 0x0053, 0x0054, 0x0055, 0x0056, 0x0057,
    0x0058, 0x0059, 0x005a, 0x005b, 0x005c, 0x005d, 0x005e, 0x005f,
    0x0060, 0x0061, 0x0062, 0x0063, 0x0064, 0x0065, 0x0066, 0x0067,
    0x0068, 0x0069, 0x006a, 0x006b, 0x006c, 0x006d, 0x006e, 0x006f,
    0x0070, 0x0071, 0x0072, 0x0073, 0x0074, 0x0075, 0x0076, 0x0077,
    0x0078, 0x0079, 0x007a, 0x007b, 0x007c, 0x007d, 0x007e, 0x007f,
    0x00c4, 0x00c5, 0x00c7, 0x00c9, 0x00d1, 0x00d6, 0x00dc, 0x00e1,
    0x00e0, 0x00e2, 0x00e4, 0x00e3, 0x00e5, 0x00e7, 0x00e9, 0x00e8,
    0x00ea, 0x00eb, 0x00ed, 0x00ec, 0x00ee, 0x00ef, 0x00f1, 0x00f3,
    0x00f2, 0x00f4, 0x00f6, 0x00f5, 0x00fa, 0x00f9, 0x00fb, 0x00fc,
    0x2020, 0x00b0, 0x00a2, 0x00a3, 0x00a7, 0x2022, 0x00b6, 0x00df,
    0x00ae, 0x00a9, 0x2122, 0x00b4, 0x00a8, 0x2260, 0x00c6, 0x00d8,
    0x221e, 0x00b1, 0x2264, 0x2265, 0x00a5, 0x00b5, 0x2202, 0x2211,
    0x220f, 0x03c0, 0x222b, 0x00aa, 0x00ba, 0x03a9, 0x00e6, 0x00f8,
    0x00bf, 0x00a1, 0x00ac, 0x221a, 0x0192, 0x2248, 0x2206, 0x00ab,
    0x00bb, 0x2026, 0x00a0, 0x00c0, 0x00c3, 0x00d5, 0x0152, 0x0153,
    0x2013, 0x2014, 0x201c, 0x201d, 0x2018, 0x2019, 0x00f7, 0x25ca,
    0x00ff, 0x0178, 0x2044, 0x20ac, 0x2039, 0x203a, 0xfb01, 0xfb02,
    0x2021, 0x00b7, 0x201a, 0x201e, 0x2030, 0x00c2, 0x00ca, 0x00c1,
    0x00cb, 0x00c8, 0x00cd, 0x00ce, 0x00cf, 0x00cc, 0x00d3, 0x00d4,
    0xf8ff, 0x00d2, 0x00da, 0x00db, 0x00d9, 0x0131, 0x02c6, 0x02dc,
    0x00af, 0x02d8, 0x02d9, 0x02da, 0x00b8, 0x02dd, 0x02db, 0x02c7
};

/* Mac feature → OpenType tag lookup. Retained because tottfaat.c iterates
 * through it to convert between Mac AAT and OT feature identifiers. */
struct macsettingname macfeat_otftag[] = {
    { 1, 0, CHR('r','l','i','g') },
    { 1, 2, CHR('l','i','g','a') },
    { 1, 4, CHR('d','l','i','g') },
    { 2, 2, CHR('i','s','o','l') },
    { 2, 2, CHR('c','a','l','t') },
    { 3, 3, CHR('s','m','c','p') },
    { 4, 0, CHR('v','r','t','2') },
    { 6, 0, CHR('t','n','u','m') },
    { 10, 1, CHR('s','u','p','s') },
    { 10, 2, CHR('s','u','b','s') },
    { 11, 1, CHR('a','f','r','c') },
    { 11, 2, CHR('f','r','a','c') },
    { 16, 1, CHR('o','r','n','m') },
    { 20, 0, CHR('t','r','a','d') },
    { 20, 1, CHR('s','m','p','l') },
    { 20, 2, CHR('j','p','7','8') },
    { 20, 3, CHR('j','p','8','3') },
    { 20, 4, CHR('j','p','9','0') },
    { 21, 0, CHR('o','n','u','m') },
    { 22, 0, CHR('p','w','i','d') },
    { 22, 2, CHR('h','w','i','d') },
    { 22, 3, CHR('f','w','i','d') },
    { 25, 0, CHR('f','w','i','d') },
    { 25, 1, CHR('p','w','i','d') },
    { 26, 0, CHR('f','w','i','d') },
    { 26, 1, CHR('p','w','i','d') },
    { 103, 0, CHR('h','w','i','d') },
    { 103, 1, CHR('p','w','i','d') },
    { 103, 3, CHR('f','w','i','d') },
    { 0, 0, 0 }
};
struct macsettingname *user_macfeat_otftag = NULL;

MacFeat *default_mac_feature_map = NULL;

char *FindEnglishNameInMacName(struct macname *mn) {
    return NULL;
}

char *MacLanguageFromCode(int code) {
    return "Mac Language";
}

char *MacStrToUtf8(const char *str, int macenc, int maclang) {
    return NULL;
}

char *PickNameFromMacName(struct macname *mn) {
    return NULL;
}

char *Utf8ToMacStr(const char *ustr, int macenc, int maclang) {
    return NULL;
}

const int32_t *MacEncToUnicode(int script, int lang) {
    return NULL;
}

int CanEncodingWinLangAsMac(int winlang) {
    return 0;
}

int MacLangFromLocale(void) {
    return 0;
}

int UserFeaturesDiffer(void) {
    return 0;
}

MacFeat *FindMacFeature(SplineFont *sf, int feat, MacFeat **secondary) {
    if (secondary) *secondary = NULL;
    return NULL;
}

struct macname *FindMacSettingName(SplineFont *sf, int feat, int set) {
    return NULL;
}

struct macname *MacNameCopy(struct macname *mn) {
    return NULL;
}

uint16_t WinLangFromMac(int maclang) {
    return 0x409; /* US English */
}

uint16_t WinLangToMac(int winlang) {
    return 0xffff; /* unknown */
}

uint8_t MacEncFromMacLang(int maclang) {
    return 0; /* Mac Roman */
}

struct macsetting *FindMacSetting(SplineFont *sf, int feat, int set,
                                  struct macsetting **secondary) {
    if (secondary) *secondary = NULL;
    return NULL;
}
