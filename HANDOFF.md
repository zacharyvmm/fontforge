# HANDOFF.md — FontForge Library: Phase 2 Progress & Remaining Work

**Date**: 2026-06-04 (updated after iteration 20)
**Status**: Phase 2 in progress. Groups A, B complete. Group C 50% done.
Groups D, E pending.
**Goal**: Transform FontForge into the smallest viable C library for font
manipulation, then port to Rust. Phase 1 delivered a slimmer C library with
Rust FFI bindings and 3 core module ports. Phase 2 is ~60% complete.

---

## 1. Phase 1 Summary — What We Have Now

A headless `libfontforge.so` (2,207 dynamic symbols, 15 MB) built from ~188K
LOC of C/C++ plus a Rust workspace with `bindgen`-generated FFI bindings and
9 integration tests covering SFD, spline ops, and TTF/OTF round-trips.

### C Library

| Directory | LOC | Files | Role |
|-----------|-----|-------|------|
| `fontforge/` | ~171,000 | 67 (.c/.cpp/.h) | Core library |
| `gutils/` | ~1,700 | 5 | Filesystem, color conversion, mINI |
| `Unicode/` | ~12,000 | 8 | Unicode tables, ustring |
| `extern/` | ~2,500 | 2 | Third-party (mINI, cxxopts) |
| `inc/` | ~3,800 | 14 | Public & internal headers |
| `tests/` | ~300 | 2 | C test harness (SFD + TTF round-trip) |

**Build**: `cmake .. -DBUILD_SHARED_LIBS=ON && make` — 100% success, 0 errors.
**C tests**: `ctest` — 2/2 pass (test_roundtrip, test_ttf_roundtrip).
**CI**: Single Linux job (build + test + dist) in `.github/workflows/main.yml`.

### Rust Workspace

| Crate | Role |
|-------|------|
| `Cargo.toml` (root) | Workspace root |
| `rust-bindings/` | `bindgen`-generated FFI bindings + integration tests |

**Rust tests**: `cargo test` — 9/9 pass:
- 2 FFI smoke tests (SplineFont create/free)
- 1 SFD round-trip (4 glyphs, write→reload→verify)
- 4 spline manipulation tests (overlap removal, add extrema, simplify, correct direction)
- 2 TTF/OTF round-trips (write→reload→verify)

### What Was Removed (3 Purges)

- **GUI**: fontforgeexe/, gdraw/ (~135K LOC)
- **Scripting/Python**: python.cpp, pyhook/, scripting.cpp (~33K LOC)
- **Undo/Clipboard**: cvundoes.c/h (3,534 LOC), clipnoui.c/h (67 LOC)
- **Bitmap Editing**: bvedit.c/h (1,005 LOC)
- **Background Images**: cvimages.c/h (1,189 LOC)
- **Image I/O**: 18 gutils/gimage* files (~3,700 LOC, 4 library deps dropped)
- **Misc**: autosave, mathconstants, prefs, packaging, CI, wheel infra
- **Stubs**: 150+ stubs in `fontforge/formatstubs.c/h` for all removed functionality

### What Was Kept

| Module | LOC | Rationale |
|--------|-----|-----------|
| `macenc.c/h` | 2,364 | Mac cmap subtable I/O and AAT feature tables |
| `spiro.c/h` + `bezctx_ff.c/h` | 500 | Deeply embedded in SplineSet struct |
| `stemdb.c` + `ttfinstrs.c` + `autohint.c` | ~10,100 | Auto-hinting (deferred decision) |
| `namelist.c` | 20,797 | Adobe Glyph List lookup table |
| `Unicode/` | ~12,000 | Unicode character database tables |
| `featurefile.c` | 7,573 | OpenType feature file parser |

---

## 2. Phase 2: Deep Rust Port — Progress

Phase 2 continues the incremental port. The goal is to move as much
functionality as possible from C into Rust, reducing the C surface to
a set of FFI-wrapped stragglers or zero.

### 2.1 What's Done (RALPH-015 through RALPH-020)

