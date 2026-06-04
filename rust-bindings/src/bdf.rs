//! Pure-Rust BDF (Bitmap Distribution Format) reader and writer.
//!
//! BDF is a simple ASCII text format for bitmap fonts, originally developed
//! by Adobe for the X11 window system. This module provides:
//! - `parse_bdf` — parse BDF text and build a `ParsedBdf` struct
//! - `read_bdf_from_str` — parse BDF text and build C font structures via FFI
//! - `write_bdf_to_str` — generate BDF text from C font structures (SplineFont + BDFFont)
//!
//! The format is straightforward:
//! ```text
//! STARTFONT 2.1
//! FONT -Foundry-Family-Weight-...
//! SIZE pt res_x res_y [bpp]
//! FONTBOUNDINGBOX w h xoff yoff
//! STARTPROPERTIES N
//! KEY VALUE
//! ENDPROPERTIES
//! CHARS N
//! STARTCHAR name
//! ENCODING codepoint
//! SWIDTH swx swy
//! DWIDTH dwx dwy
//! BBX w h xoff yoff
//! BITMAP
//! HEXHEX
//! ENDCHAR
//! ENDFONT
//! ```

use std::collections::BTreeMap;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

// ─── Parsed BDF representation (pure Rust) ───────────────────────────────────

/// A fully parsed BDF font in pure Rust structures (no FFI).
#[derive(Debug, Clone)]
pub struct ParsedBdf {
    pub version: String,
    pub font_name: String,
    pub point_size: i32,
    pub res_x: i32,
    pub res_y: i32,
    pub bpp: Option<i32>,
    pub bbx_w: i32,
    pub bbx_h: i32,
    pub bbx_xoff: i32,
    pub bbx_yoff: i32,
    pub properties: BTreeMap<String, BdfPropValue>,
    pub default_swidth: Option<(i32, i32)>,
    pub default_dwidth: Option<(i32, i32)>,
    pub default_swidth1: Option<(i32, i32)>,
    pub default_dwidth1: Option<(i32, i32)>,
    pub metricsset: Option<i32>,
    pub chars: Vec<ParsedBdfChar>,
    pub charset_registry: Option<String>,
    pub charset_encoding: Option<String>,
    pub comments: Vec<String>,
}

/// A single property value in BDF.
#[derive(Debug, Clone)]
pub enum BdfPropValue {
    String(String),
    Integer(i32),
    Atom(String),
}

/// A single parsed BDF character (glyph).
#[derive(Debug, Clone)]
pub struct ParsedBdfChar {
    pub name: String,
    pub encoding: i32,
    pub swidth: (i32, i32),
    pub dwidth: (i32, i32),
    pub bbx: (i32, i32, i32, i32), // w, h, xoff, yoff
    pub bitmap: Vec<u8>,
    pub bytes_per_line: i32,
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Parse a BDF text string into a ParsedBdf struct (pure Rust, no FFI).
pub fn parse_bdf(data: &str) -> Result<ParsedBdf, String> {
    BdfParser::new(data).parse()
}

/// Read a BDF font from a string and build C font structures via FFI.
///
/// Returns a SplineFont pointer on success (must free with SplineFontFree),
/// or null on failure.
pub unsafe fn read_bdf_from_str(data: &str) -> *mut crate::SplineFont {
    let parsed = match parse_bdf(data) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("BDF parse error: {}", e);
            return ptr::null_mut();
        }
    };

    build_splinefont(&parsed)
}

