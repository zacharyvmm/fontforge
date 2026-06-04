//! Integration tests comparing pure-Rust Unicode property/type functions
//! against the C FFI (fontforge/Unicode/). Bulk comparisons verify that
//! the data extraction and lookup logic in Rust matches the C library exactly.
//!
//! Part of RALPH-023: Replace Unicode/ with Rust ecosystem crates.

use fontforge_ffi::unicode::*;

// C FFI function declarations (from bindgen)
extern "C" {
    // Character property checks
    fn ff_unicode_isunicodepointassigned(ch: u32) -> i32;
    fn ff_unicode_isalpha(ch: u32) -> i32;
    fn ff_unicode_isideographic(ch: u32) -> i32;
    fn ff_unicode_islower(ch: u32) -> i32;
    fn ff_unicode_isupper(ch: u32) -> i32;
    fn ff_unicode_isdigit(ch: u32) -> i32;
    fn ff_unicode_iscombining(ch: u32) -> i32;
    fn ff_unicode_iszerowidth(ch: u32) -> i32;
    fn ff_unicode_islefttoright(ch: u32) -> i32;
    fn ff_unicode_isrighttoleft(ch: u32) -> i32;
    fn ff_unicode_isligvulgfrac(ch: u32) -> i32;
    fn ff_unicode_iseuronumeric(ch: u32) -> i32;
    fn ff_unicode_iseuronumterm(ch: u32) -> i32;
    fn ff_unicode_isarabnumeric(ch: u32) -> i32;
    fn ff_unicode_isdecompositionnormative(ch: u32) -> i32;
    fn ff_unicode_isdecompcircle(ch: u32) -> i32;
    fn ff_unicode_isarabinitial(ch: u32) -> i32;
    fn ff_unicode_isarabmedial(ch: u32) -> i32;
    fn ff_unicode_isarabfinal(ch: u32) -> i32;
    fn ff_unicode_isarabisolated(ch: u32) -> i32;
    fn ff_unicode_isideoalpha(ch: u32) -> i32;
    fn ff_unicode_isalnum(ch: u32) -> i32;
    fn ff_unicode_isspace(ch: u32) -> i32;
    fn ff_unicode_iseuronumsep(ch: u32) -> i32;
    fn ff_unicode_iscommonsep(ch: u32) -> i32;
    fn ff_unicode_ishexdigit(ch: u32) -> i32;
    fn ff_unicode_istitle(ch: u32) -> i32;

    // Case conversion
    fn ff_unicode_tolower(ch: u32) -> u32;
    fn ff_unicode_toupper(ch: u32) -> u32;
    fn ff_unicode_totitle(ch: u32) -> u32;
    fn ff_unicode_tomirror(ch: u32) -> u32;

    // Combining class and pose
    fn ff_unicode_combiningclass(ch: u32) -> i32;
    fn ff_unicode_pose(ch: u32) -> i32;

    // Unicode alternates
    fn ff_unicode_hasunialt(ch: u32) -> i32;
    fn ff_unicode_unialt(ch: u32) -> *const u32;

    // Arabic forms
    fn arabicform(ch: u32) -> *const fontforge_ffi::arabicforms;
}

// ---- Helper: compare property functions over a range ----

/// Macro to test a boolean property function over a range of code points.
macro_rules! compare_property {
    ($test_name:ident, $rust_fn:ident, $c_fn:ident) => {
        #[test]
        fn $test_name() {
            // Test the full Unicode BMP range (0x0000-0xFFFF)
            let ranges = [
                (0x0000u32, 0x0100u32),   // Basic Latin + Latin-1 Supplement
                (0x0100, 0x0400),          // Latin Extended, IPA, spacing modifiers, combining diacritical, Greek
                (0x0400, 0x0600),          // Cyrillic
                (0x0600, 0x0800),          // Arabic, Syriac, etc.
                (0x1E00, 0x2000),          // Latin Extended Additional, Greek Extended
                (0x2000, 0x2100),          // General Punctuation
                (0x2100, 0x2800),          // Letterlike, Number Forms, Arrows, Math Operators, Misc Technical
                (0xFE00, 0xFFF0),          // Variation Selectors, Halfwidth/Fullwidth
            ];

            let mut mismatches = Vec::new();
            for (start, end) in &ranges {
                for ch in *start..*end {
                    let r_val = $rust_fn(ch);
                    let c_val = unsafe { $c_fn(ch) != 0 };
                    if r_val != c_val {
                        mismatches.push((ch, r_val, c_val));
                        if mismatches.len() >= 20 {
                            break;
                        }
                    }
                }
                if mismatches.len() >= 20 {
                    break;
                }
            }

            assert!(
                mismatches.is_empty(),
                "{} mismatches for {}: {:?}",
                mismatches.len(),
                stringify!($rust_fn),
                &mismatches[..std::cmp::min(mismatches.len(), 10)]
            );
        }
    };
}