#### Group A: Remaining Format I/O — ✅ COMPLETE

| Format | Rust Source | Test File | Status |
|--------|-------------|-----------|--------|
| **UFO** | `src/ufo.rs` (writer) + `src/ufo_read.rs` (reader) | `tests/test_ufo_roundtrip.rs` | Pure-Rust, quick-xml. V2/V3. Round-trip test passes. ✅ |
| **SVG** | `src/svg.rs` | `tests/test_svg_import.rs` | Pure-Rust SVG font import. Path parsing + contour extraction. ✅ |
| **BDF** | `src/bdf.rs` | `tests/test_bdf_roundtrip.rs` | Pure-Rust BDF import/export. Round-trip test passes. ✅ |
| **PS Type 1** | `src/pstype1.rs` | `tests/test_pstype1_roundtrip.rs` | Pure-Rust PFA writer (eexec encryption, charstring encoding). Reads via C FFI. Round-trip + structure tests pass. ✅ |
| **SFD** | (via C FFI from Phase 1) | `tests/test_sfd_roundtrip.rs` | Done in Phase 1. ✅ |
| **TTF/OTF** | (via C FFI from Phase 1) | `tests/test_ttf_otf_roundtrip.rs` | Done in Phase 1. ✅ |

**All 7 format I/O paths have working Rust round-trip tests.**

#### Group B: OpenType Layout FFI — ✅ COMPLETE

| Artifact | Description |
|----------|-------------|
| `src/ot_layout.rs` | Safe Rust wrapper with tag constants (scripts, langs, features) and wrappers for apply_feature_file, find_lookup, has_gsub/has_gpos, lookup_count_in_feature |
| `tests/test_ot_layout.rs` | 3 integration tests: feature file apply + round-trip, tag utilities, feature navigation |
| FFI allowlists | 13 types (OTLookup, FeatureScriptLangList, lookup_subtable, FPST, etc.) + 14 functions |

**Strategy**: FFI-first (as recommended). C code stays; Rust wraps it safely. ✅

#### Group C: Spline Algorithms (Pure Rust) — 🔄 50% DONE

| Task | Status | Notes |
|------|--------|-------|
| RALPH-020: Overlap removal | ✅ DONE | `src/overlap.rs` (~400 LOC). Polygon-based boolean union. 3 integration tests vs C output match. Handles proper/vertex/collinear intersections. |
| RALPH-021: Simplify/Extrema/Direction | ❌ PENDING | Pure-Rust equivalents for SplineCharSimplify, SplineCharAddExtrema, SplineSetsCorrect. |

### 2.2 What's Remaining

#### RALPH-021: Spline simplify/extrema/direction in pure Rust (NEXT TASK)

Port the remaining spline operations from C FFI calls to pure Rust:
- **SplineCharSimplify**: Remove redundant points, merge colinear edges
- **SplineCharAddExtrema**: Insert points at curve extrema (x/y-axis aligned)
- **SplineSetsCorrect**: Ensure clockwise outer / counter-clockwise inner contours

Validation: compare pure-Rust output character-by-character against C FFI results on the same input contours. Use the existing 4-glyph test font as a benchmark.

C source files to port from:
- `fontforge/splineutil.c` (7,938 LOC) — simplify, transform, intersect, bounds
- `fontforge/splineutil2.c` (4,429 LOC) — additional operations

#### Group D: Heavy Data Tables — ❌ NOT STARTED

| Task | C Files | LOC | Approach |
|------|---------|-----|----------|
| RALPH-022: Namelist replacement | `fontforge/namelist.c` | 20,797 | Extract Adobe Glyph List data → Rust `phf::Map` or sorted array. Delete C file. |
| RALPH-023: Unicode replacement | `Unicode/` (8 files) | ~12,000 | Replace with `unicode-normalization`, `unicode-segmentation` Rust crates. |
| RALPH-024: Mac encoding removal | `fontforge/macenc.c/h` | 2,364 | Stub with identity mapping. Legacy Mac cmap support not needed for modern Rust port. |

