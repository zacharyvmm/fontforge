// Integration tests: pure-Rust spline simplify/extrema/direction vs C FFI.
//
// For each operation, we:
// 1. Create a font with test contours via FFI
// 2. Run the C operation
// 3. Extract the resulting contours
// 4. Run the pure-Rust equivalent on the same input
// 5. Compare contour counts and bounding boxes

use std::ffi::CString;

const LY_FORE: isize = 1;

// ---------------------------------------------------------------------------
// Helper: Create a basic SplineFont with one character
// ---------------------------------------------------------------------------

unsafe fn create_font() -> (*mut fontforge_ffi::SplineFont, *mut fontforge_ffi::SplineChar) {
    fontforge_ffi::doinitFontForgeMain();

    let sf = fontforge_ffi::SplineFontNew();
    assert!(!sf.is_null());

    let fontname = CString::new("SplineRustTest").unwrap();
    if !(*sf).fontname.is_null() {
        libc::free((*sf).fontname as *mut libc::c_void);
    }
    (*sf).fontname = libc::strdup(fontname.as_ptr());

    let cname = CString::new("A").unwrap();
    let sc = fontforge_ffi::SFGetOrMakeChar(sf, 0x0041, cname.as_ptr());
    assert!(!sc.is_null());

    (sf, sc)
}

// ---------------------------------------------------------------------------
// Helper: Extract contours (as Vec<Vec<Point>>) from a SplineSet linked list
// ---------------------------------------------------------------------------

unsafe fn extract_contours_from_splineset(
    head: *mut fontforge_ffi::SplineSet,
) -> Vec<Vec<fontforge_ffi::spline_ops::Point>> {
    use fontforge_ffi::spline_ops::Point;
    let mut contours: Vec<Vec<Point>> = Vec::new();
    let mut cur_ss = head;

    while !cur_ss.is_null() {
        let mut points: Vec<Point> = Vec::new();
        let first = (*cur_ss).first;
        if !first.is_null() {
            let mut cur_pt: *mut fontforge_ffi::SplinePoint = first;
            loop {
                points.push(Point::new((*cur_pt).me.x, (*cur_pt).me.y));
                let spline = (*cur_pt).next;
                if spline.is_null() {
                    break;
                }
                let next_pt = (*spline).to;
                if next_pt.is_null() || next_pt == first {
                    break;
                }
                cur_pt = next_pt;
            }
        }
        if points.len() >= 3 {
            contours.push(points);
        }
        cur_ss = (*cur_ss).next;
    }

    contours
}

// ---------------------------------------------------------------------------
// Helper: Create a simple rectangular contour via FFI
// ---------------------------------------------------------------------------

unsafe fn create_rect_contour(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) -> *mut fontforge_ffi::SplinePointList {
    let sp = [
        fontforge_ffi::SplinePointCreate(x1, y1),
        fontforge_ffi::SplinePointCreate(x2, y1),
        fontforge_ffi::SplinePointCreate(x2, y2),
        fontforge_ffi::SplinePointCreate(x1, y2),
    ];
    fontforge_ffi::SplineMake(sp[0], sp[1], 0);
    fontforge_ffi::SplineMake(sp[1], sp[2], 0);
    fontforge_ffi::SplineMake(sp[2], sp[3], 0);
    fontforge_ffi::SplineMake(sp[3], sp[0], 0);

    let spl = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
        as *mut fontforge_ffi::SplinePointList;
    (*spl).first = sp[0];
    (*spl).last = sp[0];
    fontforge_ffi::SPLCategorizePoints(spl);
    spl
}

// ===========================================================================
// Test 1: Direction Correction — pure-Rust vs C SplineSetsCorrect
// ===========================================================================

