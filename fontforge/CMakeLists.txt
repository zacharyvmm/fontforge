# Distributed under the original FontForge BSD 3-clause license

set(FONTFORGE_NOINST_HEADERS
  asmfpst.h
  hinting/autohint.h
  autosave.h
  autotrace.h
  autowidth.h
  autowidth2.h
  bitmapchar.h
  bitmapcontrol.h
  hinting/bvedit.h
  core/clipnoui.h
  core/crctab.h
  formats/cvexport.h
  math/cvimages.h
  math/cvundoes.h
  formats/dumpbdf.h
  formats/dumppfa.h
  math/effects.h
  core/encoding.h
  featurefile.h
  math/fvcomposite.h
  core/fvfonts.h
  formats/fvimportbdf.h
  formats/ikarus.h
  langfreq.h
  formats/macbinary.h
  formats/macenc.h
  math/mathconstants.h
  multidialog.h
  core/namelist.h
  formats/othersubrs.h
  formats/palmfonts.h
  formats/parsepdf.h
  formats/parsepfa.h
  formats/parsettf.h
  formats/parsettfatt.h
  formats/parsettfbmf.h
  formats/parsettfvar.h
  core/plugin.h
  psread.h
  pua.h
  savefont.h
  math/scstyles.h
  formats/sfd.h
  math/spiro.h
  math/splinefill.h
  math/splinefit.h
  math/splineorder2.h
  math/splineoverlap.h
  math/splinerefigure.h
  splinesave.h
  splinesaveafm.h
  math/splinestroke.h
  core/splineutil.h
  core/splineutil2.h
  core/start.h
  scripting/std_maps.hpp
  formats/svg.h
  formats/tottf.h
  formats/tottfaat.h
  formats/tottfgpos.h
  formats/tottfvar.h
  ttfspecial.h
  math/utanvec.h
  formats/winfonts.h
  formats/woff.h
  math/zapfnomen.h
)

set(FONTFORGE_INST_HEADERS
  autowidth.h
  autowidth2.h
  core/baseviews.h
  math/bezctx_ff.h
  bitmapcontrol.h
  core/delta.h
  hinting/edgelist.h
  hinting/edgelist2.h
  core/encoding.h
  hinting/fffreetype.h
  scripting/ffpython.h
  core/flaglist.h
  core/fontforge.h
  core/fontforgevw.h
  math/fvmetrics.h
  core/getline.h
  core/glif_name_hash.h
  glyphcomp.h
  math/groups.h
  lookups.h
  core/mem.h
  mm.h
  core/namehash.h
  nonlineartrans.h
  core/ofl.h
  core/PfEd.h
  core/print.h
  psfont.h
  savefont.h
  scripting/scriptfuncs.h
  scripting/scripting.h
  math/sd.h
  math/search.h
  formats/sfd1.h
  math/sflayoutP.h
  core/splinefont.h
  hinting/stemdb.h
  ttf.h
  hinting/ttfinstrs.h
  core/uiinterface.h
  unicoderange.h
)

add_library(fontforge
  ${FONTFORGE_NOINST_HEADERS}
  ${FONTFORGE_INST_HEADERS}
  core/activeinui.c
  scripting/ffglib_compat.cpp
  asmfpst.c
  hinting/autohint.c
  autosave.c
  autotrace.c
  autowidth.c
  autowidth2.c
  math/bezctx_ff.c
  bitmapchar.c
  bitmapcontrol.c
  hinting/bvedit.c
  core/clipnoui.c
  core/crctab.cpp
  formats/cvexport.c
  math/cvimages.c
  math/cvundoes.c
  formats/dumpbdf.c
  formats/dumppfa.c
  math/effects.c
  core/encoding.c
  featurefile.c
  scripting/ffprocess.c
  core/flaglist.c
  core/fontviewbase.c
  freetype.c
  math/ftdelta.c
  math/fvcomposite.c
  core/fvfonts.c
  formats/fvimportbdf.c
  math/fvmetrics.c
  core/getline.c
  core/glif_name_hash.cpp
  glyphcomp.c
  math/groups.c
  formats/ikarus.c
  langfreq.c
  lookups.c
  formats/macbinary.c
  formats/macenc.c
  math/mathconstants.c
  core/mem.c
  mm.c
  core/namelist.c
  nonlineartrans.c
  noprefs.c
  core/nouiutil.c
  hinting/nowakowskittfinstr.c
  core/ofl.c
  formats/othersubrs.c
  formats/palmfonts.c
  formats/parsepdf.c
  formats/parsepfa.c
  formats/parsettf.c
  formats/parsettfatt.c
  formats/parsettfbmf.c
  formats/parsettfvar.c
  core/plugin.cpp
  core/print.cpp
  psread.c
  pua.c
  scripting/python.cpp
  savefont.c
  scripting/scripting.cpp
  math/scstyles.c
  math/search.c
  formats/sfd.cpp
  formats/sfd1.c
  math/sflayout.cpp
  math/spiro.c
  core/splinechar.c
  math/splinefill.c
  math/splinefit.c
  core/splinefont.c
  math/splineorder2.c
  math/splineoverlap.c
  math/splinerefigure.c
  splinesave.c
  splinesaveafm.c
  math/splinestroke.c
  core/splineutil.c
  core/splineutil2.c
  core/start.c
  hinting/stemdb.c
  scripting/std_maps.cpp
  formats/svg.c
  formats/tottf.c
  formats/tottfaat.c
  formats/tottfgpos.c
  formats/tottfvar.c
  hinting/ttfinstrs.c
  ttfspecial.c
  formats/ufo.c
  unicoderange.c
  math/utanvec.c
  formats/winfonts.c
  formats/woff.c
  math/zapfnomen.c
)