**Potential LOC reduction**: ~34,000 LOC, 9+ files eliminated.

#### Group E: Auto-Hinting — ❌ NOT STARTED

| Task | C Files | LOC | Options |
|------|---------|-----|---------|
| RALPH-025: Auto-hinting decision | `autohint.c`, `stemdb.c`, `ttfinstrs.c` | 10,113 | Keep as C FFI, remove entirely, or replace via HarfBuzz (`harfbuzz-rs`). |

**Savings if removed**: ~10,100 LOC, 3 files. Recommendation: decide based on whether downstream consumers need the native hinting pipeline. HarfBuzz provides modern alternatives.

#### Verification — ❌ NOT STARTED

| Task | Description |
|------|-------------|
| RALPH-026 | Phase 2 verification pass 1/3 |
| RALPH-027 | Phase 2 verification pass 2/3 |
| RALPH-028 | Phase 2 verification pass 3/3 |

Each pass: full `cargo build`, `cargo test`, `ctest`, symbol audit, no regressions.

---

## 3. Phase 3: Pure Rust Endgame (Vision)

Once all C modules are either ported to Rust or wrapped via FFI, the final
step is to eliminate the C dependency entirely.

### 3.1 Endgame Criteria

- All format I/O implemented in pure Rust
- OpenType layout engine in pure Rust (or a thin FFI wrapper on a known-good library)
- Namelist/Unicode data from Rust ecosystem crates
- Auto-hinting via HarfBuzz Rust bindings or removed
- `libfontforge.so` no longer linked
- C source deleted from the repository

### 3.2 Target Crate Structure

```
fontforge-rs/
├── fontforge-core/         # Core types: SplineFont, SplineChar, SplineSet, Point
├── fontforge-io/           # Format I/O: SFD, TTF, OTF, UFO, SVG, BDF, PS
├── fontforge-layout/       # OpenType GPOS/GSUB/feature files
├── fontforge-ops/          # Spline operations: simplify, overlap, extrema, direction
├── fontforge-ffi/          # (Temporary) C FFI bridge — deleted in Phase 3 endgame
└── fontforge/              # Top-level re-export crate
```

---

## 4. Phase 2 Immediate Next Steps (Ordered)

All planning is done. `.ralph/TODO.md` has tasks RALPH-015 through RALPH-028
with dependencies and acceptance criteria. RALPH-015 through RALPH-020 are
done. Start at RALPH-021.

### ⏭️ Next: RALPH-021 — Pure-Rust Spline Simplify/Extrema/Direction

**This is the immediate next task.** Port SplineCharSimplify,
SplineCharAddExtrema, and SplineSetsCorrect to pure Rust. The overlap
removal (RALPH-020) is already done, establishing the pattern:

1. Read C source (`fontforge/splineutil.c`, `fontforge/splineutil2.c`)
   to understand the algorithm.
2. Implement pure-Rust equivalent in `rust-bindings/src/simplify.rs` or similar.
3. Add integration tests in `rust-bindings/tests/` that compare
   pure-Rust output against C FFI results on the same input contours.
4. Use the 4-glyph test font as a benchmark.

Reference: RALPH-020 (`rust-bindings/src/overlap.rs`, `tests/test_overlap_pure_rust.rs`)
shows the exact pattern — extract contours via FFI, run both C and Rust,
compare contour count + bounding boxes + SFD round-trip.

### After RALPH-021

In recommended order:

| Order | Task | Description |
|-------|------|-------------|
| 2 | RALPH-022 | Replace `fontforge/namelist.c` with Rust static data |
| 3 | RALPH-023 | Replace `Unicode/` with Rust ecosystem crates |
| 4 | RALPH-024 | Remove or stub `fontforge/macenc.c/h` |
| 5 | RALPH-025 | Decide auto-hinting strategy (keep/remove/replace) |
| 6 | RALPH-026 | Phase 2 verification pass 1/3 |
| 7 | RALPH-027 | Phase 2 verification pass 2/3 |
| 8 | RALPH-028 | Phase 2 verification pass 3/3 |

---

