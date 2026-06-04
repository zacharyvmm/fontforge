# Distributed under the original FontForge BSD 3-clause license

add_library(gutils OBJECT
  dlist.cpp
  ffdir.cpp
  fsys.cpp
  gcol.c
  gutils.cpp
  gwwintl.cpp
)

if(BUILD_SHARED_LIBS)
  set_property(TARGET gutils PROPERTY POSITION_INDEPENDENT_CODE ON)
endif()

if(Intl_FOUND)
  list(APPEND gutils_LIBRARIES Intl::Intl)
endif()

if(${CMAKE_VERSION} VERSION_LESS "3.12.0")
  make_object_interface(gutils
    fontforge_common_headers
    gunicode_interface
    ${gutils_LIBRARIES}
  )
else()
  target_link_libraries(gutils
    PUBLIC
      fontforge_common_headers
      gunicode
      ${gutils_LIBRARIES}
  )
endif()
