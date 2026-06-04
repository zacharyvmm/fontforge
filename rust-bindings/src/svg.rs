//! Pure-Rust SVG font import.
//!
//! Reads SVG font files containing `<font>` elements with `<font-face>`,
//! `<glyph>`, `<missing-glyph>`, `<hkern>`, and `<vkern>` sub-elements.
//! Uses quick-xml for XML parsing. Reconstructs SplineFont via FFI.

use quick_xml::events::Event;
use quick_xml::Reader;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

const LY_FORE: isize = 1;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read an SVG font from a string in memory.
/// Returns a SplineFont on success (must free with SplineFontFree), or null.
pub unsafe fn read_svg_font_from_str(data: &str) -> *mut crate::SplineFont {
    // Use bytes-based reader for consistency
    let mut reader = Reader::from_str(data);
    reader.config_mut().trim_text(true);

    let mut font_buf = String::new();
    let mut in_font = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref())
                    .into_owned()
                    .to_lowercase();
                if name == "font" && !in_font {
                    in_font = true;
                    font_buf.clear();
                    font_buf.push_str(&build_start_tag(e, "font", false));
                } else if in_font {
                    font_buf.push_str(&build_start_tag(e, &name, false));
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref())
                    .into_owned()
                    .to_lowercase();
                if name == "font" && !in_font {
                    in_font = true;
                    font_buf.clear();
                    font_buf.push_str(&build_start_tag(e, "font", true));
                } else if in_font {
                    font_buf.push_str(&build_start_tag(e, &name, true));
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref())
                    .into_owned()
                    .to_lowercase();
                if in_font && name == "font" {
                    font_buf.push_str("</font>");
                    break;
                } else if in_font {
                    font_buf.push_str(&format!("</{}>", name));
                }
            }
            Ok(Event::Text(ref t)) => {
                if in_font {
                    let s = String::from_utf8_lossy(t.as_ref());
                    font_buf.push_str(&s);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    if font_buf.is_empty() || !in_font {
        eprintln!("read_svg_font: no <font> element found");
        return ptr::null_mut();
    }

    parse_font_element(&font_buf)
}

/// Read an SVG font from a file.
pub unsafe fn read_svg_font_file(path: &str) -> *mut crate::SplineFont {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("read_svg_font_file: cannot read {}: {}", path, e);
            return ptr::null_mut();
        }
    };
    read_svg_font_from_str(&data)
}

// ─── Font element parser ──────────────────────────────────────────────────────