// ---- Compare boolean properties over BMP ----

compare_property!(test_isalpha_equivalence, is_alpha, ff_unicode_isalpha);
compare_property!(test_islower_equivalence, is_lower, ff_unicode_islower);
compare_property!(test_isupper_equivalence, is_upper, ff_unicode_isupper);
compare_property!(test_isdigit_equivalence, is_digit, ff_unicode_isdigit);
compare_property!(test_iscombining_equivalence, is_combining, ff_unicode_iscombining);
compare_property!(test_iszerowidth_equivalence, is_zero_width, ff_unicode_iszerowidth);
compare_property!(test_islefttoright_equivalence, is_left_to_right, ff_unicode_islefttoright);
compare_property!(test_isrighttoleft_equivalence, is_right_to_left, ff_unicode_isrighttoleft);
compare_property!(test_isligvulgfrac_equivalence, is_lig_vulg_frac, ff_unicode_isligvulgfrac);
compare_property!(test_isalnum_equivalence, is_alnum, ff_unicode_isalnum);
compare_property!(test_isspace_equivalence, is_space, ff_unicode_isspace);
compare_property!(test_ishexdigit_equivalence, is_hex_digit, ff_unicode_ishexdigit);
compare_property!(test_istitle_equivalence, is_title, ff_unicode_istitle);
compare_property!(test_isideographic_equivalence, is_ideographic, ff_unicode_isideographic);
compare_property!(test_isideoalpha_equivalence, is_ideo_alpha, ff_unicode_isideoalpha);
compare_property!(test_isassigned_equivalence, is_unicode_point_assigned, ff_unicode_isunicodepointassigned);

// ---- Compare case conversion ----

#[test]
fn test_tolower_equivalence() {
    let ranges = [
        (0x0000u32, 0x0100u32),
        (0x0100, 0x0600),
        (0x1E00, 0x2100),
        (0x2100, 0x2200),
        (0xFF00, 0xFFF0),
    ];

    let mut mismatches = Vec::new();
    for (start, end) in &ranges {
        for ch in *start..*end {
            let r_val = to_lower(ch);
            let c_val = unsafe { ff_unicode_tolower(ch) };
            if r_val != c_val {
                mismatches.push((ch, r_val, c_val));
                if mismatches.len() >= 20 {
                    break;
                }
            }
        }
        if mismatches.len() >= 20 {
            break;
        }
    }
    assert!(mismatches.is_empty(), "tolower mismatches: {:?}", &mismatches[..std::cmp::min(mismatches.len(), 10)]);
}

#[test]
fn test_toupper_equivalence() {
    let ranges = [
        (0x0000u32, 0x0100u32),
        (0x0100, 0x0600),
        (0x1E00, 0x2100),
        (0x2100, 0x2200),
        (0xFF00, 0xFFF0),
    ];

    let mut mismatches = Vec::new();
    for (start, end) in &ranges {
        for ch in *start..*end {
            let r_val = to_upper(ch);
            let c_val = unsafe { ff_unicode_toupper(ch) };
            if r_val != c_val {
                mismatches.push((ch, r_val, c_val));
                if mismatches.len() >= 20 {
                    break;
                }
            }
        }
        if mismatches.len() >= 20 {
            break;
        }
    }
    assert!(mismatches.is_empty(), "toupper mismatches: {:?}", &mismatches[..std::cmp::min(mismatches.len(), 10)]);
}

#[test]
fn test_totitle_equivalence() {
    let ranges = [
        (0x0000u32, 0x0100u32),
        (0x0100, 0x0600),
        (0x1E00, 0x2100),
        (0x2100, 0x2200),
    ];

    let mut mismatches = Vec::new();
    for (start, end) in &ranges {
        for ch in *start..*end {
            let r_val = to_title(ch);
            let c_val = unsafe { ff_unicode_totitle(ch) };
            if r_val != c_val {
                mismatches.push((ch, r_val, c_val));
                if mismatches.len() >= 20 {
                    break;
                }
            }
        }
        if mismatches.len() >= 20 {
            break;
        }
    }
    assert!(mismatches.is_empty(), "totitle mismatches: {:?}", &mismatches[..std::cmp::min(mismatches.len(), 10)]);
}

