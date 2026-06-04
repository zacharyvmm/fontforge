/* Stubs for removed legacy format support modules.
 * These provide no-op / failure-return implementations so the rest
 * of the library compiles without the removed modules. */

#include <fontforge-config.h>

#include "splinefont.h"
#include "baseviews.h"
#include "formatstubs.h"
#include "psread.h"
#include "splineutil.h"
#include "uiinterface.h"

#include <assert.h>

/* ---- macbinary stubs ---- */

char **NamesReadMacBinary(char *UNUSED(filename)) {
    return NULL;
}

int LoadKerningDataFromMacFOND(SplineFont *UNUSED(sf), char *UNUSED(filename),
                               EncMap *UNUSED(map)) {
    return 0;
}

int WriteMacBitmaps(char *UNUSED(filename), SplineFont *UNUSED(sf),
                    int32_t *UNUSED(sizes), int UNUSED(is_dfont),
                    EncMap *UNUSED(enc)) {
    return 0;
}

int WriteMacFamily(char *UNUSED(filename), struct sflist *UNUSED(sfs),
                   enum fontformat UNUSED(format), enum bitmapformat UNUSED(bf),
                   int UNUSED(flags), int UNUSED(layer)) {
    return 0;
}

int WriteMacPSFont(char *UNUSED(filename), SplineFont *UNUSED(sf),
                   enum fontformat UNUSED(format), int UNUSED(flags),
                   EncMap *UNUSED(enc), int UNUSED(layer)) {
    return 0;
}

int WriteMacTTFFont(char *UNUSED(filename), SplineFont *UNUSED(sf),
                    enum fontformat UNUSED(format), int32_t *UNUSED(bsizes),
                    enum bitmapformat UNUSED(bf), int UNUSED(flags),
                    EncMap *UNUSED(enc), int UNUSED(layer)) {
    return 0;
}

long mactime(void) {
    return 0;
}

SplineChar *SFFindExistingCharMac(SplineFont *UNUSED(sf), EncMap *UNUSED(map),
                                  int UNUSED(unienc)) {
    return NULL;
}

SplineFont *SFReadMacBinary(char *UNUSED(filename), int UNUSED(flags),
                            enum openflags UNUSED(openflags)) {
    return NULL;
}

uint16_t _MacStyleCode(const char *UNUSED(styles), SplineFont *UNUSED(sf),
                       uint16_t *UNUSED(psstylecode)) {
    return 0;
}

void SfListFree(struct sflist *UNUSED(sfs)) {
    /* No-op */
}

/* ---- autosave stubs ---- */

int DoAutoRecoveryExtended(int UNUSED(inquire)) {
    return 0;
}

void DoAutoSaves(void) {
    /* No-op in headless builds */
}

void CleanAutoRecovery(void) {
    /* No-op in headless builds */
}

/* ---- palmfonts stubs ---- */

int WritePalmBitmaps(const char *UNUSED(filename), SplineFont *UNUSED(sf),
                     int32_t *UNUSED(sizes), EncMap *UNUSED(map)) {
    return 0; /* failure */
}

SplineFont *SFReadPalmPdb(char *UNUSED(filename)) {
    return NULL; /* cannot read */
}

/* ---- winfonts stubs ---- */

int FNTFontDump(char *UNUSED(filename), BDFFont *UNUSED(font),
                EncMap *UNUSED(map), int UNUSED(res)) {
    return 0;
}

int FONFontDump(char *UNUSED(filename), SplineFont *UNUSED(sf),
                int32_t *UNUSED(sizes), int UNUSED(resol),
                EncMap *UNUSED(map)) {
    return 0;
}

SplineFont *SFReadWinFON(char *UNUSED(filename), int UNUSED(toback)) {
    return NULL;
}

/* ---- ikarus stubs ---- */

SplineFont *SFReadIkarus(char *UNUSED(fontname)) {
    return NULL;
}

/* ---- metafont stubs ---- */

SplineFont *SFFromMF(char *UNUSED(fontname)) {
    return NULL;
}

/* ---- scripting globals ---- */

int running_script = 0;

/* ---- cvexport stubs ---- */

int _ExportEPS(FILE *UNUSED(eps), SplineChar *UNUSED(sc), int UNUSED(layer),
               int UNUSED(preview)) {
    return 0; /* failure */
}

int ExportEPS(char *UNUSED(filename), SplineChar *UNUSED(sc), int UNUSED(layer)) {
    return 0;
}

/* ---- print globals stubs ---- */

