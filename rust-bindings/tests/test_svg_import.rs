//! Integration test for Rust SVG font import (RALPH-016).
//!
//! Creates SVG font XML as embedded strings, reads them using the
//! pure-Rust SVG reader, and verifies glyph data via FFI.

#[allow(unused_imports)]
use std::ptr;

const LY_FORE: isize = 1;

/// Minimal SVG font with 4 glyphs: .notdef, A, B, C
const SVG_FONT: &str = r#"<?xml version="1.0" standalone="no"?>
<svg xmlns="http://www.w3.org/2000/svg" version="1.1">
<defs>
<font id="TestSVGFont" horiz-adv-x="500">
  <font-face
    font-family="Test SVG Font"
    font-weight="400"
    units-per-em="1000"
    ascent="800"
    descent="200"
    underline-thickness="50"
    underline-position="-100"
    panose-1="2 0 5 3 0 0 0 0 0 0"
  />
  <missing-glyph horiz-adv-x="500" d="M0 0l500 0l0 700l-500 0z" />
  <glyph unicode="A" glyph-name="A" horiz-adv-x="500"
    d="M250 700l250 -700l-100 0l-50 150l-200 0l-50 -150l-100 0l250 700zM250 550l-75 -250l150 0z" />
  <glyph unicode="B" glyph-name="B" horiz-adv-x="500"
    d="M50 700l300 0c50 0 100 -20 100 -70c0 -30 -20 -50 -50 -60
      c40 -10 60 -40 60 -80c0 -50 -50 -90 -110 -90l-300 0z
      M50 400l250 0c30 0 60 10 60 45c0 30 -30 45 -60 45l-250 0z
      M50 50l200 0c25 0 50 10 50 40c0 30 -25 40 -50 40l-200 0z" />
  <glyph unicode="C" glyph-name="C" horiz-adv-x="500"
    d="M450 150l-100 0c0 -50 -40 -100 -100 -100c-75 0 -150 60 -150 175
      c0 105 75 160 150 160c60 0 100 -50 100 -100l100 0
      c0 100 -80 200 -200 200c-150 0 -250 -100 -250 -260
      c0 -160 100 -260 250 -260c120 0 200 100 200 185z" />
</font>
</defs>
</svg>"#;

/// SVG font with horizontal and vertical kerning
const SVG_FONT_WITH_KERN: &str = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg">
<defs>
<font id="KernTest" horiz-adv-x="600">
  <font-face font-family="KernTest" units-per-em="1000" ascent="800" descent="200"/>
  <glyph unicode="A" glyph-name="A" d="M300 700l300 -700l-600 0z"/>
  <glyph unicode="V" glyph-name="V" d="M0 700l300 -700l300 700z"/>
  <hkern k="50" g1="A" g2="V"/>
</font>
</defs>
</svg>"#;

#[test]
fn test_svg_import_basic() {
    unsafe {
        // Initialize FontForge
        fontforge_ffi::doinitFontForgeMain();

        // ─── Parse the SVG font ────────────────────────────────────
        let sf = fontforge_ffi::svg::read_svg_font_from_str(SVG_FONT);
        assert!(!sf.is_null(), "read_svg_font_from_str returned null");

        let sf_ref = &*sf;

        // ─── Verify font metadata ─────────────────────────────────
        assert_eq!(
            sf_ref.glyphcnt, 4,
            "glyph count mismatch: expected 4, got {}",
            sf_ref.glyphcnt
        );

        assert_eq!(sf_ref.ascent, 800, "ascent mismatch");
        assert_eq!(sf_ref.descent, -200, "descent mismatch");
        assert!((sf_ref.upos - (-100.0)).abs() < 0.1, "underline position mismatch: {}", sf_ref.upos);
        assert!((sf_ref.uwidth - 50.0).abs() < 0.1, "underline thickness mismatch: {}", sf_ref.uwidth);

        // Check font names
        let family = std::ffi::CStr::from_ptr(sf_ref.familyname)
            .to_str()
            .unwrap_or("");
        assert_eq!(family, "Test SVG Font", "family name mismatch");

        let weight = std::ffi::CStr::from_ptr(sf_ref.weight)
            .to_str()
            .unwrap_or("");
        assert_eq!(weight, "Regular", "weight name mismatch");

        // ─── Verify each glyph ─────────────────────────────────────
        let expected_glyphs = [
            (".notdef", 0i32),
            ("A", 0x0041i32),
            ("B", 0x0042i32),
            ("C", 0x0043i32),
        ];

        for i in 0..sf_ref.glyphcnt as isize {
            let glyph_ptr = *sf_ref.glyphs.offset(i);
            assert!(!glyph_ptr.is_null(), "glyph[{}] is null", i);

            let gname = std::ffi::CStr::from_ptr((*glyph_ptr).name)
                .to_str()
                .unwrap_or("");
            println!(
                "  glyph[{}]: '{}' (U+{:04X})",
                i, gname, (*glyph_ptr).unicodeenc
            );

            // Check spline data present
            let layer_ptr = (*glyph_ptr).layers.offset(LY_FORE);
            assert!(
                !(*layer_ptr).splines.is_null(),
                "glyph '{}' has no spline data",
                gname
            );

            // Verify splines are valid (have first/last points)
            let splines = (*layer_ptr).splines;
            assert!(
                !(*splines).first.is_null(),
                "glyph '{}' has no first spline point",
                gname
            );
            assert!(
                !(*splines).last.is_null(),
                "glyph '{}' has no last spline point",
                gname
            );

            // Check glyph name and unicode match expected
            let expected = &expected_glyphs[i as usize];
            assert_eq!(
                gname, expected.0,
                "glyph[{}] name mismatch: expected '{}', got '{}'",
                i, expected.0, gname
            );
            assert_eq!(
                (*glyph_ptr).unicodeenc, expected.1,
                "glyph[{}] unicode mismatch: expected U+{:04X}, got U+{:04X}",
                i, expected.1 as u32, (*glyph_ptr).unicodeenc as u32
            );
        }

        // ─── Verify encoding map ───────────────────────────────────
        assert!(
            !sf_ref.map.is_null(),
            "encoding map is null"
        );

        // Clean up
        fontforge_ffi::SplineFontFree(sf);

        println!("PASS: SVG font import basic test passed");
    }
}

