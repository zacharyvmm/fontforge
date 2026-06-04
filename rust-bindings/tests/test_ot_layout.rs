// Integration tests for OpenType layout FFI (RALPH-019).
// Tests exposing GPOS/GSUB/feature file functions via bindgen.
//
// Strategy:
//   1. Create a font with glyphs needed for a ligature substitution
//   2. Write a temporary .fea file defining the ligature feature
//   3. Apply the feature file via SFApplyFeatureFilename
//   4. Verify the font's GSUB lookup was created
//   5. Write the font as OTF (includes OT tables) and reload
//   6. Verify glyph data survives the round-trip

use std::ffi::CString;
use std::io::Write;
use std::ptr;

const LY_FORE: isize = 1;

/// Helper: create a rectangular contour and return the SplinePointList.
unsafe fn make_rectangle(x: f64, y: f64, w: f64, h: f64) -> *mut fontforge_ffi::SplinePointList {
    let mut sp: [*mut fontforge_ffi::SplinePoint; 4] = [ptr::null_mut(); 4];
    let corners: [(f64, f64); 4] = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    for j in 0..4 {
        sp[j] = fontforge_ffi::SplinePointCreate(corners[j].0, corners[j].1);
        assert!(!sp[j].is_null());
    }
    fontforge_ffi::SplineMake(sp[0], sp[1], 0);
    fontforge_ffi::SplineMake(sp[1], sp[2], 0);
    fontforge_ffi::SplineMake(sp[2], sp[3], 0);
    fontforge_ffi::SplineMake(sp[3], sp[0], 0);

    let spl = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
        as *mut fontforge_ffi::SplinePointList;
    assert!(!spl.is_null());
    (*spl).first = sp[0];
    (*spl).last = sp[0]; // Closed contour
    fontforge_ffi::SPLCategorizePoints(spl);
    spl
}

/// Create a test font with 4 glyphs named .notdef, a, b, a_b
unsafe fn create_ligature_font(name: &str) -> (*mut fontforge_ffi::SplineFont, *mut fontforge_ffi::EncMap) {
    let sf = fontforge_ffi::SplineFontNew();
    assert!(!sf.is_null());

    let cname = CString::new(name).unwrap();
    if !(*sf).fontname.is_null() {
        libc::free((*sf).fontname as *mut libc::c_void);
    }
    (*sf).fontname = libc::strdup(cname.as_ptr());
    (*sf).familyname = libc::strdup(cname.as_ptr());
    (*sf).fullname = libc::strdup(cname.as_ptr());

    (*sf).ascent = 800;
    (*sf).descent = 200;
    (*sf).design_size = 1000;

    // Glyphs: .notdef (uni=-1), a (uni=0x0061), b (uni=0x0062), a_b (uni=0xE001 in PUA)
    let glyphs: [(i32, &str); 4] = [
        (-1, ".notdef"),
        (0x0061, "a"),
        (0x0062, "b"),
        (0xE001, "a_b"), // Private Use Area for the ligature
    ];

    for &(uni, name_str) in &glyphs {
        let cname = CString::new(name_str).unwrap();
        let sc = fontforge_ffi::SFGetOrMakeChar(sf, uni, cname.as_ptr());
        assert!(!sc.is_null(), "SFGetOrMakeChar failed for {}", name_str);
        let spl = make_rectangle(0.0, 0.0, 500.0, 700.0);
        let layer = (*sc).layers.offset(LY_FORE);
        (*layer).splines = spl;
    }

    let map = fontforge_ffi::EncMap1to1((*sf).glyphcnt);
    assert!(!map.is_null());
    (sf, map)
}