int pagewidth = 0;
int pageheight = 0;
int printtype = -1;

/* ---- zapfnomen stubs ---- */
/* Zapf Dingbats arrays (192 entries for 0x00-0xBF). All zeroed so
 * Zapf-specific code paths are skipped. */
#define ZAPF_SIZE 192
char *zapfnomen[ZAPF_SIZE] = { NULL };
short zapfwx[ZAPF_SIZE] = { 0 };
short zapfbb[ZAPF_SIZE][4] = {{0}};
char zapfexists[ZAPF_SIZE] = { 0 };

/* ================================================================
 * Undo / Clipboard stubs (RALPH-003)
 *
 * Activated by RALPH-005 when cvundoes.c/h and clipnoui.c/h were removed.
 * ================================================================ */
#if 1

/* ---- undo globals ---- */

int no_windowing_ui = 0;
int maxundoes = 0;

/* ---- undo serialize/state functions ---- */

char *UndoToString(SplineChar *UNUSED(sc), Undoes *UNUSED(undo)) {
    return NULL;
}

/* ---- clipboard query functions ---- */

const Undoes *CopyBufferGet(void) {
    return NULL;
}

enum undotype CopyUndoType(void) {
    return (enum undotype)0;
}

int CopyContainsBitmap(void) {
    return 0;
}

int CopyContainsSomething(void) {
    return 0;
}

int CopyContainsVectors(void) {
    return 0;
}

int CVLayer(CharViewBase *UNUSED(cv)) {
    return 0;
}

int getAdobeEnc(const char *UNUSED(name)) {
    return -1;
}

int SCDependsOnSC(SplineChar *UNUSED(parent), SplineChar *UNUSED(child)) {
    return 0;
}

int SCWasEmpty(SplineChar *UNUSED(sc), int UNUSED(skip_this_layer)) {
    return 1; /* treat as empty for safety */
}

RefChar *CopyContainsRef(SplineFont *UNUSED(sf)) {
    return NULL;
}

RefChar *RefCharsCopyState(SplineChar *UNUSED(sc), int UNUSED(layer)) {
    return NULL;
}

SplineSet *ClipBoardToSplineSet(void) {
    return NULL;
}

int SCClipboardHasPasteableContents(void) {
    return 0;
}

/* ---- preserve state functions (return NULL = no undo saved) ---- */

Undoes *BCPreserveState(BDFChar *UNUSED(bc)) {
    return NULL;
}

Undoes *CVPreserveState(CharViewBase *UNUSED(cv)) {
    return NULL;
}

Undoes *CVPreserveStateHints(CharViewBase *UNUSED(cv)) {
    return NULL;
}

Undoes *_CVPreserveTState(CharViewBase *UNUSED(cv), PressedOn *UNUSED(p)) {
    return NULL;
}

Undoes *CVPreserveVWidth(CharViewBase *UNUSED(cv), int UNUSED(vwidth)) {
    return NULL;
}

Undoes *CVPreserveWidth(CharViewBase *UNUSED(cv), int UNUSED(width)) {
    return NULL;
}

Undoes *SCPreserveBackground(SplineChar *UNUSED(sc)) {
    return NULL;
}

Undoes *SCPreserveHints(SplineChar *UNUSED(sc), int UNUSED(layer)) {
    return NULL;
}

Undoes *_SCPreserveLayer(SplineChar *UNUSED(sc), int UNUSED(layer), int UNUSED(dohints)) {
    return NULL;
}

Undoes *SCPreserveLayer(SplineChar *UNUSED(sc), int UNUSED(layer), int UNUSED(dohints)) {
    return NULL;
}

Undoes *SCPreserveState(SplineChar *UNUSED(sc), int UNUSED(dohints)) {
    return NULL;
}

Undoes *SCPreserveVWidth(SplineChar *UNUSED(sc)) {
    return NULL;
}

Undoes *SCPreserveWidth(SplineChar *UNUSED(sc)) {
    return NULL;
}

Undoes *_SFPreserveGuide(SplineFont *UNUSED(sf)) {
    return NULL;
}

Undoes *SFPreserveGuide(SplineFont *UNUSED(sf)) {
    return NULL;
}

/* ---- bitmap clipboard functions ---- */

void BCCopyReference(BDFChar *UNUSED(bc), int UNUSED(pixelsize), int UNUSED(depth)) {
    /* No-op in headless builds */
}

void BCCopySelected(BDFChar *UNUSED(bc), int UNUSED(pixelsize), int UNUSED(depth)) {
    /* No-op in headless builds */
}

