#!/usr/bin/env python3
"""Extract Unicode data from FontForge C source files and generate Rust source.

Parses:
- Unicode/utype.c: character properties (flags, pose/combining class, case mappings)
- Unicode/unialt.c: NFKD decomposition + visual alternatives
- Unicode/ArabicForms.c: Arabic form shaping

Generates:
- rust-bindings/src/unicode/utype_data.rs
- rust-bindings/src/unicode/unialt_data.rs
- rust-bindings/src/unicode/arabic_forms_data.rs
"""

import re
import os
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)


def parse_c_array(filepath, array_name, lines=None):
    """Parse a C array definition like:
       static const TYPE NAME[] = {
           val1, val2, ..., valN
       };
    Returns list of values as ints.
    """
    if lines is None:
        with open(filepath, 'r') as f:
            lines = f.readlines()

    # Find array start
    start = None
    pattern = re.compile(rf'\b{re.escape(array_name)}\[\]\s*=\s*{{')
    for i, line in enumerate(lines):
        if pattern.search(line):
            start = i + 1
            break

    if start is None:
        raise ValueError(f"Could not find array '{array_name}' in {filepath}")

    # Collect values until closing }
    values = []
    for i in range(start, len(lines)):
        line = lines[i].strip()
        if line == '};' or line == '};':
            break
        # Extract hex and decimal integers
        for token in re.finditer(r'-?0x[0-9a-fA-F]+|-?\d+', line):
            val = token.group(0)
            if val.startswith('0x') or val.startswith('0X') or val.startswith('-0x') or val.startswith('-0X'):
                values.append(int(val, 16 if 'x' in val.lower() else 10))
            elif val.startswith('-'):
                values.append(-int(val[1:]))
            else:
                values.append(int(val))
    return values


def parse_c_struct_array(filepath, array_name, num_fields, lines=None):
    """Parse a C struct array like:
       static const struct NAME[] = {
           {f1, f2, ..., fN},
           ...
       };
    Returns list of tuples.
    """
    if lines is None:
        with open(filepath, 'r') as f:
            lines = f.readlines()

    # Find array start
    pattern = re.compile(rf'{re.escape(array_name)}\[\]\s*=\s*{{')
    start = None
    for i, line in enumerate(lines):
        if pattern.search(line):
            start = i + 1
            break

    if start is None:
        raise ValueError(f"Could not find struct array '{array_name}' in {filepath}")

    records = []
    # Collect until };  (closing of the top-level array)
    brace_depth = 0
    current = []
    for i in range(start, len(lines)):
        line = lines[i].strip()
        # Extract integers
        for token in re.finditer(r'-?0x[0-9a-fA-F]+|-?\d+', line):
            val = token.group(0)
            if val.startswith('0x') or val.startswith('0X') or val.startswith('-0x') or val.startswith('-0X'):
                current.append(int(val, 16 if 'x' in val.lower() else 10))
            elif val.startswith('-'):
                current.append(-int(val[1:]))
            else:
                current.append(int(val))
        brace_depth += line.count('{') - line.count('}')
        if brace_depth < 0:
            break
        if len(current) >= num_fields:
            records.append(tuple(current[:num_fields]))
            current = []
    return records


def extract_utype():
    """Extract data from Unicode/utype.c"""
    utype_path = os.path.join(PROJECT_ROOT, "Unicode", "utype.c")
    with open(utype_path, 'r') as f:
        lines = f.readlines()

    # Extract casing_data (struct utypecasing: upper, lower, title, mirror)
    casing_data = parse_c_struct_array(utype_path, "casing_data", 4)

    # Extract type_data (struct utypeflags: flags, pose)
    type_data = parse_c_struct_array(utype_path, "type_data", 2)

    # Extract index arrays
    casing_index1 = parse_c_array(utype_path, "casing_index1")
    casing_index2 = parse_c_array(utype_path, "casing_index2")
    type_index1 = parse_c_array(utype_path, "type_index1")
    type_index2 = parse_c_array(utype_path, "type_index2")

    return {
        "casing_data": casing_data,
        "type_data": type_data,
        "casing_index1": casing_index1,
        "casing_index2": casing_index2,
        "type_index1": type_index1,
        "type_index2": type_index2,
    }


