//! Glyph name <-> Unicode mapping (replacement for fontforge/namelist.c).
//!
//! Provides pure-Rust implementations of:
//! - `UniFromName`  — resolve a glyph name to a Unicode code point
//! - `StdGlyphName` — resolve a Unicode code point to a standard glyph name
//! - `StdGlyphNameBoundsCheck` — same, with range validation
//!
//! Data is auto-generated from the original C source by
//! `scripts/extract_namelist.py` into `glyphname_data.rs`.

mod glyphname_data;
use glyphname_data::{
    GLYPH_NAME_TO_UNICODE, NAMELIST_CHAIN_ORDER, NAMELIST_METAS, NamelistMeta,
};

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ---------------------------------------------------------------------------
// Unicode interpretation modes (must match enum uni_interp in splinefont.h)
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum UniInterp {
    Unset = -1,
    None = 0,
    Adobe = 1,
    Greek = 2,
    Japanese = 3,
    TradChinese = 4,
    SimpChinese = 5,
    Korean = 6,
    Ams = 7,
}

impl UniInterp {
    fn from_i32(v: i32) -> Self {
        match v {
            -1 => UniInterp::Unset,
            0 => UniInterp::None,
            1 => UniInterp::Adobe,
            2 => UniInterp::Greek,
            3 => UniInterp::Japanese,
            4 => UniInterp::TradChinese,
            5 => UniInterp::SimpChinese,
            6 => UniInterp::Korean,
            7 => UniInterp::Ams,
            _ => UniInterp::None,
        }
    }
}

// ---------------------------------------------------------------------------
// Namelist index (for identifying which NameList to use)
// ---------------------------------------------------------------------------

/// Returns the index of a namelist in the static data by name.
pub fn namelist_index(name: &str) -> Option<usize> {
    NAMELIST_CHAIN_ORDER.iter().position(|&n| n == name)
}

/// Returns the index of the default namelist for new fonts (AGL For New Fonts).
pub fn default_namelist_index() -> usize {
    namelist_index("agl_nf").unwrap_or(1)
}

// ---------------------------------------------------------------------------
// Pure-Rust lookup functions
// ---------------------------------------------------------------------------

/// Look up a glyph name and return its Unicode code point.
///
/// This follows the same resolution order as `UniFromName` in namelist.c:
/// 1. "uniXXXX" / "uXXXX" / "U+XXXX" hex conventions
/// 2. Single-character names (ASCII)
/// 3. Built-in glyph name tables (psaltnames + AGL name lists)
///
/// PUA recognition can be disabled with `recognize_pua = false`.
pub fn name_to_unicode(name: &str, interp: UniInterp, recognize_pua: bool) -> Option<u32> {
    let mut result: Option<u32> = None;
    let mut _recognize_pua = recognize_pua;

    // "uniXXXX" convention: "uni" + exactly 4 hex digits (total 7 chars)
    // "uniXXXXXXXX" (8 hex digits = total 11 chars) means a ligature → reject
    if name.starts_with("uni") && name.len() == 7 {
        if let Ok(uni) = u32::from_str_radix(&name[3..], 16) {
            result = Some(uni);
            _recognize_pua = true;
        }
    }

    // "U+XXXX" (6) or "U+XXXXX" (7) convention (Unifont)
    if result.is_none()
        && (name.starts_with("U+") || name.starts_with("u+"))
        && (name.len() == 6 || name.len() == 7)
    {
        if let Ok(uni) = u32::from_str_radix(&name[2..], 16) {
            result = Some(uni);
            _recognize_pua = true;
        }
    }

    // "uXXXX" (5) or "uXXXXX" (6) convention
    if result.is_none()
        && name.starts_with('u')
        && (name.len() == 5 || name.len() == 6)
        && name.as_bytes().get(1).map_or(false, |&b| b != b'+')
    {
        if let Ok(uni) = u32::from_str_radix(&name[1..], 16) {
            result = Some(uni);
            _recognize_pua = true;
        }
    }

    // Single ASCII character
    if result.is_none() && name.len() == 1 {
        result = Some(name.as_bytes()[0] as u32);
    }

    // Look up in the glyph name table
    if result.is_none() {
        result = lookup_glyph_name(name);
    }

    // Filter PUA if recognition is off
    result.filter(|&uni| {
        _recognize_pua || !(0xE000..=0xF8FF).contains(&uni)
    })
}

