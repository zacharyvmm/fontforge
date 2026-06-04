# Distributed under the original FontForge BSD 3-clause license

include(CTest)

add_executable(test_roundtrip test_roundtrip.c)
target_link_libraries(test_roundtrip PRIVATE fontforge)

set_target_properties(test_roundtrip PROPERTIES
  C_STANDARD 17
  C_STANDARD_REQUIRED ON
)

add_test(NAME test_roundtrip COMMAND test_roundtrip)

add_executable(test_ttf_roundtrip test_ttf_roundtrip.c)
target_link_libraries(test_ttf_roundtrip PRIVATE fontforge)

set_target_properties(test_ttf_roundtrip PROPERTIES
  C_STANDARD 17
  C_STANDARD_REQUIRED ON
)

add_test(NAME test_ttf_roundtrip COMMAND test_ttf_roundtrip)