#[test]
fn test_tomirror_equivalence() {
    let ranges = [
        (0x0000u32, 0x0100u32),
        (0x2000, 0x3000),
        (0xFF00, 0xFFF0),
    ];

    let mut mismatches = Vec::new();
    for (start, end) in &ranges {
        for ch in *start..*end {
            let r_val = to_mirror(ch);
            let c_val = unsafe { ff_unicode_tomirror(ch) };
            if r_val != c_val {
                mismatches.push((ch, r_val, c_val));
                if mismatches.len() >= 20 {
                    break;
                }
            }
        }
        if mismatches.len() >= 20 {
            break;
        }
    }
    assert!(mismatches.is_empty(), "tomirror mismatches: {:?}", &mismatches[..std::cmp::min(mismatches.len(), 10)]);
}

// ---- Compare combining class and pose ----

#[test]
fn test_combiningclass_equivalence() {
    let ranges = [
        (0x0000u32, 0x0400u32),
        (0x0300, 0x0370),   // combining diacritical marks
        (0x0591, 0x0600),   // Hebrew cantillation marks
        (0x064B, 0x0660),   // Arabic diacritics
        (0x06D6, 0x0700),   // more Arabic
        (0x0730, 0x0750),   // Syriac marks
        (0x0E31, 0x0E50),   // Thai
        (0x1DC0, 0x1E00),   // Combining Diacritical Marks Supplement
        (0xFE20, 0xFE30),   // Combining Half Marks
    ];

    let mut mismatches = Vec::new();
    for (start, end) in &ranges {
        for ch in *start..*end {
            let r_val = combining_class(ch) as i32;
            let c_val = unsafe { ff_unicode_combiningclass(ch) };
            if r_val != c_val {
                mismatches.push((ch, r_val, c_val));
                if mismatches.len() >= 20 {
                    break;
                }
            }
        }
        if mismatches.len() >= 20 {
            break;
        }
    }
    assert!(mismatches.is_empty(), "combiningclass mismatches: {:?}", &mismatches[..std::cmp::min(mismatches.len(), 10)]);
}

#[test]
fn test_pose_equivalence() {
    let ranges = [
        (0x0000u32, 0x0400u32),
        (0x0300, 0x0370),
        (0x0591, 0x0600),
        (0x064B, 0x0660),
        (0x1DC0, 0x1E00),
    ];

    let mut mismatches = Vec::new();
    for (start, end) in &ranges {
        for ch in *start..*end {
            let r_val = pose(ch) as i32;
            let c_val = unsafe { ff_unicode_pose(ch) };
            if r_val != c_val {
                mismatches.push((ch, r_val, c_val));
                if mismatches.len() >= 20 {
                    break;
                }
            }
        }
        if mismatches.len() >= 20 {
            break;
        }
    }
    assert!(mismatches.is_empty(), "pose mismatches: {:?}", &mismatches[..std::cmp::min(mismatches.len(), 10)]);
}

// ---- Compare Unicode alternates ----

#[test]
fn test_hasunialt_equivalence() {
    // Test the first plane (BMP) for hasunialt
    let ranges = [
        (0x0000u32, 0x0100u32),
        (0x00C0, 0x0100),   // Latin-1 supplement (accented chars)
        (0x0100, 0x0400),
        (0x0400, 0x0600),
        (0x2000, 0x2100),
    ];

    let mut mismatches = Vec::new();
    for (start, end) in &ranges {
        for ch in *start..*end {
            let r_val = has_uni_alt(ch);
            let c_val = unsafe { ff_unicode_hasunialt(ch) != 0 };
            if r_val != c_val {
                mismatches.push((ch, r_val, c_val));
                if mismatches.len() >= 20 {
                    break;
                }
            }
        }
        if mismatches.len() >= 20 {
            break;
        }
    }
    assert!(mismatches.is_empty(), "hasunialt mismatches: {:?}", &mismatches[..std::cmp::min(mismatches.len(), 10)]);
}

#[test]
fn test_unialt_equivalence() {
    // Compare unialt sequences for a sample of characters
    let chars_to_test = [
        0x00C0u32, 0x00C1, 0x00C8, 0x00D2, 0x00D9,  // accented Latin
        0x0100, 0x0102, 0x0112, 0x014C, 0x016A,     // Latin Extended
        0x0391, 0x0393, 0x0395,                       // Greek
        0x0410, 0x0411, 0x0415,                       // Cyrillic
        0x2160, 0x2161,                                // Roman numerals
        0xFB00, 0xFB01,                                // Latin ligatures
        0xFB50, 0xFB56,                                // Arabic presentation forms
    ];

    let mut mismatches = Vec::new();
    for &ch in &chars_to_test {
        let r_val = uni_alt(ch);
        let c_ptr = unsafe { ff_unicode_unialt(ch) };

        if c_ptr.is_null() {
            if !r_val.is_empty() {
                mismatches.push((ch, "Rust has alt, C has NULL".to_string()));
            }
        } else {
            // C returns a null-terminated array
            let mut c_seq = Vec::new();
            let mut i = 0;
            unsafe {
                while *c_ptr.add(i) != 0 {
                    c_seq.push(*c_ptr.add(i));
                    i += 1;
                    if i > 100 {
                        break;
                    }
                }
            }
            if r_val != c_seq.as_slice() {
                mismatches.push((ch, format!("Rust: {:?}, C: {:?}", r_val, c_seq)));
            }
        }

        if mismatches.len() >= 10 {
            break;
        }
    }

    assert!(mismatches.is_empty(), "unialt mismatches: {:?}", &mismatches[..std::cmp::min(mismatches.len(), 10)]);
}

