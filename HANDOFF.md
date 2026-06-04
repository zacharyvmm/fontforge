# HANDOFF.md — FontForge Library: State & Vision

**Date**: 2026-06-04
**Goal**: Transform FontForge into the smallest viable C library for font
manipulation, then port to Rust.
**Status**: Two purge passes complete. Builds clean. Ready for Rust FFI work.

---

## 1. What We Have Now

A headless `libfontforge.so` (1,602 dynamic symbols, 15.8 MB) built from
~249K LOC of C/C++ across 94 source files in 4 directories.

| Directory | LOC | Files | Role |
|-----------|-----|-------|------|
| `fontforge/` | ~185,000 | 71 (.c/.cpp/.h) | Core library |
| `gutils/` | ~5,400 | 23 | Image I/O, filesystem, utilities |
| `Unicode/` | ~12,000 | 8 | Unicode tables, ustring |
| `extern/` | ~2,500 | 2 | Third-party (mINI, cxxopts) |
| `inc/` | ~3,800 | 14 | Public & internal headers |
| `tests/` | ~150 | 2 | Minimal test harness |

**Build**: `cmake .. -DBUILD_SHARED_LIBS=ON && make` — 100% success.
**Tests**: `ctest` — 1/1 pass (SFD round-trip).
**CI**: Single Linux job (build + test + dist) in `.github/workflows/main.yml`.

---

## 2. What Was Removed (Both Purges)

### First Purge (~150,000 LOC)

Removed the entire GUI application and associated infrastructure:

- `fontforgeexe/` — GTK GUI (~85K LOC)
- `gdraw/` — Custom UI toolkit (~50K LOC)
- `python.cpp` / `pyhook/` — Python bindings (~22K LOC)
- `scripting.cpp` — Native scripting interpreter (~11K LOC)
- `scstyles.c` / `glyphcomp.c` / `effects.c` / `search.c` — UI-only features (~9K LOC)
- `desktop/`, `osx/`, `po/`, `doc/`, `tests/` — platform/docs (~30K LOC)
- `inc/` headers — 13 deleted (gwidget, ggadget, gdraw, gresource, hotkeys, ffglib, etc.)

### Second Purge (~7,000 LOC)

Removed undo/clipboard, background images, bitmap editing, and dead infrastructure:

- `cvundoes.c/h` (3,534 LOC) — Undo/redo/copy/paste. 63 stubs in formatstubs.c.
- `clipnoui.c/h` (67 LOC) — No-UI clipboard stubs.
- `fontviewbase.c` (1,426 → 728 LOC) — Stripped 42 dead FV* batch functions.
  Kept FVTrans, FVTransFunc, and fv_interface/mv_interface defaults.
- `cvimages.c/h` (1,189 LOC) — Background images. SCAppendEntityLayers preserved.
- `bvedit.c/h` (1,005 LOC) — Bitmap editing. 22 stubs.
- `autosave.c/h` — Already removed in first purge; stubs confirmed.
- `mathconstants.c/h` (224 LOC) — OpenType MATH. Single call site guarded.
- `gutils/prefs.c` + `inc/prefs.h` (180 LOC) — Dead preference parsing.
- `Packaging/` (42 files) — Debian/RedHat/AppImage/Windows packaging.
- `pyproject.toml`, `README_PYPI.md`, `WHEEL_BUILD_STATUS.md`, `_build_meta/` — Python wheel infra.
- Dead CMake options: `ENABLE_NATIVE_SCRIPTING`, `ENABLE_PYTHON_SCRIPTING`, `ENABLE_PYTHON_EXTENSION`.
- Dead CI: `wheels.yml`, `appveyor.yml` deleted; `main.yml` rewritten (230→46 lines).
- Dead declarations from `fontforge/fontforge.h` (7 removed).
- Stale `#include "ffglib.h"` from `gutils/fsys.cpp`.

### What Was Evaluated and KEPT

| Module | LOC | Rationale |
|--------|-----|-----------|
| `macenc.c/h` | 2,364 | Essential for Mac cmap subtable I/O and AAT feature tables |
| `spiro.c/h` + `bezctx_ff.c/h` | 500 | Deeply embedded in SplineSet struct (298+ references) |
| `stemdb.c/h` + `ttfinstrs.c/h` + `autohint.c` | ~10,100 | Auto-hinting kept for now; optional removal later |

---

## 3. Stub Infrastructure

All removed functionality is stubbed in two files:

- **`fontforge/formatstubs.c`** (779 lines) — 150+ no-op/return-NULL stubs for
  undo, clipboard, bitmap editing, background images, autosave, palm/win/mac/ikarus
  format I/O, print globals, zapf dingbats, scripting globals, and prefs.
- **`fontforge/formatstubs.h`** (181 lines) — Extern declarations for all stubs.

Stubs follow the project pattern: void functions are no-ops, pointer-returning
functions return NULL, int-returning functions return 0. Two stubs provide
sensible defaults: `BDFCharFindBounds`/`BDFCharQuickBounds` (return stored
xmin/xmax/ymin/ymax for BDF output), and `SFFindTable` (real implementation
for TTF table lookup).