#[test]
fn test_direction_pure_rust_vs_c() {
    unsafe {
        let (sf, sc) = create_font();
        let layer = (*sc).layers.offset(LY_FORE);

        // Create an outer contour (clockwise) and inner (CCW in C convention)
        let outer = create_rect_contour(0.0, 0.0, 500.0, 700.0);
        let inner = create_rect_contour(100.0, 100.0, 400.0, 600.0);

        // Link contours
        (*outer).next = inner;

        // Extract input contours before correction
        let input_before = extract_contours_from_splineset(outer as *mut fontforge_ffi::SplineSet);
        assert_eq!(input_before.len(), 2, "Should have 2 input contours");

        // Run C SplineSetsCorrect
        let mut changed: i32 = 0;
        let result_c =
            fontforge_ffi::SplineSetsCorrect(outer as *mut fontforge_ffi::SplineSet, &mut changed);
        let c_contours = extract_contours_from_splineset(result_c);
        let c_count = c_contours.len();
        eprintln!("C SplineSetsCorrect produced {} contour(s), changed={}", c_count, changed);

        // Run pure-Rust correct_contour_directions
        use fontforge_ffi::spline_ops;
        let rust_contours = spline_ops::correct_contour_directions(&input_before);
        let rust_count = rust_contours.len();
        eprintln!(
            "Rust correct_contour_directions produced {} contour(s)",
            rust_count
        );

        // Compare contour counts
        assert_eq!(
            rust_count, c_count,
            "Rust and C should produce same contour count: Rust={} vs C={}",
            rust_count, c_count
        );

        // Compare bounding boxes
        let c_bbox = spline_ops::contours_bbox(&c_contours);
        let rust_bbox = spline_ops::contours_bbox(&rust_contours);

        eprintln!("C bbox: {:?}", c_bbox);
        eprintln!("Rust bbox: {:?}", rust_bbox);

        let eps = 1.0;
        assert!(
            (c_bbox.0 - rust_bbox.0).abs() < eps,
            "min_x mismatch: C={} vs Rust={}",
            c_bbox.0,
            rust_bbox.0
        );
        assert!(
            (c_bbox.1 - rust_bbox.1).abs() < eps,
            "min_y mismatch: C={} vs Rust={}",
            c_bbox.1,
            rust_bbox.1
        );
        assert!(
            (c_bbox.2 - rust_bbox.2).abs() < eps,
            "max_x mismatch: C={} vs Rust={}",
            c_bbox.2,
            rust_bbox.2
        );
        assert!(
            (c_bbox.3 - rust_bbox.3).abs() < eps,
            "max_y mismatch: C={} vs Rust={}",
            c_bbox.3,
            rust_bbox.3
        );

        // Verify outer contour is clockwise in Rust result
        use fontforge_ffi::overlap;
        assert!(
            overlap::is_clockwise(&rust_contours[0]),
            "Rust outer contour should be clockwise"
        );

        // Verify SFD round-trip still works
        let map = fontforge_ffi::EncMap1to1((*sf).glyphcnt);
        assert!(!map.is_null());

        let tmpfile = CString::new(format!("/tmp/fontforge_rs_spline_dir_{}.sfd", std::process::id()))
            .unwrap();
        (*layer).splines = result_c;
        let ok = fontforge_ffi::SFDWrite(tmpfile.as_ptr() as *mut libc::c_char, sf, map, map, 0);
        assert!(ok != 0, "SFDWrite after direction correction failed");

        let sf2 = fontforge_ffi::LoadSplineFont(tmpfile.as_ptr(), 0);
        assert!(!sf2.is_null(), "LoadSplineFont returned null");
        assert_eq!((*sf2).glyphcnt, (*sf).glyphcnt);

        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf2);
        fontforge_ffi::SplineFontFree(sf);
        libc::unlink(tmpfile.as_ptr());
    }
}

// ===========================================================================
// Test 2: Simplify — pure-Rust vs C SplineCharSimplify
// ===========================================================================