void BCDoRedo(BDFChar *UNUSED(bc)) {
    /* No-op in headless builds */
}

void BCDoUndo(BDFChar *UNUSED(bc)) {
    /* No-op in headless builds */
}

/* ---- clipboard management functions ---- */

void ClipboardClear(void) {
    /* No-op in headless builds */
}

void CopyBufferClearCopiedFrom(SplineFont *UNUSED(dying)) {
    /* No-op in headless builds */
}

void CopyBufferFree(void) {
    /* No-op in headless builds */
}

void CopyWidth(CharViewBase *UNUSED(cv), enum undotype UNUSED(ut)) {
    /* No-op in headless builds */
}

void CVCopyGridFit(CharViewBase *UNUSED(cv)) {
    /* No-op in headless builds */
}

void CVDoRedo(CharViewBase *UNUSED(cv)) {
    /* No-op in headless builds */
}

void CVDoUndo(CharViewBase *UNUSED(cv)) {
    /* No-op in headless builds */
}

void CVRemoveTopUndo(CharViewBase *UNUSED(cv)) {
    /* No-op in headless builds */
}

void _CVRestoreTOriginalState(CharViewBase *UNUSED(cv), PressedOn *UNUSED(p)) {
    /* No-op in headless builds */
}

void _CVUndoCleanup(CharViewBase *UNUSED(cv), PressedOn *UNUSED(p)) {
    /* No-op in headless builds */
}

void dumpUndoChain(char *UNUSED(msg), SplineChar *UNUSED(sc), Undoes *UNUSED(undo)) {
    /* No-op in headless builds */
}

void ExtractHints(SplineChar *UNUSED(sc), void *UNUSED(hints), int UNUSED(docopy)) {
    /* No-op in headless builds */
}

void FVCopyAnchors(FontViewBase *UNUSED(fv)) {
    /* No-op in headless builds */
}

void FVCopyWidth(FontViewBase *UNUSED(fv), enum undotype UNUSED(ut)) {
    /* No-op in headless builds */
}

void FVCopy(FontViewBase *UNUSED(fv), enum fvcopy_type UNUSED(copytype)) {
    /* No-op in headless builds */
}

void MVCopyChar(FontViewBase *UNUSED(fv), BDFFont *UNUSED(mvbdf),
                SplineChar *UNUSED(sc), enum fvcopy_type UNUSED(fullcopy)) {
    /* No-op in headless builds */
}

void PasteAnchorClassMerge(SplineFont *UNUSED(sf), AnchorClass *UNUSED(into),
                           AnchorClass *UNUSED(from)) {
    /* No-op in headless builds */
}

void PasteIntoFV(FontViewBase *UNUSED(fv), int UNUSED(pasteinto),
                 real UNUSED(trans[6])) {
    /* No-op in headless builds */
}

void PasteIntoMV(FontViewBase *UNUSED(fv), BDFFont *UNUSED(mvbdf),
                 SplineChar *UNUSED(sc), int UNUSED(doclear)) {
    /* No-op in headless builds */
}

void PasteRemoveAnchorClass(SplineFont *UNUSED(sf), AnchorClass *UNUSED(dying)) {
    /* No-op in headless builds */
}

void PasteRemoveSFAnchors(SplineFont *UNUSED(sf)) {
    /* No-op in headless builds */
}

void PasteToBC(BDFChar *UNUSED(bc), int UNUSED(pixelsize), int UNUSED(depth)) {
    /* No-op in headless builds */
}

void PasteToCV(CharViewBase *UNUSED(cv)) {
    /* No-op in headless builds */
}

void SCCopyLookupData(SplineChar *UNUSED(sc)) {
    /* No-op in headless builds */
}

void SCCopyWidth(SplineChar *UNUSED(sc), enum undotype UNUSED(ut)) {
    /* No-op in headless builds */
}

void SCDoRedo(SplineChar *UNUSED(sc), int UNUSED(layer)) {
    /* No-op in headless builds */
}

void SCDoUndo(SplineChar *UNUSED(sc), int UNUSED(layer)) {
    /* No-op in headless builds */
}

void SCUndoSetLBearingChange(SplineChar *UNUSED(sc), int UNUSED(lbc)) {
    /* No-op in headless builds */
}

void *UHintCopy(SplineChar *UNUSED(sc), int UNUSED(docopy)) {
    return NULL;
}

void UndoesFreeButRetainFirstN(Undoes **UNUSED(undopp), int UNUSED(retainAmount)) {
    /* No-op in headless builds */
}

