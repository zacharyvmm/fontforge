// Integration test for BDF (Bitmap Distribution Format) 
// read/write round-trip via the Rust BDF module.
//
// Creates a known BDF text string, parses it to build SplineFont + BDFFont via FFI,
// writes BDF text back, parses the output, and verifies glyph bitmap data is preserved.

/// A known BDF font with 3 glyphs (.notdef, space, letter A with some bitmap data)
static BDF_INPUT: &str = r#"STARTFONT 2.1
FONT -TestBdf-BDFTest-Medium-R-Normal--12-120-75-75-P-60-ISO10646-1
SIZE 12 75 75
FONTBOUNDINGBOX 8 13 0 -2
STARTPROPERTIES 5
FAMILY_NAME "BDFTest"
WEIGHT_NAME "Medium"
PIXEL_SIZE 12
FONT_ASCENT 10
FONT_DESCENT 3
ENDPROPERTIES
CHARS 3
STARTCHAR .notdef
ENCODING -1
SWIDTH 500 0
DWIDTH 8 0
BBX 8 13 0 -2
BITMAP
00
00
18
24
42
42
7E
42
42
42
42
00
00
ENDCHAR
STARTCHAR space
ENCODING 32
SWIDTH 500 0
DWIDTH 4 0
BBX 1 1 0 0
BITMAP
00
ENDCHAR
STARTCHAR A
ENCODING 65
SWIDTH 600 0
DWIDTH 8 0
BBX 8 10 0 -1
BITMAP
18
24
42
42
7E
42
42
42
42
00
ENDCHAR
ENDFONT
"#;

#[test]
fn test_bdf_import() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();

        // Parse BDF and build SplineFont
        let sf = fontforge_ffi::bdf::read_bdf_from_str(BDF_INPUT);
        assert!(!sf.is_null(), "read_bdf_from_str returned null");

        let sf_ref = &*sf;

        // Verify font metadata
        assert!(sf_ref.glyphcnt >= 3, "Expected >= 3 glyphs, got {}", sf_ref.glyphcnt);
        assert!(!sf_ref.familyname.is_null(), "familyname is null");
        let family = std::ffi::CStr::from_ptr(sf_ref.familyname)
            .to_str()
            .unwrap_or("");
        assert_eq!(family, "BDFTest", "Family name mismatch");

        // Verify bitmaps are attached
        let bdf = sf_ref.bitmaps;
        assert!(!bdf.is_null(), "BDFFont (bitmaps) is null");
        let bdf_ref = &*bdf;

        assert!(bdf_ref.pixelsize > 0, "pixelsize is 0");
        assert_eq!(bdf_ref.ascent, 10, "ascent mismatch");
        assert_eq!(bdf_ref.descent, 3, "descent mismatch");

        // Verify glyph bitmap data
        assert!(!bdf_ref.glyphs.is_null(), "glyphs array is null");
        let glyphs = bdf_ref.glyphs;

        // Character at index 0 should be .notdef (encoding -1)
        let bc0 = *glyphs;
        assert!(!bc0.is_null(), "glyph[0] is null");
        let bc0_ref = &*bc0;
        // BBX was 8x13, so xmin=0, xmax=7, ymin=-2, ymax=10
        assert_eq!(bc0_ref.xmin, 0 as i16);
        assert_eq!(bc0_ref.ymin, -2 as i16);
        assert_eq!(bc0_ref.xmax, 7 as i16);
        assert_eq!(bc0_ref.ymax, 10 as i16);
        assert!(!bc0_ref.bitmap.is_null(), "glyph[0] bitmap is null");
        // Verify specific bitmap data for .notdef (the A-like shape)
        // BBX 8 13, rows: 0x00 0x00 0x18 0x24 0x42 0x42 0x7E ...
        assert_eq!(*bc0_ref.bitmap.add(2), 0x18, "Row 2 should be 0x18");
        assert_eq!(*bc0_ref.bitmap.add(6), 0x7E, "Row 6 should be 0x7E");

        // Cleanup
        fontforge_ffi::SplineFontFree(sf);
        println!("PASS: BDF import test complete");
    }
}

