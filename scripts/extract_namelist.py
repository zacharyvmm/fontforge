#!/usr/bin/env python3
"""Extract glyph name data from fontforge/namelist.c and generate Rust source.

This parses the C array initializer data in namelist.c and produces
namelist_data.rs containing static lookup tables for:
- name -> unicode (via phf, but fallback to sorted array for now)
- unicode -> name (per namelist, with basedon chain resolution)
"""

import re
import sys
import os

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
NAMELIST_C = os.path.join(PROJECT_ROOT, "fontforge", "namelist.c")
OUTPUT = os.path.join(PROJECT_ROOT, "rust-bindings", "src", "glyphname", "glyphname_data.rs")


def parse_sub_arrays(lines):
    """Parse all sub-array definitions like:
       static const char *NAME[] = { "str1", "str2", ..., NULL };
       Returns: dict name -> list of 256 entries (string or None)
    """
    arrays = {}
    i = 0
    while i < len(lines):
        line = lines[i]
        # Match: static const char *array_name[] = {
        m = re.match(r'^static const char \*(\w+)\[\]\s*=\s*\{', line)
        if m:
            name = m.group(1)
            entries = []
            i += 1
            # Collect entries until closing }
            brace_depth = 1
            while i < len(lines) and brace_depth > 0:
                line = lines[i]
                # Extract string literals and NULL from this line
                # Handle both quoted strings and NULL
                for token in re.finditer(r'"([^"\\]*(?:\\.[^"\\]*)*)"|NULL', line):
                    if token.group(0) == 'NULL':
                        entries.append(None)
                    else:
                        # Get the string content (un-escape basic escapes)
                        s = token.group(1)
                        entries.append(s)
                brace_depth += line.count('{') - line.count('}')
                i += 1
            # Pad to 256 entries
            if len(entries) < 256:
                entries.extend([None] * (256 - len(entries)))
            arrays[name] = entries[:256]
            continue
        i += 1
    return arrays


def parse_top_level_arrays(lines):
    """Parse top-level pointer arrays like:
       static const char **agl_sans_p0[] = {
           agl_sans_p0_b0,
           agl_sans_p0_b1,
           ...
           NULL
       };
       Returns: dict name -> list of 256 sub-array names (or None)
    """
    arrays = {}
    i = 0
    while i < len(lines):
        line = lines[i]
        # Match: static const char **NAME[] = {
        m = re.match(r'^static const char \*\*(\w+)\[\]\s*=\s*\{', line)
        if m:
            name = m.group(1)
            entries = []
            i += 1
            while i < len(lines):
                line = lines[i]
                # Look for sub-array names or NULL
                for token in re.finditer(r'(\w+)|NULL', line):
                    t = token.group(0)
                    if t == 'NULL':
                        entries.append(None)
                    else:
                        entries.append(t)
                if '}' in line:
                    break
                i += 1
            # Pad to 256 entries
            if len(entries) < 256:
                entries.extend([None] * (256 - len(entries)))
            arrays[name] = entries[:256]
            i += 1
            continue
        i += 1
    return arrays