unsafe fn parse_font_element(font_xml: &str) -> *mut crate::SplineFont {
    let sf = crate::SplineFontNew();
    if sf.is_null() {
        return ptr::null_mut();
    }

    let mut reader = Reader::from_str(font_xml);
    reader.config_mut().trim_text(true);

    let mut defh: f64 = 0.0;
    let mut defv: f64 = 0.0;
    let mut has_font_face = false;
    let mut glyph_elements: Vec<String> = Vec::new();

    // First pass: parse <font> attributes and collect child elements
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref())
                    .into_owned()
                    .to_lowercase();
                match name.as_str() {
                    "font" => {
                        extract_font_attrs(sf, e, &mut defh, &mut defv);
                    }
                    "font-face" => {
                        has_font_face = true;
                        parse_font_face(sf, e, &mut defh, &mut defv);
                    }
                    "glyph" | "missing-glyph" => {
                        let glyph_xml =
                            collect_element_xml(sf, &mut reader, e, &name);
                        if !glyph_xml.is_empty() {
                            glyph_elements.push(glyph_xml);
                        }
                    }
                    "hkern" | "vkern" => {
                        parse_kern_attrs(sf, e, name == "vkern");
                        skip_element(&mut reader, &name);
                    }
                    _ => {
                        skip_element(&mut reader, &name);
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref())
                    .into_owned()
                    .to_lowercase();
                match name.as_str() {
                    "font" => {
                        extract_font_attrs(sf, e, &mut defh, &mut defv);
                    }
                    "font-face" => {
                        has_font_face = true;
                        parse_font_face(sf, e, &mut defh, &mut defv);
                    }
                    "glyph" | "missing-glyph" => {
                        let tag =
                            build_start_tag(e, &name, true);
                        glyph_elements.push(tag);
                    }
                    "hkern" | "vkern" => {
                        parse_kern_attrs(sf, e, name == "vkern");
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref())
                    .into_owned()
                    .to_lowercase();
                if name == "font" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    if !has_font_face {
        eprintln!("parse_font_element: no <font-face> element");
        crate::SplineFontFree(sf);
        return ptr::null_mut();
    }

    // Set default weight and names
    if (*sf).weight.is_null() {
        set_cstring(&mut (*sf).weight, "Regular");
    }
    ensure_font_names(sf);

    // Second pass: parse glyph elements
    for glyph_xml in &glyph_elements {
        let trimmed = glyph_xml.trim_start();
        let is_missing = trimmed.starts_with("<missing-glyph");
        if is_missing {
            parse_missing_glyph_element(sf, glyph_xml, defh, defv);
        } else {
            parse_glyph_element(sf, glyph_xml, defh, defv);
        }
    }

    // Create encoding map
    if !(*sf).map.is_null() {
        crate::EncMapFree((*sf).map);
    }
    (*sf).map = crate::EncMap1to1((*sf).glyphcnt);

    sf
}

// ─── Font-face parser ─────────────────────────────────────────────────────────

unsafe fn parse_font_face(
    sf: *mut crate::SplineFont,
    e: &quick_xml::events::BytesStart,
    defh: &mut f64,
    defv: &mut f64,
) {
    let sf_ref = &mut *sf;

    for attr in e.attributes().flatten() {
        let k = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let v = String::from_utf8_lossy(&attr.value).into_owned();

        match k.as_str() {
            "units-per-em" => {
                if let Ok(val) = v.parse::<i32>() {
                    if val > 0 {
                        sf_ref.ascent = val * 800 / 1000;
                        sf_ref.descent = val - sf_ref.ascent;
                        if *defv == 0.0 {
                            *defv = val as f64;
                        }
                        if *defh == 0.0 {
                            *defh = val as f64;
                        }
                        sf_ref.pfminfo.set_pfmset(1);
                    }
                }
            }
            "font-family" => {
                let family = v.split(',').next().unwrap_or(&v).trim();
                set_cstring(&mut sf_ref.familyname, family);
            }
            "font-weight" => {
                match v.as_str() {
                    "normal" => {
                        sf_ref.pfminfo.weight = 400;
                        set_cstring(&mut sf_ref.weight, "Regular");
                        sf_ref.pfminfo.panose[2] = 5i8;
                        sf_ref.pfminfo.set_panose_set(1);
                        sf_ref.pfminfo.set_pfmset(1);
                    }
                    "bold" => {
                        sf_ref.pfminfo.weight = 700;
                        set_cstring(&mut sf_ref.weight, "Bold");
                        sf_ref.pfminfo.panose[2] = 8i8;
                        sf_ref.pfminfo.set_panose_set(1);
                        sf_ref.pfminfo.set_pfmset(1);
                    }
                    wval => {
                        if let Ok(w) = wval.parse::<u32>() {
                            sf_ref.pfminfo.weight = w as i16;
                            let (wstr, panose) = match w {
                                ..=100 => ("Thin", 2i8),
                                101..=200 => ("Extra-Light", 3i8),
                                201..=300 => ("Light", 4i8),
                                301..=400 => ("Regular", 5i8),
                                401..=500 => ("Medium", 6i8),
                                501..=600 => ("DemiBold", 7i8),
                                601..=700 => ("Bold", 8i8),
                                701..=800 => ("Heavy", 9i8),
                                _ => ("Black", 10i8),
                            };
                            set_cstring(&mut sf_ref.weight, wstr);
                            sf_ref.pfminfo.panose[2] = panose;
                            sf_ref.pfminfo.set_panose_set(1);
                            sf_ref.pfminfo.set_pfmset(1);
                        }
                    }
                }
            }
            "font-stretch" => {
                match v.as_str() {
                    "ultra-condensed" => {
                        sf_ref.pfminfo.panose[3] = 8;
                        sf_ref.pfminfo.width = 1;
                    }
                    "extra-condensed" => {
                        sf_ref.pfminfo.panose[3] = 8;
                        sf_ref.pfminfo.width = 2;
                    }
                    "condensed" => {
                        sf_ref.pfminfo.panose[3] = 6;
                        sf_ref.pfminfo.width = 3;
                    }
                    "semi-condensed" => {
                        sf_ref.pfminfo.panose[3] = 6;
                        sf_ref.pfminfo.width = 4;
                    }
                    "normal" => {
                        sf_ref.pfminfo.panose[3] = 3;
                        sf_ref.pfminfo.width = 5;
                    }
                    "semi-expanded" => {
                        sf_ref.pfminfo.panose[3] = 5;
                        sf_ref.pfminfo.width = 6;
                    }
                    "expanded" => {
                        sf_ref.pfminfo.panose[3] = 5;
                        sf_ref.pfminfo.width = 7;
                    }
                    "extra-expanded" => {
                        sf_ref.pfminfo.panose[3] = 7;
                        sf_ref.pfminfo.width = 8;
                    }
                    "ultra-expanded" => {
                        sf_ref.pfminfo.panose[3] = 7;
                        sf_ref.pfminfo.width = 9;
                    }
                    _ => {}
                }
                sf_ref.pfminfo.set_panose_set(1);
                sf_ref.pfminfo.set_pfmset(1);
            }
            "panose-1" => {
                let parts: Vec<&str> = v.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if i < 10 {
                        sf_ref.pfminfo.panose[i] = part.parse::<i8>().unwrap_or(0);
                    }
                }
                sf_ref.pfminfo.set_panose_set(1);
            }
            "slope" => {
                sf_ref.italicangle = v.parse::<f64>().unwrap_or(0.0);
            }
            "underline-position" => {
                sf_ref.upos = v.parse::<f64>().unwrap_or(0.0);
            }
            "underline-thickness" => {
                sf_ref.uwidth = v.parse::<f64>().unwrap_or(0.0);
            }
            "ascent" => {
                if let Ok(val) = v.parse::<i32>() {
                    sf_ref.ascent = val;
                }
            }
            "descent" => {
                if let Ok(val) = v.parse::<i32>() {
                    // SVG stores descent as positive, FontForge as negative
                    sf_ref.descent = -val;
                }
            }
            _ => {}
        }
    }

    sf_ref.pfminfo.set_pfmset(1);
}

