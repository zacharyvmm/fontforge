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

#ifndef FONTFORGE_FORMATSTUBS_H
#define FONTFORGE_FORMATSTUBS_H

#include "baseviews.h"
#include "splinefont.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ---- macbinary stubs ---- */

extern char **NamesReadMacBinary(char *filename);
extern int LoadKerningDataFromMacFOND(SplineFont *sf, char *filename, EncMap *map);
extern int WriteMacBitmaps(char *filename, SplineFont *sf, int32_t *sizes, int is_dfont, EncMap *enc);
extern int WriteMacFamily(char *filename, struct sflist *sfs, enum fontformat format, enum bitmapformat bf, int flags, int layer);
extern int WriteMacPSFont(char *filename, SplineFont *sf, enum fontformat format, int flags, EncMap *enc, int layer);
extern int WriteMacTTFFont(char *filename, SplineFont *sf, enum fontformat format, int32_t *bsizes, enum bitmapformat bf, int flags, EncMap *enc, int layer);
extern long mactime(void);
extern SplineChar *SFFindExistingCharMac(SplineFont *sf, EncMap *map, int unienc);
extern SplineFont *SFReadMacBinary(char *filename, int flags, enum openflags openflags);
extern uint16_t _MacStyleCode(const char *styles, SplineFont *sf, uint16_t *psstylecode);
extern void SfListFree(struct sflist *sfs);

/* ---- autosave stubs ---- */

extern int DoAutoRecoveryExtended(int inquire);
extern void DoAutoSaves(void);
extern void CleanAutoRecovery(void);

/* ---- palmfonts stubs ---- */

extern int WritePalmBitmaps(const char *filename, SplineFont *sf, int32_t *sizes, EncMap *map);
extern SplineFont *SFReadPalmPdb(char *filename);

/* ---- winfonts stubs ---- */

extern int FNTFontDump(char *filename, BDFFont *font, EncMap *map, int res);
extern int FONFontDump(char *filename, SplineFont *sf, int32_t *sizes, int resol, EncMap *map);
extern SplineFont *SFReadWinFON(char *filename, int toback);

/* ---- ikarus stubs ---- */

extern SplineFont *SFReadIkarus(char *fontname);

/* ---- metafont stubs ---- */

extern SplineFont *SFFromMF(char *fontname);

/* ---- scripting globals ---- */

extern int running_script;

/* ---- cvexport stubs ---- */

extern int _ExportEPS(FILE *eps, SplineChar *sc, int layer, int preview);
extern int ExportEPS(char *filename, SplineChar *sc, int layer);

/* ---- print globals ---- */

extern int pagewidth;
extern int pageheight;
extern int printtype;

/* ---- zapfnomen globals ---- */

#define ZAPF_SIZE 192
extern char *zapfnomen[ZAPF_SIZE];
extern short zapfwx[ZAPF_SIZE];
extern short zapfbb[ZAPF_SIZE][4];
extern char zapfexists[ZAPF_SIZE];

/* ---- undo/clipboard stubs ---- */

extern int no_windowing_ui;
extern int maxundoes;