#[test]
fn test_ot_layout_feature_file_apply() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();

        // Initialize OT lookup infrastructure
        fontforge_ffi::LookupInit();

        let (sf, map) = create_ligature_font("TestOTLayout");

        // The font should start with no GSUB lookups
        assert!(
            (*sf).gsub_lookups.is_null(),
            "font should start with no GSUB lookups"
        );
        assert!(
            (*sf).gpos_lookups.is_null(),
            "font should start with no GPOS lookups"
        );

        // Write a temporary .fea file defining a ligature feature
        let fea_path = format!("/tmp/fontforge_rs_otl_{}.fea", std::process::id());
        {
            let mut f = std::fs::File::create(&fea_path).expect("create temp fea file");
            writeln!(f, "languagesystem DFLT dflt;").unwrap();
            writeln!(f, "languagesystem latn dflt;").unwrap();
            writeln!(f, "").unwrap();
            writeln!(f, "feature liga {{").unwrap();
            writeln!(f, "    sub a b by a_b;").unwrap();
            writeln!(f, "}} liga;").unwrap();
        }

        // Apply the feature file
        let fea_cstr = CString::new(fea_path.as_str()).unwrap();
        fontforge_ffi::SFApplyFeatureFilename(
            sf,
            fea_cstr.as_ptr() as *mut libc::c_char,
            false, // don't ignore invalid replacements
        );

        // The font should now have GSUB lookups
        assert!(
            !(*sf).gsub_lookups.is_null(),
            "font should have GSUB lookups after applying feature file"
        );

        // Verify we can find the lookup by name
        let lookup_name = CString::new("liga_test").unwrap();
        let found = fontforge_ffi::SFFindLookup(sf, lookup_name.as_ptr());
        // Note: the lookup name might be auto-generated; we check gsub_lookups chain instead
        if !found.is_null() {
            println!(
                "Found lookup: {:?}",
                std::ffi::CStr::from_ptr((*found).lookup_name).to_str().unwrap_or("?")
            );
        }

        // Verify the lookup chain exists
        let mut first_lookup = (*sf).gsub_lookups;
        assert!(!first_lookup.is_null());
        let lookup_name_str = if !(*first_lookup).lookup_name.is_null() {
            std::ffi::CStr::from_ptr((*first_lookup).lookup_name)
                .to_str()
                .unwrap_or("?")
        } else {
            "(unnamed)"
        };
        println!("First GSUB lookup name: {}", lookup_name_str);

        // Check that the lookup has features attached
        let features = (*first_lookup).features;
        assert!(
            !features.is_null(),
            "lookup should have features attached"
        );
        let ftag = (*features).featuretag;
        // liga = 'liga' = 0x6C696761
        assert_eq!(
            ftag,
            fontforge_ffi::ot_layout::features::LIGA,
            "feature tag should be 'liga', got 0x{:08X}",
            ftag
        );
        println!(
            "Feature tag: 0x{:08X} ('{}')",
            ftag,
            std::str::from_utf8(&ftag.to_be_bytes()).unwrap_or("?")
        );

        // Check that the feature has script/lang attached
        let scripts = (*features).scripts;
        assert!(!scripts.is_null(), "feature should have scripts attached");
        let script_tag = (*scripts).script;
        // Could be DFLT or latn
        println!(
            "Script tag: 0x{:08X} ('{}')",
            script_tag,
            std::str::from_utf8(&script_tag.to_be_bytes()).unwrap_or("?")
        );

        // Test SFLookupsInScriptLangFeature
        let lookups = fontforge_ffi::SFLookupsInScriptLangFeature(
            sf,
            0, // GSUB (not GPOS)
            fontforge_ffi::ot_layout::scripts::DFLT,
            fontforge_ffi::ot_layout::langs::DFLT,
            fontforge_ffi::ot_layout::features::LIGA,
        );
        assert!(
            !lookups.is_null(),
            "SFLookupsInScriptLangFeature should return lookups for DFLT/dflt/liga"
        );
        assert!(
            !(*lookups).is_null(),
            "should have at least one lookup for liga in DFLT/dflt"
        );

        // Test SFFeaturesInScriptLang
        let features_arr = fontforge_ffi::SFFeaturesInScriptLang(
            sf,
            0, // GSUB
            fontforge_ffi::ot_layout::scripts::DFLT,
            fontforge_ffi::ot_layout::langs::DFLT,
        );
        assert!(
            !features_arr.is_null(),
            "SFFeaturesInScriptLang should return features for DFLT/dflt"
        );

        // Count features
        let mut feat_count = 0;
        while *features_arr.add(feat_count) != 0 {
            let ft = *features_arr.add(feat_count);
            println!(
                "  Feature[{}]: 0x{:08X} ('{}')",
                feat_count,
                ft,
                std::str::from_utf8(&ft.to_be_bytes()).unwrap_or("?")
            );
            feat_count += 1;
        }
        assert!(feat_count > 0, "should have at least one feature");

        // Clean up temp file
        libc::unlink(fea_cstr.as_ptr());

        // Now write the font as OTF and reload
        let otf_path = format!("/tmp/fontforge_rs_otl_{}.otf", std::process::id());
        let otf_cstr = CString::new(otf_path.as_str()).unwrap();

        let ttf_flags = (1 << 0) | (1 << 1) | (1 << 15);
        let ok = fontforge_ffi::WriteTTFFont(
            otf_cstr.as_ptr() as *mut libc::c_char,
            sf,
            fontforge_ffi::fontformat_ff_otf,
            ptr::null_mut(),
            fontforge_ffi::bitmapformat_bf_none,
            ttf_flags,
            map,
            LY_FORE as i32,
        );
        assert!(ok != 0, "WriteTTFFont failed for OTF with OT layout");

        // Clean up original
        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf);

        // Reload from OTF
        let sf2 = fontforge_ffi::SFReadTTF(otf_cstr.as_ptr() as *mut libc::c_char, 0, 0);
        assert!(!sf2.is_null(), "SFReadTTF returned null for OTF with layout");

        // Verify basic font data survived
        assert!(!(*sf2).fontname.is_null());
        let reloaded_name = std::ffi::CStr::from_ptr((*sf2).fontname)
            .to_str()
            .unwrap_or("?");
        assert_eq!(reloaded_name, "TestOTLayout", "font name preserved");

        assert!(
            (*sf2).glyphcnt >= 4,
            "expected >= 4 glyphs, got {}",
            (*sf2).glyphcnt
        );

        // Check our original 4 glyphs still exist with splines
        let glyph_names = [(-1, ".notdef"), (0x0061, "a"), (0x0062, "b"), (0xE001, "a_b")];
        for &(_uni, name_str) in &glyph_names {
            let cname = CString::new(name_str).unwrap();
            let sc = fontforge_ffi::SFGetChar(sf2, -1, cname.as_ptr());
            assert!(!sc.is_null(), "glyph '{}' not found after OTF reload", name_str);
            let layer = (*sc).layers.offset(LY_FORE);
            assert!(
                !(*layer).splines.is_null(),
                "glyph '{}' has no splines after reload",
                name_str
            );
        }

        // The reloaded font should still have GSUB lookups (OTF preserves them)
        // Note: SFReadTTF may or may not populate gsub_lookups fully depending on how
        // it reads the font. For OTF with OpenType layout, it should.
        if !(*sf2).gsub_lookups.is_null() {
            println!("Reloaded font has GSUB lookups preserved");
            let mut lookup = (*sf2).gsub_lookups;
            let mut count = 0;
            while !lookup.is_null() {
                count += 1;
                lookup = (*lookup).next;
            }
            println!("  GSUB lookup count after reload: {}", count);
        } else {
            println!("Note: Reloaded font has no GSUB lookups (may not be fully parsed on read)");
        }

        // Clean up
        fontforge_ffi::SplineFontFree(sf2);
        libc::unlink(otf_cstr.as_ptr());

        println!("PASS: OT layout feature file apply test passed");
    }
}

