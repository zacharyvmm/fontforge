//! Integration test for UFO round-trip (write → read) in pure Rust.
//!
//! Creates a SplineFont with glyphs via FFI, writes as UFO using the Rust
//! writer, reads it back using the Rust reader, and verifies fidelity.

use std::ptr;

const LY_FORE: isize = 1;

#[test]
fn test_ufo_roundtrip() {
    unsafe {
        // Initialize FontForge
        fontforge_ffi::doinitFontForgeMain();

        // ─── Create a test font ──────────────────────────────────────────
        let sf = fontforge_ffi::SplineFontNew();
        assert!(!sf.is_null(), "SplineFontNew() returned null");

        // Set font name and family via C strings (using strdup from libc)
        if !(*sf).fontname.is_null() {
            libc::free((*sf).fontname as *mut libc::c_void);
        }
        (*sf).fontname = libc::strdup(b"TestFont-Regular\0".as_ptr() as *const libc::c_char);

        if !(*sf).familyname.is_null() {
            libc::free((*sf).familyname as *mut libc::c_void);
        }
        (*sf).familyname = libc::strdup(b"TestFont\0".as_ptr() as *const libc::c_char);

        if !(*sf).fullname.is_null() {
            libc::free((*sf).fullname as *mut libc::c_void);
        }
        (*sf).fullname = libc::strdup(b"TestFont Regular\0".as_ptr() as *const libc::c_char);

        // Set metrics
        (*sf).ascent = 800;
        (*sf).descent = 200;
        (*sf).italicangle = 0.0;

        // Create 4 glyphs: .notdef, A, B, C
        let glyphs: [(i32, &str); 4] = [
            (-1, ".notdef"),
            (0x0041, "A"),
            (0x0042, "B"),
            (0x0043, "C"),
        ];

        for &(uni, name) in &glyphs {
            let cname = std::ffi::CString::new(name).unwrap();
            let sc = fontforge_ffi::SFGetOrMakeChar(sf, uni, cname.as_ptr());
            assert!(!sc.is_null(), "SFGetOrMakeChar returned null for U+{:04X}", uni);

            // Add a simple rectangular contour
            let mut sp: [*mut fontforge_ffi::SplinePoint; 4] = [ptr::null_mut(); 4];
            for j in 0..4 {
                let x: f64 = if j == 0 || j == 3 { 0.0 } else { 500.0 };
                let y: f64 = if j < 2 { 0.0 } else { 700.0 };
                sp[j] = fontforge_ffi::SplinePointCreate(x, y);
                assert!(!sp[j].is_null(), "SplinePointCreate returned null");
            }

            // Create cubic splines
            fontforge_ffi::SplineMake(sp[0], sp[1], 0);
            fontforge_ffi::SplineMake(sp[1], sp[2], 0);
            fontforge_ffi::SplineMake(sp[2], sp[3], 0);
            fontforge_ffi::SplineMake(sp[3], sp[0], 0);

            // Create SplinePointList
            let spl = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
                as *mut fontforge_ffi::SplinePointList;
            assert!(!spl.is_null(), "calloc for SplinePointList failed");
            (*spl).first = sp[0];
            (*spl).last = sp[0]; // Closed contour

            // Attach to foreground layer
            let layer_ptr = (*sc).layers.offset(LY_FORE);
            (*layer_ptr).splines = spl;

            fontforge_ffi::SPLCategorizePoints(spl);
        }

        // ─── Write UFO in Rust ───────────────────────────────────────────
        let ufo_dir = format!("/tmp/fontforge_rs_ufo_test_{}", std::process::id());
        let write_result = fontforge_ffi::ufo::write_ufo_font(&ufo_dir, sf);
        assert_eq!(write_result, 0, "write_ufo_font failed");

        // Free original font
        fontforge_ffi::SplineFontFree(sf);

        // ─── Read UFO back in Rust ───────────────────────────────────────
        let sf2 = fontforge_ffi::ufo_read::read_ufo_font(&ufo_dir);
        assert!(!sf2.is_null(), "read_ufo_font returned null");

        // ─── Verify results ──────────────────────────────────────────────
        let expected_glyphcnt = 4i32;
        assert_eq!(
            (*sf2).glyphcnt, expected_glyphcnt,
            "glyph count mismatch: expected {}, got {}",
            expected_glyphcnt, (*sf2).glyphcnt
        );

        // Check font name
        assert!(
            !(*sf2).fontname.is_null(),
            "fontname is null after read"
        );
        let reloaded_name = std::ffi::CStr::from_ptr((*sf2).fontname)
            .to_str()
            .unwrap_or("(invalid utf8)");
        assert_eq!(
            reloaded_name, "TestFont-Regular",
            "font name mismatch: expected 'TestFont-Regular', got '{}'",
            reloaded_name
        );

        // Check ascent/descent
        assert_eq!((*sf2).ascent, 800, "ascent mismatch");
        assert_eq!((*sf2).descent, 200, "descent mismatch");

        // Check all glyphs have names and spline data
        for i in 0..(*sf2).glyphcnt {
            let glyph_ptr = *(*sf2).glyphs.offset(i as isize);
            assert!(!glyph_ptr.is_null(), "glyph[{}] is null after read", i);

            assert!(
                !(*glyph_ptr).name.is_null(),
                "glyph[{}] has no name after read", i
            );
            let gname = std::ffi::CStr::from_ptr((*glyph_ptr).name)
                .to_str()
                .unwrap_or("(invalid utf8)");
            println!(
                "  glyph[{}]: '{}' (U+{:04X})",
                i, gname, (*glyph_ptr).unicodeenc
            );

            let layer_ptr = (*glyph_ptr).layers.offset(LY_FORE);
            assert!(
                !(*layer_ptr).splines.is_null(),
                "glyph '{}' has no spline data after read",
                gname
            );
        }

        // Clean up
        fontforge_ffi::SplineFontFree(sf2);

        // Remove UFO directory
        let _ = std::fs::remove_dir_all(&ufo_dir);

        println!("PASS: UFO round-trip test passed");
    }
}