add_subdirectory(shapers)

if(ENABLE_WOFF2_RESULT)
  target_sources(fontforge PRIVATE formats/woff2.cc)
endif()

# Turn up warnings on a few source files
set_property(SOURCE math/splineoverlap.c APPEND PROPERTY COMPILE_OPTIONS ${FONTFORGE_EXTRA_CFLAGS})

set_property(TARGET fontforge PROPERTY VERSION 4)
if(BUILD_SHARED_LIBS)
  if(WIN32 OR CYGWIN)
    # Both fontforge.exe and *.pyd Python modules look for libfontforge.dll in
    # their directory, so we need to produce a second copy.
    add_custom_command(TARGET fontforge POST_BUILD
            COMMAND ${CMAKE_COMMAND} -E copy $<TARGET_FILE:fontforge> ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}
    )
    target_link_options(fontforge PRIVATE -Wl,--export-all-symbols) # FIXME
  endif()
endif()

# This is pretty bad...
target_include_directories(fontforge
  PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/core
    ${CMAKE_CURRENT_SOURCE_DIR}/formats
    ${CMAKE_CURRENT_SOURCE_DIR}/math
    ${CMAKE_CURRENT_SOURCE_DIR}/hinting
    ${CMAKE_CURRENT_SOURCE_DIR}/scripting
)

# Export all symbols from the DLL on Windows (mimics MinGW's --export-all-symbols)
set_target_properties(fontforge PROPERTIES WINDOWS_EXPORT_ALL_SYMBOLS ON)

target_link_libraries(fontforge
  PUBLIC
    fontforge_common_headers
    shapers
    Freetype::Freetype
    LibXml2::LibXml2
    $<$<BOOL:${Intl_FOUND}>:Intl::Intl>
  PRIVATE
    MathLib::MathLib
    Iconv::Iconv
    ZLIB::ZLIB
    mINI
)

if(APPLE)
  target_link_libraries(fontforge
    PRIVATE
    "-framework CoreServices"
  )
endif()

if(ENABLE_FREETYPE_DEBUGGER)
  target_link_libraries(fontforge PUBLIC FreeTypeSource::FreeTypeSource)
endif()
if(ENABLE_LIBSPIRO_RESULT)
  target_link_libraries(fontforge PUBLIC Libspiro::Libspiro)
endif()
if(ENABLE_LIBREADLINE_RESULT)
  target_link_libraries(fontforge PUBLIC Readline::Readline)
endif()
if(ENABLE_PYTHON_SCRIPTING_RESULT)
  if(APPLE)
    # On macOS, don't link libfontforge directly to Python library.
    # The Python symbols should be resolved at runtime from the interpreter
    # that loads the extension module. Use -undefined dynamic_lookup to allow
    # undefined symbols that will be resolved when the extension is loaded.
    target_include_directories(fontforge PRIVATE ${Python3_INCLUDE_DIRS})
    target_link_options(fontforge PRIVATE -undefined dynamic_lookup)
  else()
    if(TARGET Python3::Python)
      target_link_libraries(fontforge PRIVATE Python3::Python)
    elseif(TARGET Python3::Module)
      target_link_libraries(fontforge PRIVATE Python3::Module)
    else()
      message(FATAL_ERROR "Python3 imported target not found")
    endif()
  endif()
endif()
if(ENABLE_WOFF2_RESULT)
  target_link_libraries(fontforge PRIVATE WOFF2::WOFF2)
endif()
if(ENABLE_LIBUNIBREAK_RESULT)
  target_link_libraries(fontforge PRIVATE Libunibreak::Libunibreak)
endif()

if(${CMAKE_VERSION} VERSION_LESS "3.12.0")
  target_link_libraries(fontforge PRIVATE gunicode_interface gutils_interface)
else()
  target_link_libraries(fontforge PRIVATE gunicode gutils)
endif()

# No dev package -> no need to install if static
if(BUILD_SHARED_LIBS)
  if(BUILDING_WHEEL)
    # Wheel: install alongside Python modules for bundling
    install(TARGETS fontforge
      RUNTIME DESTINATION "."
      LIBRARY DESTINATION "."
    )
  elseif(WIN32 OR CYGWIN)
    install(TARGETS fontforge RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR} LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR})
  else()
    install(TARGETS fontforge RUNTIME DESTINATION ${CMAKE_INSTALL_LIBDIR} LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR})
  endif()
endif()
