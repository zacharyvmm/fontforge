//! Pure-Rust Unicode character property lookup.
//!
//! Replaces the functionality of FontForge's `gunicode` library
//! (`Unicode/utype.c`, `Unicode/unialt.c`, `Unicode/ArabicForms.c`).
//!
//! Data tables are auto-generated from the C sources by
//! `scripts/extract_unicode_data.py`.

pub mod utype_data;
pub mod unialt_data;
pub mod arabic_forms_data;

use utype_data::*;
use unialt_data::*;
use arabic_forms_data::*;

// ---- Two-level table indexing helpers ----

/// Look up casing data (upper/lower/title/mirror deltas) for a code point.
fn get_casing(ch: u32) -> &'static UtypeCasing {
    let mut index: usize = 0;
    if ch < UNICODE_MAX {
        let i1 = CASING_INDEX1[(ch >> CASING_SHIFT) as usize] as usize;
        index = CASING_INDEX2[(i1 << CASING_SHIFT as usize) + (ch as usize & ((1 << CASING_SHIFT) - 1))] as usize;
    }
    &CASING_DATA[index]
}

/// Look up type flags and pose for a code point.
fn get_type(ch: u32) -> &'static UtypeFlags {
    let mut index: usize = 0;
    if ch < UNICODE_MAX {
        let i1 = TYPE_INDEX1[(ch >> TYPE_SHIFT) as usize] as usize;
        index = TYPE_INDEX2[(i1 << TYPE_SHIFT as usize) + (ch as usize & ((1 << TYPE_SHIFT) - 1))] as usize;
    }
    &TYPE_DATA[index]
}

/// Look up unicode alternate (NFKD decomposition) for a code point.
/// Returns a pointer into the UNIALT_DATA array for the given character.
fn get_unialt_offset(ch: u32) -> usize {
    let mut index: usize = 0;
    if ch < UNICODE_MAX {
        let i1 = UNIALT_INDEX1[(ch >> UNIALT_SHIFT) as usize] as usize;
        index = UNIALT_INDEX2[(i1 << UNIALT_SHIFT as usize) + (ch as usize & ((1 << UNIALT_SHIFT) - 1))] as usize;
    }
    index
}

// ---- Character property checks ----

pub fn is_unicode_point_assigned(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISUNICODEPOINTASSIGNED != 0
}

pub fn is_alpha(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISALPHA != 0
}

pub fn is_ideographic(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISIDEOGRAPHIC != 0
}

pub fn is_left_to_right(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISLEFTTORIGHT != 0
}

pub fn is_right_to_left(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISRIGHTTOLEFT != 0
}

pub fn is_lower(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISLOWER != 0
}

pub fn is_upper(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISUPPER != 0
}

pub fn is_digit(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISDIGIT != 0
}

pub fn is_lig_vulg_frac(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISLIGVULGFRAC != 0
}

pub fn is_combining(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISCOMBINING != 0
}

pub fn is_zero_width(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISZEROWIDTH != 0
}

pub fn is_euro_numeric(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISEURONUMERIC != 0
}

pub fn is_euro_num_term(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISEURONUMTERM != 0
}

pub fn is_arab_numeric(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISARABNUMERIC != 0
}

pub fn is_decomposition_normative(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISDECOMPOSITIONNORMATIVE != 0
}

pub fn is_decomp_circle(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISDECOMPCIRCLE != 0
}

pub fn is_arab_initial(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISARABINITIAL != 0
}

pub fn is_arab_medial(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISARABMEDIAL != 0
}

pub fn is_arab_final(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISARABFINAL != 0
}

pub fn is_arab_isolated(ch: u32) -> bool {
    get_type(ch).flags & FF_UNICODE_ISARABISOLATED != 0
}

pub fn is_ideo_alpha(ch: u32) -> bool {
    get_type(ch).flags & (FF_UNICODE_ISALPHA | FF_UNICODE_ISIDEOGRAPHIC) != 0
}

pub fn is_alnum(ch: u32) -> bool {
    get_type(ch).flags & (FF_UNICODE_ISALPHA | FF_UNICODE_ISDIGIT) != 0
}