#endif /* 1 — undo/clipboard stubs */

/* ================================================================
 * cvimages stubs (RALPH-009)
 *
 * Background image support removed. Entity-to-layer conversion
 * (SCAppendEntityLayers) retained for PS/PDF/SVG import.
 * ================================================================ */

void InitImportParams(ImportParams *ip) {
    assert( ip!=NULL );
    memset(ip, 0, sizeof(ImportParams));
    ip->initialized = true;
    ip->correct_direction = true;
    ip->simplify = true;
    ip->clip = true;
    ip->scale = true;
    ip->accuracy_target = 0.25;
    ip->default_joinlimit = JLIMIT_INHERITED;
}

ImportParams *ImportParamsState(void) {
    static ImportParams ips;

    if ( !ips.initialized )
	InitImportParams(&ips);

    return &ips;
}

void SCAppendEntityLayers(SplineChar *sc, Entity *ent, ImportParams *ip) {
    int cnt, pos;
    Entity *e, *enext;
    Layer *old = sc->layers;
    SplineSet *ss;

    for ( e=ent, cnt=0; e!=NULL; e=e->next, ++cnt );
    pos = sc->layer_cnt;
    if ( cnt==0 )
return;
    EntityDefaultStrokeFill(ent);

    sc->layers = realloc(sc->layers,(sc->layer_cnt+cnt)*sizeof(Layer));
    for ( pos = sc->layer_cnt, e=ent; e!=NULL ; e=enext, ++pos ) {
	enext = e->next;
	LayerDefault(&sc->layers[pos]);
	sc->layers[pos].splines = NULL;
	sc->layers[pos].refs = NULL;
	sc->layers[pos].images = NULL;
	if ( e->type == et_splines ) {
	    sc->layers[pos].dofill = e->u.splines.fill.col != 0xffffffff;
	    sc->layers[pos].dostroke = e->u.splines.stroke.col != 0xffffffff;
	    if ( !sc->layers[pos].dofill && !sc->layers[pos].dostroke )
		sc->layers[pos].dofill = true;		/* If unspecified, assume an implied fill in BuildGlyph */
	    sc->layers[pos].fill_brush.col = e->u.splines.fill.col==0xffffffff ?
		    COLOR_INHERITED : e->u.splines.fill.col;
	    sc->layers[pos].fill_brush.gradient = e->u.splines.fill.grad;
	    sc->layers[pos].stroke_pen.brush.col = e->u.splines.stroke.col==0xffffffff ? COLOR_INHERITED : e->u.splines.stroke.col;
	    sc->layers[pos].stroke_pen.brush.gradient = e->u.splines.stroke.grad;
	    sc->layers[pos].stroke_pen.width = e->u.splines.stroke_width;
	    sc->layers[pos].stroke_pen.linejoin = e->u.splines.join;
	    sc->layers[pos].stroke_pen.linecap = e->u.splines.cap;
	    memcpy(sc->layers[pos].stroke_pen.trans, e->u.splines.transform,
		    4*sizeof(real));
	    sc->layers[pos].splines = e->u.splines.splines;
	} else if ( e->type == et_image ) {
	    /* Background image support removed — skip image entities */
	}
	if ( e->clippath ) {
	    for ( ss=e->clippath; ss->next!=NULL; ss=ss->next )
		ss->is_clip_path = true;
	    ss->is_clip_path = true;
	    ss->next = sc->layers[pos].splines;
	    sc->layers[pos].splines = e->clippath;
	}
	free(e);
    }
    sc->layer_cnt += cnt;
    SCMoreLayers(sc,old);
}

/* ---- SCImport stubs (dead — callers were in removed FV* batch functions) ---- */

void SCImportPS(SplineChar *UNUSED(sc), int UNUSED(layer), char *UNUSED(path),
                bool UNUSED(doclear), ImportParams *UNUSED(ip)) {
    /* No-op — image import removed */
}

void SCImportPDF(SplineChar *UNUSED(sc), int UNUSED(layer), char *UNUSED(path),
                 bool UNUSED(doclear), ImportParams *UNUSED(ip)) {
    /* No-op — image import removed */
}

void SCImportFig(SplineChar *UNUSED(sc), int UNUSED(layer), char *UNUSED(path),
                 bool UNUSED(doclear), ImportParams *UNUSED(ip)) {
    /* No-op — image import removed */
}