extern char *UndoToString(SplineChar *sc, Undoes *undo);
extern const Undoes *CopyBufferGet(void);
extern enum undotype CopyUndoType(void);
extern int CopyContainsBitmap(void);
extern int CopyContainsSomething(void);
extern int CopyContainsVectors(void);
extern int CVLayer(CharViewBase *cv);
extern int getAdobeEnc(const char *name);
extern int SCDependsOnSC(SplineChar *parent, SplineChar *child);
extern int SCWasEmpty(SplineChar *sc, int skip_this_layer);
extern RefChar *CopyContainsRef(SplineFont *sf);
extern RefChar *RefCharsCopyState(SplineChar *sc, int layer);
extern SplineSet *ClipBoardToSplineSet(void);
extern int SCClipboardHasPasteableContents(void);
extern Undoes *BCPreserveState(BDFChar *bc);
extern Undoes *CVPreserveState(CharViewBase *cv);
extern Undoes *CVPreserveStateHints(CharViewBase *cv);
extern Undoes *_CVPreserveTState(CharViewBase *cv, PressedOn *p);
extern Undoes *CVPreserveVWidth(CharViewBase *cv, int vwidth);
extern Undoes *CVPreserveWidth(CharViewBase *cv, int width);
extern Undoes *SCPreserveBackground(SplineChar *sc);
extern Undoes *SCPreserveHints(SplineChar *sc, int layer);
extern Undoes *_SCPreserveLayer(SplineChar *sc, int layer, int dohints);
extern Undoes *SCPreserveLayer(SplineChar *sc, int layer, int dohints);
extern Undoes *SCPreserveState(SplineChar *sc, int dohints);
extern Undoes *SCPreserveVWidth(SplineChar *sc);
extern Undoes *SCPreserveWidth(SplineChar *sc);
extern Undoes *_SFPreserveGuide(SplineFont *sf);
extern Undoes *SFPreserveGuide(SplineFont *sf);
extern void BCCopyReference(BDFChar *bc, int pixelsize, int depth);
extern void BCCopySelected(BDFChar *bc, int pixelsize, int depth);
extern void BCDoRedo(BDFChar *bc);
extern void BCDoUndo(BDFChar *bc);
extern void ClipboardClear(void);
extern void CopyBufferClearCopiedFrom(SplineFont *dying);
extern void CopyBufferFree(void);
extern void CopyWidth(CharViewBase *cv, enum undotype ut);
extern void CVCopyGridFit(CharViewBase *cv);
extern void CVDoRedo(CharViewBase *cv);
extern void CVDoUndo(CharViewBase *cv);
extern void CVRemoveTopUndo(CharViewBase *cv);
extern void _CVRestoreTOriginalState(CharViewBase *cv, PressedOn *p);
extern void _CVUndoCleanup(CharViewBase *cv, PressedOn *p);
extern void dumpUndoChain(char *msg, SplineChar *sc, Undoes *undo);
extern void ExtractHints(SplineChar *sc, void *hints, int docopy);
extern void FVCopyAnchors(FontViewBase *fv);
extern void FVCopyWidth(FontViewBase *fv, enum undotype ut);
extern void FVCopy(FontViewBase *fv, enum fvcopy_type copytype);
extern void MVCopyChar(FontViewBase *fv, BDFFont *mvbdf, SplineChar *sc, enum fvcopy_type fullcopy);
extern void PasteAnchorClassMerge(SplineFont *sf, AnchorClass *into, AnchorClass *from);
extern void PasteIntoFV(FontViewBase *fv, int pasteinto, real trans[6]);
extern void PasteIntoMV(FontViewBase *fv, BDFFont *mvbdf, SplineChar *sc, int doclear);
extern void PasteRemoveAnchorClass(SplineFont *sf, AnchorClass *dying);
extern void PasteRemoveSFAnchors(SplineFont *sf);
extern void PasteToBC(BDFChar *bc, int pixelsize, int depth);
extern void PasteToCV(CharViewBase *cv);
extern void SCCopyLookupData(SplineChar *sc);
extern void SCCopyWidth(SplineChar *sc, enum undotype ut);
extern void SCDoRedo(SplineChar *sc, int layer);
extern void SCDoUndo(SplineChar *sc, int layer);
extern void SCUndoSetLBearingChange(SplineChar *sc, int lbc);
extern void *UHintCopy(SplineChar *sc, int docopy);
extern void UndoesFreeButRetainFirstN(Undoes **undopp, int retainAmount);

/* ---- cvimages stubs ---- */