/// Look up a glyph name in the built-in tables using binary search.
fn lookup_glyph_name(name: &str) -> Option<u32> {
    GLYPH_NAME_TO_UNICODE
        .binary_search_by_key(&name, |e| e.name)
        .ok()
        .map(|idx| GLYPH_NAME_TO_UNICODE[idx].unicode)
}

/// Look up a Unicode code point and return its standard glyph name from the
/// specified namelist chain.
///
/// The `namelist_index` parameter selects which name list to use:
/// - 0: agl (Adobe Glyph List)
/// - 1: agl_nf (AGL For New Fonts)
/// - 2: agl_sans (AGL without afii)
/// - 3: adobepua (AGL with PUA)
/// - 4: greeksc (Greek small caps)
/// - 5: tex (TeX Names)
/// - 6: ams (AMS Names)
///
/// Walks the `basedon` chain to find the best name for the code point.
pub fn unicode_to_name(uni: u32, namelist_idx: usize) -> Option<&'static str> {
    if uni > 0x10FFFF {
        return None;
    }

    // Walk the namelist itself and its basedon chain
    let mut current_idx = namelist_idx;
    loop {
        if current_idx >= NAMELIST_METAS.len() {
            break;
        }
        let meta = &NAMELIST_METAS[current_idx];
        // Binary search in this namelist's entries
        if let Ok(pos) = meta.entries.binary_search_by_key(&uni, |&(u, _)| u) {
            return Some(meta.entries[pos].1);
        }
        // Follow basedon chain
        match meta.basedon {
            Some(parent_name) => {
                if let Some(parent_idx) = namelist_index(parent_name) {
                    current_idx = parent_idx;
                } else {
                    break;
                }
            }
            None => break,
        }
    }
    None
}

/// Generate a "uniXXXX" or "uXXXXX" name for a code point not in any namelist.
pub fn fallback_uni_name(uni: u32) -> String {
    if uni >= 0x10000 {
        format!("u{:04X}", uni)
    } else {
        format!("uni{:04X}", uni)
    }
}

/// Full `StdGlyphName` equivalent: returns a glyph name for a Unicode code point
/// using the specified namelist. Writes to the provided buffer and returns a
/// pointer to it.  If the name doesn't fit in the buffer, truncates.
pub fn std_glyph_name(buffer: &mut [u8], uni: u32, _interp: UniInterp, namelist_idx: usize) -> &[u8] {
    // Check control characters
    if (uni < 0x0020) || (0x007F..0x00A0).contains(&uni) {
        // Control characters: use uniXXXX format
        let name = fallback_uni_name(uni);
        let bytes = name.as_bytes();
        let len = bytes.len().min(buffer.len() - 1);
        buffer[..len].copy_from_slice(&bytes[..len]);
        buffer[len] = 0;
        return &buffer[..len + 1];
    }

    if let Some(name) = unicode_to_name(uni, namelist_idx) {
        let bytes = name.as_bytes();
        let len = bytes.len().min(buffer.len() - 1);
        buffer[..len].copy_from_slice(&bytes[..len]);
        buffer[len] = 0;
        &buffer[..len + 1]
    } else {
        let name = fallback_uni_name(uni);
        let bytes = name.as_bytes();
        let len = bytes.len().min(buffer.len() - 1);
        buffer[..len].copy_from_slice(&bytes[..len]);
        buffer[len] = 0;
        &buffer[..len + 1]
    }
}