// ─── Helper to extract font-level attributes ──────────────────────────────────

unsafe fn extract_font_attrs(
    sf: *mut crate::SplineFont,
    e: &quick_xml::events::BytesStart,
    defh: &mut f64,
    defv: &mut f64,
) {
    let sf_ref = &mut *sf;
    for attr in e.attributes().flatten() {
        let k = String::from_utf8_lossy(attr.key.as_ref());
        let v = String::from_utf8_lossy(&attr.value);
        match k.as_ref() {
            "horiz-adv-x" => {
                *defh = v.parse::<f64>().unwrap_or(0.0);
            }
            "vert-adv-y" => {
                *defv = v.parse::<f64>().unwrap_or(0.0);
                // hasvmetrics is a bitfield — use setter if available
                sf_ref.set_hasvmetrics(1);
            }
            "id" => {
                set_cstring(&mut sf_ref.fontname, &v);
            }
            _ => {}
        }
    }
}

// ─── Glyph element parser ─────────────────────────────────────────────────────

unsafe fn parse_glyph_element(
    sf: *mut crate::SplineFont,
    glyph_xml: &str,
    defh: f64,
    defv: f64,
) {
    let mut reader = Reader::from_str(glyph_xml);
    reader.config_mut().trim_text(true);

    let mut glyph_name: Option<String> = None;
    let mut unicode_str: Option<String> = None;
    let mut horiz_adv_x: Option<String> = None;
    let mut vert_adv_y: Option<String> = None;
    let mut orientation: Option<String> = None;
    let mut path_d: Option<String> = None;

    // Parse first <glyph> element attributes
    match reader.read_event() {
        Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
            for attr in e.attributes().flatten() {
                let k = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                let v = String::from_utf8_lossy(&attr.value).into_owned();
                match k.as_str() {
                    "unicode" => unicode_str = Some(v),
                    "glyph-name" => glyph_name = Some(v),
                    "horiz-adv-x" => horiz_adv_x = Some(v),
                    "vert-adv-y" => vert_adv_y = Some(v),
                    "orientation" => orientation = Some(v),
                    "d" => path_d = Some(v),
                    _ => {}
                }
            }
        }
        _ => return,
    }

    // Determine glyph name and unicode
    let unienc: i32 = unicode_str
        .as_ref()
        .map(|u| parse_unicode_attr(u))
        .unwrap_or(-1);

    let mut final_name = if let Some(ref gname) = glyph_name {
        if !gname.is_empty() {
            gname.clone()
        } else if unienc != -1 {
            format!("uni{:04X}", unienc as u32)
        } else {
            format!("glyph{:04}", (*sf).glyphcnt)
        }
    } else if unienc != -1 {
        format!("uni{:04X}", unienc as u32)
    } else {
        format!("glyph{:04}", (*sf).glyphcnt)
    };

    // Apply orientation suffix
    if let Some(ref orient) = orientation {
        if orient == "v" && !final_name.ends_with(".vert") {
            final_name.push_str(".vert");
        }
    }

    // Get or create glyph
    let cname = CString::new(final_name.clone()).unwrap();
    let sc = crate::SFGetOrMakeChar(sf, unienc, cname.as_ptr());
    if sc.is_null() {
        return;
    }
    let sc_ref = &mut *sc;

    // Set metrics
    if let Some(ref h) = horiz_adv_x {
        if let Ok(w) = h.parse::<f64>() {
            sc_ref.width = w as i16;
        }
    } else {
        sc_ref.width = defh as i16;
    }

    if let Some(ref v) = vert_adv_y {
        if let Ok(w) = v.parse::<f64>() {
            sc_ref.vwidth = w as i16;
        }
    } else {
        sc_ref.vwidth = defv as i16;
    }

    // Parse path data
    if let Some(ref d) = path_d {
        let splines = parse_svg_path(d);
        if !splines.is_null() {
            let layer_ptr = sc_ref.layers.offset(LY_FORE);
            if !(*layer_ptr).splines.is_null() {
                crate::SplinePointListFree((*layer_ptr).splines);
            }
            (*layer_ptr).splines = splines;
            crate::SPLCategorizePoints(splines);
        }
    }

    // Mark glyph as having width set (bitfield)
    sc_ref.set_widthset(1);
}

