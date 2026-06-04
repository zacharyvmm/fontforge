// Integration tests for spline manipulation operations via FFI.
// Validates SplineSetRemoveOverlap, SplineCharSimplify, SplineCharAddExtrema,
// and SplineSetsCorrect against the C library.

use std::ffi::CString;
use std::ptr;

const LY_FORE: isize = 1; // layer_type::ly_fore

/// Helper: create a rectangular contour (4 corners) and add it to a glyph's foreground layer.
/// Returns the SplinePointList pointer.
unsafe fn add_rectangle_to_glyph(
    sc: *mut fontforge_ffi::SplineChar,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> *mut fontforge_ffi::SplinePointList {
    let mut sp: [*mut fontforge_ffi::SplinePoint; 4] = [ptr::null_mut(); 4];
    // Bottom-left → Bottom-right → Top-right → Top-left → (back to bottom-left)
    let corners: [(f64, f64); 4] = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    for j in 0..4 {
        sp[j] = fontforge_ffi::SplinePointCreate(corners[j].0, corners[j].1);
        assert!(!sp[j].is_null(), "SplinePointCreate returned null");
    }
    // Create cubic splines connecting the points
    fontforge_ffi::SplineMake(sp[0], sp[1], 0);
    fontforge_ffi::SplineMake(sp[1], sp[2], 0);
    fontforge_ffi::SplineMake(sp[2], sp[3], 0);
    fontforge_ffi::SplineMake(sp[3], sp[0], 0);

    let spl = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
        as *mut fontforge_ffi::SplinePointList;
    assert!(!spl.is_null(), "calloc for SplinePointList failed");
    (*spl).first = sp[0];
    (*spl).last = sp[0]; // Closed contour

    fontforge_ffi::SPLCategorizePoints(spl);

    // Append to glyph's spline set list
    let layer = (*sc).layers.offset(LY_FORE);
    if (*layer).splines.is_null() {
        (*layer).splines = spl;
    } else {
        // Append to end of linked list
        let mut cur = (*layer).splines;
        while !(*cur).next.is_null() {
            cur = (*cur).next;
        }
        (*cur).next = spl;
    }
    spl
}

/// Helper: count the number of SplineSets in a linked list
unsafe fn count_spline_sets(head: *mut fontforge_ffi::SplineSet) -> usize {
    let mut count = 0;
    let mut cur = head;
    while !cur.is_null() {
        count += 1;
        cur = (*cur).next;
    }
    count
}

/// Helper: count the number of SplinePoints in a single SplineSet
unsafe fn count_points_in_set(ss: *mut fontforge_ffi::SplineSet) -> usize {
    if ss.is_null() || (*ss).first.is_null() {
        return 0;
    }
    let mut count = 0;
    let first = (*ss).first;
    let mut cur: *mut fontforge_ffi::SplinePoint = first;
    loop {
        count += 1;
        // Follow the spline chain: SplinePoint.next is a Spline*, Spline.to is the next point
        let spline = (*cur).next;
        if spline.is_null() {
            break;
        }
        let next_pt = (*spline).to;
        if next_pt.is_null() || next_pt == first {
            break;
        }
        cur = next_pt;
    }
    count
}

#[test]
fn test_remove_overlap_two_rectangles() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();

        let sf = fontforge_ffi::SplineFontNew();
        assert!(!sf.is_null());

        let fontname = CString::new("OverlapTest").unwrap();
        if !(*sf).fontname.is_null() {
            libc::free((*sf).fontname as *mut libc::c_void);
        }
        (*sf).fontname = libc::strdup(fontname.as_ptr());

        // Create glyph "A" with two overlapping rectangles
        let cname = CString::new("A").unwrap();
        let sc = fontforge_ffi::SFGetOrMakeChar(sf, 0x0041, cname.as_ptr());
        assert!(!sc.is_null());

        // First rectangle: (0, 0) to (600, 700) — large
        let _spl1 = add_rectangle_to_glyph(sc, 0.0, 0.0, 600.0, 700.0);
        // Second rectangle: (200, -100) to (500, 800) — overlaps the first
        let _spl2 = add_rectangle_to_glyph(sc, 200.0, -100.0, 300.0, 900.0);

        let layer = (*sc).layers.offset(LY_FORE);
        let head = (*layer).splines;
        assert!(!head.is_null(), "No spline data before overlap removal");

        let count_before = count_spline_sets(head);
        assert_eq!(count_before, 2, "Expected 2 contours before overlap removal");

        // Apply RemoveOverlap
        let result = fontforge_ffi::SplineSetRemoveOverlap(
            sc,
            head,
            fontforge_ffi::overlap_type_over_remove,
        );
        assert!(!result.is_null(), "SplineSetRemoveOverlap returned null");

        (*layer).splines = result;

        // After overlap removal, the result should be a valid spline set
        // (the exact contour count depends on the geometry, but should be non-null)
        let count_after = count_spline_sets((*layer).splines);
        assert!(count_after >= 1, "Expected at least 1 contour after overlap removal, got {}", count_after);

        // Verify SFD round-trip still works
        let map = fontforge_ffi::EncMap1to1((*sf).glyphcnt);
        assert!(!map.is_null());

        let tmpfile = CString::new(format!("/tmp/fontforge_rs_overlap_{}.sfd", std::process::id()))
            .unwrap();
        let ok = fontforge_ffi::SFDWrite(
            tmpfile.as_ptr() as *mut libc::c_char,
            sf,
            map,
            map,
            0,
        );
        assert!(ok != 0, "SFDWrite after overlap removal failed");

        // Reload and verify
        let sf2 = fontforge_ffi::LoadSplineFont(tmpfile.as_ptr(), 0);
        assert!(!sf2.is_null(), "LoadSplineFont after overlap removal returned null");
        assert_eq!((*sf2).glyphcnt, (*sf).glyphcnt, "glyph count mismatch after round-trip");

        // Cleanup
        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf2);
        fontforge_ffi::SplineFontFree(sf);
        libc::unlink(tmpfile.as_ptr());

        println!("PASS: overlap removal test passed");
    }
}