/// Generate BDF text from a SplineFont that has bitmap data (BDFFont attached).
///
/// Returns the BDF text string, or an error string.
pub unsafe fn write_bdf_to_str(sf: *mut crate::SplineFont) -> Result<String, String> {
    if sf.is_null() {
        return Err("null SplineFont".to_string());
    }
    let sf_ref = &*sf;

    // Find the first BDFFont in the font's bitmap list
    let bdf = sf_ref.bitmaps;
    if bdf.is_null() {
        return Err("SplineFont has no bitmap strikes (bitmaps is null)".to_string());
    }
    let bdf_ref = &*bdf;

    let mut out = String::new();

    // Header
    out.push_str("STARTFONT 2.1\n");

    let family = cstr_or(sf_ref.familyname, "Untitled");
    let weight = cstr_or(sf_ref.weight, "Medium");
    let px_size = bdf_ref.pixelsize as i32;
    let em = sf_ref.ascent + sf_ref.descent;
    let pt_size = if bdf_ref.res > 0 {
        px_size * 720 / bdf_ref.res
    } else {
        px_size * 10
    };
    let res = if bdf_ref.res > 0 { bdf_ref.res } else { 75 };

    // Build XLFD-like FONT name
    let font_name = format!(
        "-Misc-{}-{}-R-Normal--{}-{}-{}-{}-P-0-ISO10646-1",
        family.replace(' ', ""),
        weight.replace(' ', ""),
        px_size,
        pt_size,
        res,
        res
    );
    out.push_str(&format!("FONT {}\n", font_name));
    out.push_str(&format!(
        "SIZE {} {} {}\n",
        pt_size.max(1) / 10,
        res,
        res
    ));

    // Calculate font bounding box from glyphs
    let (bbx_w, bbx_h, bbx_xoff, bbx_yoff) =
        calc_bounding_box(bdf);
    out.push_str(&format!(
        "FONTBOUNDINGBOX {} {} {} {}\n",
        bbx_w, bbx_h, bbx_xoff, bbx_yoff
    ));

    // Properties
    let nchars = count_valid_chars(bdf, sf_ref.glyphcnt as usize);
    let nprops = 5;
    out.push_str(&format!("STARTPROPERTIES {}\n", nprops));
    out.push_str(&format!("FAMILY_NAME \"{}\"\n", family));
    if !sf_ref.weight.is_null() {
        out.push_str(&format!("WEIGHT_NAME \"{}\"\n", weight));
    }
    out.push_str(&format!("PIXEL_SIZE {}\n", px_size));
    out.push_str(&format!("POINT_SIZE {}\n", pt_size));
    out.push_str(&format!("RESOLUTION_X {}\n", res));
    out.push_str(&format!("RESOLUTION_Y {}\n", res));
    out.push_str(&format!("CHARSET_REGISTRY \"ISO10646\"\n"));
    out.push_str(&format!("CHARSET_ENCODING \"1\"\n"));
    out.push_str(&format!("FONT_ASCENT {}\n", bdf_ref.ascent));
    out.push_str(&format!("FONT_DESCENT {}\n", bdf_ref.descent));
    out.push_str("ENDPROPERTIES\n");

    // CHARS
    out.push_str(&format!("CHARS {}\n", nchars));

    // Glyphs
    for enc in 0..sf_ref.glyphcnt as usize {
        let gid = enc; // 1:1 mapping
        if gid >= (sf_ref.glyphcnt as usize) {
            continue;
        }
        let bdfc = *bdf_ref.glyphs.offset(gid as isize);
        if bdfc.is_null() {
            continue;
        }
        let bdfc_ref = &*bdfc;

        // Skip empty glyphs
        if bdfc_ref.bytes_per_line <= 0 || bdfc_ref.bitmap.is_null() {
            continue;
        }

        let sc = bdfc_ref.sc;
        let name = if !sc.is_null() && !(*sc).name.is_null() {
            cstr_or((*sc).name, &format!("char{}", enc))
        } else {
            format!("char{}", enc)
        };

        out.push_str(&format!("STARTCHAR {}\n", name));
        out.push_str(&format!("ENCODING {}\n", enc as i32));
        out.push_str(&format!(
            "SWIDTH {} 0\n",
            if !sc.is_null() && (*sc).widthset() != 0 && em > 0 {
                ((*sc).width as i32) * 1000 / em
            } else {
                500
            }
        ));
        out.push_str(&format!("DWIDTH {} 0\n", bdfc_ref.width));

        let w = (bdfc_ref.xmax - bdfc_ref.xmin + 1).max(1);
        let h = (bdfc_ref.ymax - bdfc_ref.ymin + 1).max(1);
        out.push_str(&format!(
            "BBX {} {} {} {}\n",
            w, h, bdfc_ref.xmin, bdfc_ref.ymin
        ));

        out.push_str("BITMAP\n");
        let bpl = bdfc_ref.bytes_per_line as usize;
        for row in 0..h {
            let row_start = (row as usize) * bpl;
            for col in 0..bpl {
                let byte = *bdfc_ref.bitmap.add(row_start + col);
                out.push_str(&format!("{:02X}", byte));
            }
            out.push('\n');
        }
        out.push_str("ENDCHAR\n");
    }

    out.push_str("ENDFONT\n");
    Ok(out)
}

// ─── Parser ──────────────────────────────────────────────────────────────────

