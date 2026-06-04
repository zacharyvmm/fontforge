// Integration test for FontForge FFI bindings.
// Calls C functions through generated bindings and verifies basic behavior.

#[test]
fn test_spline_font_new() {
    unsafe {
        // Initialize the FontForge library first
        fontforge_ffi::doinitFontForgeMain();

        // Create a new SplineFont via FFI
        let sf = fontforge_ffi::SplineFontNew();
        assert!(!sf.is_null(), "SplineFontNew() returned null");

        // Free the font
        fontforge_ffi::SplineFontFree(sf);
    }
}

#[test]
fn test_spline_font_create_and_free_many() {
    unsafe {
        // Initialize once
        fontforge_ffi::doinitFontForgeMain();

        // Create and free fonts sequentially to ensure no memory corruption
        for _ in 0..10 {
            let sf = fontforge_ffi::SplineFontNew();
            assert!(!sf.is_null(), "SplineFontNew() returned null in loop iteration");
            fontforge_ffi::SplineFontFree(sf);
        }
    }
}
