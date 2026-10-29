# Ninja writes UTF-8 response files without a BOM. MSVC otherwise decodes them
# using the active ANSI codepage, corrupting Chinese paths in static archives.
find_program(JINGYI_BUILD_PYTHON NAMES python REQUIRED)
set_property(GLOBAL PROPERTY RULE_LAUNCH_LINK
  "\"${JINGYI_BUILD_PYTHON}\" \"${CMAKE_CURRENT_LIST_DIR}/msvc_response.py\"")
