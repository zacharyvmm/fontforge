//! Pure-Rust UFO reader.
//! Parses UFO directory trees (v2/v3) into SplineFont via FFI.

use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;

const LY_FORE: isize = 1;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read a UFO directory tree and create a SplineFont.
/// Returns null on error. Caller must free with SplineFontFree.
pub unsafe fn read_ufo_font(base_dir: &str) -> *mut crate::SplineFont {
    use crate::SplineFontNew;

    let sf = SplineFontNew();
    if sf.is_null() {
        eprintln!("read_ufo_font: SplineFontNew returned null");
        return ptr::null_mut();
    }

    // Parse metainfo.plist (ignore for now, just validate it exists)
    let metainfo_path = format!("{}/metainfo.plist", base_dir);
    if !std::path::Path::new(&metainfo_path).exists() {
        eprintln!("read_ufo_font: metainfo.plist not found at {}", metainfo_path);
        crate::SplineFontFree(sf);
        return ptr::null_mut();
    }

    // Parse fontinfo.plist
    let fontinfo_path = format!("{}/fontinfo.plist", base_dir);
    if let Err(e) = parse_fontinfo(sf, &fontinfo_path) {
        eprintln!("read_ufo_font: error parsing fontinfo.plist: {}", e);
        crate::SplineFontFree(sf);
        return ptr::null_mut();
    }

    // Parse glyphs/contents.plist
    let contents_path = format!("{}/glyphs/contents.plist", base_dir);
    let glyph_map = match parse_contents(&contents_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("read_ufo_font: error parsing contents.plist: {}", e);
            crate::SplineFontFree(sf);
            return ptr::null_mut();
        }
    };

    // Parse each glyph's .glif file
    let glyphs_dir = format!("{}/glyphs", base_dir);
    for (glyph_name, glif_filename) in &glyph_map {
        let glif_path = format!("{}/{}", glyphs_dir, glif_filename);
        if let Err(e) = parse_glif(sf, glyph_name, &glif_path) {
            eprintln!(
                "read_ufo_font: error parsing glif '{}': {}",
                glif_path, e
            );
            // Continue with other glyphs
        }
    }

    // Create encoding map
    if !(*sf).map.is_null() {
        crate::EncMapFree((*sf).map);
    }
    (*sf).map = crate::EncMap1to1((*sf).glyphcnt);

    sf
}

// ─── fontinfo.plist parser ────────────────────────────────────────────────────

unsafe fn parse_fontinfo(sf: *mut crate::SplineFont, path: &str) -> Result<(), String> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {}", path, e))?;
    let mut reader = Reader::from_str(&data);
    reader.config_mut().trim_text(true);

    let mut in_key = false;
    let mut in_value = false;
    let mut current_key = String::new();
    let mut current_tag = String::new();
    let sf_ref = &mut *sf;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if name == "key" {
                    in_key = true;
                    current_key.clear();
                } else if name == "string" || name == "integer" || name == "real" {
                    in_value = true;
                    current_tag = name;
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if name == "key" {
                    in_key = false;
                } else if name == "string" || name == "integer" || name == "real" {
                    in_value = false;
                }
            }
            Ok(Event::Text(ref t)) => {
                let text = String::from_utf8_lossy(t.as_ref()).into_owned();
                if in_key {
                    current_key.push_str(&text);
                } else if in_value {
                    apply_fontinfo_key(sf_ref, &current_key, &text, &current_tag);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
    }
    Ok(())
}