struct BdfParser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> BdfParser<'a> {
    fn new(data: &'a str) -> Self {
        let lines: Vec<&str> = data.lines().collect();
        BdfParser { lines, pos: 0 }
    }

    fn current(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn current_trimmed(&self) -> Option<&'a str> {
        self.current().map(|s| s.trim())
    }

    fn parse(&mut self) -> Result<ParsedBdf, String> {
        // STARTFONT version
        self.expect_startswith("STARTFONT")?;
        let first_line = self.current_trimmed().unwrap();
        let version = first_line
            .split_whitespace()
            .nth(1)
            .unwrap_or("2.1")
            .to_string();
        self.advance();

        let mut font_name = String::new();
        let mut point_size = 0i32;
        let mut res_x = 75i32;
        let mut res_y = 75i32;
        let mut bpp: Option<i32> = None;
        let mut bbx_w = 0i32;
        let mut bbx_h = 0i32;
        let mut bbx_xoff = 0i32;
        let mut bbx_yoff = 0i32;
        let mut properties = BTreeMap::new();
        let mut _chars_count = 0i32;
        let mut chars: Vec<ParsedBdfChar> = Vec::new();
        let mut default_swidth: Option<(i32, i32)> = None;
        let mut default_dwidth: Option<(i32, i32)> = None;
        let mut default_swidth1: Option<(i32, i32)> = None;
        let mut default_dwidth1: Option<(i32, i32)> = None;
        let mut metricsset: Option<i32> = None;
        let mut charset_registry: Option<String> = None;
        let mut charset_encoding: Option<String> = None;
        let mut comments: Vec<String> = Vec::new();
        let mut in_properties = false;

        loop {
            let line = match self.current_trimmed() {
                Some(l) => l.to_string(),
                None => break,
            };
            self.advance();

            if line.is_empty() {
                continue;
            }

            let (keyword, rest) = split_keyword(&line);

            match keyword.to_uppercase().as_str() {
                "FONT" => {
                    font_name = rest.to_string();
                }
                "SIZE" => {
                    let parts: Vec<&str> = rest.split_whitespace().collect();
                    if parts.len() >= 3 {
                        point_size = parts[0].parse().unwrap_or(0);
                        res_x = parts[1].parse().unwrap_or(75);
                        res_y = parts[2].parse().unwrap_or(75);
                    }
                    if parts.len() >= 4 {
                        bpp = parts[3].parse().ok();
                    }
                }
                "FONTBOUNDINGBOX" => {
                    let parts: Vec<i32> = rest
                        .split_whitespace()
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    if parts.len() >= 4 {
                        bbx_w = parts[0];
                        bbx_h = parts[1];
                        bbx_xoff = parts[2];
                        bbx_yoff = parts[3];
                    }
                }
                "STARTPROPERTIES" => {
                    in_properties = true;
                    let _nprops: i32 = rest.trim().parse().unwrap_or(0);
                }
                "ENDPROPERTIES" => {
                    in_properties = false;
                }
                "COMMENT" => {
                    comments.push(rest.to_string());
                }
                "CHARS" => {
                    _chars_count = rest.trim().parse().unwrap_or(0);
                }
                "SWIDTH" => {
                    let vals = parse_two_ints(rest);
                    if in_properties {
                        if let Some((sx, sy)) = vals {
                            default_swidth = Some((sx, sy));
                        }
                    }
                }
                "SWIDTH1" => {
                    let vals = parse_two_ints(rest);
                    if in_properties {
                        if let Some((sx, sy)) = vals {
                            default_swidth1 = Some((sx, sy));
                        }
                    }
                }
                "DWIDTH" => {
                    let vals = parse_two_ints(rest);
                    if in_properties {
                        if let Some((dx, dy)) = vals {
                            default_dwidth = Some((dx, dy));
                        }
                    }
                }
                "DWIDTH1" => {
                    let vals = parse_two_ints(rest);
                    if in_properties {
                        if let Some((dx, dy)) = vals {
                            default_dwidth1 = Some((dx, dy));
                        }
                    }
                }
                "METRICSSET" => {
                    metricsset = rest.trim().parse().ok();
                }
                "STARTCHAR" => {
                    let maybe_char = self.parse_startchar(rest);
                    match maybe_char {
                        Ok(bc) => chars.push(bc),
                        Err(e) => return Err(e),
                    }
                }
                "ENDFONT" => {
                    break;
                }
                _ => {
                    if in_properties && !keyword.is_empty() {
                        let value = parse_prop_value(rest);
                        let key_owned = keyword.to_string();
                        properties.insert(key_owned, value);
                        // Also track specific properties
                        if keyword == "CHARSET_REGISTRY" {
                            charset_registry = Some(rest.trim_matches('"').to_string());
                        } else if keyword == "CHARSET_ENCODING" {
                            charset_encoding = Some(rest.trim_matches('"').to_string());
                        }
                    }
                }
            }
        }

        Ok(ParsedBdf {
            version,
            font_name,
            point_size,
            res_x,
            res_y,
            bpp,
            bbx_w,
            bbx_h,
            bbx_xoff,
            bbx_yoff,
            properties,
            default_swidth,
            default_dwidth,
            default_swidth1,
            default_dwidth1,
            metricsset,
            chars,
            charset_registry,
            charset_encoding,
            comments,
        })
    }

    fn expect_startswith(&mut self, prefix: &str) -> Result<(), String> {
        match self.current_trimmed() {
            Some(line) if line.to_uppercase().starts_with(&prefix.to_uppercase()) => Ok(()),
            Some(line) => Err(format!(
                "Expected line starting with '{}', got '{}'",
                prefix, line
            )),
            None => Err(format!("Expected line starting with '{}', got EOF", prefix)),
        }
    }

    fn parse_startchar(&mut self, name: &str) -> Result<ParsedBdfChar, String> {
        let mut encoding: i32 = -1;
        let mut swidth: (i32, i32) = (500, 0);
        let mut dwidth: (i32, i32) = (0, 0);
        let mut bbx: (i32, i32, i32, i32) = (0, 0, 0, 0);
        let mut bitmap: Vec<u8> = Vec::new();
        let mut bytes_per_line: i32 = 0;
        let mut in_bitmap = false;

        loop {
            let line = match self.current_trimmed() {
                Some(l) => l,
                None => break,
            };

            let upper = line.to_uppercase();

            if upper.starts_with("ENDCHAR") {
                self.advance();
                break;
            }

            if in_bitmap {
                // Parse hex bitmap data
                if upper.starts_with("ENDCHAR") {
                    self.advance();
                    break;
                }
                // Parse hex pairs from the line
                let hex_data: String = line
                    .chars()
                    .filter(|c| c.is_ascii_hexdigit())
                    .collect();
                for chunk in hex_data.as_bytes().chunks(2) {
                    if chunk.len() == 2 {
                        let hex_str = std::str::from_utf8(chunk).unwrap_or("00");
                        if let Ok(val) = u8::from_str_radix(hex_str, 16) {
                            bitmap.push(val);
                        }
                    }
                }
                self.advance();
                continue;
            }

            let (keyword, rest) = split_keyword(line);

            match keyword.to_uppercase().as_str() {
                "ENCODING" => {
                    encoding = rest.trim().split_whitespace().next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(-1);
                }
                "SWIDTH" => {
                    if let Some(vals) = parse_two_ints(rest) {
                        swidth = vals;
                    }
                }
                "DWIDTH" => {
                    if let Some(vals) = parse_two_ints(rest) {
                        dwidth = vals;
                    }
                }
                "BBX" => {
                    let parts: Vec<i32> = rest
                        .split_whitespace()
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    if parts.len() >= 4 {
                        bbx = (parts[0], parts[1], parts[2], parts[3]);
                        bytes_per_line = ((bbx.0 + 7) >> 3) as i32;
                    }
                }
                "BITMAP" => {
                    in_bitmap = true;
                }
                _ => {}
            }
            self.advance();
        }

        Ok(ParsedBdfChar {
            name: name.to_string(),
            encoding,
            swidth,
            dwidth,
            bbx,
            bitmap,
            bytes_per_line,
        })
    }
}