// ---------------------------------------------------------------------------
// C-compatible extern functions (FFI bridge)
// ---------------------------------------------------------------------------

/// C-compatible `UniFromName`: resolve a glyph name to a Unicode code point.
///
/// Returns -1 if the name cannot be resolved.
#[no_mangle]
pub extern "C" fn r_UniFromName(
    name: *const c_char,
    interp: i32,
    _encname: *const std::ffi::c_void, // Encoding* (unused for now)
) -> i32 {
    if name.is_null() {
        return -1;
    }
    let c_str = unsafe { CStr::from_ptr(name) };
    let name_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match name_to_unicode(name_str, UniInterp::from_i32(interp), true) {
        Some(uni) => uni as i32,
        None => -1,
    }
}

/// C-compatible `StdGlyphName`: resolve a Unicode code point to a glyph name.
///
/// Writes the result into `buffer` (which must be at least 256 bytes) and
/// returns a pointer to `buffer`. The `for_this_font` pointer is interpreted
/// as a namelist index if it is small (< 10), or as the default namelist.
#[no_mangle]
pub extern "C" fn r_StdGlyphName(
    buffer: *mut c_char,
    uni: i32,
    interp: i32,
    for_this_font: usize, // Interpreted as namelist index or pointer
) -> *const c_char {
    if buffer.is_null() || uni < 0 || uni > 0x10FFFF {
        return std::ptr::null();
    }

    let namelist_idx = if for_this_font < NAMELIST_METAS.len() {
        for_this_font
    } else if for_this_font == usize::MAX {
        // (NameList *) -1 means use AGL (index 0)
        0
    } else {
        default_namelist_index()
    };

    let uni = uni as u32;
    let buf = unsafe { std::slice::from_raw_parts_mut(buffer as *mut u8, 256) };

    let result = std_glyph_name(buf, uni, UniInterp::from_i32(interp), namelist_idx);

    // Return pointer to start of buffer (result is a slice from the buffer)
    buffer
}

/// C-compatible `StdGlyphNameBoundsCheck`: same as StdGlyphName but returns
/// NULL for out-of-range code points instead of generating a uniXXXX name.
#[no_mangle]
pub extern "C" fn r_StdGlyphNameBoundsCheck(
    buffer: *mut c_char,
    uni: i32,
    interp: i32,
    for_this_font: usize,
) -> *const c_char {
    if uni < 0 || uni > 0x10FFFF {
        return std::ptr::null();
    }
    r_StdGlyphName(buffer, uni, interp, for_this_font)
}

/// C-compatible `DefaultNameListForNewFonts`: returns the index of the default
/// namelist for new fonts (agl_nf).
#[no_mangle]
pub extern "C" fn r_DefaultNameListForNewFonts() -> usize {
    default_namelist_index()
}