/// Check if character is a common number separator.
pub fn is_common_sep(ch: u32) -> bool {
    matches!(ch,
        0x002C | 0x002E | 0x002F | 0x003A | 0x00A0 | 0x060C |
        0x202F | 0x2044 | 0xFE50 | 0xFE52 | 0xFE55 | 0xFF0C |
        0xFF0E | 0xFF0F | 0xFF1A
    )
}

/// Check if character is a European number separator.
pub fn is_euro_num_sep(ch: u32) -> bool {
    matches!(ch,
        0x002B | 0x002D | 0x207A | 0x207B | 0x208A | 0x208B |
        0x2212 | 0xFB29 | 0xFE62 | 0xFE63 | 0xFF0B | 0xFF0D
    )
}

/// Check if character is a hex digit (0-9, A-F, a-f).
pub fn is_hex_digit(ch: u32) -> bool {
    matches!(ch,
        0x0030..=0x0039 | 0x0041..=0x0046 | 0x0061..=0x0066
    )
}

/// Check if character is a whitespace character.
pub fn is_space(ch: u32) -> bool {
    matches!(ch,
        0x0009 | 0x000A | 0x000B | 0x000C | 0x000D | 0x001C |
        0x001D | 0x001E | 0x001F | 0x0020 | 0x0085 | 0x00A0 |
        0x1680 | 0x2000 | 0x2001 | 0x2002 | 0x2003 | 0x2004 |
        0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200A |
        0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000
    )
}

/// Check if character is a titlecase character.
pub fn is_title(ch: u32) -> bool {
    matches!(ch,
        0x01C5 | 0x01C8 | 0x01CB | 0x01F2 |
        0x1F88 | 0x1F89 | 0x1F8A | 0x1F8B | 0x1F8C | 0x1F8D | 0x1F8E | 0x1F8F |
        0x1F98 | 0x1F99 | 0x1F9A | 0x1F9B | 0x1F9C | 0x1F9D | 0x1F9E | 0x1F9F |
        0x1FA8 | 0x1FA9 | 0x1FAA | 0x1FAB | 0x1FAC | 0x1FAD | 0x1FAE | 0x1FAF |
        0x1FBC | 0x1FCC | 0x1FFC
    )
}

// ---- Combining class and pose ----

/// Get the positioning information (pose) for a character.
/// Returns the pose flags without the combining class in the low 8 bits.
pub fn pose(ch: u32) -> u32 {
    get_type(ch).pose & !0xff
}

/// Get the canonical combining class for a character.
pub fn combining_class(ch: u32) -> u32 {
    get_type(ch).pose & 0xff
}

// ---- Case conversion ----

/// Convert a character to lowercase.
/// Returns the input character if no lowercase mapping exists.
pub fn to_lower(ch: u32) -> u32 {
    let rec = get_casing(ch);
    (ch as i32 + rec.lower) as u32
}

/// Convert a character to uppercase.
/// Returns the input character if no uppercase mapping exists.
pub fn to_upper(ch: u32) -> u32 {
    let rec = get_casing(ch);
    (ch as i32 + rec.upper) as u32
}

/// Convert a character to titlecase.
/// Returns the input character if no titlecase mapping exists.
pub fn to_title(ch: u32) -> u32 {
    let rec = get_casing(ch);
    (ch as i32 + rec.title) as u32
}

/// Get the mirror character for a code point.
/// Returns 0 if no mirror mapping exists.
pub fn to_mirror(ch: u32) -> u32 {
    let rec = get_casing(ch);
    if rec.mirror == 0 {
        0
    } else {
        (ch as i32 + rec.mirror) as u32
    }
}

// ---- Unicode alternates (NFKD + visual alternatives) ----

/// Check if a character has a Unicode alternate (NFKD decomposition or visual alternative).
pub fn has_uni_alt(ch: u32) -> bool {
    let offset = get_unialt_offset(ch);
    UNIALT_DATA[offset] != 0
}

/// Get the Unicode alternate sequence for a character.
/// Returns a slice of code points terminated by 0, or empty slice if none.
pub fn uni_alt(ch: u32) -> &'static [u32] {
    let start = get_unialt_offset(ch);
    if UNIALT_DATA[start] == 0 {
        return &[];
    }
    // Find the terminating 0
    let mut end = start;
    while end < UNIALT_DATA.len() && UNIALT_DATA[end] != 0 {
        end += 1;
    }
    &UNIALT_DATA[start..end]
}