---

## 4. What Remains That Could Still Be Removed

### 4.1 gutils/gimage* — Image I/O (~3,750 LOC)

18 files for PNG, JPEG, GIF, TIFF, BMP, XPM, XBM, RAS, RGB image reading/writing.
With `cvimages.c` removed, the primary consumer is gone. These are still compiled
and linked into `libfontforge.so`.

**Current consumers of GImage types:**
- `sfd.cpp` — reads/writes glyph background image data in SFD format
- `svg.c` — SVG image element handling
- `dumppfa.c` — PFA bitmap font output
- `fvimportbdf.c` — BDF bitmap import (creates GImage objects)
- `psread.c` — PostScript Type 3 image operators
- `splinefill.c` — fill operations on image data
- `sflayout.cpp` — glyph layout rendering (FontImage/SFDefaultImage)

**Approach**: The image-related code paths in these files handle glyph background
images and bitmap strikes — editor features, not core font manipulation. Each
consumer would need its image paths stubbed (skip image data in SFD, return empty
images in SVG, etc.). After that, all 18 gimage files can be removed from
`gutils/CMakeLists.txt` and deleted.

**Savings**: ~3,750 LOC, 18 files, plus removal of libpng/libjpeg/libtiff/libgif
optional dependencies.

### 4.2 stemdb + ttfinstrs + autohint — Auto-hinting (~10,100 LOC)

The auto-hinting pipeline:
- `autohint.c` (3,448 LOC) — main auto-hinting algorithm
- `stemdb.c` (6,088 LOC, 6th largest file) — stem detection database
- `ttfinstrs.c` (577 LOC) — TrueType instruction bytecode assembly

If auto-hinting is not required by the Rust port (hinting can be done in Rust
or with external tools), these can be removed. `autohint.c` is called from
`splinefont.c`, `fontviewbase.c`, and `splineutil2.c`. The call sites would
be guarded or stubbed.

**Savings**: ~10,100 LOC, 3 files.

### 4.3 macenc.c/h — Mac Encoding Tables (~2,360 LOC)

Mac OS character encoding tables used for reading/writing Mac-encoded cmap
subtables in TrueType fonts. If the Rust port does not need to support legacy
Mac-encoded TrueType fonts, this can be stubbed (`MacEncToUnicode` → identity
mapping, `MacFeatureAdd` → no-op).

**Savings**: ~2,360 LOC.

### 4.4 Expanded Test Suite

Currently only one test exists: SFD round-trip with 4 glyphs. The HANDOFF from
the first purge recommended restoring tests before the Rust port. Priority tests:

| Test | What It Validates |
|------|-------------------|
| TTF round-trip | Read .ttf → save .ttf → verify font data intact |
| OTF round-trip | Read .otf → save .otf → verify OpenType layout tables |
| UFO round-trip | Read .ufo → save .ufo → verify glyph/spline fidelity |
| SVG import | Read .svg font → verify glyph contours |
| Spline ops | Simplify, overlap removal, direction correction |
| Property-based | Random fonts through save→load→compare pipeline |

---

## 5. Vision: The Rust Port

The entire purpose of slimming the C codebase is to create a minimal,
well-understood FFI surface for a Rust font manipulation library.

### 5.1 Architecture

```
┌─────────────────────────────────────────────────┐
│                 Rust Library                      │
│  (font I/O, spline ops, OpenType layout,        │
│   validation, hinting)                           │
├─────────────────────────────────────────────────┤
│              C FFI Bindings                       │
│  (bindgen-generated or hand-written extern "C")  │
├─────────────────────────────────────────────────┤
│           libfontforge.so                         │
│  (thin C shim — only what hasn't been ported)    │
└─────────────────────────────────────────────────┘
```

The port should be incremental:
1. Identify functions by category (font I/O, spline ops, OpenType layout)
2. Write Rust equivalents, validated against C output
3. Cut over to Rust implementation, remove C code
4. Eventually delete `libfontforge.so` entirely

### 5.2 FFI Surface

The natural API boundary is at the `SplineFont*` / `SplineChar*` level:

**Font I/O**:
- `SFReadSFD()`, `SFWriteSFD()`
- `SFReadTTF()`, `SFWriteTTF()`
- `_ReadUFO()`, `_WriteUFO()`
- `SFReadSVG()`
- `ReadPSFont()` (Type 1)

**Spline manipulation**:
- `SplineCharSimplify()`, `SplineCharOverlapRemove()`
- `SplineCharAddExtrema()`, `SplineCharCorrectDir()`
- `SplinePointListTransform()`, `SplineTransform()`

**OpenType layout**:
- `FeatRead()` (feature file parsing)
- GPOS/GSUB table construction functions
- Lookup/chaining context builders

**Key fact**: The `FontViewBase` / `CharViewBase` / `Undoes` wrapper types
have been removed or neutered in the second purge. The FFI binds directly
against `SplineFont` and `SplineChar` structs — no view layer indirection.