/// C-compatible `AllGlyphNames`: return all possible glyph names for a unicode.
///
/// Returns a null-terminated array of C strings (or NULL).
/// The caller must free each string and the array.
#[no_mangle]
pub extern "C" fn r_AllGlyphNames(
    uni: i32,
    for_this_font: usize,
    _sc: *const std::ffi::c_void, // SplineChar* (unused for now)
) -> *mut *mut c_char {
    if uni < 0 || uni > 0x10FFFF {
        return std::ptr::null_mut();
    }

    let uni = uni as u32;
    let namelist_idx = if for_this_font < NAMELIST_METAS.len() {
        for_this_font
    } else {
        default_namelist_index()
    };

    // Collect all possible names from the namelist chain
    let mut names: Vec<String> = Vec::new();

    // Check the primary namelist and its basedon chain
    let mut current_idx = namelist_idx;
    loop {
        if current_idx >= NAMELIST_METAS.len() {
            break;
        }
        let meta = &NAMELIST_METAS[current_idx];
        if let Ok(pos) = meta.entries.binary_search_by_key(&uni, |&(u, _)| u) {
            names.push(meta.entries[pos].1.to_string());
        }
        match meta.basedon {
            Some(parent_name) => {
                if let Some(parent_idx) = namelist_index(parent_name) {
                    current_idx = parent_idx;
                } else {
                    break;
                }
            }
            None => break,
        }
    }

    // Also include fallback uniXXXX name
    names.push(fallback_uni_name(uni));

    if names.is_empty() {
        return std::ptr::null_mut();
    }

    // Allocate array of C strings
    let count = names.len();
    let array_size = (count + 1) * std::mem::size_of::<*mut c_char>();
    let ptr = unsafe {
        let layout = std::alloc::Layout::from_size_align(array_size, std::mem::align_of::<*mut c_char>())
            .unwrap();
        std::alloc::alloc(layout) as *mut *mut c_char
    };

    if ptr.is_null() {
        return std::ptr::null_mut();
    }

    for (i, name) in names.iter().enumerate() {
        let c_str = CString::new(name.as_str()).unwrap();
        unsafe {
            *ptr.add(i) = c_str.into_raw();
        }
    }
    unsafe {
        *ptr.add(count) = std::ptr::null_mut();
    }

    ptr
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_to_unicode_ascii() {
        assert_eq!(name_to_unicode("A", UniInterp::None, true), Some(0x0041));
        assert_eq!(name_to_unicode("space", UniInterp::None, true), Some(0x0020));
        assert_eq!(name_to_unicode("exclam", UniInterp::None, true), Some(0x0021));
    }

    #[test]
    fn test_name_to_unicode_uni_hex() {
        assert_eq!(name_to_unicode("uni0041", UniInterp::None, true), Some(0x0041));
        assert_eq!(name_to_unicode("uni00C6", UniInterp::None, true), Some(0x00C6));
        assert_eq!(name_to_unicode("u0041", UniInterp::None, true), Some(0x0041));
        assert_eq!(name_to_unicode("U+0041", UniInterp::None, true), Some(0x0041));
    }

    #[test]
    fn test_name_to_unicode_agl() {
        assert_eq!(name_to_unicode("Aacute", UniInterp::None, true), Some(0x00C1));
        assert_eq!(name_to_unicode("Alpha", UniInterp::None, true), Some(0x0391));
        assert_eq!(name_to_unicode("zero", UniInterp::None, true), Some(0x0030));
    }

    #[test]
    fn test_name_to_unicode_nonexistent() {
        assert_eq!(name_to_unicode("NoSuchGlyph", UniInterp::None, true), None);
    }

    #[test]
    fn test_unicode_to_name() {
        let idx = namelist_index("agl").unwrap();
        assert_eq!(unicode_to_name(0x0041, idx), Some("A"));
        assert_eq!(unicode_to_name(0x00C1, idx), Some("Aacute"));
        assert_eq!(unicode_to_name(0x0020, idx), Some("space"));
    }

    #[test]
    fn test_unicode_to_name_chain() {
        // agl_nf is the default for new fonts
        let idx = namelist_index("agl_nf").unwrap();
        // A should be found in agl_nf
        assert_eq!(unicode_to_name(0x0041, idx), Some("A"));
    }

    #[test]
    fn test_unicode_to_name_out_of_range() {
        let idx = namelist_index("agl").unwrap();
        assert_eq!(unicode_to_name(0x110000, idx), None);
    }

    #[test]
    fn test_fallback_uni_name() {
        assert_eq!(fallback_uni_name(0x0041), "uni0041");
        assert_eq!(fallback_uni_name(0x10000), "u10000");
    }

    #[test]
    fn test_std_glyph_name_buffer() {
        let mut buf = [0u8; 256];
        let idx = namelist_index("agl").unwrap();
        let result_len = {
            let result = std_glyph_name(&mut buf, 0x0041, UniInterp::None, idx);
            result.len()
        };
        assert_eq!(&buf[..result_len - 1], b"A");
    }
}
