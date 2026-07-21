cmake_minimum_required(VERSION 3.20)
project(whyvec_demo C)

find_package(Python3 REQUIRED COMPONENTS Interpreter)

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

add_library(whyvec_demo STATIC
    src/kernel.c
    src/ffi.c
)
target_include_directories(whyvec_demo PUBLIC include)
target_compile_options(whyvec_demo PRIVATE -O3 -march=x86-64-v3)

add_library(whyvec_demo_refusal OBJECT EXCLUDE_FROM_ALL src/refusal.c)
target_compile_options(whyvec_demo_refusal PRIVATE -O3 -march=x86-64-v3)

enable_testing()
add_executable(whyvec_demo_test tests/test_kernel.c)
target_link_libraries(whyvec_demo_test PRIVATE whyvec_demo)
add_test(NAME whyvec_demo_test COMMAND whyvec_demo_test)

option(WHYVEC_VALIDATE_CANDIDATE "Enable candidate-only validation targets" OFF)
if(WHYVEC_VALIDATE_CANDIDATE)
  add_executable(whyvec_demo_differential tests/test_differential.c)
  target_include_directories(whyvec_demo_differential PRIVATE include)
  target_compile_options(whyvec_demo_differential PRIVATE -O3 -march=x86-64-v3)
  add_test(NAME whyvec_demo_differential COMMAND whyvec_demo_differential)

  add_executable(whyvec_demo_benchmark tests/benchmark.c)
  target_link_libraries(whyvec_demo_benchmark PRIVATE whyvec_demo)
  target_compile_options(whyvec_demo_benchmark PRIVATE -O3 -march=x86-64-v3)

  add_custom_target(whyvec_validate
      COMMAND ${Python3_EXECUTABLE} ${CMAKE_SOURCE_DIR}/tools/validate.py
              --source-dir ${CMAKE_SOURCE_DIR}
              --build-dir ${CMAKE_BINARY_DIR}
              --clang ${CMAKE_C_COMPILER}
              --artifact-dir ${CMAKE_BINARY_DIR}/whyvec-validation
      DEPENDS whyvec_demo_test whyvec_demo_differential whyvec_demo_benchmark
      USES_TERMINAL
  )
endif()