extern void InitImportParams(ImportParams *ip);
extern ImportParams *ImportParamsState(void);
extern void SCAppendEntityLayers(SplineChar *sc, Entity *ent, ImportParams *ip);
extern void SCImportPS(SplineChar *sc, int layer, char *path, bool doclear, ImportParams *ip);
extern void SCImportPDF(SplineChar *sc, int layer, char *path, bool doclear, ImportParams *ip);
extern void SCImportFig(SplineChar *sc, int layer, char *path, bool doclear, ImportParams *ip);
extern void SCImportGlif(SplineChar *sc, int layer, char *path, char *memory, int memlen, bool doclear, ImportParams *ip);
extern void SCImportSVG(SplineChar *sc, int layer, char *path, char *memory, int memlen, bool doclear, ImportParams *ip);
extern void SCImportPSFile(SplineChar *sc, int layer, FILE *ps, bool doclear, ImportParams *ip);
extern void SCImportPDFFile(SplineChar *sc, int layer, FILE *pdf, bool doclear, ImportParams *ip);
extern void SCImportPlateFile(SplineChar *sc, int layer, FILE *plate, bool doclear, ImportParams *ip);
extern int FVImportImages(FontViewBase *fv, char **path_list, int format, int toback, bool preclear, ImportParams *ip);
extern int FVImportImageTemplate(FontViewBase *fv, char *path, int format, int toback, bool preclear, ImportParams *ip);

/* ---- bvedit stubs ---- */

extern void BCTrans(BDFFont *bdf, BDFChar *bc, BVTFunc *bvts, FontViewBase *fv);
extern void BCTransFunc(BDFChar *bc, enum bvtools type, int xoff, int yoff);
extern void skewselect(BVTFunc *bvtf, real t);
extern void BCRotateCharForVert(BDFChar *bc, BDFChar *from, BDFFont *frombdf);
extern void BCExpandBitmapToEmBox(BDFChar *bc, int xmin, int ymin, int xmax, int ymax);
extern void BCSetPoint(BDFChar *bc, int x, int y, int color);
extern void BCFlattenFloat(BDFChar *bc);
extern void BDFFloatFree(BDFFloat *sel);
extern BDFFloat *BDFFloatCopy(BDFFloat *sel);
extern BDFFloat *BDFFloatConvert(BDFFloat *sel, int todepth, int fromdepth);
extern BDFFloat *BDFFloatCreate(BDFChar *bc, int xmin, int xmax, int ymin, int ymax, int clear);
extern void BCPasteInto(BDFChar *bc, BDFChar *rbc, int ixoff, int iyoff, int invert, int cleartoo);
extern void BCMergeReferences(BDFChar *base, BDFChar *cur, int8_t xoff, int8_t yoff);
extern void BCMakeDependent(BDFChar *dependent, BDFChar *base);
extern void BCRemoveDependent(BDFChar *dependent, BDFRefChar *ref);
extern void BCUnlinkThisReference(struct fontviewbase *fv, BDFChar *bc);
extern BDFChar *BDFGetMergedChar(BDFChar *bc);
extern void BCPrepareForOutput(BDFChar *bc, int mergeall);
extern void BCRestoreAfterOutput(BDFChar *bc);
extern void BDFCharFindBounds(BDFChar *bc, IBounds *bb);
extern int BDFCharQuickBounds(BDFChar *bc, IBounds *bb, int8_t xoff, int8_t yoff, int use_backup, int first);
extern BDFFont *BitmapFontScaleTo(BDFFont *old, int to);

/* ---- Missing symbol stubs (pre-existing from first purge) ---- */

extern int copymetadata;
extern int onlycopydisplayed;
extern int preserve_hint_undoes;

extern struct ttf_table *SFFindTable(SplineFont *sf, uint32_t tag);
extern int TTF__getcvtval(SplineFont *sf, int val);
extern int TTF_getcvtval(SplineFont *sf, int val);
extern void UndoesFree(Undoes *undo);

#ifdef __cplusplus
}
#endif

#endif /* FONTFORGE_FORMATSTUBS_H */