def parse_namelist_defs(lines):
    """Parse NameList struct definitions:
       static NameList NAME = {
           BASEDON,
           TITLE,
           { PLANE0, NULL, NULL, ... },
           NEXT, RENAMES, USES_UNICODE, UTF8_NAME
       };
       Returns list of dicts with name, basedon, title, planes
    """
    namelists = []
    i = 0
    while i < len(lines):
        line = lines[i]
        m = re.match(r'^static NameList (\w+)\s*=\s*\{', line)
        if m:
            nl_name = m.group(1)
            i += 1
            # Collect the struct initializer
            # It spans multiple lines; collect until we have all fields
            content = []
            brace_depth = 1
            while i < len(lines) and brace_depth > 0:
                line = lines[i]
                brace_depth += line.count('{') - line.count('}')
                content.append(line.strip())
                i += 1
            
            full = ' '.join(content)
            # Remove trailing } and ;
            full = full.strip()
            if full.endswith('};'):
                full = full[:-2]
            elif full.endswith('}'):
                full = full[:-1]
            
            # Split by commas, but respect braces
            fields = []
            depth = 0
            current = []
            for ch in full:
                if ch == '{':
                    depth += 1
                    current.append(ch)
                elif ch == '}':
                    depth -= 1
                    current.append(ch)
                elif ch == ',' and depth == 0:
                    fields.append(''.join(current).strip())
                    current = []
                else:
                    current.append(ch)
            if current:
                fields.append(''.join(current).strip())
            
            # Fields: basedon, title, planes_array, next, renames, uses_unicode, utf8_name
            basedon = fields[0].strip() if len(fields) > 0 else 'NULL'
            title = fields[1].strip() if len(fields) > 1 else '""'
            planes_str = fields[2].strip() if len(fields) > 2 else '{}'
            
            # Parse planes: { p0, p1, ..., NULL }
            # Extract plane array names
            plane_names = []
            planes_inner = planes_str.strip('{}')
            for p in planes_inner.split(','):
                p = p.strip()
                if p and p != 'NULL':
                    plane_names.append(p)
                else:
                    plane_names.append(None)
            
            namelists.append({
                'name': nl_name,
                'basedon': None if basedon == 'NULL' else basedon.lstrip('&'),
                'title': title,
                'planes': plane_names,
            })
            continue
        i += 1
    return namelists


def parse_next_chain(lines):
    """Parse the next chain assignment:
       agl.next = &agl_nf;
       agl_nf.next = &agl_sans;
       etc.
       Returns: dict name -> next_name
    """
    next_chain = {}
    i = 0
    while i < len(lines):
        line = lines[i]
        m = re.search(r'(\w+)\.next\s*=\s*&(\w+)\s*;', line)
        if m:
            next_chain[m.group(1)] = m.group(2)
        i += 1
    return next_chain


def parse_psaltnames(lines):
    """Parse psaltnames array:
       struct psaltnames psaltnames[] = {
           { "name", 0xXXXX, provenance },
           ...
           { NULL, 0, 0 }
       };
       Returns: list of (name, unicode) tuples
    """
    entries = []
    in_array = False
    brace_depth = 0
    current_entry = []
    
    for line in lines:
        if 'psaltnames[] = {' in line:
            in_array = True
            brace_depth = 1
            continue
        if not in_array:
            continue
        
        brace_depth += line.count('{') - line.count('}')
        
        # Extract { "name", 0xXXXX, X } entries
        for m in re.finditer(r'\{\s*"([^"\\]*(?:\\.[^"\\]*)*)"\s*,\s*(0x[0-9a-fA-F]+)\s*,\s*\d+\s*\}', line):
            name = m.group(1)
            uni = int(m.group(2), 16)
            if name == '' or uni == 0:
                continue  # skip sentinel
            entries.append((name, uni))
        
        if brace_depth == 0:
            break
    
    return entries