unsafe fn parse_missing_glyph_element(
    sf: *mut crate::SplineFont,
    glyph_xml: &str,
    _defh: f64,
    _defv: f64,
) {
    let mut reader = Reader::from_str(glyph_xml);
    reader.config_mut().trim_text(true);

    let mut path_d: Option<String> = None;

    match reader.read_event() {
        Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
            for attr in e.attributes().flatten() {
                let k = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                let v = String::from_utf8_lossy(&attr.value).into_owned();
                if k == "d" {
                    path_d = Some(v);
                }
            }
        }
        _ => return,
    }

    let cname = CString::new(".notdef").unwrap();
    let sc = crate::SFGetOrMakeChar(sf, 0, cname.as_ptr());
    if sc.is_null() {
        return;
    }
    let sc_ref = &mut *sc;

    sc_ref.unicodeenc = 0;

    if let Some(ref d) = path_d {
        let splines = parse_svg_path(d);
        if !splines.is_null() {
            let layer_ptr = sc_ref.layers.offset(LY_FORE);
            if !(*layer_ptr).splines.is_null() {
                crate::SplinePointListFree((*layer_ptr).splines);
            }
            (*layer_ptr).splines = splines;
            crate::SPLCategorizePoints(splines);
        }
    }
}

// ─── SVG Path Data Parser ─────────────────────────────────────────────────────