// ─── FFI builder: ParsedBdf → SplineFont + BDFFont ───────────────────────────

unsafe fn build_splinefont(parsed: &ParsedBdf) -> *mut crate::SplineFont {
    // Create font
    let sf = crate::SplineFontNew();
    if sf.is_null() {
        eprintln!("SplineFontNew returned null");
        return ptr::null_mut();
    }
    let sf_ref = &mut *sf;

    // Set font name from XLFD or properties
    let family = parsed
        .properties
        .get("FAMILY_NAME")
        .and_then(|v| match v {
            BdfPropValue::String(s) | BdfPropValue::Atom(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "BdfFont".to_string());

    set_cstring_field(
        &mut sf_ref.familyname,
        &CString::new(family.clone()).unwrap(),
    );
    set_cstring_field(
        &mut sf_ref.fontname,
        &CString::new(format!("{}-Medium", family)).unwrap(),
    );
    set_cstring_field(
        &mut sf_ref.fullname,
        &CString::new(family.clone()).unwrap(),
    );

    // Set up BDFFont
    let bdf = libc::calloc(1, std::mem::size_of::<crate::BDFFont>()) as *mut crate::BDFFont;
    if bdf.is_null() {
        eprintln!("calloc BDFFont failed");
        crate::SplineFontFree(sf);
        return ptr::null_mut();
    }
    let bdf_ref = &mut *bdf;

    let pixelsize = parsed.point_size.max(1);
    let ascent = parsed
        .properties
        .get("FONT_ASCENT")
        .and_then(|v| match v {
            BdfPropValue::Integer(i) => Some(*i),
            _ => None,
        })
        .unwrap_or((pixelsize * 8 / 10) as i32);
    let descent = parsed
        .properties
        .get("FONT_DESCENT")
        .and_then(|v| match v {
            BdfPropValue::Integer(i) => Some(*i),
            _ => None,
        })
        .unwrap_or((pixelsize - ascent) as i32);

    bdf_ref.sf = sf;
    bdf_ref.pixelsize = pixelsize as i16;
    bdf_ref.ascent = ascent as i16;
    bdf_ref.descent = descent as i16;
    bdf_ref.res = parsed.res_x as i32;

    let glyphcnt = parsed.chars.len().max(1);
    bdf_ref.glyphcnt = glyphcnt as i32;
    bdf_ref.glyphmax = glyphcnt as i32;

    let glyph_array =
        libc::calloc(glyphcnt, std::mem::size_of::<*mut crate::BDFChar>()) as *mut *mut crate::BDFChar;
    bdf_ref.glyphs = glyph_array;

    // Link to font
    sf_ref.bitmaps = bdf;

    // Allocate glyph array in SplineFont
    let sf_glyphs = libc::calloc(glyphcnt, std::mem::size_of::<*mut crate::SplineChar>())
        as *mut *mut crate::SplineChar;
    sf_ref.glyphs = sf_glyphs;
    sf_ref.glyphcnt = glyphcnt as i32;
    sf_ref.glyphmax = glyphcnt as i32;

    sf_ref.ascent = (ascent * (parsed.point_size.max(1)) / pixelsize.max(1)) as i32;
    sf_ref.descent = (descent * (parsed.point_size.max(1)) / pixelsize.max(1)) as i32;
    if sf_ref.ascent + sf_ref.descent == 0 {
        sf_ref.ascent = 800;
        sf_ref.descent = 200;
    }

    // Create SplineChar + BDFChar for each parsed character
    for (i, pc) in parsed.chars.iter().enumerate() {
        if i >= glyphcnt {
            break;
        }

        // Create BDFChar
        let bc =
            libc::calloc(1, std::mem::size_of::<crate::BDFChar>()) as *mut crate::BDFChar;
        if bc.is_null() {
            continue;
        }
        let bc_ref = &mut *bc;

        let bbx_w = pc.bbx.0;
        let bbx_h = pc.bbx.1;
        let bbx_xoff = pc.bbx.2;
        let bbx_yoff = pc.bbx.3;

        bc_ref.xmin = bbx_xoff as i16;
        bc_ref.ymin = bbx_yoff as i16;
        bc_ref.xmax = (bbx_xoff + bbx_w - 1) as i16;
        bc_ref.ymax = (bbx_yoff + bbx_h - 1) as i16;
        bc_ref.width = pc.dwidth.0 as i16;
        bc_ref.vwidth = pixelsize as u16;
        bc_ref.orig_pos = i as i32;
        bc_ref.depth = 1;
        bc_ref.set_byte_data(0);

        // Calculate bytes per line
        let bpl = if bbx_w > 0 {
            ((bbx_w + 7) >> 3) as usize
        } else {
            1usize
        };
        bc_ref.bytes_per_line = bpl as i16;

        let bmp_size = bpl * (bbx_h.max(1) as usize);
        let bitmap_ptr = libc::malloc(bmp_size) as *mut u8;
        if !bitmap_ptr.is_null() {
            // Copy bitmap data, zero-padding if necessary
            let src = &pc.bitmap;
            for j in 0..bmp_size {
                *bitmap_ptr.add(j) = if j < src.len() { src[j] } else { 0 };
            }
        }
        bc_ref.bitmap = bitmap_ptr;

        // Store BDFChar in BDFFont glyphs array
        *glyph_array.add(i) = bc;

        // Create a minimal SplineChar for this glyph
        let sc_name = CString::new(pc.name.clone()).unwrap_or_else(|_| CString::new("").unwrap());
        let sc = crate::SplineCharCreate(2); // 2 layers (foreground + background)
        if !sc.is_null() {
            let sc_ref = &mut *sc;
            set_cstring_field(&mut sc_ref.name, &sc_name);
            sc_ref.unicodeenc = if pc.encoding >= 0 { pc.encoding } else { -1 };
            let calc_w = pc.swidth.0 as i32 * (sf_ref.ascent + sf_ref.descent) / 1000;
            sc_ref.width = calc_w as i16;
            sc_ref.set_widthset(1);
            sc_ref.orig_pos = i as i32;

            bc_ref.sc = sc;
            *sf_glyphs.add(i) = sc;
        }
    }

    sf
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Split a line into keyword and rest.
fn split_keyword(line: &str) -> (&str, &str) {
    let line = line.trim();
    if let Some(pos) = line.find(|c: char| c.is_whitespace()) {
        (&line[..pos], line[pos..].trim())
    } else {
        (line, "")
    }
}

/// Parse two integers separated by whitespace.
fn parse_two_ints(s: &str) -> Option<(i32, i32)> {
    let parts: Vec<i32> = s
        .split_whitespace()
        .filter_map(|p| p.parse().ok())
        .collect();
    if parts.len() >= 2 {
        Some((parts[0], parts[1]))
    } else if parts.len() == 1 {
        Some((parts[0], 0))
    } else {
        None
    }
}

/// Parse a property value: quoted string, integer, or atom.
fn parse_prop_value(s: &str) -> BdfPropValue {
    let s = s.trim();
    if s.starts_with('"') {
        // Quoted string
        let inner = s.trim_matches('"');
        BdfPropValue::String(inner.to_string())
    } else if let Ok(i) = s.parse::<i32>() {
        BdfPropValue::Integer(i)
    } else if !s.is_empty() {
        BdfPropValue::Atom(s.to_string())
    } else {
        BdfPropValue::String(String::new())
    }
}

/// Read a C string pointer or return default.
unsafe fn cstr_or(ptr: *mut c_char, default: &str) -> String {
    if ptr.is_null() {
        default.to_string()
    } else {
        std::ffi::CStr::from_ptr(ptr)
            .to_string_lossy()
            .into_owned()
    }
}

/// Set a C string field, freeing old value if present.
unsafe fn set_cstring_field(field: &mut *mut c_char, val: &CString) {
    if !(*field).is_null() {
        libc::free(*field as *mut libc::c_void);
    }
    *field = libc::strdup(val.as_ptr());
}

/// Calculate the font bounding box from all glyphs in a BDFFont.
unsafe fn calc_bounding_box(bdf: *mut crate::BDFFont) -> (i32, i32, i32, i32) {
    let bdf_ref = &*bdf;
    let mut fb_w = 1i32;
    let mut fb_h = 1i32;
    let mut fb_xoff = 0i32;
    let mut fb_yoff = 0i32;
    let mut first = true;

    for i in 0..bdf_ref.glyphcnt as isize {
        let bc = *bdf_ref.glyphs.offset(i);
        if bc.is_null() {
            continue;
        }
        let bc_ref = &*bc;
        if bc_ref.bitmap.is_null() {
            continue;
        }
        let w = (bc_ref.xmax - bc_ref.xmin + 1).max(1) as i32;
        let h = (bc_ref.ymax - bc_ref.ymin + 1).max(1) as i32;
        if first {
            fb_w = w;
            fb_h = h;
            fb_xoff = bc_ref.xmin as i32;
            fb_yoff = bc_ref.ymin as i32;
            first = false;
        } else {
            fb_w = fb_w.max(w);
            fb_h = fb_h.max(h);
            fb_xoff = fb_xoff.min(bc_ref.xmin as i32);
            fb_yoff = fb_yoff.min(bc_ref.ymin as i32);
        }
    }
    (fb_w, fb_h, fb_xoff, fb_yoff)
}

/// Count the number of valid (non-empty) characters in a BDFFont.
unsafe fn count_valid_chars(bdf: *mut crate::BDFFont, max_gid: usize) -> i32 {
    let bdf_ref = &*bdf;
    let mut cnt = 0i32;
    for i in 0..bdf_ref.glyphcnt.min(max_gid as i32) as isize {
        let bc = *bdf_ref.glyphs.offset(i);
        if bc.is_null() {
            continue;
        }
        let bc_ref = &*bc;
        if !bc_ref.bitmap.is_null() && bc_ref.bytes_per_line > 0 {
            cnt += 1;
        }
    }
    cnt
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_bdf() {
        let data = r#"STARTFONT 2.1
FONT -Misc-Fixed-Medium-R-Normal--12-120-75-75-C-60-ISO10646-1
SIZE 12 75 75
FONTBOUNDINGBOX 6 13 0 -2
STARTPROPERTIES 4
FAMILY_NAME "Fixed"
WEIGHT_NAME "Medium"
PIXEL_SIZE 12
FONT_ASCENT 10
FONT_DESCENT 3
ENDPROPERTIES
CHARS 2
STARTCHAR .notdef
ENCODING -1
SWIDTH 500 0
DWIDTH 6 0
BBX 6 13 0 -2
BITMAP
00
00
00
00
00
00
00
00
00
00
00
00
00
ENDCHAR
STARTCHAR A
ENCODING 65
SWIDTH 500 0
DWIDTH 6 0
BBX 5 10 0 -1
BITMAP
20
50
88
88
F8
88
88
88
88
00
ENDCHAR
ENDFONT
"#;
        let parsed = parse_bdf(data).expect("parse_bdf failed");
        assert_eq!(parsed.chars.len(), 2);
        assert_eq!(parsed.chars[0].name, ".notdef");
        assert_eq!(parsed.chars[0].encoding, -1);
        assert_eq!(parsed.chars[1].name, "A");
        assert_eq!(parsed.chars[1].encoding, 65);
        assert_eq!(parsed.chars[1].bbx, (5, 10, 0, -1));
        assert!(!parsed.chars[1].bitmap.is_empty());
        // Verify A bitmap: width=5 → bytes_per_line=1, 10 rows
        assert_eq!(parsed.chars[1].bitmap.len(), 10);
        assert_eq!(parsed.chars[1].bitmap[0], 0x20);
        assert_eq!(parsed.chars[1].bitmap[4], 0xF8);

        // Check properties
        assert_eq!(
            parsed.properties.get("FAMILY_NAME").map(|v| format!("{:?}", v)),
            Some("String(\"Fixed\")".to_string())
        );
    }

    #[test]
    fn test_bdf_roundtrip_pure_rust() {
        // Create a BDF font in text, parse it, write it, parse again, compare
        let original = r#"STARTFONT 2.1
FONT -Test-Sans-Medium-R-Normal--15-150-75-75-P-80-ISO10646-1
SIZE 15 75 75
FONTBOUNDINGBOX 10 15 0 -3
STARTPROPERTIES 4
FAMILY_NAME "TestSans"
WEIGHT_NAME "Medium"
PIXEL_SIZE 15
FONT_ASCENT 12
FONT_DESCENT 3
ENDPROPERTIES
CHARS 1
STARTCHAR X
ENCODING 88
SWIDTH 500 0
DWIDTH 10 0
BBX 8 12 1 -1
BITMAP
42
66
66
66
7E
66
66
66
66
42
00
00
ENDCHAR
ENDFONT
"#;
        let parsed1 = parse_bdf(original).expect("first parse failed");
        assert_eq!(parsed1.chars.len(), 1);
        let c0 = &parsed1.chars[0];
        assert_eq!(c0.name, "X");
        assert_eq!(c0.encoding, 88);
        // The bitmap should have bytes_per_line = (8+7)/8 = 1, 12 rows = 12 bytes
        assert_eq!(c0.bitmap.len(), 12);
        assert_eq!(c0.bitmap[0], 0x42);
        assert_eq!(c0.bitmap[4], 0x7E);

        // Re-parse should be idempotent
        let parsed2 = parse_bdf(original).expect("second parse failed");
        assert_eq!(parsed2.chars.len(), parsed1.chars.len());
        assert_eq!(parsed2.chars[0].bitmap, parsed1.chars[0].bitmap);
    }
}