unsafe fn apply_fontinfo_key(
    sf: &mut crate::SplineFont,
    key: &str,
    value: &str,
    _tag: &str,
) {
    match key {
        "familyName" => {
            set_string(&mut sf.familyname, value);
        }
        "fontName" | "postscriptFontName" => {
            set_string(&mut sf.fontname, value);
        }
        "fullName" | "postscriptFullName" => {
            set_string(&mut sf.fullname, value);
        }
        "styleName" => {
            // If fontname is not set, combine with familyName
            if sf.fontname.is_null() && !sf.familyname.is_null() {
                let family = CStr::from_ptr(sf.familyname).to_string_lossy();
                let combined = format!("{}-{}", family, value);
                set_string(&mut sf.fontname, &combined);
            }
        }
        "weightName" | "postscriptWeightName" => {
            set_string(&mut sf.weight, value);
        }
        "unitsPerEm" => {
            if let Ok(v) = value.parse::<i32>() {
                if sf.ascent + sf.descent != v {
                    sf.ascent = (v as f64 * 0.8) as i32;
                    sf.descent = v - sf.ascent;
                }
            }
        }
        "ascender" => {
            if let Ok(v) = value.parse::<i32>() {
                sf.ascent = v;
            }
        }
        "descender" => {
            if let Ok(v) = value.parse::<i32>() {
                sf.descent = -(v as i32);
            }
        }
        "italicAngle" => {
            if let Ok(v) = value.parse::<f64>() {
                sf.italicangle = v;
            }
        }
        "note" => {
            set_string(&mut sf.comments, value);
        }
        "copyright" => {
            set_string(&mut sf.copyright, value);
        }
        _ => {
            // Ignore other keys for now
        }
    }
}

// ─── contents.plist parser ────────────────────────────────────────────────────