/// Parses an SVG path 'd' attribute string into a SplineSet chain.
/// Supports: M/m, L/l, H/h, V/v, C/c, Q/q, S/s, T/t, Z/z, A/a.
unsafe fn parse_svg_path(path_data: &str) -> *mut crate::SplineSet {
    if path_data.is_empty() {
        return ptr::null_mut();
    }

    let mut head: *mut crate::SplineSet = ptr::null_mut();
    let mut last_ss: *mut crate::SplineSet = ptr::null_mut();
    let mut cur_ss: *mut crate::SplineSet = ptr::null_mut();
    let mut current = crate::BasePoint { x: 0.0, y: 0.0 };
    let mut cmd: u8 = b'M';
    let mut order2: i32 = 0;

    let chars: Vec<char> = path_data.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        // Skip whitespace and commas
        while i < chars.len() && (chars[i].is_ascii_whitespace() || chars[i] == ',') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        // Check for command letter
        if chars[i].is_ascii_alphabetic() {
            cmd = chars[i] as u8;
            i += 1;
            if i >= chars.len() && cmd != b'z' && cmd != b'Z' {
                break;
            }
            // Skip whitespace after command
            while i < chars.len()
                && (chars[i].is_ascii_whitespace() || chars[i] == ',')
            {
                i += 1;
            }
        }

        match cmd {
            b'm' | b'M' => {
                close_prev_subpath(cur_ss);

                let (x, y, ni) = expect_2_numbers(&chars, i);
                i = ni;
                let (x, y) = if cmd == b'm' {
                    (current.x + x, current.y + y)
                } else {
                    (x, y)
                };

                let sp = crate::SplinePointCreate(x, y);
                if sp.is_null() {
                    break;
                }
                current.x = x;
                current.y = y;

                cur_ss = alloc_splineset();
                if cur_ss.is_null() {
                    break;
                }
                (*cur_ss).first = sp;
                (*cur_ss).last = sp;

                if head.is_null() {
                    head = cur_ss;
                } else if !last_ss.is_null() {
                    (*last_ss).next = cur_ss;
                }
                last_ss = cur_ss;

                cmd = if cmd == b'm' { b'l' } else { b'L' };
            }

            b'z' | b'Z' => {
                close_subpath(cur_ss, &mut current, order2);
                cmd = b' ';
            }

            b'l' | b'L' => {
                ensure_cur_ss(
                    &mut head,
                    &mut last_ss,
                    &mut cur_ss,
                    current,
                );

                let (x, y, ni) = expect_2_numbers(&chars, i);
                i = ni;
                let (x, y) = if cmd == b'l' {
                    (current.x + x, current.y + y)
                } else {
                    (x, y)
                };

                let sp = crate::SplinePointCreate(x, y);
                if sp.is_null() || cur_ss.is_null() {
                    break;
                }
                crate::SplineMake((*cur_ss).last, sp, order2);
                (*cur_ss).last = sp;
                current.x = x;
                current.y = y;
            }

            b'h' | b'H' => {
                ensure_cur_ss(
                    &mut head,
                    &mut last_ss,
                    &mut cur_ss,
                    current,
                );

                let (val, ni) = expect_1_number(&chars, i);
                i = ni;
                let x = if cmd == b'h' { current.x + val } else { val };
                let y = current.y;

                let sp = crate::SplinePointCreate(x, y);
                if sp.is_null() || cur_ss.is_null() {
                    break;
                }
                crate::SplineMake((*cur_ss).last, sp, order2);
                (*cur_ss).last = sp;
                current.x = x;
                current.y = y;
            }

            b'v' | b'V' => {
                ensure_cur_ss(
                    &mut head,
                    &mut last_ss,
                    &mut cur_ss,
                    current,
                );

                let (val, ni) = expect_1_number(&chars, i);
                i = ni;
                let y = if cmd == b'v' { current.y + val } else { val };
                let x = current.x;

                let sp = crate::SplinePointCreate(x, y);
                if sp.is_null() || cur_ss.is_null() {
                    break;
                }
                crate::SplineMake((*cur_ss).last, sp, order2);
                (*cur_ss).last = sp;
                current.x = x;
                current.y = y;
            }

            b'c' | b'C' => {
                ensure_cur_ss(
                    &mut head,
                    &mut last_ss,
                    &mut cur_ss,
                    current,
                );

                let (x1, y1, x2, y2, x, y, ni) =
                    expect_6_numbers(&chars, i);
                i = ni;

                let (x1, y1, x2, y2, x, y) = if cmd == b'c' {
                    (
                        current.x + x1,
                        current.y + y1,
                        current.x + x2,
                        current.y + y2,
                        current.x + x,
                        current.y + y,
                    )
                } else {
                    (x1, y1, x2, y2, x, y)
                };

                let sp = crate::SplinePointCreate(x, y);
                if sp.is_null() || cur_ss.is_null() {
                    break;
                }
                (*sp).prevcp.x = x2;
                (*sp).prevcp.y = y2;
                let last = &mut *(*cur_ss).last;
                last.nextcp.x = x1;
                last.nextcp.y = y1;
                crate::SplineMake((*cur_ss).last, sp, 0);
                (*cur_ss).last = sp;
                current.x = x;
                current.y = y;
                order2 = 0;
            }

            b's' | b'S' => {
                ensure_cur_ss(
                    &mut head,
                    &mut last_ss,
                    &mut cur_ss,
                    current,
                );

                let (x1, y1) = reflect_prev_cp(cur_ss, current);
                let (x2, y2, x, y, ni) =
                    expect_4_numbers(&chars, i);
                i = ni;

                let (x2, y2, x, y) = if cmd == b's' {
                    (
                        current.x + x2,
                        current.y + y2,
                        current.x + x,
                        current.y + y,
                    )
                } else {
                    (x2, y2, x, y)
                };

                let sp = crate::SplinePointCreate(x, y);
                if sp.is_null() || cur_ss.is_null() {
                    break;
                }
                (*sp).prevcp.x = x2;
                (*sp).prevcp.y = y2;
                let last = &mut *(*cur_ss).last;
                last.nextcp.x = x1;
                last.nextcp.y = y1;
                crate::SplineMake((*cur_ss).last, sp, 0);
                (*cur_ss).last = sp;
                current.x = x;
                current.y = y;
                order2 = 0;
            }

            b'q' | b'Q' => {
                ensure_cur_ss(
                    &mut head,
                    &mut last_ss,
                    &mut cur_ss,
                    current,
                );

                let (x1, y1, x, y, ni) =
                    expect_4_numbers(&chars, i);
                i = ni;

                let (x1, y1, x, y) = if cmd == b'q' {
                    (
                        current.x + x1,
                        current.y + y1,
                        current.x + x,
                        current.y + y,
                    )
                } else {
                    (x1, y1, x, y)
                };

                let sp = crate::SplinePointCreate(x, y);
                if sp.is_null() || cur_ss.is_null() {
                    break;
                }
                (*sp).prevcp.x = x1;
                (*sp).prevcp.y = y1;
                let last = &mut *(*cur_ss).last;
                last.nextcp.x = x1;
                last.nextcp.y = y1;
                crate::SplineMake((*cur_ss).last, sp, 1);
                (*cur_ss).last = sp;
                current.x = x;
                current.y = y;
                order2 = 1;
            }

            b't' | b'T' => {
                ensure_cur_ss(
                    &mut head,
                    &mut last_ss,
                    &mut cur_ss,
                    current,
                );

                let (x1, y1) = reflect_prev_cp(cur_ss, current);
                let (x, y, ni) = expect_2_numbers(&chars, i);
                i = ni;

                let (x, y) = if cmd == b't' {
                    (current.x + x, current.y + y)
                } else {
                    (x, y)
                };

                let sp = crate::SplinePointCreate(x, y);
                if sp.is_null() || cur_ss.is_null() {
                    break;
                }
                (*sp).prevcp.x = x1;
                (*sp).prevcp.y = y1;
                let last = &mut *(*cur_ss).last;
                last.nextcp.x = x1;
                last.nextcp.y = y1;
                crate::SplineMake((*cur_ss).last, sp, 1);
                (*cur_ss).last = sp;
                current.x = x;
                current.y = y;
                order2 = 1;
            }

            b'a' | b'A' => {
                ensure_cur_ss(
                    &mut head,
                    &mut last_ss,
                    &mut cur_ss,
                    current,
                );

                let (rx, ry, axisrot, large_arc_f, sweep_f, x, y, ni) =
                    expect_7_numbers(&chars, i);
                i = ni;

                let (x, y) = if cmd == b'a' {
                    (current.x + x, current.y + y)
                } else {
                    (x, y)
                };

                let large_arc = large_arc_f != 0.0;
                let sweep = sweep_f != 0.0;

                approximate_arc(
                    cur_ss,
                    &mut current,
                    rx,
                    ry,
                    axisrot,
                    large_arc,
                    sweep,
                    x,
                    y,
                );
            }

            _ => {
                break;
            }
        }
    }

    close_prev_subpath(cur_ss);

    head
}

// ─── Kerning parser ───────────────────────────────────────────────────────────