#[test]
fn test_bdf_roundtrip() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();

        // Step 1: Parse BDF and build SplineFont
        let sf1 = fontforge_ffi::bdf::read_bdf_from_str(BDF_INPUT);
        assert!(!sf1.is_null(), "Step 1: read_bdf_from_str returned null");

        // Step 2: Write BDF back
        let bdf_output = fontforge_ffi::bdf::write_bdf_to_str(sf1)
            .expect("write_bdf_to_str failed");
        assert!(!bdf_output.is_empty(), "BDF output is empty");
        assert!(bdf_output.contains("STARTFONT"), "Missing STARTFONT");
        assert!(bdf_output.contains("ENDFONT"), "Missing ENDFONT");

        // Step 3: Parse the output back
        let sf2 = fontforge_ffi::bdf::read_bdf_from_str(&bdf_output);
        assert!(!sf2.is_null(), "Step 3: re-read returned null");
        let sf2_ref = &*sf2;

        // Step 4: Verify round-trip
        // at least as many glyphs
        assert!(sf2_ref.glyphcnt >= 2, "Round-trip glyph count too low: {}", sf2_ref.glyphcnt);

        // Verify bitmaps survived
        let bdf2 = sf2_ref.bitmaps;
        assert!(!bdf2.is_null(), "BDFFont was lost in round-trip");
        let bdf2_ref = &*bdf2;

        assert!(bdf2_ref.pixelsize > 0, "pixelsize lost");
        assert_eq!(bdf2_ref.ascent, 10, "ascent changed in round-trip");
        assert_eq!(bdf2_ref.descent, 3, "descent changed in round-trip");

        // Check that glyph[0] bitmap exists and has non-zero data
        let bc0 = *bdf2_ref.glyphs;
        if !bc0.is_null() {
            let bc0_ref = &*bc0;
            assert!(!bc0_ref.bitmap.is_null(), "Round-trip: glyph[0] bitmap lost");
            // Should have the .notdef shape preserved
            assert_eq!(*bc0_ref.bitmap.add(2), 0x18, "Round-trip: row 2 bitmap data changed");
            assert_eq!(*bc0_ref.bitmap.add(6), 0x7E, "Round-trip: row 6 bitmap data changed");
        }

        // Write the re-read font again and verify idempotence
        let bdf_output2 = fontforge_ffi::bdf::write_bdf_to_str(sf2)
            .expect("Second write_bdf_to_str failed");
        assert!(bdf_output2.contains("STARTFONT"), "Second output: Missing STARTFONT");
        assert!(bdf_output2.contains("ENDFONT"), "Second output: Missing ENDFONT");

        // Third parse should still work
        let sf3 = fontforge_ffi::bdf::read_bdf_from_str(&bdf_output2);
        assert!(!sf3.is_null(), "Step 5: third parse returned null");
        let sf3_ref = &*sf3;
        assert!(sf3_ref.glyphcnt >= 2, "Third parse glyph count too low");
        let bdf3 = sf3_ref.bitmaps;
        assert!(!bdf3.is_null(), "Third parse: BDFFont lost");
        let bdf3_ref = &*bdf3;
        assert_eq!(bdf3_ref.ascent, 10);
        assert_eq!(bdf3_ref.descent, 3);

        // Cleanup
        fontforge_ffi::SplineFontFree(sf3);
        fontforge_ffi::SplineFontFree(sf2);
        fontforge_ffi::SplineFontFree(sf1);
        println!("PASS: BDF round-trip test complete (3 reads, 2 writes)");
    }
}

#[test]
fn test_bdf_simple_glyph_roundtrip() {
    // Test with a single simple bitmap glyph to verify exact data preservation
    let input = r#"STARTFONT 2.1
FONT -Test-Simple-Medium-R-Normal--10-100-75-75-C-50-ISO10646-1
SIZE 10 75 75
FONTBOUNDINGBOX 6 8 0 -2
STARTPROPERTIES 3
FAMILY_NAME "Simple"
PIXEL_SIZE 10
FONT_ASCENT 7
FONT_DESCENT 3
ENDPROPERTIES
CHARS 1
STARTCHAR X
ENCODING 88
SWIDTH 500 0
DWIDTH 6 0
BBX 6 8 0 -1
BITMAP
78
84
84
78
84
84
84
78
ENDCHAR
ENDFONT
"#;

    unsafe {
        fontforge_ffi::doinitFontForgeMain();

        // Parse
        let sf1 = fontforge_ffi::bdf::read_bdf_from_str(input);
        assert!(!sf1.is_null());
        let bdf1 = (*sf1).bitmaps;
        assert!(!bdf1.is_null());
        let glyphs1 = (*bdf1).glyphs;
        let bc1 = *glyphs1;
        assert!(!bc1.is_null());

        // Record original bitmap data
        let bytes_per_line = (*bc1).bytes_per_line as usize;
        let h = ((*bc1).ymax - (*bc1).ymin + 1) as usize;
        let bmp_size = bytes_per_line * h;
        let mut original_bmp = vec![0u8; bmp_size];
        for i in 0..bmp_size {
            original_bmp[i] = *(*bc1).bitmap.add(i);
        }

        // Write and re-read
        let output = fontforge_ffi::bdf::write_bdf_to_str(sf1).expect("write failed");
        let sf2 = fontforge_ffi::bdf::read_bdf_from_str(&output);
        assert!(!sf2.is_null());
        let bdf2 = (*sf2).bitmaps;
        assert!(!bdf2.is_null());

        // Find the X glyph (should be first, encoding 88)
        let glyphs2 = (*bdf2).glyphs;
        let bc2 = *glyphs2;
        assert!(!bc2.is_null());
        let bc2_ref = &*bc2;

        assert_eq!(bc2_ref.width, (*bc1).width);
        assert_eq!(bc2_ref.xmin, (*bc1).xmin);
        assert_eq!(bc2_ref.ymin, (*bc1).ymin);
        assert_eq!(bc2_ref.xmax, (*bc1).xmax);
        assert_eq!(bc2_ref.ymax, (*bc1).ymax);
        assert_eq!(bc2_ref.bytes_per_line, (*bc1).bytes_per_line);

        // Verify bitmap data is identical
        let bmp2_size = bc2_ref.bytes_per_line as usize
            * ((bc2_ref.ymax - bc2_ref.ymin + 1).max(1) as usize);
        let mut roundtrip_bmp = vec![0u8; bmp2_size];
        for i in 0..bmp2_size {
            roundtrip_bmp[i] = *bc2_ref.bitmap.add(i);
        }
        assert_eq!(
            &original_bmp[..bmp2_size.min(bmp_size)],
            &roundtrip_bmp[..bmp2_size.min(bmp_size)],
            "Bitmap data changed in round-trip"
        );

        fontforge_ffi::SplineFontFree(sf2);
        fontforge_ffi::SplineFontFree(sf1);
        println!("PASS: BDF simple glyph round-trip test complete");
    }
}
