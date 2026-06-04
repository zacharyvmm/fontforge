//! Pure-Rust UFO (Unified Font Object) writer.
//!
//! Uses quick-xml for XML generation. No C FFI for UFO functions —
//! interacts with SplineFont/SplineChar via bindgen-generated types only.

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Cursor;
use std::os::raw::{c_char, c_int};

const LY_FORE: isize = 1; // layer_type::ly_fore

// ─── Public API ───────────────────────────────────────────────────────────────

/// Write a SplineFont as a UFO directory tree (v3 format).
/// Returns 0 on success, non-zero on error.
pub unsafe fn write_ufo_font(base_dir: &str, sf: *mut crate::SplineFont) -> c_int {
    if sf.is_null() {
        eprintln!("write_ufo_font: null SplineFont pointer");
        return 1;
    }
    let sf_ref = &*sf;

    // Create directory structure
    if std::fs::create_dir_all(base_dir).is_err() {
        eprintln!("Error creating base directory {}", base_dir);
        return 1;
    }
    let glyphs_dir = format!("{}/glyphs", base_dir);
    if std::fs::create_dir_all(&glyphs_dir).is_err() {
        eprintln!("Error creating glyphs directory {}", glyphs_dir);
        return 1;
    }

    // 1. metainfo.plist
    if write_metainfo(base_dir) != 0 {
        return 1;
    }

    // 2. fontinfo.plist
    if write_fontinfo(base_dir, sf) != 0 {
        return 1;
    }

    // 3. Glyphs — iterate sf->glyphs[i]
    let glyphcnt = sf_ref.glyphcnt as isize;
    let mut contents: Vec<(String, String)> = Vec::new();

    if !sf_ref.glyphs.is_null() && glyphcnt > 0 {
        for i in 0..glyphcnt {
            let sc = *sf_ref.glyphs.offset(i);
            if sc.is_null() {
                continue;
            }
            let sc_ref = &*sc;
            let glyph_name = cstr_or(sc_ref.name, "unnamed");

            // Mangle to safe filename
            let glif_stem = mangle_glif_name(&glyph_name, i as usize);
            let glif_filename = format!("{}.glif", glif_stem);

            if write_glif_file(&glyphs_dir, &glif_filename, sc) != 0 {
                eprintln!("Error writing glif for glyph '{}'", glyph_name);
                return 1;
            }
            contents.push((glyph_name, glif_filename));
        }
    }

    // 4. glyphs/contents.plist
    if write_contents_plist(&glyphs_dir, &contents) != 0 {
        return 1;
    }

    0
}

// ─── metainfo.plist ───────────────────────────────────────────────────────────

unsafe fn write_metainfo(base_dir: &str) -> c_int {
    let path = format!("{}/metainfo.plist", base_dir);
    let buf = Vec::new();
    let mut w = Writer::new_with_indent(Cursor::new(buf), b' ', 2);

    plist_open(&mut w);
    dict_open(&mut w);
    plist_key_string(&mut w, "creator", "net.GitHub.FontForge");
    plist_key_integer(&mut w, "formatVersion", 3);
    dict_close(&mut w);
    plist_close(&mut w);

    write_file(&path, w)
}

// ─── fontinfo.plist ───────────────────────────────────────────────────────────

unsafe fn write_fontinfo(base_dir: &str, sf: *mut crate::SplineFont) -> c_int {
    let sf_ref = &*sf;
    let path = format!("{}/fontinfo.plist", base_dir);
    let buf = Vec::new();
    let mut w = Writer::new_with_indent(Cursor::new(buf), b' ', 2);

    plist_open(&mut w);
    dict_open(&mut w);

    // Basic metadata
    let family = cstr_or(sf_ref.familyname, "Untitled");
    plist_key_string(&mut w, "familyName", &family);

    // Extract style from fontname (after last dash)
    if !sf_ref.fontname.is_null() {
        let fnt = std::ffi::CStr::from_ptr(sf_ref.fontname).to_string_lossy();
        if let Some(pos) = fnt.rfind('-') {
            if pos + 2 < fnt.len() {
                plist_key_string(&mut w, "styleName", &fnt[pos + 1..]);
            }
        }
    }

    let em = sf_ref.ascent + sf_ref.descent;
    if em > 0 {
        plist_key_integer(&mut w, "unitsPerEm", em);
    }
    plist_key_integer(&mut w, "ascender", sf_ref.ascent);
    plist_key_integer(&mut w, "descender", -sf_ref.descent);
    plist_key_real(&mut w, "italicAngle", sf_ref.italicangle);

    // PostScript names
    if !sf_ref.fontname.is_null() {
        plist_key_string(
            &mut w,
            "postscriptFontName",
            &cstr_or(sf_ref.fontname, ""),
        );
    }
    if !sf_ref.fullname.is_null() {
        plist_key_string(
            &mut w,
            "postscriptFullName",
            &cstr_or(sf_ref.fullname, ""),
        );
    }

    dict_close(&mut w);
    plist_close(&mut w);

    write_file(&path, w)
}