unsafe fn parse_kern_attrs(
    _sf: *mut crate::SplineFont,
    e: &quick_xml::events::BytesStart,
    _is_v: bool,
) {
    // Kerning deferred — read attributes for future implementation
    let _k_val: f64 = e
        .attributes()
        .flatten()
        .find(|a| {
            String::from_utf8_lossy(a.key.as_ref()) == "k"
        })
        .map(|a| {
            String::from_utf8_lossy(&a.value)
                .parse::<f64>()
                .unwrap_or(0.0)
        })
        .unwrap_or(0.0);
}

// ─── SVG Path subpath helpers ─────────────────────────────────────────────────

unsafe fn close_prev_subpath(cur_ss: *mut crate::SplineSet) {
    if cur_ss.is_null() {
        return;
    }
    let first = (*cur_ss).first;
    let last = (*cur_ss).last;
    if first.is_null() || last.is_null() {
        return;
    }
    if first == last {
        return;
    }
    let dx = (*last).me.x - (*first).me.x;
    let dy = (*last).me.y - (*first).me.y;
    if dx * dx + dy * dy < 1e-6 {
        (*cur_ss).last = first;
    }
}

unsafe fn close_subpath(
    cur_ss: *mut crate::SplineSet,
    current: &mut crate::BasePoint,
    order2: i32,
) {
    if cur_ss.is_null() {
        return;
    }
    let first = (*cur_ss).first;
    let last = (*cur_ss).last;
    if first.is_null() || last.is_null() {
        return;
    }
    if first == last {
        return;
    }
    let dx = (*last).me.x - (*first).me.x;
    let dy = (*last).me.y - (*first).me.y;
    if dx * dx + dy * dy < 1e-6 {
        (*cur_ss).last = first;
        current.x = (*first).me.x;
        current.y = (*first).me.y;
    } else {
        crate::SplineMake(last, first, order2);
        (*cur_ss).last = first;
        current.x = (*first).me.x;
        current.y = (*first).me.y;
    }
}

unsafe fn ensure_cur_ss(
    head: &mut *mut crate::SplineSet,
    last_ss: &mut *mut crate::SplineSet,
    cur_ss: &mut *mut crate::SplineSet,
    current: crate::BasePoint,
) {
    if cur_ss.is_null() || (*cur_ss).is_null() {
        let sp = crate::SplinePointCreate(current.x, current.y);
        if !sp.is_null() {
            let ss = alloc_splineset();
            if !ss.is_null() {
                (*ss).first = sp;
                (*ss).last = sp;
                if head.is_null() || (*head).is_null() {
                    *head = ss;
                } else if !last_ss.is_null() && !(*last_ss).is_null() {
                    (**last_ss).next = ss;
                }
                *last_ss = ss;
                *cur_ss = ss;
            }
        }
    }
}

unsafe fn reflect_prev_cp(
    cur_ss: *mut crate::SplineSet,
    current: crate::BasePoint,
) -> (f64, f64) {
    if !cur_ss.is_null() && !(*cur_ss).last.is_null() {
        let last = &*(*cur_ss).last;
        (
            2.0 * last.me.x - last.prevcp.x,
            2.0 * last.me.y - last.prevcp.y,
        )
    } else {
        (current.x, current.y)
    }
}

// ─── Arc approximation ────────────────────────────────────────────────────────