#[test]
fn test_simplify_pure_rust_vs_c() {
    unsafe {
        let (sf, sc) = create_font();
        let layer = (*sc).layers.offset(LY_FORE);

        // Create a rectangle with extra colinear points on the top edge
        let sp = [
            fontforge_ffi::SplinePointCreate(0.0, 0.0),
            fontforge_ffi::SplinePointCreate(100.0, 0.0),
            fontforge_ffi::SplinePointCreate(200.0, 0.0),
            fontforge_ffi::SplinePointCreate(300.0, 0.0),
            fontforge_ffi::SplinePointCreate(300.0, 200.0),
            fontforge_ffi::SplinePointCreate(0.0, 200.0),
        ];
        for i in 0..5 {
            fontforge_ffi::SplineMake(sp[i], sp[i + 1], 0);
        }
        fontforge_ffi::SplineMake(sp[5], sp[0], 0);

        let spl = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
            as *mut fontforge_ffi::SplinePointList;
        (*spl).first = sp[0];
        (*spl).last = sp[0];
        fontforge_ffi::SPLCategorizePoints(spl);

        (*layer).splines = spl;

        // Extract input contours
        let input_contours = extract_contours_from_splineset(spl as *mut fontforge_ffi::SplineSet);
        eprintln!("Input contour has {} points", input_contours[0].len());

        // Run C simplify
        let mut smpl: fontforge_ffi::simplifyinfo = std::mem::zeroed();
        smpl.flags = fontforge_ffi::simpify_flags_sf_normal as i32;
        smpl.err = 0.1;
        smpl.tan_bounds = 0.1;
        smpl.linefixup = 0.1;
        smpl.linelenmax = 10000.0;
        smpl.set_as_default = 0;
        smpl.check_selected_contours = 0;

        let c_result = fontforge_ffi::SplineCharSimplify(
            sc,
            spl as *mut fontforge_ffi::SplineSet,
            &mut smpl,
        );
        let c_contours = extract_contours_from_splineset(c_result);
        let c_count = c_contours.len();

        if c_contours.len() > 0 {
            eprintln!(
                "C simplify produced {} contour(s), first has {} points",
                c_count,
                c_contours[0].len()
            );
        } else {
            eprintln!("C simplify produced {} contour(s)", c_count);
        }

        // Run pure-Rust simplify on the extracted input
        use fontforge_ffi::spline_ops;
        let rust_contours = spline_ops::simplify_contours(&input_contours);
        let rust_count = rust_contours.len();

        if rust_contours.len() > 0 {
            eprintln!(
                "Rust simplify produced {} contour(s), first has {} points",
                rust_count,
                rust_contours[0].len()
            );
        } else {
            eprintln!("Rust simplify produced {} contour(s)", rust_count);
        }

        // Both should at least produce the same number of contours
        assert_eq!(
            rust_count, c_count,
            "Rust and C should produce same contour count: Rust={} vs C={}",
            rust_count, c_count
        );

        // Compare bounding boxes
        if c_count > 0 {
            let c_bbox = spline_ops::contours_bbox(&c_contours);
            let rust_bbox = spline_ops::contours_bbox(&rust_contours);
            eprintln!("C bbox: {:?}", c_bbox);
            eprintln!("Rust bbox: {:?}", rust_bbox);

            let eps = 2.0;
            assert!(
                (c_bbox.0 - rust_bbox.0).abs() < eps,
                "min_x mismatch: C={} vs Rust={}",
                c_bbox.0,
                rust_bbox.0
            );
            assert!(
                (c_bbox.1 - rust_bbox.1).abs() < eps,
                "min_y mismatch: C={} vs Rust={}",
                c_bbox.1,
                rust_bbox.1
            );
            assert!(
                (c_bbox.2 - rust_bbox.2).abs() < eps,
                "max_x mismatch: C={} vs Rust={}",
                c_bbox.2,
                rust_bbox.2
            );
            assert!(
                (c_bbox.3 - rust_bbox.3).abs() < eps,
                "max_y mismatch: C={} vs Rust={}",
                c_bbox.3,
                rust_bbox.3
            );
        }

        // Verify SFD round-trip
        let map = fontforge_ffi::EncMap1to1((*sf).glyphcnt);
        assert!(!map.is_null());
        let tmpfile = CString::new(format!("/tmp/fontforge_rs_spline_simplify_{}.sfd", std::process::id()))
            .unwrap();
        (*layer).splines = c_result;
        let ok = fontforge_ffi::SFDWrite(tmpfile.as_ptr() as *mut libc::c_char, sf, map, map, 0);
        assert!(ok != 0, "SFDWrite after simplify failed");
        let sf2 = fontforge_ffi::LoadSplineFont(tmpfile.as_ptr(), 0);
        assert!(!sf2.is_null(), "LoadSplineFont after simplify returned null");
        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf2);
        libc::unlink(tmpfile.as_ptr());

        fontforge_ffi::SplineFontFree(sf);
    }
}

// ===========================================================================
// Test 3: Add Extrema — pure-Rust vs C SplineCharAddExtrema
// ===========================================================================

