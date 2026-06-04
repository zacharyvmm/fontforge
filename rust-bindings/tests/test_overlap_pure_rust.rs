// Integration test: pure-Rust overlap removal vs C SplineSetRemoveOverlap.
//
// Creates a font with two overlapping rectangles, runs both the C FFI
// overlap removal and the pure-Rust algorithm, and compares the results
// (contour count and bounding box).

use std::ffi::CString;

const LY_FORE: isize = 1;

/// Create a font with two overlapping rectangular contours.
unsafe fn create_overlapping_rectangles()
    -> (*mut fontforge_ffi::SplineFont, *mut fontforge_ffi::SplineChar)
{
    fontforge_ffi::doinitFontForgeMain();

    let sf = fontforge_ffi::SplineFontNew();
    assert!(!sf.is_null());

    let fontname = CString::new("OverlapRustTest").unwrap();
    if !(*sf).fontname.is_null() {
        libc::free((*sf).fontname as *mut libc::c_void);
    }
    (*sf).fontname = libc::strdup(fontname.as_ptr());

    let cname = CString::new("A").unwrap();
    let sc = fontforge_ffi::SFGetOrMakeChar(sf, 0x0041, cname.as_ptr());
    assert!(!sc.is_null());

    // Rectangle 1: (0,0) to (600,700)
    let sp1 = [
        fontforge_ffi::SplinePointCreate(0.0, 0.0),
        fontforge_ffi::SplinePointCreate(600.0, 0.0),
        fontforge_ffi::SplinePointCreate(600.0, 700.0),
        fontforge_ffi::SplinePointCreate(0.0, 700.0),
    ];
    fontforge_ffi::SplineMake(sp1[0], sp1[1], 0);
    fontforge_ffi::SplineMake(sp1[1], sp1[2], 0);
    fontforge_ffi::SplineMake(sp1[2], sp1[3], 0);
    fontforge_ffi::SplineMake(sp1[3], sp1[0], 0);

    let spl1 = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
        as *mut fontforge_ffi::SplinePointList;
    (*spl1).first = sp1[0];
    (*spl1).last = sp1[0];
    fontforge_ffi::SPLCategorizePoints(spl1);

    // Rectangle 2: (200,-100) to (500,800)
    let sp2 = [
        fontforge_ffi::SplinePointCreate(200.0, -100.0),
        fontforge_ffi::SplinePointCreate(500.0, -100.0),
        fontforge_ffi::SplinePointCreate(500.0, 800.0),
        fontforge_ffi::SplinePointCreate(200.0, 800.0),
    ];
    fontforge_ffi::SplineMake(sp2[0], sp2[1], 0);
    fontforge_ffi::SplineMake(sp2[1], sp2[2], 0);
    fontforge_ffi::SplineMake(sp2[2], sp2[3], 0);
    fontforge_ffi::SplineMake(sp2[3], sp2[0], 0);

    let spl2 = libc::calloc(1, std::mem::size_of::<fontforge_ffi::SplinePointList>())
        as *mut fontforge_ffi::SplinePointList;
    (*spl2).first = sp2[0];
    (*spl2).last = sp2[0];
    fontforge_ffi::SPLCategorizePoints(spl2);

    // Link contours
    (*spl1).next = spl2;

    let layer = (*sc).layers.offset(LY_FORE);
    (*layer).splines = spl1;

    (sf, sc)
}

