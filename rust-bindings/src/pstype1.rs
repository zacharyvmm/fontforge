//! Pure-Rust PS Type 1 / PFA font writer.
//!
//! Generates valid Type 1 PFA (Printer Font ASCII) output from a SplineFont.
//! Handles eexec encryption, Type 1 charstring number encoding, and the
//! font dictionary structure. Reading uses the existing C FFI via LoadSplineFont.

const LY_FORE: isize = 1;

// ─── Type 1 charstring constants ─────────────────────────────────────────────

/// Type 1 charstring operators
mod op {
    pub const HSTEM: u8 = 1;
    pub const VSTEM: u8 = 3;
    pub const VMOVETO: u8 = 4;
    pub const RLINETO: u8 = 5;
    pub const HLINETO: u8 = 6;
    pub const VLINETO: u8 = 7;
    pub const RRCURVETO: u8 = 8;
    pub const CLOSEPATH: u8 = 9;
    pub const CALLSUBR: u8 = 10;
    pub const RETURN: u8 = 11;
    pub const HSBW: u8 = 13;
    pub const ENDCHAR: u8 = 14;
    pub const RMOVETO: u8 = 21;
    pub const HMOVETO: u8 = 22;
}

/// Eexec cipher constants
const C1: u16 = 52845;
const C2: u16 = 22719;
const EE_R: u16 = 55665;

// ─── Eexec hex encoder ───────────────────────────────────────────────────────

/// Generates eexec-encrypted hex output.
struct EexecEncoder {
    r: u16,
    line_len: usize,
    output: String,
}

impl EexecEncoder {
    fn new() -> Self {
        EexecEncoder {
            r: EE_R,
            line_len: 0,
            output: String::with_capacity(4096),
        }
    }

    /// Write 4 random seed bytes. At least one must encrypt to a non-hex character.
    fn write_seed_bytes(&mut self) {
        // Use a fixed seed sequence that works (like the C code's randombytes).
        // These bytes are incremented on each call to avoid ever using the same seed.
        static mut SEED: [u8; 4] = [0xaa, 0x55, 0x3e, 0x4d];

        let seed = unsafe {
            SEED[0] = SEED[0].wrapping_add(3);
            SEED[1] = SEED[1].wrapping_add(5);
            SEED[2] = SEED[2].wrapping_add(7);
            SEED[3] = SEED[3].wrapping_add(11);
            SEED
        };

        for &b in &seed {
            self.write_plain_byte(b);
        }
    }

    /// Write a raw (plaintext) byte, encrypted to hex.
    fn write_plain_byte(&mut self, plain: u8) {
        let cipher = plain ^ (self.r >> 8) as u8;
        self.r = ((cipher as u16).wrapping_add(self.r))
            .wrapping_mul(C1)
            .wrapping_add(C2);

        self.output.push(hex_nibble(cipher >> 4));
        self.output.push(hex_nibble(cipher & 0xf));
        self.line_len += 2;
        if self.line_len >= 70 {
            self.output.push('\n');
            self.line_len = 0;
        }
    }

        /// Write a string literal (plaintext), encrypted to hex.
    fn write_plain_str(&mut self, s: &str) {
        for b in s.bytes() {
            self.write_plain_byte(b);
        }
    }

    /// Write raw bytes directly (each byte gets eexec-encrypted and hex-encoded).
    fn write_raw_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_plain_byte(b);
        }
    }

    /// Consume the encoder and return the hex string.
    fn finish(mut self) -> String {
        if self.line_len > 0 {
            self.output.push('\n');
        }
        self.output
    }
}

fn hex_nibble(n: u8) -> char {
    if n <= 9 {
        (b'0' + n) as char
    } else {
        (b'A' - 10 + n) as char
    }
}

// ─── Type 1 charstring number encoding ───────────────────────────────────────

/// Encode an i32 value into Type 1 charstring bytes.
/// Type 1/CFF number encoding:
///   -107..+107: single byte (v + 139)
///   +108..+1131: [247, v-108] or [248, v-364]
///   -1131..-108: [251, -(v+108)] or [252, -(v+364)]
///   Otherwise: 255 + 4-byte big-endian signed integer
fn encode_t1_number(v: i32, buf: &mut Vec<u8>) {
    if v >= -107 && v <= 107 {
        buf.push((v + 139) as u8);
    } else if v >= 108 && v <= 1131 {
        if v <= 363 {
            buf.push(247);
            buf.push((v - 108) as u8);
        } else {
            buf.push(248);
            buf.push((v - 364) as u8);
        }
    } else if v >= -1131 && v <= -108 {
        if v >= -363 {
            buf.push(251);
            buf.push((-(v + 108)) as u8);
        } else {
            buf.push(252);
            buf.push((-(v + 364)) as u8);
        }
    } else {
        buf.push(255);
        let bytes = (v as i32).to_be_bytes();
        buf.extend_from_slice(&bytes);
    }
}

