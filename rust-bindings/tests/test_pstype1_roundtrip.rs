// Integration tests for PS Type 1 / PFA I/O round-trip.
//
// Creates a SplineFont with glyphs via FFI, writes PFA using the pure-Rust
// pstype1 writer, then reads back using LoadSplineFont (C FFI) and verifies
// the round-trip preserves glyph data.

use std::ffi::CString;
use std::io::Write;
use std::ptr;

/// Helper: create a rectangular contour and add it to a glyph.
unsafe fn add_rect_to_glyph(
    sc: *mut fontforge_ffi::SplineChar,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let mut sp: [*mut fontforge_ffi::SplinePoint; 4] = [ptr::null_mut(); 4];
    let corners: [(f64, f64); 4] = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    for j in 0..4 {
        sp[j] = fontforge_ffi::SplinePointCreate(corners[j].0, corners[j].1);
        assert!(!sp[j].is_null(), "SplinePointCreate returned null");
    }
    // Create cubic splines (straight lines)
    fontforge_ffi::SplineMake(sp[0], sp[1], 0);
    fontforge_ffi::SplineMake(sp[1], sp[2], 0);
    fontforge_ffi::SplineMake(sp[2], sp[3], 0);
    fontforge_ffi::SplineMake(sp[3], sp[0], 0);

    let spl = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
        as *mut fontforge_ffi::SplinePointList;
    assert!(!spl.is_null());
    (*spl).first = sp[0];
    (*spl).last = sp[0];

    fontforge_ffi::SPLCategorizePoints(spl);

    // Append to glyph's foreground layer
    let layer = (*sc).layers.offset(1); // ly_fore = 1
    if (*layer).splines.is_null() {
        (*layer).splines = spl;
    } else {
        let mut cur = (*layer).splines;
        while !(*cur).next.is_null() {
            cur = (*cur).next;
        }
        (*cur).next = spl;
    }
}

/// Helper: allocate a null-terminated C string on the heap.
unsafe fn alloc_cstring(s: &str) -> *mut std::os::raw::c_char {
    let bytes = s.as_bytes();
    let ptr = libc::malloc(bytes.len() + 1) as *mut std::os::raw::c_char;
    ptr::copy_nonoverlapping(bytes.as_ptr() as *const std::os::raw::c_char, ptr, bytes.len());
    *ptr.add(bytes.len()) = 0;
    ptr
}

/// Helper: add glyphs to a SplineFont by allocating a new glyphs array.
unsafe fn add_glyphs_to_font(sf: *mut fontforge_ffi::SplineFont, glyphs: Vec<*mut fontforge_ffi::SplineChar>) {
    let sf_ref = &mut *sf;
    let cnt = glyphs.len();

    // Allocate new glyphs array
    let new_glyphs = libc::calloc(cnt, std::mem::size_of::<*mut fontforge_ffi::SplineChar>())
        as *mut *mut fontforge_ffi::SplineChar;

    // Free old array if exists
    if !sf_ref.glyphs.is_null() {
        libc::free(sf_ref.glyphs as *mut libc::c_void);
    }

    // Copy glyph pointers
    for (i, g) in glyphs.into_iter().enumerate() {
        *new_glyphs.add(i) = g;
    }

    sf_ref.glyphs = new_glyphs;
    sf_ref.glyphcnt = cnt as i32;
    sf_ref.glyphmax = cnt as i32;
}