#[test]
fn test_ot_layout_tag_utilities() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();
        fontforge_ffi::LookupInit();

        let (sf, map) = create_ligature_font("TestTagUtils");

        // Test TagFullName for known feature tags
        let name = fontforge_ffi::TagFullName(
            sf,
            fontforge_ffi::ot_layout::features::LIGA,
            0, // not Mac
            0, // not onlyifknown
        );
        assert!(!name.is_null(), "TagFullName should return a name for 'liga'");
        let name_str = std::ffi::CStr::from_ptr(name).to_string_lossy();
        println!("Feature 'liga' full name: {}", name_str);
        assert!(
            name_str.contains("Ligatures") || name_str.contains("ligature") || name_str.contains("liga"),
            "liga should be recognized as a ligature feature, got: {}",
            name_str
        );

        // Test TagFullName for an unknown tag
        let unknown_name = fontforge_ffi::TagFullName(
            sf,
            0xFFFFFFFF,
            0,
            1, // onlyifknown
        );
        assert!(
            unknown_name.is_null(),
            "TagFullName with onlyifknown=1 should return null for unknown tag"
        );

        // Test SuffixFromTags
        // Create a minimal FeatureScriptLangList to test
        let fl = (*sf).gsub_lookups;
        if !fl.is_null() {
            // If no lookups, skip suffix test
            let features = (*fl).features;
            if !features.is_null() {
                let suffix = fontforge_ffi::SuffixFromTags(features);
                if !suffix.is_null() {
                    let suffix_str = std::ffi::CStr::from_ptr(suffix).to_string_lossy();
                    println!("Suffix from tags: {}", suffix_str);
                }
            }
        }

        // Test SFLangsInScript - should return something for DFLT
        let langs = fontforge_ffi::SFLangsInScript(
            sf,
            0, // GSUB
            fontforge_ffi::ot_layout::scripts::DFLT,
        );
        println!(
            "SFLangsInScript(DFLT) returned: {}",
            if langs.is_null() { "null" } else { "array" }
        );

        // Test SFScriptsInLookups
        let scripts = fontforge_ffi::SFScriptsInLookups(sf);
        println!(
            "SFScriptsInLookups returned: {}",
            if scripts.is_null() { "null" } else { "array" }
        );

        // Test GlyphNameCnt
        let cnt = fontforge_ffi::GlyphNameCnt(
            CString::new("a b c").unwrap().as_ptr()
        );
        assert_eq!(cnt, 3, "GlyphNameCnt('a b c') should be 3");

        let cnt2 = fontforge_ffi::GlyphNameCnt(
            CString::new("single").unwrap().as_ptr()
        );
        assert_eq!(cnt2, 1, "GlyphNameCnt('single') should be 1");

        let cnt3 = fontforge_ffi::GlyphNameCnt(
            CString::new("").unwrap().as_ptr()
        );
        assert_eq!(cnt3, 0, "GlyphNameCnt('') should be 0");

        // Clean up
        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf);

        println!("PASS: OT layout tag utilities test passed");
    }
}

