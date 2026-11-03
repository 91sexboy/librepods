cmake_minimum_required(VERSION 3.22.1)
project(librepods_native_regressions LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
find_package(JNI REQUIRED)
enable_testing()

set(NATIVE_SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../app/src/main/cpp")

# Compile the unmodified production translation unit and its real XZ decoder.
# Only Android's log boundary is replaced for execution on a Linux CI runner.
add_library(native_module SHARED
  "${NATIVE_SOURCE_DIR}/l2c_fcr_hook.cpp"
  android_log.cpp
  "${NATIVE_SOURCE_DIR}/xz/xz_crc32.c"
  "${NATIVE_SOURCE_DIR}/xz/xz_crc64.c"
  "${NATIVE_SOURCE_DIR}/xz/xz_sha256.c"
  "${NATIVE_SOURCE_DIR}/xz/xz_dec_stream.c"
  "${NATIVE_SOURCE_DIR}/xz/xz_dec_lzma2.c"
  "${NATIVE_SOURCE_DIR}/xz/xz_dec_bcj.c"
)
target_include_directories(native_module PRIVATE
  include "${NATIVE_SOURCE_DIR}/xz" ${JNI_INCLUDE_DIRS}
)
target_compile_definitions(native_module PRIVATE
  XZ_DEC_X86 XZ_DEC_ARM XZ_DEC_ARMTHUMB XZ_DEC_ARM64 XZ_DEC_ANY_CHECK
  XZ_USE_CRC64 XZ_USE_SHA256 XZ_DEC_CONCATENATED
)

# A real mapped ELF named as the Bluetooth library, with exported SDK symbols.
add_library(bluetooth_fixture SHARED bluetooth_fixture.cpp)
set_target_properties(bluetooth_fixture PROPERTIES OUTPUT_NAME bluetooth_jni)
target_include_directories(bluetooth_fixture PRIVATE "${NATIVE_SOURCE_DIR}")

add_executable(sdp_registration_test sdp_registration_test.cpp)
target_include_directories(sdp_registration_test PRIVATE
  "${NATIVE_SOURCE_DIR}" ${JNI_INCLUDE_DIRS}
)
target_compile_options(sdp_registration_test PRIVATE -Wall -Wextra -Werror)
target_link_libraries(sdp_registration_test PRIVATE ${CMAKE_DL_LIBS})

foreach(target native_module bluetooth_fixture sdp_registration_test)
  target_compile_options(${target} PRIVATE
    -fsanitize=address,undefined -fno-sanitize-recover=undefined -fno-omit-frame-pointer
  )
  target_link_options(${target} PRIVATE -fsanitize=address,undefined)
endforeach()

add_test(NAME sdp_one_registration
  COMMAND sdp_registration_test
    $<TARGET_FILE:native_module> $<TARGET_FILE:bluetooth_fixture>
)
add_test(NAME sdp_disabled_preserves_vendor
  COMMAND sdp_registration_test
    $<TARGET_FILE:native_module> $<TARGET_FILE:bluetooth_fixture> disabled_vendor
)
add_test(NAME sdp_sdk_forwarding_contract
  COMMAND sdp_registration_test
    $<TARGET_FILE:native_module> $<TARGET_FILE:bluetooth_fixture> forwarding_contract
)
set_tests_properties(
  sdp_one_registration sdp_disabled_preserves_vendor sdp_sdk_forwarding_contract
  PROPERTIES TIMEOUT 10
)