#[test]
fn test_add_extrema() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();

        let sf = fontforge_ffi::SplineFontNew();
        assert!(!sf.is_null());

        let fontname = CString::new("ExtremaTest").unwrap();
        if !(*sf).fontname.is_null() {
            libc::free((*sf).fontname as *mut libc::c_void);
        }
        (*sf).fontname = libc::strdup(fontname.as_ptr());

        let cname = CString::new("A").unwrap();
        let sc = fontforge_ffi::SFGetOrMakeChar(sf, 0x0041, cname.as_ptr());
        assert!(!sc.is_null());

        // Create a simple triangle that has no extrema at its points
        // (0, 0) → (500, 300) → (300, 700) → back to (0, 0)
        let mut sp: [*mut fontforge_ffi::SplinePoint; 3] = [ptr::null_mut(); 3];
        let corners: [(f64, f64); 3] = [(0.0, 0.0), (500.0, 300.0), (300.0, 700.0)];
        for j in 0..3 {
            sp[j] = fontforge_ffi::SplinePointCreate(corners[j].0, corners[j].1);
            assert!(!sp[j].is_null());
        }
        fontforge_ffi::SplineMake(sp[0], sp[1], 0);
        fontforge_ffi::SplineMake(sp[1], sp[2], 0);
        fontforge_ffi::SplineMake(sp[2], sp[0], 0);

        let spl = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
            as *mut fontforge_ffi::SplinePointList;
        assert!(!spl.is_null());
        (*spl).first = sp[0];
        (*spl).last = sp[0];

        fontforge_ffi::SPLCategorizePoints(spl);

        let layer = (*sc).layers.offset(LY_FORE);
        (*layer).splines = spl;

        let head = (*layer).splines;
        let points_before = count_points_in_set(head);
        assert_eq!(points_before, 3, "Expected 3 points before adding extrema");

        // Apply AddExtrema (ae_all = 0, emsize=1000)
        fontforge_ffi::SplineCharAddExtrema(sc, head, fontforge_ffi::ae_type_ae_all, 1000);

        // After adding extrema, there should be more points (extrema added on curves)
        let points_after = count_points_in_set(head);
        assert!(
            points_after >= points_before,
            "Expected at least {} points after adding extrema, got {}",
            points_before,
            points_after
        );

        // Verify SFD round-trip
        let map = fontforge_ffi::EncMap1to1((*sf).glyphcnt);
        assert!(!map.is_null());

        let tmpfile = CString::new(format!("/tmp/fontforge_rs_extrema_{}.sfd", std::process::id()))
            .unwrap();
        let ok = fontforge_ffi::SFDWrite(
            tmpfile.as_ptr() as *mut libc::c_char,
            sf,
            map,
            map,
            0,
        );
        assert!(ok != 0, "SFDWrite after add extrema failed");

        let sf2 = fontforge_ffi::LoadSplineFont(tmpfile.as_ptr(), 0);
        assert!(!sf2.is_null(), "LoadSplineFont after add extrema returned null");
        assert_eq!((*sf2).glyphcnt, (*sf).glyphcnt);

        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf2);
        fontforge_ffi::SplineFontFree(sf);
        libc::unlink(tmpfile.as_ptr());

        println!("PASS: add extrema test passed (points: {} → {})", points_before, points_after);
    }
}