def extract_unialt():
    """Extract data from Unicode/unialt.c"""
    unialt_path = os.path.join(PROJECT_ROOT, "Unicode", "unialt.c")
    with open(unialt_path, 'r') as f:
        lines = f.readlines()

    unialt_data = parse_c_array(unialt_path, "unialt_data")
    unialt_index1 = parse_c_array(unialt_path, "unialt_index1")
    unialt_index2 = parse_c_array(unialt_path, "unialt_index2")

    return {
        "unialt_data": unialt_data,
        "unialt_index1": unialt_index1,
        "unialt_index2": unialt_index2,
    }


def extract_arabic_forms():
    """Extract data from Unicode/ArabicForms.c"""
    arabic_path = os.path.join(PROJECT_ROOT, "Unicode", "ArabicForms.c")
    with open(arabic_path, 'r') as f:
        lines = f.readlines()

    arabic_forms = parse_c_struct_array(arabic_path, "arabic_forms", 7)
    return {"arabic_forms": arabic_forms}


def format_rust_array(name, values, ty="u32", indent=4):
    """Format a Rust array literal."""
    prefix = " " * indent
    lines_out = [f"{prefix}pub(crate) static {name}: [{ty}; {len(values)}] = ["]
    # Group in rows of 16
    for i in range(0, len(values), 16):
        chunk = values[i:i+16]
        nums = ", ".join(f"0x{v:04X}" if isinstance(v, int) else str(v) for v in chunk)
        if i + 16 < len(values):
            lines_out.append(f"{prefix}    {nums},")
        else:
            lines_out.append(f"{prefix}    {nums}")
    lines_out.append(f"{prefix}];")
    return "\n".join(lines_out)


def format_rust_struct_array(name, records, struct_name, field_names, bool_fields=None, indent=4):
    """Format a Rust struct array literal.
    bool_fields: set of field names that are bool (0->false, 1->true).
    """
    if bool_fields is None:
        bool_fields = set()
    prefix = " " * indent
    lines_out = [f"{prefix}pub(crate) static {name}: [{struct_name}; {len(records)}] = ["]
    for rec in records:
        parts = []
        for fn, fv in zip(field_names, rec):
            if fn in bool_fields:
                parts.append(f"{fn}: {'true' if fv else 'false'}")
            else:
                parts.append(f"{fn}: {fv}")
        fields = ", ".join(parts)
        lines_out.append(f"{prefix}    {struct_name} {{ {fields} }},")
    lines_out.append(f"{prefix}];")
    return "\n".join(lines_out)