unsafe fn approximate_arc(
    cur_ss: *mut crate::SplineSet,
    current: &mut crate::BasePoint,
    rx: f64,
    ry: f64,
    axisrot: f64,
    large_arc: bool,
    sweep: bool,
    x: f64,
    y: f64,
) {
    use std::f64::consts::PI;

    if rx == 0.0 || ry == 0.0 {
        let sp = crate::SplinePointCreate(x, y);
        if sp.is_null() || cur_ss.is_null() {
            return;
        }
        crate::SplineMake((*cur_ss).last, sp, 0);
        (*cur_ss).last = sp;
        current.x = x;
        current.y = y;
        return;
    }

    let phi = axisrot * PI / 180.0;
    let cos_phi = phi.cos();
    let sin_phi = phi.sin();

    let dx = (current.x - x) / 2.0;
    let dy = (current.y - y) / 2.0;
    let x1p = cos_phi * dx + sin_phi * dy;
    let y1p = -sin_phi * dx + cos_phi * dy;

    let rx = rx.abs();
    let ry = ry.abs();
    let lambda = x1p * x1p / (rx * rx) + y1p * y1p / (ry * ry);
    let (rx, ry) = if lambda > 1.0 {
        (rx * lambda.sqrt(), ry * lambda.sqrt())
    } else {
        (rx, ry)
    };
    let rx_sq = rx * rx;
    let ry_sq = ry * ry;

    let sign = if large_arc != sweep { -1.0 } else { 1.0 };
    let numerator = (rx_sq * ry_sq - rx_sq * y1p * y1p - ry_sq * x1p * x1p)
        .max(0.0);
    let denominator = (rx_sq * y1p * y1p + ry_sq * x1p * x1p).max(1e-12);
    let sq = (numerator / denominator).sqrt();
    let cxp = sign * sq * rx * y1p / ry;
    let cyp = sign * sq * -ry * x1p / rx;

    let cx = cos_phi * cxp - sin_phi * cyp + (current.x + x) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (current.y + y) / 2.0;

    let theta1 = f64::atan2((y1p - cyp) / ry, (x1p - cxp) / rx);
    let mut delta_theta =
        f64::atan2(-(y1p + cyp) / ry, -(x1p + cxp) / rx) - theta1;

    if sweep && delta_theta < 0.0 {
        delta_theta += 2.0 * PI;
    } else if !sweep && delta_theta > 0.0 {
        delta_theta -= 2.0 * PI;
    }

    let n = (delta_theta.abs() / (PI / 2.0)).ceil() as usize;
    let n = n.max(1).min(8);
    let dt = delta_theta / n as f64;

    for seg in 0..n {
        let t1 = theta1 + seg as f64 * dt;
        let t2 = t1 + dt;

        let alpha = (4.0 / 3.0)
            * (1.0 - (dt / 4.0).cos())
            / (dt / 2.0).sin().max(1e-12);

        let x0 = cx + rx * t1.cos() * cos_phi - ry * t1.sin() * sin_phi;
        let y0 = cy + rx * t1.cos() * sin_phi + ry * t1.sin() * cos_phi;
        let x3 = cx + rx * t2.cos() * cos_phi - ry * t2.sin() * sin_phi;
        let y3 = cy + rx * t2.cos() * sin_phi + ry * t2.sin() * cos_phi;

        let x1 = x0 - alpha * (rx * t1.sin() * cos_phi + ry * t1.cos() * sin_phi);
        let y1 = y0 - alpha * (rx * t1.sin() * sin_phi - ry * t1.cos() * cos_phi);
        let x2 = x3 + alpha * (rx * t2.sin() * cos_phi + ry * t2.cos() * sin_phi);
        let y2 = y3 + alpha * (rx * t2.sin() * sin_phi - ry * t2.cos() * cos_phi);

        let sp = crate::SplinePointCreate(x3, y3);
        if sp.is_null() || cur_ss.is_null() || (*cur_ss).last.is_null() {
            return;
        }
        (*sp).prevcp.x = x2;
        (*sp).prevcp.y = y2;
        let last = &mut *(*cur_ss).last;
        last.nextcp.x = x1;
        last.nextcp.y = y1;
        crate::SplineMake((*cur_ss).last, sp, 0);
        (*cur_ss).last = sp;
    }

    current.x = x;
    current.y = y;
}

// ─── Number parsing helpers ───────────────────────────────────────────────────

fn expect_1_number(chars: &[char], start: usize) -> (f64, usize) {
    let mut i = start;
    while i < chars.len()
        && (chars[i].is_ascii_whitespace() || chars[i] == ',')
    {
        i += 1;
    }
    let begin = i;
    if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
        i += 1;
    }
    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
        i += 1;
    }
    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
        i += 1;
        if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
            i += 1;
        }
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }
    let num_str: String = chars[begin..i].iter().collect();
    let val: f64 = num_str.parse().unwrap_or(0.0);
    (val, i)
}

fn expect_2_numbers(chars: &[char], start: usize) -> (f64, f64, usize) {
    let (x, i) = expect_1_number(chars, start);
    let (y, i) = expect_1_number(chars, i);
    (x, y, i)
}

fn expect_4_numbers(
    chars: &[char],
    start: usize,
) -> (f64, f64, f64, f64, usize) {
    let (x1, i) = expect_1_number(chars, start);
    let (y1, i) = expect_1_number(chars, i);
    let (x, i) = expect_1_number(chars, i);
    let (y, i) = expect_1_number(chars, i);
    (x1, y1, x, y, i)
}

fn expect_6_numbers(
    chars: &[char],
    start: usize,
) -> (f64, f64, f64, f64, f64, f64, usize) {
    let (x1, i) = expect_1_number(chars, start);
    let (y1, i) = expect_1_number(chars, i);
    let (x2, i) = expect_1_number(chars, i);
    let (y2, i) = expect_1_number(chars, i);
    let (x, i) = expect_1_number(chars, i);
    let (y, i) = expect_1_number(chars, i);
    (x1, y1, x2, y2, x, y, i)
}

fn expect_7_numbers(
    chars: &[char],
    start: usize,
) -> (f64, f64, f64, f64, f64, f64, f64, usize) {
    let (rx, i) = expect_1_number(chars, start);
    let (ry, i) = expect_1_number(chars, i);
    let (axisrot, i) = expect_1_number(chars, i);
    let (large_arc, i) = expect_1_number(chars, i);
    let (sweep, i) = expect_1_number(chars, i);
    let (x, i) = expect_1_number(chars, i);
    let (y, i) = expect_1_number(chars, i);
    (rx, ry, axisrot, large_arc, sweep, x, y, i)
}

// ─── XML helpers ──────────────────────────────────────────────────────────────

