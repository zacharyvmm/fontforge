use std::env;
use std::path::PathBuf;

fn main() {
    let project_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .to_path_buf();

    let inc_dir = project_root.join("inc");
    let fontforge_dir = project_root.join("fontforge");
    let build_config_dir = project_root.join("build").join("inc");

    // Tell cargo to link against fontforge
    println!("cargo:rustc-link-lib=fontforge");
    println!(
        "cargo:rustc-link-search=native={}",
        project_root.join("build").join("lib").display()
    );

    // Add system include paths for libc headers (needed since clang binary is not installed)
    let gcc_include = "/usr/lib/gcc/x86_64-linux-gnu/15/include";
    let sys_include = "/usr/include/x86_64-linux-gnu";
    let usr_include = "/usr/include";
    let devroot_include = "/tmp/dev-root/usr/include";

    // Generate bindings via bindgen
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", inc_dir.display()))
        .clang_arg(format!("-I{}", fontforge_dir.display()))
        .clang_arg(format!("-I{}", build_config_dir.display()))
        .clang_arg(format!("-I{}", gcc_include))
        .clang_arg(format!("-I{}", sys_include))
        .clang_arg(format!("-I{}", usr_include))
        .clang_arg(format!("-I{}", devroot_include))
        // Allowlist only the types and functions we need for now
        .allowlist_type("SplineFont")
        .allowlist_type("SplineChar")
        .allowlist_type("SplineSet")
        .allowlist_type("SplinePointList")
        .allowlist_type("SplinePoint")
        .allowlist_type("BasePoint")
        .allowlist_type("EncMap")
        .allowlist_type("Encoding")
        .allowlist_function("SplineFontNew")
        .allowlist_function("SplineFontFree")
        .allowlist_function("SFDWrite")
        .allowlist_function("SFReadSplineFontFDir")
        .allowlist_function("doinitFontForgeMain")
        .allowlist_function("LoadSplineFont")
        .allowlist_function("SFGetOrMakeChar")
        .allowlist_function("SplinePointCreate")
        .allowlist_function("SplineMake")
        .allowlist_function("SPLCategorizePoints")
        .allowlist_function("EncMap1to1")
        .allowlist_function("EncMapFree")
        // Spline manipulation functions (RALPH-010)
        .allowlist_function("SplineCharSimplify")
        .allowlist_function("SplineCharAddExtrema")
        .allowlist_function("SplineSetRemoveOverlap")
        .allowlist_function("SplineSetsCorrect")
        // TTF/OTF I/O functions (RALPH-011)
        .allowlist_function("WriteTTFFont")
        .allowlist_function("SFReadTTF")
        .allowlist_function("SFGetChar")
        // SVG font I/O (RALPH-016)
        .allowlist_function("SplinePointListFree")
        // BDF bitmap font types & functions (RALPH-017)
        .allowlist_type("BDFFont")
        .allowlist_type("BDFChar")
        .allowlist_type("BDFProperties")
        .allowlist_type("bdffont")
        .allowlist_type("bdfchar")
        .allowlist_type("bdfprops")
        .allowlist_function("SplineCharCreate")
        .allowlist_function("SFMakeChar")
        .allowlist_function("EncMapNew")
        .allowlist_function("SFDefaultAscent")
        // Supporting types for spline manipulation
        .allowlist_type("simplifyinfo")
        .allowlist_type("simpify_flags")
        .allowlist_type("overlap_type")
        .allowlist_type("ae_type")
        // OpenType layout types (RALPH-019)
        .allowlist_type("OTLookup")
        .allowlist_type("otlookup")
        .allowlist_type("FeatureScriptLangList")
        .allowlist_type("featurescriptlanglist")
        .allowlist_type("lookup_subtable")
        .allowlist_type("scriptlanglist")
        .allowlist_type("otlookup_type")
        .allowlist_type("pst_flags")
        .allowlist_type("FPST")
        .allowlist_type("fpst")
        .allowlist_type("fpst_rule")
        .allowlist_type("generic_fpst")
        // OpenType layout functions (RALPH-019)
        .allowlist_function("SFApplyFeatureFilename")
        .allowlist_function("SFFindLookup")
        .allowlist_function("SFLookupsInScriptLangFeature")
        .allowlist_function("SFFeaturesInScriptLang")
        .allowlist_function("SFLangsInScript")
        .allowlist_function("SFScriptsInLookups")
        .allowlist_function("SFFindLookupSubtable")
        .allowlist_function("TagFullName")
        .allowlist_function("SuffixFromTags")
        .allowlist_function("LookupInit")
        .allowlist_function("SortInsertLookup")
        .allowlist_function("ScriptLangListFree")
        .allowlist_function("FeatureOrderId")
        .allowlist_function("FeatureScriptTagInFeatureScriptList")
        .allowlist_function("GlyphNameCnt")
        // Generate default traits
        .derive_default(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