def generate_utype_data(data):
    """Generate rust-bindings/src/unicode/utype_data.rs"""
    import textwrap

    output = textwrap.dedent("""\
    // Auto-generated by scripts/extract_unicode_data.py
    // DO NOT EDIT MANUALLY
    // Source: Unicode/utype.c (generated by makeutype.py, Unicode 17.0.0)
    //
    // Character property flags, combining class / pose, and case mapping data
    // for Unicode code points 0x0000..0x10FFFF.

    /// Unicode version this data was generated from.
    pub const UNICODE_VERSION: &str = "17.0.0";
    pub const UNICODE_MAX: u32 = 0x110000;

    // Shift constants for two-level table indexing
    pub const CASING_SHIFT: u32 = 8;
    pub const TYPE_SHIFT: u32 = 8;

    // ----- Flag bit definitions -----
    pub const FF_UNICODE_ISUNICODEPOINTASSIGNED: u32   = 0x1;
    pub const FF_UNICODE_ISALPHA: u32                  = 0x2;
    pub const FF_UNICODE_ISIDEOGRAPHIC: u32            = 0x4;
    pub const FF_UNICODE_ISLEFTTORIGHT: u32            = 0x10;
    pub const FF_UNICODE_ISRIGHTTOLEFT: u32            = 0x20;
    pub const FF_UNICODE_ISLOWER: u32                  = 0x40;
    pub const FF_UNICODE_ISUPPER: u32                  = 0x80;
    pub const FF_UNICODE_ISDIGIT: u32                  = 0x100;
    pub const FF_UNICODE_ISLIGVULGFRAC: u32            = 0x200;
    pub const FF_UNICODE_ISCOMBINING: u32              = 0x400;
    pub const FF_UNICODE_ISZEROWIDTH: u32              = 0x800;
    pub const FF_UNICODE_ISEURONUMERIC: u32            = 0x1000;
    pub const FF_UNICODE_ISEURONUMTERM: u32            = 0x2000;
    pub const FF_UNICODE_ISARABNUMERIC: u32            = 0x4000;
    pub const FF_UNICODE_ISDECOMPOSITIONNORMATIVE: u32 = 0x8000;
    pub const FF_UNICODE_ISDECOMPCIRCLE: u32           = 0x10000;
    pub const FF_UNICODE_ISARABINITIAL: u32            = 0x20000;
    pub const FF_UNICODE_ISARABMEDIAL: u32             = 0x40000;
    pub const FF_UNICODE_ISARABFINAL: u32              = 0x80000;
    pub const FF_UNICODE_ISARABISOLATED: u32           = 0x100000;

    // ----- Pose flags -----
    pub const FF_UNICODE_NOPOSDATAGIVEN: u32 = 0xFFFF_FFFF;
    pub const FF_UNICODE_ABOVE: u32          = 0x100;
    pub const FF_UNICODE_BELOW: u32          = 0x200;
    pub const FF_UNICODE_OVERSTRIKE: u32     = 0x400;
    pub const FF_UNICODE_LEFT: u32           = 0x800;
    pub const FF_UNICODE_RIGHT: u32          = 0x1000;
    pub const FF_UNICODE_JOINS2: u32         = 0x2000;
    pub const FF_UNICODE_CENTERLEFT: u32     = 0x4000;
    pub const FF_UNICODE_CENTERRIGHT: u32    = 0x8000;
    pub const FF_UNICODE_CENTEREDOUTSIDE: u32 = 0x10000;
    pub const FF_UNICODE_OUTSIDE: u32        = 0x20000;
    pub const FF_UNICODE_RIGHTEDGE: u32      = 0x40000;
    pub const FF_UNICODE_LEFTEDGE: u32       = 0x80000;
    pub const FF_UNICODE_TOUCHING: u32       = 0x100000;

    // ----- Data structures -----
    #[derive(Clone, Copy)]
    pub(crate) struct UtypeCasing {
        /// Delta from the character to get upper case
        pub upper: i32,
        /// Delta from the character to get lower case
        pub lower: i32,
        /// Delta from the character to get title case
        pub title: i32,
        /// Delta from the character to get mirror character (0 if no mirror)
        pub mirror: i32,
    }

    #[derive(Clone, Copy)]
    pub(crate) struct UtypeFlags {
        /// One or more of the FF_UNICODE_* flag bits
        pub flags: u32,
        /// Combined pose (high bits) and combining class (low 8 bits)
        pub pose: u32,
    }
    """)

    # casing_data
    output += "\n"
    output += format_rust_struct_array("CASING_DATA", data["casing_data"],
                                       "UtypeCasing", ["upper", "lower", "title", "mirror"])
    output += "\n\n"
    # type_data
    output += format_rust_struct_array("TYPE_DATA", data["type_data"],
                                       "UtypeFlags", ["flags", "pose"])
    output += "\n\n"
    # casing_index1
    output += format_rust_array("CASING_INDEX1", data["casing_index1"], "u8")
    output += "\n\n"
    # casing_index2
    output += format_rust_array("CASING_INDEX2", data["casing_index2"], "u8")
    output += "\n\n"
    # type_index1
    output += format_rust_array("TYPE_INDEX1", data["type_index1"], "u8")
    output += "\n\n"
    # type_index2
    output += format_rust_array("TYPE_INDEX2", data["type_index2"], "u8")
    output += "\n"

    return output