fn build_start_tag(
    e: &quick_xml::events::BytesStart,
    name: &str,
    self_closing: bool,
) -> String {
    let mut tag = format!("<{}", name);
    for attr in e.attributes().flatten() {
        let k = String::from_utf8_lossy(attr.key.as_ref());
        let v = String::from_utf8_lossy(&attr.value);
        tag.push_str(&format!(" {}=\"{}\"", k, v));
    }
    if self_closing {
        tag.push_str("/>");
    } else {
        tag.push('>');
    }
    tag
}

unsafe fn skip_element(reader: &mut Reader<&[u8]>, tag_name: &str) {
    let mut depth = 1i32;
    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref())
                    .into_owned()
                    .to_lowercase();
                if name == tag_name {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
            }
            Ok(Event::Eof) => return,
            Err(_) => return,
            _ => {}
        }
    }
}

/// Collect the full XML text of an element including its children.
/// `start_e` is the Start event that opened the element.
unsafe fn collect_element_xml(
    _sf: *mut crate::SplineFont,
    reader: &mut Reader<&[u8]>,
    start_e: &quick_xml::events::BytesStart,
    tag_name: &str,
) -> String {
    let mut xml = format!("<{}{}>", tag_name, attr_string(start_e));
    let mut depth = 1i32;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref())
                    .into_owned()
                    .to_lowercase();
                if name == tag_name {
                    depth += 1;
                }
                xml.push_str(&format!(
                    "<{}{}>",
                    name,
                    attr_string(e)
                ));
            }
            Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref())
                    .into_owned()
                    .to_lowercase();
                xml.push_str(&format!(
                    "<{}{}/>",
                    name,
                    attr_string(e)
                ));
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref())
                    .into_owned()
                    .to_lowercase();
                xml.push_str(&format!("</{}>", name));
                if name == tag_name {
                    depth -= 1;
                    if depth == 0 {
                        return xml;
                    }
                }
            }
            Ok(Event::Text(ref t)) => {
                let s = String::from_utf8_lossy(t.as_ref());
                xml.push_str(&s);
            }
            Ok(Event::Eof) => return xml,
            Err(_) => return xml,
            _ => {}
        }
    }
}

fn attr_string(e: &quick_xml::events::BytesStart) -> String {
    let mut s = String::new();
    for attr in e.attributes().flatten() {
        let k = String::from_utf8_lossy(attr.key.as_ref());
        let v = String::from_utf8_lossy(&attr.value);
        s.push_str(&format!(" {}=\"{}\"", k, v));
    }
    s
}

// ─── Utility functions ────────────────────────────────────────────────────────

unsafe fn alloc_splineset() -> *mut crate::SplineSet {
    let layout = std::alloc::Layout::new::<crate::SplineSet>();
    let ptr = std::alloc::alloc_zeroed(layout) as *mut crate::SplineSet;
    ptr
}

unsafe fn ensure_font_names(sf: *mut crate::SplineFont) {
    let sf_ref = &mut *sf;
    if sf_ref.fontname.is_null() && sf_ref.familyname.is_null() {
        set_cstring(&mut sf_ref.fontname, "Untitled");
    }
    if sf_ref.familyname.is_null() && !sf_ref.fontname.is_null() {
        let name = cstr_to_string(sf_ref.fontname);
        set_cstring(&mut sf_ref.familyname, &name);
    }
    if sf_ref.fontname.is_null() && !sf_ref.familyname.is_null() {
        let name = cstr_to_string(sf_ref.familyname);
        set_cstring(&mut sf_ref.fontname, &name);
    }
    if sf_ref.fullname.is_null() && !sf_ref.fontname.is_null() {
        let name = cstr_to_string(sf_ref.fontname);
        set_cstring(&mut sf_ref.fullname, &name);
    }
}

unsafe fn set_cstring(ptr: &mut *mut c_char, value: &str) {
    if !(*ptr).is_null() {
        extern "C" {
            fn free(p: *mut std::ffi::c_void);
        }
        free(*ptr as *mut std::ffi::c_void);
    }
    let cstr = CString::new(value).unwrap();
    let bytes = cstr.as_bytes_with_nul();
    let layout =
        std::alloc::Layout::from_size_align(bytes.len(), 1).unwrap();
    let buf = std::alloc::alloc(layout) as *mut c_char;
    if !buf.is_null() {
        std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const u8, buf as *mut u8, bytes.len());
    }
    *ptr = buf;
}

unsafe fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    std::ffi::CStr::from_ptr(ptr)
        .to_str()
        .unwrap_or("")
        .to_string()
}

fn parse_unicode_attr(attr_value: &str) -> i32 {
    if attr_value.is_empty() {
        return -1;
    }
    if let Some(hex) = attr_value.strip_prefix("&#x") {
        if let Some(end) = hex.find(';') {
            if let Ok(v) = i32::from_str_radix(&hex[..end], 16) {
                return v;
            }
        }
    }
    if let Some(dec) = attr_value.strip_prefix("&#") {
        if let Some(end) = dec.find(';') {
            if let Ok(v) = dec[..end].parse::<i32>() {
                return v;
            }
        }
    }
    if attr_value.chars().count() == 1 {
        return attr_value.chars().next().unwrap() as i32;
    }
    -1
}