### 5.3 What to Keep in C (if anything)

Some components may be impractical to port:
- **namelist.c** (20,797 LOC) — Adobe Glyph List. A static lookup table that could
  be code-generated in Rust or loaded from a data file at runtime.
- **Unicode/** (12,000 LOC) — Unicode character database tables. Could be replaced
  by the `unicode-*` Rust crates.
- **featurefile.c** (7,573 LOC) — OpenType feature file parser. Complex and
  battle-tested; may be worth keeping as C or wrapping via FFI.

---

## 6. Immediate Next Steps (Ordered)

### Step 1: Git cleanup

The working directory has two untracked files (`.pi/`, `.ralph/`) containing
Ralph agent planning artifacts. These are development process files, not project
deliverables. Add to `.gitignore` or commit separately.

The second purge changes are already committed under the message `Refactored
into a smaller font management library`. A more specific commit message is
warranted — squash or amend if appropriate.

### Step 2: Complete verification

Run the third verification pass (build → ctest → stale ref check → symbol check).
Status after pass 2/3: all checks pass, zero defects.

### Step 3: Decide on gimage cleanup

The 18 `gutils/gimage*` files are the largest remaining dead-code block
flagged by the original HANDOFF audit. Decision needed:
- **Remove now**: stub image paths in SFD/SVG/PFA/BDF consumers (~1 day)
- **Defer**: leave for Rust port phase; remove when porting those consumers

### Step 4: Begin Rust port

Starting point: use `bindgen` on the public headers to generate raw FFI
bindings. Then port one category at a time, validating each step with
format round-trip tests.

Recommended order:
1. **SFD read/write** — the native format, well-understood, good test vehicle
2. **Spline manipulation** — core algorithms, self-contained
3. **TTF/OTF I/O** — most complex, many edge cases
4. **OpenType layout** — feature files, GPOS/GSUB
5. **UFO, SVG, Type 1** — secondary formats
6. **Hinting** — either port or defer (external tools exist)

### Step 5: Remove C entirely (endgame)

Once all functionality is ported and validated, delete `libfontforge.so` and
all C source. The Rust library stands alone.

---

## 7. Summary Statistics

| Metric | Original | After Purge 1 | After Purge 2 |
|--------|----------|---------------|---------------|
| Total LOC | ~350,000 | ~251,000 | ~249,000 |
| Source files compiled | ~250 | ~85 | ~94 |
| Undo/clipboard | Present | Still present | Removed |
| Bitmap editing | Present | Still present | Stubbed |
| Background images | Present | Still present | Stubbed |
| Image I/O (gimage) | Present | Still present | Still present |
| Auto-hinting | Present | Still present | Still present |
| GUI | Present | Removed | Removed |
| Python bindings | Present | Removed | Removed |
| Scripting | Present | Removed | Removed |
| Wheel/CI/Packaging | Present | Partially broken | Cleaned |
| Tests | Present | Removed | Minimal (1 test) |

### Build & Test

```bash
mkdir build && cd build
cmake .. -DBUILD_SHARED_LIBS=ON
make -j$(nproc)
# Produces: build/lib/libfontforge.so (1602 symbols, 15.8 MB)

ctest
# 1/1 pass: test_roundtrip (SFD save→load→verify)
```

---

## 8. Project Files Reference

| File | Purpose |
|------|---------|
| `fontforge/formatstubs.c` | 150+ stubs for removed functionality |
| `fontforge/formatstubs.h` | Extern declarations for all stubs |
| `fontforge/fontviewbase.c` | FVTrans, FVTransFunc, fv_interface defaults (728 LOC) |
| `fontforge/noprefs.c` | Stub prefs_interface (NOUI_SavePrefs, etc.) |
| `fontforge/start.c` | Library init (InitSimpleStuff, DoInit) |
| `tests/test_roundtrip.c` | SFD round-trip test (4 glyphs) |
| `.github/workflows/main.yml` | CI: Linux build + test + dist |
| `.ralph/` | Ralph agent planning/state/runs (development artifacts) |

---

## 9. Decisions Made (Recorded for Future Context)

1. **fv_interface pattern preserved** — Option A from fontviewbase audit.
   Full FontViewBase removal (Option B) would touch 29 macro call sites across
   8 files; not worth the risk at this stage.

2. **macenc, spiro, bezctx_ff kept** — Too deeply integrated to remove
   without structural changes to SplineSet or breaking Mac cmap support.

3. **autohint/stemdb/ttfinstrs kept** — Deferred decision. Auto-hinting is
   a candidate for Rust implementation or external tooling.

4. **gimage deferred** — The 18 image I/O files remain. This is the
   largest remaining cleanup opportunity and should be addressed before
   or during early Rust port phases.

5. **_NO_PYTHON and _NO_FFSCRIPT hardcoded to 1** — These macros are
   defined unconditionally in CMakeLists.txt and FontForgeConfigure.cmake.
   Any code paths guarded by `#ifndef _NO_PYTHON` or `#ifndef _NO_FFSCRIPT`
   are excluded from the build.