#[test]
fn test_extrema_pure_rust_vs_c() {
    unsafe {
        let (sf, sc) = create_font();
        let layer = (*sc).layers.offset(LY_FORE);

        // Create a 4-point closed contour (rectangle).
        // For straight lines, add extrema is a no-op, but we verify
        // contour count and bbox are preserved.
        let outer = create_rect_contour(0.0, 0.0, 500.0, 400.0);
        (*layer).splines = outer;

        // Extract input contours
        let input_contours =
            extract_contours_from_splineset(outer as *mut fontforge_ffi::SplineSet);
        assert!(!input_contours.is_empty(), "Should have at least 1 input contour");
        eprintln!("Input contour has {} points", input_contours[0].len());

        // Run C add extrema
        fontforge_ffi::SplineCharAddExtrema(
            sc,
            outer as *mut fontforge_ffi::SplineSet,
            fontforge_ffi::ae_type_ae_all,
            1000,
        );
        // After C add extrema, re-extract
        let c_contours = extract_contours_from_splineset(outer as *mut fontforge_ffi::SplineSet);
        let c_count = c_contours.len();
        if c_contours.len() > 0 {
            eprintln!(
                "C add extrema produced {} contour(s), first has {} points",
                c_count,
                c_contours[0].len()
            );
        } else {
            eprintln!("C add extrema produced {} contour(s)", c_count);
        }

        // Run pure-Rust add extrema on the input
        use fontforge_ffi::spline_ops;
        let rust_contours = spline_ops::add_extrema_to_contours(&input_contours);
        let rust_count = rust_contours.len();
        if rust_contours.len() > 0 {
            eprintln!(
                "Rust add extrema produced {} contour(s), first has {} points",
                rust_count,
                rust_contours[0].len()
            );
        } else {
            eprintln!("Rust add extrema produced {} contour(s)", rust_count);
        }

        // Compare contour counts
        assert_eq!(
            rust_count, c_count,
            "Rust and C should produce same contour count: Rust={} vs C={}",
            rust_count, c_count
        );

        // Compare bounding boxes
        if c_count > 0 {
            let c_bbox = spline_ops::contours_bbox(&c_contours);
            let rust_bbox = spline_ops::contours_bbox(&rust_contours);
            eprintln!("C bbox: {:?}", c_bbox);
            eprintln!("Rust bbox: {:?}", rust_bbox);

            let eps = 5.0;
            assert!(
                (c_bbox.0 - rust_bbox.0).abs() < eps,
                "min_x mismatch: C={} vs Rust={}",
                c_bbox.0,
                rust_bbox.0
            );
            assert!(
                (c_bbox.1 - rust_bbox.1).abs() < eps,
                "min_y mismatch: C={} vs Rust={}",
                c_bbox.1,
                rust_bbox.1
            );
            assert!(
                (c_bbox.2 - rust_bbox.2).abs() < eps,
                "max_x mismatch: C={} vs Rust={}",
                c_bbox.2,
                rust_bbox.2
            );
            assert!(
                (c_bbox.3 - rust_bbox.3).abs() < eps,
                "max_y mismatch: C={} vs Rust={}",
                c_bbox.3,
                rust_bbox.3
            );
        }

        // Verify SFD round-trip
        let map = fontforge_ffi::EncMap1to1((*sf).glyphcnt);
        assert!(!map.is_null());
        let tmpfile = CString::new(format!(
            "/tmp/fontforge_rs_spline_extrema_{}.sfd",
            std::process::id()
        ))
        .unwrap();
        let ok = fontforge_ffi::SFDWrite(tmpfile.as_ptr() as *mut libc::c_char, sf, map, map, 0);
        assert!(ok != 0, "SFDWrite after add extrema failed");
        let sf2 = fontforge_ffi::LoadSplineFont(tmpfile.as_ptr(), 0);
        assert!(!sf2.is_null(), "LoadSplineFont after add extrema returned null");
        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf2);
        libc::unlink(tmpfile.as_ptr());

        fontforge_ffi::SplineFontFree(sf);
    }
}

// ===========================================================================
// Test 4: All three operations combined — simulate a full pipeline
// ===========================================================================