void SCImportGlif(SplineChar *UNUSED(sc), int UNUSED(layer), char *UNUSED(path),
                  char *UNUSED(memory), int UNUSED(memlen), bool UNUSED(doclear),
                  ImportParams *UNUSED(ip)) {
    /* No-op — image import removed */
}

void SCImportSVG(SplineChar *UNUSED(sc), int UNUSED(layer), char *UNUSED(path),
                 char *UNUSED(memory), int UNUSED(memlen), bool UNUSED(doclear),
                 ImportParams *UNUSED(ip)) {
    /* No-op — image import removed */
}

void SCImportPSFile(SplineChar *UNUSED(sc), int UNUSED(layer), FILE *UNUSED(ps),
                    bool UNUSED(doclear), ImportParams *UNUSED(ip)) {
    /* No-op — image import removed */
}

void SCImportPDFFile(SplineChar *UNUSED(sc), int UNUSED(layer), FILE *UNUSED(pdf),
                     bool UNUSED(doclear), ImportParams *UNUSED(ip)) {
    /* No-op — image import removed */
}

void SCImportPlateFile(SplineChar *UNUSED(sc), int UNUSED(layer),
                       FILE *UNUSED(plate), bool UNUSED(doclear),
                       ImportParams *UNUSED(ip)) {
    /* No-op — image import removed */
}

/* ---- FV batch import stubs (dead — callers were in removed fontviewbase.c) ---- */

int FVImportImages(FontViewBase *UNUSED(fv), char **UNUSED(path_list),
                   int UNUSED(format), int UNUSED(toback),
                   bool UNUSED(preclear), ImportParams *UNUSED(ip)) {
    return 0; /* failure */
}

int FVImportImageTemplate(FontViewBase *UNUSED(fv), char *UNUSED(path),
                          int UNUSED(format), int UNUSED(toback),
                          bool UNUSED(preclear), ImportParams *UNUSED(ip)) {
    return 0; /* failure */
}

/* ================================================================
 * bvedit stubs (RALPH-010)
 *
 * Bitmap glyph editing removed. All bitmap transform / reference
 * management / output prep functions stubbed as no-ops.
 * ================================================================ */

/* ---- bitmap transform functions ---- */

void BCTrans(BDFFont *UNUSED(bdf), BDFChar *UNUSED(bc),
             BVTFunc *UNUSED(bvts), FontViewBase *UNUSED(fv)) {
    /* No-op in headless builds */
}

void BCTransFunc(BDFChar *UNUSED(bc), enum bvtools UNUSED(type),
                 int UNUSED(xoff), int UNUSED(yoff)) {
    /* No-op in headless builds */
}

void skewselect(BVTFunc *UNUSED(bvtf), real UNUSED(t)) {
    /* No-op in headless builds */
}

void BCRotateCharForVert(BDFChar *UNUSED(bc), BDFChar *UNUSED(from),
                         BDFFont *UNUSED(frombdf)) {
    /* No-op in headless builds */
}

void BCExpandBitmapToEmBox(BDFChar *UNUSED(bc),
                           int UNUSED(xmin), int UNUSED(ymin),
                           int UNUSED(xmax), int UNUSED(ymax)) {
    /* No-op in headless builds */
}

void BCSetPoint(BDFChar *UNUSED(bc), int UNUSED(x), int UNUSED(y),
                int UNUSED(color)) {
    /* No-op in headless builds */
}

/* ---- bitmap float / selection functions ---- */

void BCFlattenFloat(BDFChar *UNUSED(bc)) {
    /* No-op in headless builds */
}

void BDFFloatFree(BDFFloat *UNUSED(sel)) {
    /* No-op in headless builds — allowed leak for simplicity */
}

BDFFloat *BDFFloatCopy(BDFFloat *UNUSED(sel)) {
    return NULL;
}

BDFFloat *BDFFloatConvert(BDFFloat *UNUSED(sel),
                          int UNUSED(todepth), int UNUSED(fromdepth)) {
    return NULL;
}

BDFFloat *BDFFloatCreate(BDFChar *UNUSED(bc),
                         int UNUSED(xmin), int UNUSED(xmax),
                         int UNUSED(ymin), int UNUSED(ymax),
                         int UNUSED(clear)) {
    return NULL;
}

/* ---- bitmap reference management ---- */

void BCPasteInto(BDFChar *UNUSED(bc), BDFChar *UNUSED(rbc),
                 int UNUSED(ixoff), int UNUSED(iyoff),
                 int UNUSED(invert), int UNUSED(cleartoo)) {
    /* No-op in headless builds */
}