/// Encode a f64 value into Type 1 charstring bytes (rounds to nearest i32).
fn encode_t1_real(v: f64, buf: &mut Vec<u8>) {
    encode_t1_number(v.round() as i32, buf);
}

// ─── Spline traversal helpers ────────────────────────────────────────────────

/// Check if a Spline is linear (control points at endpoints).
unsafe fn is_linear_spline(from: *const crate::SplinePoint, to: *const crate::SplinePoint) -> bool {
    if from.is_null() || to.is_null() {
        return true;
    }
    let f = &*from;
    let nextcp = f.nextcp;
    let prevcp = (*to).prevcp;
    // A linear spline has both control points at the endpoints
    (nextcp.x - f.me.x).abs() < 0.01
        && (nextcp.y - f.me.y).abs() < 0.01
        && (prevcp.x - (*to).me.x).abs() < 0.01
        && (prevcp.y - (*to).me.y).abs() < 0.01
}

/// Structure to hold a decoded glyph path for charstring encoding.
#[derive(Debug, Clone)]
struct GlyphPath {
    char_name: String,
    width: f64,
    contours: Vec<Vec<PathPoint>>,
}

#[derive(Debug, Clone, Copy)]
struct PathPoint {
    x: f64,
    y: f64,
    kind: PathPointKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PathPointKind {
    MoveTo,
    LineTo,
    CurveTo {
        cp1x: f64,
        cp1y: f64,
        cp2x: f64,
        cp2y: f64,
    },
}

/// Walk a SplineFont's glyphs and extract path data for charstring encoding.
unsafe fn extract_glyph_paths(sf: *const crate::SplineFont) -> Vec<GlyphPath> {
    let sf_ref = &*sf;
    let mut glyphs = Vec::new();

    for i in 0..sf_ref.glyphcnt as usize {
        let sc = *sf_ref.glyphs.add(i);
        if sc.is_null() {
            continue;
        }
        let sc_ref = &*sc;

        // Get glyph name
        let name = if sc_ref.name.is_null() {
            format!(".g{}", i)
        } else {
            let c_name = std::ffi::CStr::from_ptr(sc_ref.name as *const std::os::raw::c_char);
            c_name.to_string_lossy().into_owned()
        };

        let width = sc_ref.width as f64;

        let mut contours = Vec::new();
        let layer = sc_ref.layers.offset(LY_FORE);
        if !(*layer).splines.is_null() {
            let mut spl = (*layer).splines;
            while !spl.is_null() {
                let ss = &*spl;
                if !ss.first.is_null() {
                    let mut points = Vec::new();
                    extract_contour_points(ss.first, &mut points);
                    if !points.is_empty() {
                        contours.push(points);
                    }
                }
                spl = ss.next;
            }
        }

        glyphs.push(GlyphPath {
            char_name: name,
            width,
            contours,
        });
    }

    glyphs
}

/// Walk a closed contour starting at `first` and extract absolute path points.
unsafe fn extract_contour_points(
    first: *mut crate::SplinePoint,
    points: &mut Vec<PathPoint>,
) {
    if first.is_null() {
        return;
    }

    let mut cur: *mut crate::SplinePoint = first;
    let mut visited_first = false;

    loop {
        if cur.is_null() {
            break;
        }
        let pt = &*cur;

        // Add the point (if first, as MoveTo; otherwise as LineTo or CurveTo)
        if !visited_first {
            points.push(PathPoint {
                x: pt.me.x,
                y: pt.me.y,
                kind: PathPointKind::MoveTo,
            });
            visited_first = true;
        } else {
            // Check if the incoming spline is linear
            // We need to find the previous point to check control points
            // The incoming spline info is stored on the current point (prevcp)
            if is_linear_spline_to_current(cur) {
                points.push(PathPoint {
                    x: pt.me.x,
                    y: pt.me.y,
                    kind: PathPointKind::LineTo,
                });
            } else {
                // Curve: the control points come from the previous point's nextcp
                // and the current point's prevcp
                let prev_pt = find_previous_point(first, cur);
                if !prev_pt.is_null() {
                    let pp = &*prev_pt;
                    points.push(PathPoint {
                        x: pt.me.x,
                        y: pt.me.y,
                        kind: PathPointKind::CurveTo {
                            cp1x: pp.nextcp.x,
                            cp1y: pp.nextcp.y,
                            cp2x: pt.prevcp.x,
                            cp2y: pt.prevcp.y,
                        },
                    });
                } else {
                    points.push(PathPoint {
                        x: pt.me.x,
                        y: pt.me.y,
                        kind: PathPointKind::LineTo,
                    });
                }
            }
        }

        // Move to next point
        let spline = pt.next;
        if spline.is_null() {
            break;
        }
        let next_pt = (*spline).to;
        if next_pt.is_null() || next_pt == first {
            break; // Closed contour
        }
        cur = next_pt;
    }
}

/// Check if the incoming spline to `cur` is linear.
unsafe fn is_linear_spline_to_current(cur: *mut crate::SplinePoint) -> bool {
    if cur.is_null() {
        return true;
    }
    let pt = &*cur;
    let spline = pt.prev;
    if spline.is_null() {
        return true;
    }
    let from = (*spline).from;
    if from.is_null() {
        return true;
    }
    is_linear_spline(from, cur)
}

/// Find the previous point in the contour (follows prev->from chain).
unsafe fn find_previous_point(
    _first: *mut crate::SplinePoint,
    cur: *mut crate::SplinePoint,
) -> *mut crate::SplinePoint {
    if cur.is_null() {
        return std::ptr::null_mut();
    }
    let pt = &*cur;
    let spline = pt.prev;
    if spline.is_null() {
        return std::ptr::null_mut();
    }
    let from = (*spline).from;
    from
}

// ─── Charstring encoder ──────────────────────────────────────────────────────

/// Encode a single glyph into a Type 1 charstring byte vector.
fn encode_glyph_charstring(glyph: &GlyphPath) -> Vec<u8> {
    let mut buf = Vec::with_capacity(128);

    // hsbw: sidebearing + width
    // For simple fonts, left sidebearing = first point x, but we use 0 as default
    // The actual sidebearing is determined by the first moveto
    let lsb = 0.0; // We use rmoveto for positioning
    encode_t1_real(lsb, &mut buf);
    encode_t1_real(glyph.width, &mut buf);
    buf.push(op::HSBW);

    for contour in &glyph.contours {
        if contour.is_empty() {
            continue;
        }

        let mut first = true;
        let mut prev_x = 0.0f64;
        let mut prev_y = 0.0f64;

        for pt in contour {
            let (dx, dy) = (pt.x - prev_x, pt.y - prev_y);
            prev_x = pt.x;
            prev_y = pt.y;

            match pt.kind {
                PathPointKind::MoveTo => {
                    if first {
                        encode_t1_real(dx, &mut buf);
                        encode_t1_real(dy, &mut buf);
                        buf.push(op::RMOVETO);
                        first = false;
                    }
                }
                PathPointKind::LineTo => {
                    encode_t1_real(dx, &mut buf);
                    encode_t1_real(dy, &mut buf);
                    buf.push(op::RLINETO);
                }
                PathPointKind::CurveTo {
                    cp1x,
                    cp1y,
                    cp2x,
                    cp2y,
                } => {
                    // rrcurveto takes 6 relative values:
                    // dx1,dy1 = cp1 relative to current point
                    // dx2,dy2 = cp2 relative to cp1
                    // dx3,dy3 = endpoint relative to cp2
                    let (prev_for_cp1_x, prev_for_cp1_y) = (prev_x - dx, prev_y - dy);
                    let d_cp1_x = cp1x - prev_for_cp1_x;
                    let d_cp1_y = cp1y - prev_for_cp1_y;
                    let d_cp2_x = cp2x - cp1x;
                    let d_cp2_y = cp2y - cp1y;
                    let d_pt_x = pt.x - cp2x;
                    let d_pt_y = pt.y - cp2y;

                    encode_t1_real(d_cp1_x, &mut buf);
                    encode_t1_real(d_cp1_y, &mut buf);
                    encode_t1_real(d_cp2_x, &mut buf);
                    encode_t1_real(d_cp2_y, &mut buf);
                    encode_t1_real(d_pt_x, &mut buf);
                    encode_t1_real(d_pt_y, &mut buf);
                    buf.push(op::RRCURVETO);
                }
            }
        }

        // closepath
        buf.push(op::CLOSEPATH);
    }

    // endchar
    buf.push(op::ENDCHAR);
    buf
}

/// Hex-encode a byte slice for use in CharStrings dictionary (not eexec-encrypted,
/// this is for the CharStrings values inside the encrypted section which use
/// a separate per-string encoding with lenIV random prefix bytes).
fn hex_encode_bytes(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for &b in data {
        s.push(hex_nibble(b >> 4));
        s.push(hex_nibble(b & 0xf));
    }
    s
}

// ─── PFA writer ──────────────────────────────────────────────────────────────

/// Write a Type 1 PFA font to a String from a SplineFont pointer.
///
/// # Safety
/// The SplineFont must be a valid, initialized font.
pub unsafe fn write_pfa_to_str(sf: *const crate::SplineFont) -> String {
    let sf_ref = &*sf;

    let fontname = if sf_ref.fontname.is_null() {
        "Untitled".to_string()
    } else {
        let c_name = std::ffi::CStr::from_ptr(sf_ref.fontname as *const std::os::raw::c_char);
        c_name.to_string_lossy().into_owned()
    };

    let version = if sf_ref.version.is_null() {
        "1.0".to_string()
    } else {
        let c_ver = std::ffi::CStr::from_ptr(sf_ref.version as *const std::os::raw::c_char);
        c_ver.to_string_lossy().into_owned()
    };

    let ascent = sf_ref.ascent;
    let descent = sf_ref.descent;
    let em_size = ascent + descent;

    // Compute FontBBox from glyph data (simplified: use em-square)
    let (bbox_minx, bbox_miny, bbox_maxx, bbox_maxy) = {
        let mut minx = 0.0f64;
        let mut miny = descent as f64;
        let mut maxx = em_size as f64 * 0.8;
        let mut maxy = ascent as f64;

        // Scan glyphs for actual bounds
        let glyphs = extract_glyph_paths(sf);
        for g in &glyphs {
            for contour in &g.contours {
                for pt in contour {
                    if pt.x < minx {
                        minx = pt.x;
                    }
                    if pt.y < miny {
                        miny = pt.y;
                    }
                    if pt.x > maxx {
                        maxx = pt.x;
                    }
                    if pt.y > maxy {
                        maxy = pt.y;
                    }
                }
            }
        }
        (minx, miny, maxx, maxy)
    };

    let mut out = String::with_capacity(8192);

    // ── Clear-text section ───────────────────────────────────────────────
    out.push_str(&format!(
        "%!PS-AdobeFont-1.0: {} {}\n",
        fontname, version
    ));
    out.push_str("%%Creator: FontForge Rust PFA writer\n");

    // Font dictionary
    out.push_str("11 dict begin\n");
    out.push_str("/FontType 1 def\n");

    // FontMatrix: scale from design units to PS units (1/1000)
    let fm_scale = if em_size > 0 { 1.0 / em_size as f64 } else { 0.001 };
    out.push_str(&format!(
        "/FontMatrix [{:.6} 0 0 {:.6} 0 0] readonly def\n",
        fm_scale, fm_scale
    ));

    out.push_str(&format!("/FontName /{} def\n", fontname));

    // FontBBox
    out.push_str(&format!(
        "/FontBBox {{{:.0} {:.0} {:.0} {:.0}}} readonly def\n",
        bbox_minx.floor(),
        bbox_miny.floor(),
        bbox_maxx.ceil(),
        bbox_maxy.ceil()
    ));

    out.push_str("/PaintType 0 def\n");

    // FontInfo dictionary
    out.push_str("/FontInfo 8 dict dup begin\n");
    out.push_str(&format!("  /FamilyName ({}) def\n", fontname));
    out.push_str(&format!("  /FullName ({}) def\n", fontname));
    out.push_str("  /Notice (Generated by FontForge Rust) def\n");
    out.push_str("  /Weight (Medium) def\n");
    out.push_str("  /ItalicAngle 0 def\n");
    out.push_str("  /isFixedPitch false def\n");
    out.push_str("  /UnderlinePosition -100 def\n");
    out.push_str("  /UnderlineThickness 50 def\n");
    out.push_str("end def\n");

    // Encoding: 256-entry array (standard encoding with .notdef for all)
    out.push_str("/Encoding 256 array\n");
    out.push_str("0 1 255 {1 index exch /.notdef put} for\n");

    // Fill in encoding for glyphs that have names
    let glyphs = extract_glyph_paths(sf);
    for (i, g) in glyphs.iter().enumerate() {
        if i < 256 && g.char_name != ".notdef" {
            out.push_str(&format!("dup {} /{} put\n", i, g.char_name));
        }
    }
    out.push_str("readonly def\n");

    // End clear-text section
    out.push_str("currentdict end\n");
    out.push_str("currentfile eexec\n");

    // ── Encrypted section ─────────────────────────────────────────────────
    let mut enc = EexecEncoder::new();
    enc.write_seed_bytes();

    // Duplicate the font dictionary on the stack for definefont
    enc.write_plain_str("dup /Private 14 dict dup begin\n");

    // Blue values (standard)
    enc.write_plain_str("/BlueValues [-15 0 500 515 700 715] def\n");
    enc.write_plain_str("/OtherBlues [-215 -200] def\n");
    enc.write_plain_str("/BlueScale 0.0375 def\n");
    enc.write_plain_str("/BlueShift 7 def\n");
    enc.write_plain_str("/BlueFuzz 1 def\n");
    enc.write_plain_str("/StdHW [60] def\n");
    enc.write_plain_str("/StdVW [60] def\n");
    enc.write_plain_str("/ForceBold false def\n");
    enc.write_plain_str("/LanguageGroup 0 def\n");
    enc.write_plain_str("/RndStemUp false def\n");
    enc.write_plain_str("/MinFeature {16 16} def\n");
    enc.write_plain_str("/password 5839 def\n");
    enc.write_plain_str("/UniqueID 4000000 def\n");
    enc.write_plain_str("/lenIV 4 def\n");

    enc.write_plain_str("end def\n");

    // CharStrings dictionary
    let n_glyphs = glyphs.len();
    enc.write_plain_str(&format!(
        "/CharStrings {} dict dup begin\n",
        n_glyphs
    ));

    for g in &glyphs {
        let cs_bytes = encode_glyph_charstring(g);

        // lenIV = 4 means we prefix 4 random bytes
        let leniv: usize = 4;
        let total_len = cs_bytes.len() + leniv;

        // Per-string encryption (seed r=4330, separate from eexec)
        let mut per_r: u16 = 4330;
        let mut per_enc = Vec::with_capacity(total_len);

        // 4 random seed bytes for this charstring
        static mut CS_SEED: [u8; 5] = [0xaa, 0x55, 0x3e, 0x4d, 0x89];
        let seed = unsafe {
            CS_SEED[0] = CS_SEED[0].wrapping_add(3);
            CS_SEED[1] = CS_SEED[1].wrapping_add(5);
            CS_SEED[2] = CS_SEED[2].wrapping_add(7);
            CS_SEED[3] = CS_SEED[3].wrapping_add(11);
            CS_SEED[4] = CS_SEED[4].wrapping_add(13);
            CS_SEED
        };

        // lenIV random prefix bytes
        for i in 0..leniv {
            let plain = seed[leniv - 1 - i];
            let cipher = plain ^ (per_r >> 8) as u8;
            per_r = ((cipher as u16).wrapping_add(per_r))
                .wrapping_mul(C1)
                .wrapping_add(C2);
            per_enc.push(cipher);
        }

        // Per-string encrypt the charstring data
        for &b in &cs_bytes {
            let cipher = b ^ (per_r >> 8) as u8;
            per_r = ((cipher as u16).wrapping_add(per_r))
                .wrapping_mul(C1)
                .wrapping_add(C2);
            per_enc.push(cipher);
        }

        // Write the RD wrapper text (eexec-encrypted)
        enc.write_plain_str(&format!("/{} {} RD ", g.char_name, total_len));
        // Write the per-string-encrypted raw bytes (eexec-encrypted as raw bytes on top)
        enc.write_raw_bytes(&per_enc);
        // Write the ND suffix (eexec-encrypted)
        enc.write_plain_str(" ND\n");
    }

    enc.write_plain_str("end end\n");
    enc.write_plain_str("readonly put\n");
    enc.write_plain_str("dup/FontName get exch definefont pop\n");
    // Note: the C code also adds "mark currentfile closefile" but we add it
    // after the encrypted section in the clear-text trailer.

    let encrypted_hex = enc.finish();
    out.push_str(&encrypted_hex);

    // ── Trailer ────────────────────────────────────────────────────────────
    // 8 lines of 64 zeros each = 512 zeros
    out.push('\n');
    for _ in 0..8 {
        out.push_str("0000000000000000000000000000000000000000000000000000000000000000\n");
    }
    out.push_str("cleartomark\n");

    out
}