// ---- Arabic forms ----

/// Get the Arabic presentation forms for a character.
/// Returns None if the character is not in the Arabic range (U+0600..U+06FF).
pub fn arabic_form(ch: u32) -> Option<&'static ArabicForm> {
    if ch >= 0x600 && ch <= 0x6FF {
        Some(&ARABIC_FORMS[(ch - 0x600) as usize])
    } else {
        None
    }
}

// ---- extern "C" wrappers (r_ prefix for coexistence with C library) ----

#[no_mangle]
pub extern "C" fn r_ff_unicode_isunicodepointassigned(ch: u32) -> i32 {
    is_unicode_point_assigned(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isalpha(ch: u32) -> i32 {
    is_alpha(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isideographic(ch: u32) -> i32 {
    is_ideographic(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_islower(ch: u32) -> i32 {
    is_lower(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isupper(ch: u32) -> i32 {
    is_upper(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isdigit(ch: u32) -> i32 {
    is_digit(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_iscombining(ch: u32) -> i32 {
    is_combining(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_iszerowidth(ch: u32) -> i32 {
    is_zero_width(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_islefttoright(ch: u32) -> i32 {
    is_left_to_right(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isrighttoleft(ch: u32) -> i32 {
    is_right_to_left(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isligvulgfrac(ch: u32) -> i32 {
    is_lig_vulg_frac(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_iseuronumeric(ch: u32) -> i32 {
    is_euro_numeric(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_iseuronumterm(ch: u32) -> i32 {
    is_euro_num_term(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isarabnumeric(ch: u32) -> i32 {
    is_arab_numeric(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isdecompositionnormative(ch: u32) -> i32 {
    is_decomposition_normative(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isdecompcircle(ch: u32) -> i32 {
    is_decomp_circle(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isarabinitial(ch: u32) -> i32 {
    is_arab_initial(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isarabmedial(ch: u32) -> i32 {
    is_arab_medial(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isarabfinal(ch: u32) -> i32 {
    is_arab_final(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isarabisolated(ch: u32) -> i32 {
    is_arab_isolated(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isideoalpha(ch: u32) -> i32 {
    is_ideo_alpha(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isalnum(ch: u32) -> i32 {
    is_alnum(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_isspace(ch: u32) -> i32 {
    is_space(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_iseuronumsep(ch: u32) -> i32 {
    is_euro_num_sep(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_iscommonsep(ch: u32) -> i32 {
    is_common_sep(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_ishexdigit(ch: u32) -> i32 {
    is_hex_digit(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_istitle(ch: u32) -> i32 {
    is_title(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_pose(ch: u32) -> i32 {
    pose(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_combiningclass(ch: u32) -> i32 {
    combining_class(ch) as i32
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_tolower(ch: u32) -> u32 {
    to_lower(ch)
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_toupper(ch: u32) -> u32 {
    to_upper(ch)
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_totitle(ch: u32) -> u32 {
    to_title(ch)
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_tomirror(ch: u32) -> u32 {
    to_mirror(ch)
}

#[no_mangle]
pub extern "C" fn r_ff_unicode_hasunialt(ch: u32) -> i32 {
    has_uni_alt(ch) as i32
}

/// Returns a pointer to the null-terminated unicode alternate sequence, or null.
#[no_mangle]
pub extern "C" fn r_ff_unicode_unialt(ch: u32) -> *const u32 {
    let start = get_unialt_offset(ch);
    if UNIALT_DATA[start] == 0 {
        std::ptr::null()
    } else {
        &UNIALT_DATA[start] as *const u32
    }
}

/// Returns a pointer to the ArabicForm struct, or null.
#[no_mangle]
pub extern "C" fn r_arabicform(ch: u32) -> *const ArabicForm {
    if ch >= 0x600 && ch <= 0x6FF {
        &ARABIC_FORMS[(ch - 0x600) as usize] as *const ArabicForm
    } else {
        std::ptr::null()
    }
}

// ---- Unit tests ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_alpha_basic() {
        assert!(is_alpha('A' as u32));
        assert!(is_alpha('z' as u32));
        assert!(!is_alpha('0' as u32));
        assert!(!is_alpha(' ' as u32));
    }

    #[test]
    fn test_is_lower_upper() {
        assert!(is_lower('a' as u32));
        assert!(!is_lower('A' as u32));
        assert!(is_upper('A' as u32));
        assert!(!is_upper('a' as u32));
    }

    #[test]
    fn test_is_digit() {
        assert!(is_digit('0' as u32));
        assert!(is_digit('9' as u32));
        assert!(!is_digit('A' as u32));
        assert!(is_digit(0x0660)); // Arabic-Indic digit zero IS flagged as ISDIGIT
    }

    #[test]
    fn test_case_conversion() {
        assert_eq!(to_lower('A' as u32), 'a' as u32);
        assert_eq!(to_upper('a' as u32), 'A' as u32);
        assert_eq!(to_lower('Z' as u32), 'z' as u32);
        assert_eq!(to_upper('z' as u32), 'Z' as u32);
        // Characters without case mapping return themselves
        assert_eq!(to_lower('0' as u32), '0' as u32);
        assert_eq!(to_upper('0' as u32), '0' as u32);
    }

    #[test]
    fn test_is_combining() {
        assert!(is_combining(0x0300)); // combining grave accent
        assert!(is_combining(0x0301)); // combining acute accent
        assert!(!is_combining('A' as u32));
        assert!(!is_combining('a' as u32));
    }

    #[test]
    fn test_is_space() {
        assert!(is_space(' ' as u32));
        assert!(is_space('\t' as u32));
        assert!(is_space('\n' as u32));
        assert!(!is_space('A' as u32));
    }

    #[test]
    fn test_is_hex_digit() {
        assert!(is_hex_digit('0' as u32));
        assert!(is_hex_digit('9' as u32));
        assert!(is_hex_digit('A' as u32));
        assert!(is_hex_digit('F' as u32));
        assert!(is_hex_digit('a' as u32));
        assert!(is_hex_digit('f' as u32));
        assert!(!is_hex_digit('G' as u32));
        assert!(!is_hex_digit('g' as u32));
    }

    #[test]
    fn test_combining_class() {
        assert_eq!(combining_class(0x0300), 230); // grave accent - CCC 230
        assert_eq!(combining_class(0x0000), 0);
    }

    #[test]
    fn test_uni_alt_basic() {
        // U+00C0 (À) decomposes to A + combining grave
        assert!(has_uni_alt(0x00C0));
        let alt = uni_alt(0x00C0);
        assert_eq!(alt, &[0x0041, 0x0300]); // A + combining grave
    }

    #[test]
    fn test_arabic_form_basic() {
        // U+0627 (alef) is in Arabic range
        let form = arabic_form(0x0627).expect("alef should have Arabic form");
        assert_eq!(form.isolated, 0xFE8D);
        assert_eq!(form.final_form, 0xFE8E);
        assert!(form.isletter);
        assert!(!form.joindual);
    }

    #[test]
    fn test_arabic_form_out_of_range() {
        assert!(arabic_form('A' as u32).is_none());
        assert!(arabic_form(0x0700).is_none());
    }

    #[test]
    fn test_is_assigned() {
        assert!(is_unicode_point_assigned('A' as u32));
        assert!(is_unicode_point_assigned(0x0000)); // assigned? let's check
    }

    #[test]
    fn test_edge_cases() {
        // Test boundary: UNICODE_MAX - 1
        assert!(!is_alpha(0x110000)); // beyond UNICODE_MAX
        assert!(!is_alpha(0x10FFFF)); // last valid code point
    }

    #[test]
    fn test_is_zero_width() {
        assert!(is_zero_width(0x200B)); // ZERO WIDTH SPACE
        assert!(!is_zero_width('A' as u32));
    }

    #[test]
    fn test_to_mirror() {
        // U+0028 '(' mirrors to ')'
        let m = to_mirror('(' as u32);
        assert_ne!(m, 0);
        let m2 = to_mirror('A' as u32);
        assert_eq!(m2, 0); // no mirror
    }

    #[test]
    fn test_tolower_toupper_extended() {
        // Test with Latin Extended characters
        // U+00C0 (À) -> U+00E0 (à)
        assert_eq!(to_lower(0x00C0), 0x00E0);
        assert_eq!(to_upper(0x00E0), 0x00C0);
    }
}