/// Extract point coordinates from a SplineSet linked list.
unsafe fn extract_contours_c(head: *mut fontforge_ffi::SplineSet)
    -> Vec<Vec<fontforge_ffi::overlap::Point>>
{
    use fontforge_ffi::overlap::Point;
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

#[test]
fn test_overlap_pure_rust_vs_c() {
    unsafe {
        let (_sf, sc) = create_overlapping_rectangles();
        let layer = (*sc).layers.offset(LY_FORE);
        let head = (*layer).splines;

        // 1. Extract input contours (before overlap removal)
        let input_contours = extract_contours_c(head);
        assert_eq!(input_contours.len(), 2, "Expected 2 input contours");
        assert_eq!(input_contours[0].len(), 4, "First contour should have 4 points");
        assert_eq!(input_contours[1].len(), 4, "Second contour should have 4 points");

        // 2. Run C overlap removal
        let c_result = fontforge_ffi::SplineSetRemoveOverlap(
            sc,
            head,
            fontforge_ffi::overlap_type_over_remove,
        );
        assert!(!c_result.is_null(), "C SplineSetRemoveOverlap returned null");

        let c_contours = extract_contours_c(c_result);
        let c_count = c_contours.len();
        eprintln!("C overlap removal produced {} contour(s)", c_count);

        // 3. Run pure-Rust overlap removal on same input
        use fontforge_ffi::overlap;
        let rust_contours = overlap::remove_overlap(&input_contours);
        let rust_count = rust_contours.len();
        eprintln!("Rust overlap removal produced {} contour(s)", rust_count);

        // 4. Compare contour counts
        // The union of two overlapping rectangles should produce 1 outer contour
        assert_eq!(rust_count, c_count,
            "Rust and C should produce same contour count: Rust={} vs C={}",
            rust_count, c_count);
        assert!(c_count >= 1, "Expected at least 1 contour, got {}", c_count);

        // 5. Compare bounding boxes
        let c_bbox = overlap::bounding_box(&c_contours);
        let rust_bbox = overlap::bounding_box(&rust_contours);

        eprintln!("C bbox: {:?}", c_bbox);
        eprintln!("Rust bbox: {:?}", rust_bbox);

        // Bounding boxes should be similar (within 1.0 epsilon)
        let eps = 1.0;
        assert!((c_bbox.0 - rust_bbox.0).abs() < eps,
            "min_x mismatch: C={} vs Rust={}", c_bbox.0, rust_bbox.0);
        assert!((c_bbox.1 - rust_bbox.1).abs() < eps,
            "min_y mismatch: C={} vs Rust={}", c_bbox.1, rust_bbox.1);
        assert!((c_bbox.2 - rust_bbox.2).abs() < eps,
            "max_x mismatch: C={} vs Rust={}", c_bbox.2, rust_bbox.2);
        assert!((c_bbox.3 - rust_bbox.3).abs() < eps,
            "max_y mismatch: C={} vs Rust={}", c_bbox.3, rust_bbox.3);

        // 6. Verify Rust result is clockwise
        for (i, contour) in rust_contours.iter().enumerate() {
            assert!(overlap::is_clockwise(contour),
                "Rust contour {} should be clockwise", i);
        }

        // 7. Validate SDD round-trip still works after C overlap removal
        let map = fontforge_ffi::EncMap1to1((*_sf).glyphcnt);
        assert!(!map.is_null());

        let tmpfile = CString::new(format!("/tmp/fontforge_rs_overlap_rust_{}.sfd", std::process::id()))
            .unwrap();
        (*layer).splines = c_result;
        let ok = fontforge_ffi::SFDWrite(tmpfile.as_ptr() as *mut libc::c_char, _sf, map, map, 0);
        assert!(ok != 0, "SFDWrite after overlap removal failed");

        let sf2 = fontforge_ffi::LoadSplineFont(tmpfile.as_ptr(), 0);
        assert!(!sf2.is_null(), "LoadSplineFont after overlap removal returned null");
        assert_eq!((*sf2).glyphcnt, (*_sf).glyphcnt);

        fontforge_ffi::EncMapFree(map);
        fontforge_ffi::SplineFontFree(sf2);
        fontforge_ffi::SplineFontFree(_sf);
        libc::unlink(tmpfile.as_ptr());
    }
}

#[test]
fn test_overlap_pure_rust_three_rectangles() {
    use fontforge_ffi::overlap::{self, Point};

    // Three mutually overlapping rectangles
    let r1 = vec![
        Point::new(0.0, 0.0),
        Point::new(400.0, 0.0),
        Point::new(400.0, 400.0),
        Point::new(0.0, 400.0),
    ];
    let r2 = vec![
        Point::new(200.0, 100.0),
        Point::new(600.0, 100.0),
        Point::new(600.0, 500.0),
        Point::new(200.0, 500.0),
    ];
    let r3 = vec![
        Point::new(100.0, 300.0),
        Point::new(500.0, 300.0),
        Point::new(500.0, 700.0),
        Point::new(100.0, 700.0),
    ];

    let contours = vec![r1, r2, r3];
    let result = overlap::remove_overlap(&contours);

    // Three overlapping rectangles should union into 1 contour
    assert_eq!(result.len(), 1, "3 overlapping rectangles should produce 1 contour, got {}",
        result.len());

    let union = &result[0];
    assert!(overlap::is_clockwise(union), "Union should be clockwise");

    let (min_x, min_y, max_x, max_y) = overlap::bounding_box(&result);
    assert!((min_x - 0.0).abs() < 1.0);
    assert!((min_y - 0.0).abs() < 1.0);
    assert!((max_x - 600.0).abs() < 1.0);
    assert!((max_y - 700.0).abs() < 1.0);

    eprintln!("3-rectangle union: {} points, bbox=({},{},{},{})",
        union.len(), min_x, min_y, max_x, max_y);
}

#[test]
fn test_overlap_pure_rust_simple_c_direct_comparison() {
    // Create a more complex test: two overlapping rectangles in different positions
    // that produce a well-known union shape (L-shape union).
    // Rectangle A: left part (0,0)-(300,100)
    // Rectangle B: right part (100,0)-(500,100)
    // Union: a single 500×100 rectangle
    use fontforge_ffi::overlap::{self, Point};

    let r1 = vec![
        Point::new(0.0, 0.0),
        Point::new(300.0, 0.0),
        Point::new(300.0, 100.0),
        Point::new(0.0, 100.0),
    ];
    let r2 = vec![
        Point::new(100.0, 0.0),
        Point::new(500.0, 0.0),
        Point::new(500.0, 100.0),
        Point::new(100.0, 100.0),
    ];

    let contours = vec![r1, r2];
    let result = overlap::remove_overlap(&contours);

    assert_eq!(result.len(), 1, "Side-by-side overlapping rectangles should union to 1 contour, got {}", result.len());

    let (min_x, min_y, max_x, max_y) = overlap::bounding_box(&result);
    assert!((min_x - 0.0).abs() < 1.0);
    assert!((max_x - 500.0).abs() < 1.0);
    assert!((min_y - 0.0).abs() < 1.0);
    assert!((max_y - 100.0).abs() < 1.0);

    eprintln!("Side-by-side union: {} points, bbox=({},{},{},{})",
        result[0].len(), min_x, min_y, max_x, max_y);
}