def generate_unialt_data(data):
    """Generate rust-bindings/src/unicode/unialt_data.rs"""
    import textwrap

    output = textwrap.dedent("""\
    // Auto-generated by scripts/extract_unicode_data.py
    // DO NOT EDIT MANUALLY
    // Source: Unicode/unialt.c (generated by makeutype.py, Unicode 17.0.0)
    //
    // NFKD decomposition sequences and visual alternatives for Unicode characters.
    // Each entry is a null-terminated sequence of code points (0 = end of list).

    pub const UNIALT_SHIFT: u32 = 7;
    """)

    output += "\n"
    output += format_rust_array("UNIALT_DATA", data["unialt_data"], "u32")
    output += "\n\n"
    output += format_rust_array("UNIALT_INDEX1", data["unialt_index1"], "u8")
    output += "\n\n"
    output += format_rust_array("UNIALT_INDEX2", data["unialt_index2"], "u16")
    output += "\n"

    return output


def generate_arabic_forms_data(data):
    """Generate rust-bindings/src/unicode/arabic_forms_data.rs"""
    import textwrap

    output = textwrap.dedent("""\
    // Auto-generated by scripts/extract_unicode_data.py
    // DO NOT EDIT MANUALLY
    // Source: Unicode/ArabicForms.c (generated by makeutype.py, Unicode 17.0.0)
    //
    // Arabic presentation form data for characters in the U+0600..U+06FF range.
    // Index = (ch - 0x600) for ch in [0x600, 0x6FF].

    #[derive(Clone, Copy)]
    pub struct ArabicForm {
        pub initial: u16,
        pub medial: u16,
        pub final_form: u16,
        pub isolated: u16,
        pub isletter: bool,
        pub joindual: bool,
        pub required_lig_with_alef: bool,
    }
    """)

    output += "\n"
    output += format_rust_struct_array("ARABIC_FORMS", data["arabic_forms"],
                                       "ArabicForm",
                                       ["initial", "medial", "final_form", "isolated",
                                        "isletter", "joindual", "required_lig_with_alef"],
                                       bool_fields={"isletter", "joindual", "required_lig_with_alef"})
    output += "\n"

    return output


def main():
    print("Extracting utype.c data...")
    utype_data = extract_utype()
    print(f"  casing_data: {len(utype_data['casing_data'])} entries")
    print(f"  type_data: {len(utype_data['type_data'])} entries")
    print(f"  casing_index1: {len(utype_data['casing_index1'])} entries")
    print(f"  casing_index2: {len(utype_data['casing_index2'])} entries")
    print(f"  type_index1: {len(utype_data['type_index1'])} entries")
    print(f"  type_index2: {len(utype_data['type_index2'])} entries")

    print("Extracting unialt.c data...")
    unialt_data = extract_unialt()
    print(f"  unialt_data: {len(unialt_data['unialt_data'])} entries")
    print(f"  unialt_index1: {len(unialt_data['unialt_index1'])} entries")
    print(f"  unialt_index2: {len(unialt_data['unialt_index2'])} entries")

    print("Extracting ArabicForms.c data...")
    arabic_data = extract_arabic_forms()
    print(f"  arabic_forms: {len(arabic_data['arabic_forms'])} entries")

    # Write output files
    output_dir = os.path.join(PROJECT_ROOT, "rust-bindings", "src", "unicode")
    os.makedirs(output_dir, exist_ok=True)

    utype_out = generate_utype_data(utype_data)
    path = os.path.join(output_dir, "utype_data.rs")
    with open(path, 'w') as f:
        f.write(utype_out)
    print(f"\nWrote {path} ({len(utype_out)} bytes)")

    unialt_out = generate_unialt_data(unialt_data)
    path = os.path.join(output_dir, "unialt_data.rs")
    with open(path, 'w') as f:
        f.write(unialt_out)
    print(f"Wrote {path} ({len(unialt_out)} bytes)")

    arabic_out = generate_arabic_forms_data(arabic_data)
    path = os.path.join(output_dir, "arabic_forms_data.rs")
    with open(path, 'w') as f:
        f.write(arabic_out)
    print(f"Wrote {path} ({len(arabic_out)} bytes)")

    print("\nDone! Run 'python3 scripts/extract_unicode_data.py' to regenerate after C source changes.")


if __name__ == "__main__":
    main()