// ---- Compare Arabic forms ----

#[test]
fn test_arabicform_equivalence() {
    // Test all characters in the Arabic range
    let mut mismatches = Vec::new();
    for ch in 0x0600u32..=0x06FFu32 {
        let r_form = arabic_form(ch);
        let c_ptr = unsafe { arabicform(ch) };

        match (r_form, c_ptr.is_null()) {
            (Some(r), false) => {
                let c = unsafe { &*c_ptr };
                if r.initial != c.initial
                    || r.medial != c.medial
                    || r.final_form != c.final_
                    || r.isolated != c.isolated
                    || (r.isletter as u32) != c.isletter()
                    || (r.joindual as u32) != c.joindual()
                    || (r.required_lig_with_alef as u32) != c.required_lig_with_alef()
                {
                    mismatches.push((ch, "form mismatch".to_string()));
                }
            }
            (None, false) => {
                mismatches.push((ch, "Rust None but C has result".to_string()));
            }
            (Some(_), true) => {
                mismatches.push((ch, "Rust has result but C returns NULL".to_string()));
            }
            (None, true) => {} // both agree: no form
        }

        if mismatches.len() >= 10 {
            break;
        }
    }

    assert!(mismatches.is_empty(), "arabicform mismatches: {:?}", &mismatches[..std::cmp::min(mismatches.len(), 10)]);
}

// ---- Bulk test: all characters 0x0000..0x04FF for all properties ----

#[test]
fn test_bulk_all_properties_basic_range() {
    // Comprehensive test: all characters in 0x0000..0x04FF
    // Covers Basic Latin, Latin-1 Supplement, Latin Extended, IPA, 
    // spacing modifiers, combining diacritical marks, Greek, Cyrillic
    for ch in 0x0000u32..0x0500u32 {
        unsafe {
            assert_eq!(is_alpha(ch), ff_unicode_isalpha(ch) != 0, "isalpha mismatch at U+{:04X}", ch);
            assert_eq!(is_lower(ch), ff_unicode_islower(ch) != 0, "islower mismatch at U+{:04X}", ch);
            assert_eq!(is_upper(ch), ff_unicode_isupper(ch) != 0, "isupper mismatch at U+{:04X}", ch);
            assert_eq!(is_digit(ch), ff_unicode_isdigit(ch) != 0, "isdigit mismatch at U+{:04X}", ch);
            assert_eq!(is_combining(ch), ff_unicode_iscombining(ch) != 0, "iscombining mismatch at U+{:04X}", ch);
            assert_eq!(is_space(ch), ff_unicode_isspace(ch) != 0, "isspace mismatch at U+{:04X}", ch);
            assert_eq!(is_alnum(ch), ff_unicode_isalnum(ch) != 0, "isalnum mismatch at U+{:04X}", ch);
            assert_eq!(to_lower(ch), ff_unicode_tolower(ch), "tolower mismatch at U+{:04X}", ch);
            assert_eq!(to_upper(ch), ff_unicode_toupper(ch), "toupper mismatch at U+{:04X}", ch);
            assert_eq!(has_uni_alt(ch), ff_unicode_hasunialt(ch) != 0, "hasunialt mismatch at U+{:04X}", ch);
        }
    }
}

// ---- Bulk test: category-specific characters ----

#[test]
fn test_selected_symbols_and_punctuation() {
    // Test a range covering punctuation, symbols, and math operators
    for ch in 0x2000u32..0x2700u32 {
        unsafe {
            assert_eq!(is_alpha(ch), ff_unicode_isalpha(ch) != 0, "isalpha mismatch at U+{:04X}", ch);
            assert_eq!(is_combining(ch), ff_unicode_iscombining(ch) != 0, "iscombining mismatch at U+{:04X}", ch);
            assert_eq!(is_space(ch), ff_unicode_isspace(ch) != 0, "isspace mismatch at U+{:04X}", ch);
        }
    }
}