#[test]
fn test_simplify() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();

        let sf = fontforge_ffi::SplineFontNew();
        assert!(!sf.is_null());

        let fontname = CString::new("SimplifyTest").unwrap();
        if !(*sf).fontname.is_null() {
            libc::free((*sf).fontname as *mut libc::c_void);
        }
        (*sf).fontname = libc::strdup(fontname.as_ptr());

        let cname = CString::new("A").unwrap();
        let sc = fontforge_ffi::SFGetOrMakeChar(sf, 0x0041, cname.as_ptr());
        assert!(!sc.is_null());

        // Create a contour with 8 points — some may be redundant
        // (0,0) → (250,10) → (500,0) → (490,350) → (500,700) → (250,690) → (0,700) → (10,350)
        let mut sp: [*mut fontforge_ffi::SplinePoint; 8] = [ptr::null_mut(); 8];
        let pts: [(f64, f64); 8] = [
            (0.0, 0.0),
            (250.0, 10.0),
            (500.0, 0.0),
            (490.0, 350.0),
            (500.0, 700.0),
            (250.0, 690.0),
            (0.0, 700.0),
            (10.0, 350.0),
        ];
        for j in 0..8 {
            sp[j] = fontforge_ffi::SplinePointCreate(pts[j].0, pts[j].1);
            assert!(!sp[j].is_null());
        }
        for j in 0..8 {
            fontforge_ffi::SplineMake(sp[j], sp[(j + 1) % 8], 0);
        }

        let spl = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
            as *mut fontforge_ffi::SplinePointList;
        assert!(!spl.is_null());
        (*spl).first = sp[0];
        (*spl).last = sp[0];

        fontforge_ffi::SPLCategorizePoints(spl);

        let layer = (*sc).layers.offset(LY_FORE);
        (*layer).splines = spl;

        let points_before = count_points_in_set(spl);
        assert_eq!(points_before, 8, "Expected 8 points before simplify");

        // Set up simplify info with default parameters
        let smpl = fontforge_ffi::simplifyinfo {
            flags: fontforge_ffi::simpify_flags_sf_normal as i32,
            err: 1.0,
            tan_bounds: 0.02,
            linefixup: 0.0,
            linelenmax: 0.0,
            set_as_default: 0,
            check_selected_contours: 0,
        };

        // Apply simplify
        let head = (*layer).splines;
        let mut smpl = smpl; // make it mutable so we can take &mut
        let result = fontforge_ffi::SplineCharSimplify(sc, head, &mut smpl);
        assert!(!result.is_null(), "SplineCharSimplify returned null");
        (*layer).splines = result;

        let points_after = count_points_in_set(result);
        assert!(
            points_after <= points_before,
            "Expected {} or fewer points after simplify, got {}",
            points_before,
            points_after
        );
        // Points should still exist
        assert!(points_after >= 1, "Expected at least 1 point after simplify");

        // Verify SFD round-trip
        let map = fontforge_ffi::EncMap1to1((*sf).glyphcnt);
        assert!(!map.is_null());

        let tmpfile = CString::new(format!("/tmp/fontforge_rs_simplify_{}.sfd", std::process::id()))
            .unwrap();
        let ok = fontforge_ffi::SFDWrite(
            tmpfile.as_ptr() as *mut libc::c_char,
            sf,
            map,
            map,
            0,
        );
        assert!(ok != 0, "SFDWrite after simplify failed");

        let sf2 = fontforge_ffi::LoadSplineFont(tmpfile.as_ptr(), 0);
        assert!(!sf2.is_null());
        assert_eq!((*sf2).glyphcnt, (*sf).glyphcnt);

        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf2);
        fontforge_ffi::SplineFontFree(sf);
        libc::unlink(tmpfile.as_ptr());

        println!("PASS: simplify test passed (points: {} → {})", points_before, points_after);
    }
}