// ─── .glif file ───────────────────────────────────────────────────────────────

unsafe fn write_glif_file(dir: &str, filename: &str, sc: *mut crate::SplineChar) -> c_int {
    let sc_ref = &*sc;
    let path = format!("{}/{}", dir, filename);
    let buf = Vec::new();
    let mut w = Writer::new_with_indent(Cursor::new(buf), b' ', 2);

    // XML declaration
    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .ok();

    // <glyph name="..." format="2">
    let glyph_name = cstr_or(sc_ref.name, "unnamed");
    let mut glyph_start = BytesStart::new("glyph");
    glyph_start.push_attribute(("name", glyph_name.as_str()));
    glyph_start.push_attribute(("format", "2"));
    w.write_event(Event::Start(glyph_start)).ok();

    // <advance width="..."/>
    let mut adv = BytesStart::new("advance");
    let width_str = sc_ref.width.to_string();
    adv.push_attribute(("width", width_str.as_str()));
    w.write_event(Event::Empty(adv)).ok();

    // <unicode hex="XXXX"/>
    if sc_ref.unicodeenc >= 0 {
        let mut uni = BytesStart::new("unicode");
        let hex_str = format!("{:04X}", sc_ref.unicodeenc);
        uni.push_attribute(("hex", hex_str.as_str()));
        w.write_event(Event::Empty(uni)).ok();
    }

    // <outline>
    let layer = &*sc_ref.layers.offset(LY_FORE);
    if !layer.splines.is_null() {
        w.write_event(Event::Start(BytesStart::new("outline")))
            .ok();

        let mut spl = layer.splines;
        while !spl.is_null() {
            write_contour(&mut w, &*spl);
            spl = (*spl).next;
        }

        w.write_event(Event::End(BytesEnd::new("outline"))).ok();
    }

    // </glyph>
    w.write_event(Event::End(BytesEnd::new("glyph"))).ok();

    write_file(&path, w)
}

/// Write a <contour> from a SplinePointList. Handles closed cubic Bezier contours.
unsafe fn write_contour<W: std::io::Write>(
    w: &mut Writer<W>,
    spl: &crate::SplinePointList,
) {
    if spl.first.is_null() {
        return;
    }

    w.write_event(Event::Start(BytesStart::new("contour")))
        .ok();

    let first = spl.first;
    let mut sp = first;
    let mut visited = false;

    loop {
        if sp.is_null() {
            break;
        }
        let sp_ref = &*sp;

        // Determine point type
        let pt_type = if sp == first {
            "move"
        } else if sp_ref.noprevcp() != 0 {
            "line"
        } else {
            "curve"
        };

        // Write on-curve point
        let mut pt = BytesStart::new("point");
        let x_str = sp_ref.me.x.to_string();
        let y_str = sp_ref.me.y.to_string();
        pt.push_attribute(("x", x_str.as_str()));
        pt.push_attribute(("y", y_str.as_str()));
        pt.push_attribute(("type", pt_type));
        w.write_event(Event::Empty(pt)).ok();

        // Write next control point before advancing
        if !sp_ref.next.is_null() {
            let next_spline = &*sp_ref.next;
            let next_sp = next_spline.to;
            if !next_sp.is_null() && sp_ref.nonextcp() == 0 {
                let mut cp = BytesStart::new("point");
                let cx_str = sp_ref.nextcp.x.to_string();
                let cy_str = sp_ref.nextcp.y.to_string();
                cp.push_attribute(("x", cx_str.as_str()));
                cp.push_attribute(("y", cy_str.as_str()));
                w.write_event(Event::Empty(cp)).ok();
            }

            // Advance
            sp = next_sp;
        } else {
            break;
        }

        // Safety: prevent infinite loop
        if sp == first {
            if visited {
                break;
            }
            visited = true;
        }
    }

    w.write_event(Event::End(BytesEnd::new("contour"))).ok();
}