## 5. Current Build & Test Commands

```bash
# Environment setup (custom dev-root, no sudo)
export PATH="/tmp/cmake-3.30.3-linux-x86_64/bin:/tmp/dev-root/usr/bin:$PATH"
export PKG_CONFIG_PATH="/tmp/dev-root/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/lib/x86_64-linux-gnu/pkgconfig"

# C build
mkdir -p build && cd build
cmake .. -DBUILD_SHARED_LIBS=ON \
  -DCMAKE_PREFIX_PATH=/tmp/dev-root/usr \
  -DCMAKE_FIND_LIBRARY_SUFFIXES=".so" \
  -DENABLE_HARFBUZZ=OFF
make -j$(nproc)

# C tests
cd build && ctest --output-on-failure

# Rust build & test
source "$HOME/.cargo/env"
export LIBCLANG_PATH=/usr/lib/llvm-21/lib
export LD_LIBRARY_PATH=$PWD/build/lib:$LD_LIBRARY_PATH
cargo build
cargo test
```

---

## 6. Phase 1 Decisions (Recorded for Context)

1. **fv_interface pattern preserved** — Full FontViewBase removal would touch
   29 macro call sites across 8 files; not worth the risk.
2. **macenc, spiro, bezctx_ff kept** — Too deeply integrated to remove without
   structural changes to SplineSet.
3. **autohint/stemdb/ttfinstrs kept** — Deferred to Phase 2 decision (see §4 Step 6).
4. **gimage removed** — 18 files excluded from build, 11 consumer sites stubbed.
5. **_NO_PYTHON and _NO_FFSCRIPT hardcoded to 1** — Unconditionally defined in
   CMakeLists.txt and FontForgeConfigure.cmake.
6. **HarfBuzz disabled** — `ENABLE_HARFBUZZ=OFF`. May re-enable in Phase 2 for
   hinting replacement.

---

## 7. Summary Statistics

| Metric | Original | After Phase 1 | Phase 2 Current | Phase 2 Target |
|--------|----------|---------------|-----------------|----------------|
| Total C LOC | ~350,000 | ~188,000 | ~188,000 | < 100,000 |
| Source files compiled | ~250 | ~76 | ~76 | < 40 |
| Format I/O ported to Rust | 0 | 3 (SFD, TTF, OTF) | 7 (UFO, SVG, BDF, PS done) | 7+ |
| Spline ops ported | 0 | 0 (FFI-validated) | 1/4 (overlap done) | Pure Rust (4 ops) |
| OpenType layout | C only | C only | FFI-wrapped ✅ | FFI-wrapped ✅ |
| Namelist | C only | C only | C only | Rust static data |
| Unicode | C only | C only | C only | Rust crates |
| Auto-hinting | Present | Present | Present | Decision pending |
| C tests | Present (many) | 2 | 2 | Kept until C deleted |
| Rust tests | 0 | 9 | **32** (21 integration + 11 unit) | 40+ |
| Rust source files | 0 | 3 | **9** | 12+ |

---

## 8. Key Project Files

### C Library
| File | Purpose |
|------|---------|
| `fontforge/formatstubs.c` | 150+ stubs for removed functionality |
| `fontforge/formatstubs.h` | Extern declarations for all stubs |
| `fontforge/fontviewbase.c` | FVTrans, FVTransFunc, fv_interface defaults (728 LOC) |
| `fontforge/noprefs.c` | Stub prefs_interface (NOUI_SavePrefs, etc.) |
| `fontforge/start.c` | Library init (InitSimpleStuff, DoInit) |
| `fontforge/ufo.c` | C UFO I/O (4,372 LOC — now superseded by Rust port) |
| `fontforge/splineutil.c` | Spline utilities (7,938 LOC — target for RALPH-021) |
| `fontforge/splineutil2.c` | Additional spline ops (4,429 LOC) |
| `fontforge/namelist.c` | Adobe Glyph List (20,797 LOC — target for RALPH-022) |
| `fontforge/macenc.c/h` | Mac encoding (2,364 LOC — target for RALPH-024) |
| `fontforge/autohint.c` | Auto-hinter (3,448 LOC — target for RALPH-025) |
| `fontforge/stemdb.c` | Stem database (6,088 LOC — target for RALPH-025) |
| `fontforge/ttfinstrs.c` | TTF instructing (577 LOC — target for RALPH-025) |
| `fontforge/featurefile.c` | OT feature file parser (FFI-wrapped ✅) |
| `fontforge/tottfgpos.c` | GPOS table (FFI-wrapped ✅) |
| `fontforge/tottfgsub.c` | GSUB table (FFI-wrapped ✅) |
| `fontforge/lookups.c` | OT lookup engine (FFI-wrapped ✅) |
| `inc/gimage.h` | Types-only stub (Color, GClut, image_type, struct _GImage) |
| `tests/test_roundtrip.c` | C SFD round-trip test (4 glyphs) |
| `tests/test_ttf_roundtrip.c` | C TTF round-trip test |