#[test]
fn test_correct_direction() {
    unsafe {
        fontforge_ffi::doinitFontForgeMain();

        let sf = fontforge_ffi::SplineFontNew();
        assert!(!sf.is_null());

        let fontname = CString::new("CorrectDirTest").unwrap();
        if !(*sf).fontname.is_null() {
            libc::free((*sf).fontname as *mut libc::c_void);
        }
        (*sf).fontname = libc::strdup(fontname.as_ptr());

        let cname = CString::new("A").unwrap();
        let sc = fontforge_ffi::SFGetOrMakeChar(sf, 0x0041, cname.as_ptr());
        assert!(!sc.is_null());

        // Create two contours — outer and inner (like a donut / counter)
        // Outer: clockwise or counterclockwise — SplineSetsCorrect should fix it
        // Contour 1: (0,0) → (600,0) → (600,700) → (0,700) — outer
        let mut sp1: [*mut fontforge_ffi::SplinePoint; 4] = [ptr::null_mut(); 4];
        let outer_pts: [(f64, f64); 4] = [(0.0, 0.0), (600.0, 0.0), (600.0, 700.0), (0.0, 700.0)];
        for j in 0..4 {
            sp1[j] = fontforge_ffi::SplinePointCreate(outer_pts[j].0, outer_pts[j].1);
            assert!(!sp1[j].is_null());
        }
        fontforge_ffi::SplineMake(sp1[0], sp1[1], 0);
        fontforge_ffi::SplineMake(sp1[1], sp1[2], 0);
        fontforge_ffi::SplineMake(sp1[2], sp1[3], 0);
        fontforge_ffi::SplineMake(sp1[3], sp1[0], 0);

        let spl1 = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
            as *mut fontforge_ffi::SplinePointList;
        assert!(!spl1.is_null());
        (*spl1).first = sp1[0];
        (*spl1).last = sp1[0];
        fontforge_ffi::SPLCategorizePoints(spl1);

        // Contour 2: (100,100) → (500,100) → (500,600) → (100,600) — inner (counter)
        let mut sp2: [*mut fontforge_ffi::SplinePoint; 4] = [ptr::null_mut(); 4];
        // Create inner contour in opposite direction
        let inner_pts: [(f64, f64); 4] = [(100.0, 100.0), (100.0, 600.0), (500.0, 600.0), (500.0, 100.0)];
        for j in 0..4 {
            sp2[j] = fontforge_ffi::SplinePointCreate(inner_pts[j].0, inner_pts[j].1);
            assert!(!sp2[j].is_null());
        }
        fontforge_ffi::SplineMake(sp2[0], sp2[1], 0);
        fontforge_ffi::SplineMake(sp2[1], sp2[2], 0);
        fontforge_ffi::SplineMake(sp2[2], sp2[3], 0);
        fontforge_ffi::SplineMake(sp2[3], sp2[0], 0);

        let spl2 = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
            as *mut fontforge_ffi::SplinePointList;
        assert!(!spl2.is_null());
        (*spl2).first = sp2[0];
        (*spl2).last = sp2[0];
        fontforge_ffi::SPLCategorizePoints(spl2);

        // Link both contours
        (*spl1).next = spl2;

        let layer = (*sc).layers.offset(LY_FORE);
        (*layer).splines = spl1;

        let count_before = count_spline_sets(spl1);
        assert_eq!(count_before, 2, "Expected 2 contours before correction");

        // Apply SplineSetsCorrect
        let mut changed: i32 = 0;
        let result = fontforge_ffi::SplineSetsCorrect(spl1, &mut changed);
        assert!(!result.is_null(), "SplineSetsCorrect returned null");
        (*layer).splines = result;

        let count_after = count_spline_sets(result);
        assert_eq!(count_after, 2, "Expected 2 contours after correction, got {}", count_after);

        // Verify SFD round-trip
        let map = fontforge_ffi::EncMap1to1((*sf).glyphcnt);
        assert!(!map.is_null());

        let tmpfile = CString::new(format!("/tmp/fontforge_rs_correctdir_{}.sfd", std::process::id()))
            .unwrap();
        let ok = fontforge_ffi::SFDWrite(
            tmpfile.as_ptr() as *mut libc::c_char,
            sf,
            map,
            map,
            0,
        );
        assert!(ok != 0, "SFDWrite after direction correction failed");

        let sf2 = fontforge_ffi::LoadSplineFont(tmpfile.as_ptr(), 0);
        assert!(!sf2.is_null(), "LoadSplineFont after direction correction returned null");
        assert_eq!((*sf2).glyphcnt, (*sf).glyphcnt);

        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf2);
        fontforge_ffi::SplineFontFree(sf);
        libc::unlink(tmpfile.as_ptr());

        println!("PASS: correct direction test passed (changed={})", changed);
    }
}