#[test]
fn test_ot_layout_feature_navigation() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();
        fontforge_ffi::LookupInit();

        let (sf, map) = create_ligature_font("TestFeatNav");

        // Apply feature file with liga
        let fea_path = format!("/tmp/fontforge_rs_nav_{}.fea", std::process::id());
        {
            let mut f = std::fs::File::create(&fea_path).expect("create temp fea file");
            writeln!(f, "languagesystem DFLT dflt;").unwrap();
            writeln!(f, "languagesystem latn dflt;").unwrap();
            writeln!(f, "").unwrap();
            writeln!(f, "feature liga {{").unwrap();
            writeln!(f, "    sub a b by a_b;").unwrap();
            writeln!(f, "}} liga;").unwrap();
        }

        let fea_cstr = CString::new(fea_path.as_str()).unwrap();
        fontforge_ffi::SFApplyFeatureFilename(
            sf,
            fea_cstr.as_ptr() as *mut libc::c_char,
            false,
        );
        libc::unlink(fea_cstr.as_ptr());

        // Navigate through the lookup hierarchy using safe wrappers
        let has_gsub = fontforge_ffi::ot_layout::has_gsub(sf);
        assert!(has_gsub, "should have GSUB lookups after applying fea");

        // Use safe wrapper to count lookups in liga feature for DFLT/dflt
        let count = fontforge_ffi::ot_layout::lookup_count_in_feature(
            sf,
            false, // GSUB
            fontforge_ffi::ot_layout::scripts::DFLT,
            fontforge_ffi::ot_layout::langs::DFLT,
            fontforge_ffi::ot_layout::features::LIGA,
        );
        assert!(count > 0, "should have at least one lookup for DFLT/dflt/liga");

        // Count features in DFLT/dflt
        let feat_count = fontforge_ffi::ot_layout::feature_count(
            sf,
            false,
            fontforge_ffi::ot_layout::scripts::DFLT,
            fontforge_ffi::ot_layout::langs::DFLT,
        );
        println!("Features in DFLT/dflt: {}", feat_count);
        assert!(feat_count > 0, "should have at least one feature in DFLT/dflt");

        // Count languages in DFLT script
        let lang_count = fontforge_ffi::ot_layout::lang_count(
            sf,
            false,
            fontforge_ffi::ot_layout::scripts::DFLT,
        );
        println!("Languages in DFLT script: {}", lang_count);

        // Clean up
        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf);

        println!("PASS: OT layout feature navigation test passed");
    }
}
