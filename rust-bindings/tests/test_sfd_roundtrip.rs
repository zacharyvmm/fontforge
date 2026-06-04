// Integration test for SFD read/write round-trip via FFI.
// Mirrors tests/test_roundtrip.c:
//   Creates a SplineFont with 4 glyphs, writes SFD, reloads, verifies.

use std::ffi::CString;
use std::ptr;

const LY_FORE: isize = 1; // layer_type::ly_fore

#[test]
fn test_sfd_roundtrip() {
    unsafe {
        // Initialize FontForge
        fontforge_ffi::doinitFontForgeMain();

        // Create a test font
        let sf = fontforge_ffi::SplineFontNew();
        assert!(!sf.is_null(), "SplineFontNew() returned null");

        // Set predictable font name
        let fontname = CString::new("TestFont").unwrap();
        // sf->fontname is a *mut c_char; SplineFontNew allocates it,
        // we free the old one and set ours. In C test, free(sf->fontname) then strdup.
        // Here we just overwrite — the C allocator owns the old pointer.
        // However, SplineFontNew may not allocate fontname, so we check.
        if !(*sf).fontname.is_null() {
            libc::free((*sf).fontname as *mut libc::c_void);
        }
        (*sf).fontname = libc::strdup(fontname.as_ptr());
        // onlybitmaps is a bitfield; default is 0/false from SplineFontNew

        // Create 4 glyphs: .notdef, A, B, C — each with a rectangular contour
        let glyphs: [(i32, &str); 4] = [
            (-1, ".notdef"),
            (0x0041, "A"),
            (0x0042, "B"),
            (0x0043, "C"),
        ];

        for &(uni, name) in &glyphs {
            let cname = CString::new(name).unwrap();
            let sc = fontforge_ffi::SFGetOrMakeChar(sf, uni, cname.as_ptr());
            assert!(!sc.is_null(), "SFGetOrMakeChar returned null for U+{:04X}", uni);

            // Add a simple rectangular contour (500x700 units)
            let mut sp: [*mut fontforge_ffi::SplinePoint; 4] = [ptr::null_mut(); 4];
            for j in 0..4 {
                let x: f64 = if j == 0 || j == 3 { 0.0 } else { 500.0 };
                let y: f64 = if j < 2 { 0.0 } else { 700.0 };
                sp[j] = fontforge_ffi::SplinePointCreate(x, y);
                assert!(!sp[j].is_null(), "SplinePointCreate returned null");
            }

            // Create cubic (order2=false) splines connecting the points
            fontforge_ffi::SplineMake(sp[0], sp[1], 0);
            fontforge_ffi::SplineMake(sp[1], sp[2], 0);
            fontforge_ffi::SplineMake(sp[2], sp[3], 0);
            fontforge_ffi::SplineMake(sp[3], sp[0], 0);

            // Create the SplinePointList for the foreground layer.
            // last == first signals a closed contour.
            let spl = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
                as *mut fontforge_ffi::SplinePointList;
            assert!(!spl.is_null(), "calloc for SplinePointList failed");
            (*spl).first = sp[0];
            (*spl).last = sp[0]; // Closed contour

            // Set splines on the foreground layer (index ly_fore = 1)
            let layer_ptr = (*sc).layers.offset(LY_FORE);
            (*layer_ptr).splines = spl;

            fontforge_ffi::SPLCategorizePoints(spl);
        }

        // Create a 1:1 encoding map and save to SFD
        let map = fontforge_ffi::EncMap1to1((*sf).glyphcnt);
        assert!(!map.is_null(), "EncMap1to1 returned null");

        let tmpfile = CString::new(format!("/tmp/fontforge_rs_test_{}.sfd", std::process::id()))
            .unwrap();
        let ok = fontforge_ffi::SFDWrite(
            tmpfile.as_ptr() as *mut libc::c_char,
            sf,
            map,
            map,
            0, // todir=false
        );
        assert!(ok != 0, "SFDWrite returned false");

        // Free original font and map
        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf);

        // Re-load from SFD
        let sf2 = fontforge_ffi::LoadSplineFont(tmpfile.as_ptr(), 0);
        assert!(!sf2.is_null(), "LoadSplineFont returned null");

        // Verify results
        let expected_glyphcnt = 4i32;
        assert_eq!(
            (*sf2).glyphcnt, expected_glyphcnt,
            "glyph count mismatch: expected {}, got {}",
            expected_glyphcnt, (*sf2).glyphcnt
        );

        // Check font name
        assert!(
            !(*sf2).fontname.is_null(),
            "fontname is null after reload"
        );
        let reloaded_name = std::ffi::CStr::from_ptr((*sf2).fontname)
            .to_str()
            .unwrap_or("(invalid utf8)");
        assert_eq!(
            reloaded_name, "TestFont",
            "font name mismatch: expected 'TestFont', got '{}'",
            reloaded_name
        );

        // Check all glyphs have spline data
        for i in 0..(*sf2).glyphcnt {
            let glyph_ptr = *(*sf2).glyphs.offset(i as isize);
            assert!(
                !glyph_ptr.is_null(),
                "glyph[{}] is null after reload", i
            );
            let layer_ptr = (*glyph_ptr).layers.offset(LY_FORE);
            assert!(
                !(*layer_ptr).splines.is_null(),
                "glyph[{}] has no spline data after reload", i
            );
        }

        // Clean up
        fontforge_ffi::SplineFontFree(sf2);
        libc::unlink(tmpfile.as_ptr());

        // Also free the fontname CString (it was strdup'd, not CString-owned)
        // Actually, fontname is owned by the font and freed by SplineFontFree — no need.

        println!("PASS: font round-trip test passed");
    }
}