#[test]
fn test_svg_import_with_kern() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();

        let sf = fontforge_ffi::svg::read_svg_font_from_str(SVG_FONT_WITH_KERN);
        assert!(!sf.is_null(), "read_svg_font_from_str returned null for kern test");

        let sf_ref = &*sf;

        assert_eq!(
            sf_ref.glyphcnt, 2,
            "glyph count mismatch: expected 2, got {}",
            sf_ref.glyphcnt
        );

        // Glyph "A"
        let a_ptr = *sf_ref.glyphs.offset(0);
        assert!(!a_ptr.is_null(), "glyph A is null");
        let a_name = std::ffi::CStr::from_ptr((*a_ptr).name)
            .to_str()
            .unwrap_or("");
        assert_eq!(a_name, "A", "glyph A name mismatch");

        // Glyph "V"
        let v_ptr = *sf_ref.glyphs.offset(1);
        assert!(!v_ptr.is_null(), "glyph V is null");
        let v_name = std::ffi::CStr::from_ptr((*v_ptr).name)
            .to_str()
            .unwrap_or("");
        assert_eq!(v_name, "V", "glyph V name mismatch");

        fontforge_ffi::SplineFontFree(sf);

        println!("PASS: SVG import with kerning test passed");
    }
}

/// Test reading an SVG font from a file (file I/O path).
#[test]
fn test_svg_import_from_file() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();

        // Write SVG font to a temp file
        let svg_path =
            format!("/tmp/fontforge_rs_svg_file_test_{}.svg", std::process::id());
        std::fs::write(&svg_path, SVG_FONT).expect("failed to write test SVG file");

        let sf = fontforge_ffi::svg::read_svg_font_file(&svg_path);
        assert!(!sf.is_null(), "read_svg_font_file returned null");

        let sf_ref = &*sf;

        assert_eq!(sf_ref.glyphcnt, 4, "glyph count mismatch from file");
        assert_eq!(sf_ref.ascent, 800, "ascent mismatch from file");
        assert_eq!(sf_ref.descent, -200, "descent mismatch from file");

        // Verify A glyph is present
        let mut found = false;
        for i in 0..sf_ref.glyphcnt as isize {
            let gp = *sf_ref.glyphs.offset(i);
            if gp.is_null() {
                continue;
            }
            let gname = std::ffi::CStr::from_ptr((*gp).name)
                .to_str()
                .unwrap_or("");
            if gname == "A" {
                found = true;
                let layer_ptr = (*gp).layers.offset(LY_FORE);
                assert!(
                    !(*layer_ptr).splines.is_null(),
                    "glyph A has no spline data from file"
                );
            }
        }
        assert!(found, "glyph A not found when reading from file");

        fontforge_ffi::SplineFontFree(sf);
        let _ = std::fs::remove_file(&svg_path);

        println!("PASS: SVG import from file test passed");
    }
}