/// Round-trip: create font → write PFA via Rust → read via LoadSplineFont → verify
#[test]
fn test_pfa_roundtrip() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();

        // ── Create a SplineFont with 3 glyphs (rectangles) ───────────────
        let sf = fontforge_ffi::SplineFontNew();
        assert!(!sf.is_null());

        let sf_ref = &mut *sf;
        sf_ref.fontname = alloc_cstring("PfaTestFont");
        sf_ref.version = alloc_cstring("1.0");
        sf_ref.familyname = alloc_cstring("PfaTestFont");
        sf_ref.fullname = alloc_cstring("PfaTestFont");
        sf_ref.ascent = 800;
        sf_ref.descent = 200;

        // Create 3 glyphs: .notdef, A, B
        let glyph_names = [".notdef", "A", "B"];
        let mut glyphs = Vec::new();

        for (i, gname) in glyph_names.iter().enumerate() {
            let sc = fontforge_ffi::SplineCharCreate(2);
            assert!(!sc.is_null());

            (*sc).name = alloc_cstring(gname);
            (*sc).unicodeenc = -1;

            match i {
                0 => {
                    (*sc).width = 500;
                    add_rect_to_glyph(sc, 50.0, 0.0, 400.0, 600.0);
                }
                1 => {
                    (*sc).width = 600;
                    add_rect_to_glyph(sc, 100.0, 50.0, 500.0, 700.0);
                }
                _ => {
                    (*sc).width = 550;
                    add_rect_to_glyph(sc, 150.0, -50.0, 450.0, 650.0);
                }
            }

            glyphs.push(sc);
        }

        add_glyphs_to_font(sf, glyphs);
        assert_eq!((*sf).glyphcnt, 3, "Expected 3 glyphs");

        // ── Write PFA using pure-Rust writer ──────────────────────────────
        let pfa_str = fontforge_ffi::pstype1::write_pfa_to_str(sf);
        assert!(!pfa_str.is_empty(), "PFA output is empty");
        assert!(
            pfa_str.contains("%!PS-AdobeFont-1.0"),
            "Missing PFA header"
        );
        assert!(pfa_str.contains("eexec"), "Missing eexec marker");
        assert!(pfa_str.contains("cleartomark"), "Missing cleartomark");

        // Write PFA string to a temp file
        let temp_dir = std::env::temp_dir();
        let pfa_path = temp_dir.join("test_pfa_roundtrip.pfa");
        {
            let mut f = std::fs::File::create(&pfa_path).expect("Failed to create temp PFA file");
            f.write_all(pfa_str.as_bytes())
                .expect("Failed to write PFA data");
        }

        // ── Read back via LoadSplineFont (C FFI) ─────────────────────────
        let path_cstr = CString::new(pfa_path.to_str().unwrap()).unwrap();
        let loaded_sf = fontforge_ffi::LoadSplineFont(
            path_cstr.as_ptr() as *mut std::os::raw::c_char,
            0,
        );
        assert!(!loaded_sf.is_null(), "LoadSplineFont returned null for PFA");

        let loaded = &*loaded_sf;

        // Verify glyph count (may have extra glyphs from .notdef resolution)
        assert!(
            loaded.glyphcnt >= 3,
            "Expected >= 3 glyphs, got {}",
            loaded.glyphcnt
        );

        // Verify font name
        let loaded_name = std::ffi::CStr::from_ptr(loaded.fontname as *const std::os::raw::c_char)
            .to_string_lossy()
            .into_owned();
        assert_eq!(loaded_name, "PfaTestFont", "Font name changed in round-trip");

        // Verify ascent/descent
        assert_eq!(loaded.ascent, 800, "Ascent changed");
        assert_eq!(loaded.descent, 200, "Descent changed");

        // Verify glyph data: each glyph should have spline data
        let mut glyphs_with_splines = 0;
        for i in 0..loaded.glyphcnt as usize {
            let sc = *loaded.glyphs.add(i);
            if sc.is_null() {
                continue;
            }
            let sc_ref = &*sc;
            let layer = sc_ref.layers.offset(1); // ly_fore
            if !(*layer).splines.is_null() {
                let spl = (*layer).splines;
                if !(*spl).first.is_null() {
                    glyphs_with_splines += 1;
                }
            }
        }
        assert!(
            glyphs_with_splines >= 3,
            "Expected >= 3 glyphs with spline data, got {}",
            glyphs_with_splines
        );

        // Clean up temp file
        let _ = std::fs::remove_file(&pfa_path);
    }
}

/// Verify that the PFA writer output is syntactically reasonable.
#[test]
fn test_pfa_output_structure() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();

        let sf = fontforge_ffi::SplineFontNew();
        assert!(!sf.is_null());

        let sf_ref = &mut *sf;
        sf_ref.fontname = alloc_cstring("StructureTest");
        sf_ref.familyname = alloc_cstring("StructureTest");
        sf_ref.fullname = alloc_cstring("StructureTest");
        sf_ref.ascent = 800;
        sf_ref.descent = 200;

        // Create one simple glyph (notdef)
        let sc = fontforge_ffi::SplineCharCreate(2);
        assert!(!sc.is_null());
        (*sc).name = alloc_cstring(".notdef");
        (*sc).width = 400;
        (*sc).unicodeenc = -1;
        add_rect_to_glyph(sc, 0.0, 0.0, 400.0, 500.0);

        add_glyphs_to_font(sf, vec![sc]);

        let pfa_str = fontforge_ffi::pstype1::write_pfa_to_str(sf);
        assert!(!pfa_str.is_empty());

        // Check key structural elements in clear-text section
        assert!(pfa_str.starts_with("%!PS-AdobeFont-1.0:"), "Missing PFA header");
        assert!(pfa_str.contains("/FontType 1 def"), "Missing FontType");
        assert!(pfa_str.contains("/FontMatrix"), "Missing FontMatrix");
        assert!(pfa_str.contains("/FontName"), "Missing FontName");
        assert!(pfa_str.contains("/FontBBox"), "Missing FontBBox");
        assert!(pfa_str.contains("/Encoding"), "Missing Encoding");
        assert!(pfa_str.contains("eexec"), "Missing eexec marker");
        assert!(pfa_str.contains("cleartomark"), "Missing cleartomark");
        // These are inside the encrypted section, so they won't appear in plaintext
        // But the round-trip test confirms LoadSplineFont decodes them correctly.

        // Verify the clear-text section has balanced dict begin/end
        let before_eexec = pfa_str.split("eexec").next().unwrap_or("");
        let begins = before_eexec.matches("begin").count();
        let ends = before_eexec.matches("end").count();
        assert_eq!(begins, ends, "Unbalanced dicts in clear-text section");
        assert!(begins >= 2, "Too few dicts in clear-text section");
    }
}