void BCMergeReferences(BDFChar *UNUSED(base), BDFChar *UNUSED(cur),
                       int8_t UNUSED(xoff), int8_t UNUSED(yoff)) {
    /* No-op in headless builds */
}

void BCMakeDependent(BDFChar *UNUSED(dependent), BDFChar *UNUSED(base)) {
    /* No-op in headless builds */
}

void BCRemoveDependent(BDFChar *UNUSED(dependent), BDFRefChar *UNUSED(ref)) {
    /* No-op in headless builds */
}

void BCUnlinkThisReference(struct fontviewbase *UNUSED(fv),
                           BDFChar *UNUSED(bc)) {
    /* No-op in headless builds */
}

BDFChar *BDFGetMergedChar(BDFChar *UNUSED(bc)) {
    return NULL; /* caller should use raw glyph */
}

/* ---- bitmap output prep / restore ---- */

void BCPrepareForOutput(BDFChar *UNUSED(bc), int UNUSED(mergeall)) {
    /* No-op in headless builds */
}

void BCRestoreAfterOutput(BDFChar *UNUSED(bc)) {
    /* No-op in headless builds */
}

/* ---- bitmap bounds queries ---- */

void BDFCharFindBounds(BDFChar *bc, IBounds *bb) {
    /* Return stored bounds from the BDFChar — a reasonable approximation
     * for a headless build where bitmap editing is not performed. */
    if (bc) {
        bb->minx = bc->xmin;
        bb->maxx = bc->xmax;
        bb->miny = bc->ymin;
        bb->maxy = bc->ymax;
    } else {
        bb->minx = bb->maxx = bb->miny = bb->maxy = 0;
    }
}

int BDFCharQuickBounds(BDFChar *bc, IBounds *bb,
                       int8_t xoff, int8_t yoff,
                       int UNUSED(use_backup), int UNUSED(first)) {
    /* Return stored bounds — reasonable approximation for headless builds */
    if (bc) {
        bb->minx = bc->xmin + xoff;
        bb->maxx = bc->xmax + xoff;
        bb->miny = bc->ymin + yoff;
        bb->maxy = bc->ymax + yoff;
    } else {
        bb->minx = bb->maxx = bb->miny = bb->maxy = 0;
    }
    return true;
}

/* ---- bitmap font scaling ---- */

BDFFont *BitmapFontScaleTo(BDFFont *UNUSED(old), int UNUSED(to)) {
    return NULL;
}

/* ================================================================
 * Missing symbol stubs (pre-existing from first purge)
 *
 * These symbols were referenced but their definitions were removed
 * in the first purge pass. The shared library built successfully
 * because no executable linked against it; now that we have a test
 * harness, we must provide stub implementations.
 * ================================================================ */

/* ---- math constants descriptor (was in mathconstants.c) ---- */

struct math_constants_descriptor math_constants_descriptor[] = {
    MATH_CONSTANTS_DESCRIPTOR_EMPTY  /* sentinel entry; loops iterate until script_name==NULL */
};

/* ---- glyph/instruction metadata globals ---- */

int copymetadata = 0;
int onlycopydisplayed = 0;
int preserve_hint_undoes = 0;
int use_utf8_in_script = 1;

/* ---- MacStyleCode (was in removed GUI code) ---- */

uint16_t MacStyleCode(SplineFont *UNUSED(sf), uint16_t *psstyle) {
    if (psstyle) *psstyle = 0;
    return 0; /* not bold, not italic */
}

/* ---- NOUI string tables (UI replacements for headless builds) ---- */

const char *NOUI_MSLangString(int UNUSED(language)) {
    return "English";
}

const char *NOUI_TTFNameIds(int UNUSED(id)) {
    return NULL;
}

/* ---- SFFindTable (was in nowakowskittfinstr.c) ---- */

struct ttf_table *SFFindTable(SplineFont *sf, uint32_t tag) {
    struct ttf_table *tab;
    for (tab = sf->ttf_tables; tab != NULL && tab->tag != tag; tab = tab->next);
    return tab;
}

/* ---- TTF CVT value lookup stubs (was in nowakowskittfinstr.c) ---- */

int TTF__getcvtval(SplineFont *UNUSED(sf), int UNUSED(val)) {
    return -1; /* not found */
}

int TTF_getcvtval(SplineFont *UNUSED(sf), int UNUSED(val)) {
    return 0;
}

/* ---- UndoesFree (was in removed undo module) ---- */

void UndoesFree(Undoes *UNUSED(undo)) {
    /* No-op in headless builds — undo chains are never created */
}