fn parse_contents(path: &str) -> Result<BTreeMap<String, String>, String> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {}", path, e))?;
    let mut reader = Reader::from_str(&data);
    reader.config_mut().trim_text(true);

    let mut map = BTreeMap::new();
    let mut in_key = false;
    let mut in_string = false;
    let mut current_key = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if name == "key" {
                    in_key = true;
                    current_key.clear();
                } else if name == "string" {
                    in_string = true;
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if name == "key" {
                    in_key = false;
                } else if name == "string" {
                    in_string = false;
                }
            }
            Ok(Event::Text(ref t)) => {
                let text = String::from_utf8_lossy(t.as_ref()).into_owned();
                if in_key {
                    current_key.push_str(&text);
                } else if in_string {
                    if !current_key.is_empty() {
                        map.insert(current_key.clone(), text);
                        current_key.clear();
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
    }
    Ok(map)
}

// ─── .glif file parser ────────────────────────────────────────────────────────

unsafe fn parse_glif(
    sf: *mut crate::SplineFont,
    glyph_name: &str,
    path: &str,
) -> Result<(), String> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {}", path, e))?;
    let mut reader = Reader::from_str(&data);
    reader.config_mut().trim_text(true);

    // Get or create the SplineChar
    let cname = CString::new(glyph_name).unwrap();
    let sc = crate::SFGetOrMakeChar(sf, -1, cname.as_ptr());
    if sc.is_null() {
        return Err(format!("SFGetOrMakeChar failed for '{}'", glyph_name));
    }
    let sc_ref = &mut *sc;

    // State tracking
    let mut in_outline = false;
    let mut in_contour = false;
    let mut contour_points: Vec<ContourPoint> = Vec::new();
    let _current_point: Option<ContourPoint> = None;
    let mut in_unicode = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                match tag_name.as_str() {
                    "glyph" => {
                        // Extract name and format attributes
                        for attr in e.attributes().flatten() {
                            let attr_name =
                                String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                            if attr_name == "name" && sc_ref.name.is_null() {
                                // Name already set
                            }
                        }
                    }
                    "advance" => {
                        for attr in e.attributes().flatten() {
                            let attr_name =
                                String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                            let val =
                                String::from_utf8_lossy(&attr.value).into_owned();
                            if attr_name == "width" {
                                if let Ok(w) = val.parse::<i16>() {
                                    sc_ref.width = w;
                                    sc_ref.set_widthset(1);
                                }
                            }
                        }
                    }
                    "unicode" => {
                        in_unicode = true;
                        for attr in e.attributes().flatten() {
                            let attr_name =
                                String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                            let val =
                                String::from_utf8_lossy(&attr.value).into_owned();
                            if attr_name == "hex" {
                                if let Ok(u) = i32::from_str_radix(&val, 16) {
                                    sc_ref.unicodeenc = u;
                                }
                            }
                        }
                    }
                    "outline" => {
                        in_outline = true;
                    }
                    "contour" => {
                        in_contour = true;
                        contour_points.clear();
                    }
                    "point" => {
                        let mut pt = ContourPoint::default();
                        for attr in e.attributes().flatten() {
                            let attr_name =
                                String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                            let val =
                                String::from_utf8_lossy(&attr.value).into_owned();
                            match attr_name.as_str() {
                                "x" => {
                                    pt.x = val.parse::<f64>().unwrap_or(0.0);
                                }
                                "y" => {
                                    pt.y = val.parse::<f64>().unwrap_or(0.0);
                                }
                                "type" => {
                                    pt.pt_type = val;
                                }
                                _ => {}
                            }
                        }
                        contour_points.push(pt);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                match tag_name.as_str() {
                    "unicode" => {
                        in_unicode = false;
                    }
                    "contour" => {
                        in_contour = false;
                        build_contour_from_points(sc, &contour_points)?;
                        contour_points.clear();
                    }
                    "outline" => {
                        in_outline = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error in {}: {}", path, e)),
            _ => {}
        }
    }

    Ok(())
}

/// Convert parsed contour points into SplinePoints and create a SplineSet.
unsafe fn build_contour_from_points(
    sc: *mut crate::SplineChar,
    points: &[ContourPoint],
) -> Result<(), String> {
    if points.is_empty() {
        return Ok(());
    }

    // Categorize points: on-curve (move, line, curve, qcurve) vs off-curve (no type)
    // GLIF format: on-curve points have type="move|line|curve|qcurve"
    //              off-curve points have no type attribute

    let mut on_curve_points: Vec<(usize, &ContourPoint)> = Vec::new();
    for (i, pt) in points.iter().enumerate() {
        if !pt.pt_type.is_empty() {
            on_curve_points.push((i, pt));
        }
    }

    if on_curve_points.len() < 2 {
        return Ok(()); // Need at least 2 on-curve points for a meaningful contour
    }

    // Use the existing FFI to create SplinePoints
    use crate::SplinePointCreate;
    use crate::SplineMake;
    use crate::SPLCategorizePoints;
    use crate::SplinePointList;

    let n_on = on_curve_points.len();
    let mut sps: Vec<*mut crate::SplinePoint> = Vec::with_capacity(n_on);

    for &(idx, pt) in &on_curve_points {
        let sp = SplinePointCreate(pt.x, pt.y);
        if sp.is_null() {
            return Err("SplinePointCreate failed".to_string());
        }

        // Set control points from adjacent off-curve points
        // In GLIF, off-curve points between on-curve points are: next cp of current, prev cp of next
        // Find next on-curve index
        sps.push(sp);
    }

    // Connect points with splines
    let n = n_on;
    for i in 0..n {
        let j = (i + 1) % n;
        // order2 = false means cubic Bezier
        SplineMake(sps[i], sps[j], 0);

        // Set prevcp of next point from off-curve points
        let cur_on_idx = on_curve_points[i].0;
        let next_on_idx = on_curve_points[j].0;

        // Find off-curve points between cur_on_idx and next_on_idx
        // In a closed contour, off-curve points between last and first wrap around
        let mut off_curve_points_between: Vec<&ContourPoint> = Vec::new();
        if next_on_idx > cur_on_idx {
            // points between cur_on_idx+1 .. next_on_idx-1 are off-curve
            for k in (cur_on_idx + 1)..next_on_idx {
                if points[k].pt_type.is_empty() {
                    off_curve_points_between.push(&points[k]);
                }
            }
        } else {
            // Wrap: points between cur_on_idx+1..end and 0..next_on_idx-1
            for k in (cur_on_idx + 1)..points.len() {
                if points[k].pt_type.is_empty() {
                    off_curve_points_between.push(&points[k]);
                }
            }
            for k in 0..next_on_idx {
                if points[k].pt_type.is_empty() {
                    off_curve_points_between.push(&points[k]);
                }
            }
        }

        // For cubic: up to 2 off-curve points: first = nextcp of current, second = prevcp of next
        match off_curve_points_between.len() {
            2 => {
                // Two off-curve points: first is nextcp of current, second is prevcp of next
                let cur_sp = &mut *sps[i];
                cur_sp.nextcp.x = off_curve_points_between[0].x;
                cur_sp.nextcp.y = off_curve_points_between[0].y;
                // Clear nonextcp bit
                cur_sp.set_nonextcp(0);
                cur_sp.set_nextcpdef(1);

                let next_sp = &mut *sps[j];
                next_sp.prevcp.x = off_curve_points_between[1].x;
                next_sp.prevcp.y = off_curve_points_between[1].y;
                next_sp.set_noprevcp(0);
                next_sp.set_prevcpdef(1);
            }
            1 => {
                // Single off-curve point: shared as both nextcp and prevcp (quadratic-like control)
                let off = off_curve_points_between[0];
                let cur_sp = &mut *sps[i];
                cur_sp.nextcp.x = off.x;
                cur_sp.nextcp.y = off.y;
                cur_sp.set_nonextcp(0);
                cur_sp.set_nextcpdef(1);

                let next_sp = &mut *sps[j];
                next_sp.prevcp.x = off.x;
                next_sp.prevcp.y = off.y;
                next_sp.set_noprevcp(0);
                next_sp.set_prevcpdef(1);
            }
            0 => {
                // No off-curve points: this is a straight line segment
                // Set noprevcp on next point to indicate linear
                let next_sp = &mut *sps[j];
                next_sp.set_noprevcp(1);
            }
            _ => {
                // More than 2 off-curve points: use first and last
                let cur_sp = &mut *sps[i];
                cur_sp.nextcp.x = off_curve_points_between[0].x;
                cur_sp.nextcp.y = off_curve_points_between[0].y;
                cur_sp.set_nonextcp(0);
                cur_sp.set_nextcpdef(1);

                let next_sp = &mut *sps[j];
                let last = off_curve_points_between.last().unwrap();
                next_sp.prevcp.x = last.x;
                next_sp.prevcp.y = last.y;
                next_sp.set_noprevcp(0);
                next_sp.set_prevcpdef(1);
            }
        }
    }

    // Create SplinePointList
    let spl = unsafe { std::alloc::alloc(std::alloc::Layout::new::<SplinePointList>()) }
        as *mut SplinePointList;
    if !spl.is_null() {
        std::ptr::write_bytes(spl, 0, 1);
    }
    if spl.is_null() {
        return Err("calloc failed for SplinePointList".to_string());
    }
    (*spl).first = sps[0];
    (*spl).last = sps[0]; // Closed contour

    // Categorize points and attach to the foreground layer
    SPLCategorizePoints(spl);

    let sc_ref = &mut *sc;
    let layer = &mut *sc_ref.layers.offset(LY_FORE);
    // Append to end of splines list
    if layer.splines.is_null() {
        layer.splines = spl;
    } else {
        let mut tail = layer.splines;
        while !(*tail).next.is_null() {
            tail = (*tail).next;
        }
        (*tail).next = spl;
    }

    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

unsafe fn set_string(ptr: &mut *mut c_char, value: &str) {
    if !ptr.is_null() {
        // Free old string using C free via the external libc allocator
        extern "C" {
            fn free(ptr: *mut c_void);
        }
        free(*ptr as *mut c_void);
    }
    let cstr = CString::new(value).unwrap();
    *ptr = strdup_rs(cstr.as_ptr());
}

/// Allocate a copy of a C string using malloc+strcpy (portable strdup equivalent).
unsafe fn strdup_rs(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    let len = CStr::from_ptr(s).to_bytes_with_nul().len();
    let buf = std::alloc::alloc(std::alloc::Layout::from_size_align(len, 1).unwrap())
        as *mut c_char;
    if buf.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(s, buf, len);
    buf
}

#[derive(Default, Debug, Clone)]
struct ContourPoint {
    x: f64,
    y: f64,
    pt_type: String, // empty = off-curve control point, "move"/"line"/"curve"/"qcurve" = on-curve
}