### Rust Workspace
| File | Purpose |
|------|---------|
| `Cargo.toml` | Rust workspace root |
| `rust-bindings/build.rs` | bindgen configuration + allowlists |
| `rust-bindings/wrapper.h` | Headers included for bindgen |
| `rust-bindings/src/lib.rs` | FFI bindings entry point + module declarations |
| `rust-bindings/src/ufo.rs` | 🆕 Pure-Rust UFO writer (quick-xml) |
| `rust-bindings/src/ufo_read.rs` | 🆕 Pure-Rust UFO reader |
| `rust-bindings/src/svg.rs` | 🆕 Pure-Rust SVG font import |
| `rust-bindings/src/bdf.rs` | 🆕 Pure-Rust BDF import/export |
| `rust-bindings/src/pstype1.rs` | 🆕 Pure-Rust PS Type 1 writer (eexec encryption) |
| `rust-bindings/src/ot_layout.rs` | 🆕 Safe OT layout FFI wrapper (scripts, langs, features) |
| `rust-bindings/src/overlap.rs` | 🆕 Pure-Rust overlap removal (~400 LOC) |
| `rust-bindings/tests/test_ffi.rs` | FFI smoke tests (SplineFont create/free) |
| `rust-bindings/tests/test_sfd_roundtrip.rs` | SFD round-trip test |
| `rust-bindings/tests/test_spline_ops.rs` | Spline ops via FFI (simplify, extrema, overlap, direction) |
| `rust-bindings/tests/test_ttf_otf_roundtrip.rs` | TTF/OTF round-trip tests |
| `rust-bindings/tests/test_ufo_roundtrip.rs` | 🆕 UFO round-trip test |
| `rust-bindings/tests/test_svg_import.rs` | 🆕 SVG font import test |
| `rust-bindings/tests/test_bdf_roundtrip.rs` | 🆕 BDF round-trip test |
| `rust-bindings/tests/test_pstype1_roundtrip.rs` | 🆕 PS Type 1 round-trip test |
| `rust-bindings/tests/test_ot_layout.rs` | 🆕 OT layout FFI wrapper tests |
| `rust-bindings/tests/test_overlap_pure_rust.rs` | 🆕 Pure-Rust overlap vs C comparison tests |

### Infrastructure
| File | Purpose |
|------|---------|
| `.github/workflows/main.yml` | CI: Linux build + test + dist |
| `.gitignore` | Updated for .pi/, .ralph/, Rust artifacts |
| `.ralph/STATE.json` | Current state (RALPH-021 pending, phase 2, iteration 20) |
| `.ralph/TODO.md` | Phase 2 task list (RALPH-015 through RALPH-028) |
| `.ralph/PROGRESS.md` | Detailed progress log (iterations 001-020) |
| `.ralph/HANDOFF.md` | Short handoff for Ralph workers |
| `.ralph/GOAL.md` | Phase 2 goal, success criteria, stop conditions |
| `.ralph/PLAN.md` | Phase 2 milestones M1-M7 |
| `.ralph/VERIFY.md` | Phase 2 verification plan |