def build_namelist_data(sub_arrays, top_level, namelist_defs, next_chain, psaltnames):
    """Build the combined data: for each namelist, produce a dict of unicode->name"""
    
    # First, build name->unicode mapping (for UniFromName)
    # Priority order: psaltnames first (lowest priority), then namelists in next chain order
    
    # Determine the next chain order (for name resolution priority)
    # The hash lookup checks namelist next chain: agl -> agl_nf -> agl_sans -> adobepua -> greeksc -> tex -> ams
    # plus psaltnames at the start (lowest priority since it's prepended first)
    
    # Build the full name->unicode map
    name_to_uni = {}  # name -> unicode (highest priority wins)
    
    # Add psaltnames (lowest priority)
    for name, uni in psaltnames:
        if name not in name_to_uni:
            name_to_uni[name] = uni
    
    # Build unicode->name for each namelist plane
    # For each namelist, build: (unicode) -> name
    namelist_data = {}
    
    for nl in namelist_defs:
        nl_name = nl['name']
        uni_to_name = {}  # unicode -> name
        
        for plane_idx, plane_name in enumerate(nl['planes']):
            if plane_name is None:
                continue
            if plane_name not in top_level:
                # Some planes might reference a NULL
                print(f"Warning: plane {plane_name} not found in top_level arrays for {nl_name}")
                continue
            
            byte_arrays = top_level[plane_name]
            for byte1_idx, sub_name in enumerate(byte_arrays):
                if sub_name is None:
                    continue
                if sub_name not in sub_arrays:
                    print(f"Warning: sub-array {sub_name} not found for {nl_name}")
                    continue
                
                entries = sub_arrays[sub_name]
                for byte2_idx, glyph_name in enumerate(entries):
                    if glyph_name is None:
                        continue
                    uni = (plane_idx << 16) | (byte1_idx << 8) | byte2_idx
                    uni_to_name[uni] = glyph_name
                    
                    # Also add to name->unicode (higher priority for later namelists)
                    # Note: we process in the order they appear in namelist_defs
                    # The actual priority in the C code is reverse of next chain order
                    name_to_uni[glyph_name] = uni
        
        namelist_data[nl_name] = {
            'title': nl['title'],
            'basedon': nl['basedon'],
            'uni_to_name': uni_to_name,
        }
    
    # Build the name->unicode priority: later namelists in the "next" chain override earlier ones
    # The C code adds: psaltnames first, then walks agl->next->next...
    # Since psaddbucket prepends, the LAST added entry has highest priority.
    # So priority (highest first) = reverse of (psaltnames + next_chain order)
    
    # Determine next chain order
    ordered_nls = []
    visited = set()
    # Find the head (agl)
    current = 'agl'
    if current in next_chain:
        # Actually, we need the head. Let's find it by looking for what's not a .next target
        heads = set(next_chain.keys()) - set(next_chain.values())
        if heads:
            current = list(heads)[0]
    
    # Follow the chain
    chain_order = []
    while current and current not in visited:
        visited.add(current)
        chain_order.append(current)
        current = next_chain.get(current)
    
    # Build name_to_uni with correct priority
    name_to_uni_priority = {}
    # Add psaltnames first (lowest priority)
    for name, uni in psaltnames:
        name_to_uni_priority[name] = uni
    # Then add namelist entries in chain order (each successive one overrides)
    for nl_name in chain_order:
        if nl_name in namelist_data:
            for uni, name in namelist_data[nl_name]['uni_to_name'].items():
                name_to_uni_priority[name] = uni
    
    return namelist_data, name_to_uni_priority, chain_order


def format_rust_string(s):
    """Format a Python string as a Rust string literal."""
    escaped = s.replace('\\', '\\\\').replace('"', '\\"')
    return f'"{escaped}"'


