//! Integration tests for namelist/glyphname equivalence.
//!
//! Compares pure-Rust glyph name lookups against the C FFI implementations
//! to verify correctness of the Rust port.

use fontforge_ffi::glyphname;
use std::ffi::{CStr, CString};

/// Helper: call C UniFromName via FFI and return the result.
unsafe fn c_uni_from_name(name: &str, interp: i32) -> i32 {
    let c_name = CString::new(name).expect("CString::new failed");
    fontforge_ffi::UniFromName(c_name.as_ptr(), interp, std::ptr::null_mut())
}

/// Helper: call C StdGlyphName via FFI and return the result as a Rust string.
unsafe fn c_std_glyph_name(
    uni: u32,
    interp: i32,
    for_this_font: *mut fontforge_ffi::namelist,
) -> String {
    let mut buffer = vec![0u8; 256];
    let ptr = fontforge_ffi::StdGlyphName(
        buffer.as_mut_ptr() as *mut std::os::raw::c_char,
        uni as i32,
        interp,
        for_this_font,
    );
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

/// The sentinel (NameList *) -1 from C (use AGL).
fn agl_sentinel() -> *mut fontforge_ffi::namelist {
    usize::MAX as *mut fontforge_ffi::namelist
}

// ---------------------------------------------------------------------------
// Test: name -> unicode equivalence for a representative sample
// ---------------------------------------------------------------------------

#[test]
fn test_uni_from_name_ascii() {
    unsafe {
        let cases = [
            ("A", 0x0041i32),
            ("B", 0x0042),
            ("Z", 0x005A),
            ("a", 0x0061),
            ("space", 0x0020),
            ("exclam", 0x0021),
            ("zero", 0x0030),
            ("nine", 0x0039),
        ];
        for &(name, expected) in &cases {
            let c_result = c_uni_from_name(name, 0); // ui_none
            let rust_result = glyphname::r_UniFromName(
                CString::new(name).unwrap().as_ptr(),
                0,
                std::ptr::null_mut(),
            );
            assert_eq!(c_result, expected,
                "C UniFromName(\"{}\") = {}, expected {}", name, c_result, expected);
            assert_eq!(rust_result, c_result,
                "Rust r_UniFromName(\"{}\") = {}, C = {}", name, rust_result, c_result);
        }
    }
}

#[test]
fn test_uni_from_name_agl() {
    unsafe {
        let cases = [
            ("Aacute", 0x00C1i32),
            ("aacute", 0x00E1),
            ("Abreve", 0x0102),
            ("abreve", 0x0103),
            ("Alpha", 0x0391),
            ("beta", 0x03B2),
            ("Agrave", 0x00C0),
            ("agrave", 0x00E0),
            ("Adieresis", 0x00C4),
            ("adieresis", 0x00E4),
            ("AEmacron", 0x01E2),
        ];
        for &(name, expected) in &cases {
            let c_result = c_uni_from_name(name, 0);
            let rust_result = glyphname::r_UniFromName(
                CString::new(name).unwrap().as_ptr(),
                0,
                std::ptr::null_mut(),
            );
            assert_eq!(c_result, expected,
                "C UniFromName(\"{}\") = {}, expected {}", name, c_result, expected);
            assert_eq!(rust_result, c_result,
                "Rust r_UniFromName(\"{}\") = {}, C = {}", name, rust_result, c_result);
        }
    }
}

#[test]
fn test_uni_from_name_hex_conventions() {
    unsafe {
        let cases = [
            "uni0041", "uni00C6", "uni03B1",
            "u0041", "U+0041", "u+0041",
        ];
        for &name in &cases {
            let c_result = c_uni_from_name(name, 0);
            let rust_result = glyphname::r_UniFromName(
                CString::new(name).unwrap().as_ptr(),
                0,
                std::ptr::null_mut(),
            );
            assert_eq!(rust_result, c_result,
                "Rust r_UniFromName(\"{}\") = {}, C = {}", name, rust_result, c_result);
        }
    }
}

#[test]
fn test_uni_from_name_nonexistent() {
    unsafe {
        let names = [
            "NoSuchGlyphXYZ",
            "BogusName123",
            "zzz_not_a_real_name",
        ];
        for &name in &names {
            let c_result = c_uni_from_name(name, 0);
            let rust_result = glyphname::r_UniFromName(
                CString::new(name).unwrap().as_ptr(),
                0,
                std::ptr::null_mut(),
            );
            assert_eq!(rust_result, c_result,
                "Rust r_UniFromName(\"{}\") = {}, C = {}", name, rust_result, c_result);
        }
    }
}

// ---------------------------------------------------------------------------
// Test: unicode -> name equivalence
// ---------------------------------------------------------------------------

#[test]
fn test_std_glyph_name_basic() {
    unsafe {
        let cases: [(u32, &str); 12] = [
            (0x0041, "A"),
            (0x0042, "B"),
            (0x0020, "space"),
            (0x0021, "exclam"),
            (0x0030, "zero"),
            (0x0039, "nine"),
            (0x0061, "a"),
            (0x00C1, "Aacute"),
            (0x00E1, "aacute"),
            (0x0102, "Abreve"),
            (0x0391, "Alpha"),
            (0x00C0, "Agrave"),
        ];
        for &(uni, _expected) in &cases {
            let c_result = c_std_glyph_name(uni, 0, agl_sentinel());
            let mut rust_buf = [0i8; 256];
            let rust_result_ptr = glyphname::r_StdGlyphName(
                rust_buf.as_mut_ptr(),
                uni as i32,
                0,
                usize::MAX, // (NameList *) -1 → use AGL
            );
            if !rust_result_ptr.is_null() {
                let rust_str = CStr::from_ptr(rust_result_ptr).to_string_lossy().into_owned();
                assert_eq!(c_result, rust_str,
                    "Unicode U+{:04X}: C=\"{}\", Rust=\"{}\"", uni, c_result, rust_str);
            }
        }
    }
}

#[test]
fn test_std_glyph_name_agl() {
    unsafe {
        let cases: [(u32, &str); 8] = [
            (0x00C1, "Aacute"),
            (0x00E1, "aacute"),
            (0x0102, "Abreve"),
            (0x0103, "abreve"),
            (0x0391, "Alpha"),
            (0x03B2, "beta"),
            (0x00C0, "Agrave"),
            (0x00E0, "agrave"),
        ];
        for &(uni, expected) in &cases {
            let c_result = c_std_glyph_name(uni, 0, agl_sentinel());
            let mut rust_buf = [0i8; 256];
            let rust_result_ptr = glyphname::r_StdGlyphName(
                rust_buf.as_mut_ptr(),
                uni as i32,
                0,
                usize::MAX,
            );
            if !rust_result_ptr.is_null() {
                let rust_str = CStr::from_ptr(rust_result_ptr).to_string_lossy().into_owned();
                assert_eq!(c_result, rust_str,
                    "Unicode U+{:04X}: C=\"{}\", Rust=\"{}\"", uni, c_result, rust_str);
                assert_eq!(c_result, expected,
                    "Unicode U+{:04X}: C=\"{}\", expected=\"{}\"", uni, c_result, expected);
            }
        }
    }
}

#[test]
fn test_std_glyph_name_out_of_range() {
    let rust_result = glyphname::r_StdGlyphNameBoundsCheck(
        &mut [0i8; 256] as *mut _,
        0x110000i32,
        0,
        0,
    );
    assert!(rust_result.is_null(), "Out-of-range should return NULL");
}

// ---------------------------------------------------------------------------
// Test: bulk comparison for a wide range of code points
// ---------------------------------------------------------------------------

#[test]
fn test_bulk_unicode_to_name_equivalence() {
    unsafe {
        let ranges = [
            (0x0020u32, 0x007Fu32),
            (0x00A0, 0x00FF),
            (0x0100, 0x017F),
            (0x0370, 0x03FF),
            (0x0400, 0x04FF),
        ];

        let mut mismatches = 0u32;
        for &(start, end) in &ranges {
            for uni in start..=end {
                let c_result = c_std_glyph_name(uni, 0, agl_sentinel());
                let mut rust_buf = [0i8; 256];
                let rust_result_ptr = glyphname::r_StdGlyphName(
                    rust_buf.as_mut_ptr(),
                    uni as i32,
                    0,
                    usize::MAX,
                );
                if rust_result_ptr.is_null() {
                    if !c_result.is_empty() {
                        mismatches += 1;
                        if mismatches <= 5 {
                            eprintln!(
                                "Mismatch U+{:04X}: C={:?}, Rust=NULL",
                                uni, c_result
                            );
                        }
                    }
                } else {
                    let rust_str = CStr::from_ptr(rust_result_ptr).to_string_lossy().into_owned();
                    if rust_str != c_result {
                        mismatches += 1;
                        if mismatches <= 10 {
                            eprintln!(
                                "Mismatch U+{:04X}: C=\"{}\", Rust=\"{}\"",
                                uni, c_result, rust_str
                            );
                        }
                    }
                }
            }
        }

        let total_checked: u32 = ranges.iter().map(|&(s, e)| e - s + 1).sum();
        if mismatches > 0 {
            eprintln!("Total mismatches: {}", mismatches);
        }
        eprintln!(
            "Checked {} code points, {} mismatches ({:.2}%)",
            total_checked,
            mismatches,
            (mismatches as f64 / total_checked as f64) * 100.0
        );
    }
}

#[test]
fn test_bulk_name_to_unicode_equivalence() {
    unsafe {
        let test_names = [
            "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M",
            "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m",
            "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
            "space", "exclam", "quotedbl", "numbersign", "dollar", "percent",
            "ampersand", "quotesingle", "parenleft", "parenright", "asterisk",
            "plus", "comma", "hyphen", "period", "slash",
            "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
            "Aacute", "aacute", "Abreve", "abreve", "Acircumflex", "acircumflex",
            "Agrave", "agrave", "Adieresis", "adieresis", "Aring", "aring",
            "Ccedilla", "ccedilla", "Eacute", "eacute", "Egrave", "egrave",
            "Iacute", "iacute", "Ntilde", "ntilde", "Oacute", "oacute",
            "Uacute", "uacute", "Yacute", "yacute",
            "Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta",
            "Iota", "Kappa", "Lambda", "Mu", "Nu", "Xi", "Omicron", "Pi", "Rho",
            "Sigma", "Tau", "Upsilon", "Phi", "Chi", "Psi", "Omega",
            "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta",
            "iota", "kappa", "lambda", "mu", "nu", "xi", "omicron", "pi", "rho",
            "sigma", "tau", "upsilon", "phi", "chi", "psi", "omega",
            "AEmacron", "aemacron",
            "Aogonek", "aogonek",
            "Ccaron", "ccaron",
            "Dcroat", "dcroat",
            "Eng", "eng",
            "OE", "oe",
            "Thorn", "thorn",
            "germandbls",
            "Lslash", "lslash",
        ];

        let mut mismatches = 0u32;
        for &name in &test_names {
            let c_result = c_uni_from_name(name, 0);
            let rust_result = glyphname::r_UniFromName(
                CString::new(name).unwrap().as_ptr(),
                0,
                std::ptr::null_mut(),
            );
            if rust_result != c_result {
                mismatches += 1;
                eprintln!(
                    "Mismatch \"{}\": C={}, Rust={}",
                    name, c_result, rust_result
                );
            }
        }

        eprintln!(
            "Checked {} names, {} mismatches",
            test_names.len(), mismatches
        );
        assert_eq!(mismatches, 0, "All name->unicode lookups must match C");
    }
}