#[test]
fn test_spline_combined_pipeline_vs_c() {
    unsafe {
        let (sf, sc) = create_font();
        let layer = (*sc).layers.offset(LY_FORE);

        // Create two overlapping rectangular contours
        let outer = create_rect_contour(0.0, 0.0, 500.0, 700.0);
        let inner = create_rect_contour(200.0, -100.0, 700.0, 800.0);
        (*outer).next = inner;
        (*layer).splines = outer;

        // Extract input
        let input_contours = extract_contours_from_splineset(outer as *mut fontforge_ffi::SplineSet);
        eprintln!("Combined test: {} input contours", input_contours.len());
        for (i, c) in input_contours.iter().enumerate() {
            eprintln!("  contour {}: {} points", i, c.len());
        }

        // --- C pipeline: correct -> add extrema -> simplify ---
        let mut changed: i32 = 0;
        let head = (*layer).splines;

        // Step 1: C correct direction
        fontforge_ffi::SplineSetsCorrect(head, &mut changed);
        eprintln!("C correct direction: changed={}", changed);

        // Step 2: C add extrema
        fontforge_ffi::SplineCharAddExtrema(sc, head, fontforge_ffi::ae_type_ae_all, 1000);

        // Step 3: C simplify
        let mut smpl: fontforge_ffi::simplifyinfo = std::mem::zeroed();
        smpl.flags = fontforge_ffi::simpify_flags_sf_normal as i32;
        smpl.err = 1.0;
        smpl.tan_bounds = 0.1;
        smpl.linefixup = 0.1;
        smpl.linelenmax = 10000.0;
        smpl.set_as_default = 0;
        smpl.check_selected_contours = 0;

        let c_result = fontforge_ffi::SplineCharSimplify(sc, head, &mut smpl);
        (*layer).splines = c_result;
        let c_contours = extract_contours_from_splineset(c_result);
        let c_count = c_contours.len();
        eprintln!("C pipeline produced {} contour(s)", c_count);

        // --- Rust pipeline: correct -> add extrema -> simplify ---
        use fontforge_ffi::spline_ops;

        let mut rust_contours = spline_ops::correct_contour_directions(&input_contours);
        rust_contours = spline_ops::add_extrema_to_contours(&rust_contours);
        rust_contours = spline_ops::simplify_contours(&rust_contours);
        let rust_count = rust_contours.len();
        eprintln!("Rust pipeline produced {} contour(s)", rust_count);

        // Compare contour counts
        assert_eq!(
            rust_count, c_count,
            "Combined pipeline: Rust={} vs C={}",
            rust_count, c_count
        );

        // Compare bounding boxes
        if c_count > 0 {
            let c_bbox = spline_ops::contours_bbox(&c_contours);
            let rust_bbox = spline_ops::contours_bbox(&rust_contours);
            eprintln!("C bbox: {:?}", c_bbox);
            eprintln!("Rust bbox: {:?}", rust_bbox);

            let eps = 5.0;
            assert!(
                (c_bbox.0 - rust_bbox.0).abs() < eps,
                "Combined min_x mismatch: C={} vs Rust={}",
                c_bbox.0,
                rust_bbox.0
            );
            assert!(
                (c_bbox.1 - rust_bbox.1).abs() < eps,
                "Combined min_y mismatch: C={} vs Rust={}",
                c_bbox.1,
                rust_bbox.1
            );
            assert!(
                (c_bbox.2 - rust_bbox.2).abs() < eps,
                "Combined max_x mismatch: C={} vs Rust={}",
                c_bbox.2,
                rust_bbox.2
            );
            assert!(
                (c_bbox.3 - rust_bbox.3).abs() < eps,
                "Combined max_y mismatch: C={} vs Rust={}",
                c_bbox.3,
                rust_bbox.3
            );
        }

        // Ensure Rust contours are properly directed
        use fontforge_ffi::overlap;
        for (i, contour) in rust_contours.iter().enumerate() {
            assert!(
                overlap::is_clockwise(contour),
                "Rust contour {} should be clockwise after correction",
                i
            );
        }

        // Verify SFD round-trip
        let map = fontforge_ffi::EncMap1to1((*sf).glyphcnt);
        assert!(!map.is_null());
        let tmpfile =
            CString::new(format!("/tmp/fontforge_rs_spline_combined_{}.sfd", std::process::id()))
                .unwrap();
        let ok = fontforge_ffi::SFDWrite(tmpfile.as_ptr() as *mut libc::c_char, sf, map, map, 0);
        assert!(ok != 0, "SFDWrite after combined pipeline failed");
        let sf2 = fontforge_ffi::LoadSplineFont(tmpfile.as_ptr(), 0);
        assert!(!sf2.is_null(), "LoadSplineFont after combined pipeline returned null");
        assert_eq!((*sf2).glyphcnt, (*sf).glyphcnt);
        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf2);
        libc::unlink(tmpfile.as_ptr());

        fontforge_ffi::SplineFontFree(sf);
    }
}