def generate_rust(namelist_data, name_to_uni, chain_order, output_path):
    """Generate Rust source code with static data."""
    
    with open(output_path, 'w') as f:
        f.write('''// Auto-generated by scripts/extract_namelist.py
// Do not edit manually. Source: fontforge/namelist.c
//
// Contains glyph name <-> Unicode mappings for the FontForge name lists.

/// A single glyph name entry: (name, unicode)
#[derive(Debug, Clone, Copy)]
pub struct GlyphNameEntry {
    pub name: &'static str,
    pub unicode: u32,
}

''')
        
        # Collect all unique entries from all namelists for the main lookup
        all_entries = {}  # name -> unicode
        for nl_name in chain_order:
            if nl_name in namelist_data:
                for uni, name in namelist_data[nl_name]['uni_to_name'].items():
                    all_entries[name] = uni
        
        # Also add psaltnames entries that might not be in any namelist
        # (those are already in name_to_uni)
        for name, uni in name_to_uni.items():
            if name not in all_entries:
                all_entries[name] = uni
        
        # Sort by name for binary search
        sorted_entries = sorted(all_entries.items())
        
        # Write general name->unicode table (sorted by name for binary search)
        f.write(f'/// Total entries: {len(sorted_entries)}\n')
        f.write('pub const GLYPH_NAME_TO_UNICODE: &[GlyphNameEntry] = &[\n')
        for name, uni in sorted_entries:
            f.write(f'    GlyphNameEntry {{ name: {format_rust_string(name)}, unicode: 0x{uni:04X} }},\n')
        f.write('];\n\n')
        
        # Write each namelist's unicode->name data as sorted arrays for binary search
        for nl_name in chain_order:
            if nl_name not in namelist_data:
                continue
            nl = namelist_data[nl_name]
            entries = sorted(nl['uni_to_name'].items())
            if not entries:
                continue
            
            f.write(f'/// NameList: {nl_name} (basedon: {nl["basedon"] or "None"})\n')
            f.write('pub const ')
            f.write(nl_name.upper())
            f.write(f'_UNI_TO_NAME: &[(u32, &\'static str)] = &[\n')
            for uni, name in entries:
                f.write(f'    (0x{uni:04X}, {format_rust_string(name)}),\n')
            f.write('];\n\n')
        
        # Write the chain order
        f.write('/// Namelist chain order (for name->unicode resolution priority, lowest first)\n')
        f.write(f'pub const NAMELIST_CHAIN_ORDER: &[&\'static str] = &[\n')
        for nl_name in chain_order:
            f.write(f'    "{nl_name}",\n')
        f.write('];\n\n')
        
        # Write namelist metadata
        f.write('/// Namelist metadata\n')
        f.write('pub struct NamelistMeta {\n')
        f.write('    pub name: &\'static str,\n')
        f.write('    pub basedon: Option<&\'static str>,\n')
        f.write('    pub title: &\'static str,\n')
        f.write('    pub entries: &\'static [(u32, &\'static str)],\n')
        f.write('}\n\n')
        
        f.write('pub const NAMELIST_METAS: &[NamelistMeta] = &[\n')
        for nl_name in chain_order:
            nl = namelist_data.get(nl_name)
            if nl is None:
                continue
            basedon_str = f'Some("{nl["basedon"]}")' if nl['basedon'] else 'None'
            # Clean up title - remove N_() wrapping
            title = nl['title']
            title = title.replace('N_("', '"').replace('")', '"')
            title = title.replace('NU_("', '"')
            f.write(f'    NamelistMeta {{\n')
            f.write(f'        name: "{nl_name}",\n')
            f.write(f'        basedon: {basedon_str},\n')
            f.write(f'        title: {title},\n')
            f.write(f'        entries: {nl_name.upper()}_UNI_TO_NAME,\n')
            f.write(f'    }},\n')
        f.write('];\n')


def main():
    with open(NAMELIST_C, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    print(f"Parsing {NAMELIST_C} ({len(lines)} lines)...")
    
    sub_arrays = parse_sub_arrays(lines)
    print(f"  Found {len(sub_arrays)} sub-arrays")
    
    top_level = parse_top_level_arrays(lines)
    print(f"  Found {len(top_level)} top-level arrays")
    
    namelist_defs = parse_namelist_defs(lines)
    print(f"  Found {len(namelist_defs)} NameList definitions: {[n['name'] for n in namelist_defs]}")
    
    next_chain = parse_next_chain(lines)
    print(f"  Found {len(next_chain)} next chain links: {next_chain}")
    
    psaltnames = parse_psaltnames(lines)
    print(f"  Found {len(psaltnames)} psaltnames entries")
    
    namelist_data, name_to_uni, chain_order = build_namelist_data(
        sub_arrays, top_level, namelist_defs, next_chain, psaltnames
    )
    
    print(f"  Built data for {len(namelist_data)} namelists")
    print(f"  Chain order: {chain_order}")
    total_entries = sum(len(nl['uni_to_name']) for nl in namelist_data.values())
    print(f"  Total uni->name entries: {total_entries}")
    print(f"  Total name->uni entries: {len(name_to_uni)}")
    
    generate_rust(namelist_data, name_to_uni, chain_order, OUTPUT)
    print(f"  Wrote {OUTPUT}")


if __name__ == '__main__':
    main()