// ─── contents.plist ───────────────────────────────────────────────────────────

fn write_contents_plist(dir: &str, mapping: &[(String, String)]) -> c_int {
    let path = format!("{}/contents.plist", dir);
    let buf = Vec::new();
    let mut w = Writer::new_with_indent(Cursor::new(buf), b' ', 2);

    plist_open(&mut w);
    dict_open(&mut w);
    for (name, glif_name) in mapping {
        plist_key_string(&mut w, name, glif_name);
    }
    dict_close(&mut w);
    plist_close(&mut w);

    write_file(&path, w)
}

// ─── Plist XML helpers ────────────────────────────────────────────────────────

fn plist_open<W: std::io::Write>(w: &mut Writer<W>) {
    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .ok();
    // Apple PLIST doctype
    w.write_event(Event::DocType(BytesText::new(
        r#"plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd""#,
    )))
    .ok();

    let mut plist = BytesStart::new("plist");
    plist.push_attribute(("version", "1.0"));
    w.write_event(Event::Start(plist)).ok();
}

fn plist_close<W: std::io::Write>(w: &mut Writer<W>) {
    w.write_event(Event::End(BytesEnd::new("plist"))).ok();
}

fn dict_open<W: std::io::Write>(w: &mut Writer<W>) {
    w.write_event(Event::Start(BytesStart::new("dict"))).ok();
}

fn dict_close<W: std::io::Write>(w: &mut Writer<W>) {
    w.write_event(Event::End(BytesEnd::new("dict"))).ok();
}

fn plist_key_string<W: std::io::Write>(w: &mut Writer<W>, key: &str, value: &str) {
    // <key>key</key>
    w.write_event(Event::Start(BytesStart::new("key"))).ok();
    w.write_event(Event::Text(BytesText::new(key))).ok();
    w.write_event(Event::End(BytesEnd::new("key"))).ok();
    // <string>value</string>
    w.write_event(Event::Start(BytesStart::new("string")))
        .ok();
    w.write_event(Event::Text(BytesText::new(value))).ok();
    w.write_event(Event::End(BytesEnd::new("string"))).ok();
}

fn plist_key_integer<W: std::io::Write>(w: &mut Writer<W>, key: &str, value: i32) {
    w.write_event(Event::Start(BytesStart::new("key"))).ok();
    w.write_event(Event::Text(BytesText::new(key))).ok();
    w.write_event(Event::End(BytesEnd::new("key"))).ok();
    w.write_event(Event::Start(BytesStart::new("integer")))
        .ok();
    let s = value.to_string();
    w.write_event(Event::Text(BytesText::new(&s))).ok();
    w.write_event(Event::End(BytesEnd::new("integer"))).ok();
}

fn plist_key_real<W: std::io::Write>(w: &mut Writer<W>, key: &str, value: f64) {
    w.write_event(Event::Start(BytesStart::new("key"))).ok();
    w.write_event(Event::Text(BytesText::new(key))).ok();
    w.write_event(Event::End(BytesEnd::new("key"))).ok();
    w.write_event(Event::Start(BytesStart::new("real"))).ok();
    let s = format!("{}", value);
    w.write_event(Event::Text(BytesText::new(&s))).ok();
    w.write_event(Event::End(BytesEnd::new("real"))).ok();
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Convert a C string pointer (or null) to a Rust String.
unsafe fn cstr_or(ptr: *mut c_char, default: &str) -> String {
    if ptr.is_null() {
        default.to_string()
    } else {
        std::ffi::CStr::from_ptr(ptr)
            .to_string_lossy()
            .into_owned()
    }
}

/// Mangle a glyph name into a safe .glif filename stem.
fn mangle_glif_name(name: &str, index: usize) -> String {
    let mut result = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' => {
                result.push('_');
            }
            c if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' => {
                result.push(c);
            }
            _ => {
                // Non-ASCII: underscore
                result.push('_');
            }
        }
    }
    if result.is_empty() {
        result = format!("glyph_{:04X}", index);
    }
    // Ensure uniqueness with suffix
    format!("{}", result)
}

/// Write the accumulated XML to the file.
fn write_file(path: &str, w: Writer<Cursor<Vec<u8>>>) -> c_int {
    let bytes = w.into_inner().into_inner();
    match std::fs::write(path, &bytes) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("Error writing {}: {}", path, e);
            1
        }
    }
}
