cmake_minimum_required(VERSION 3.20)
project(whyvec_demo C)

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

add_library(whyvec_demo STATIC
    src/kernel.c
    src/ffi.c
)
target_include_directories(whyvec_demo PUBLIC include)
target_compile_options(whyvec_demo PRIVATE -O3 -march=x86-64-v3)

enable_testing()
add_executable(whyvec_demo_test tests/test_kernel.c)
target_link_libraries(whyvec_demo_test PRIVATE whyvec_demo)
add_test(NAME whyvec_demo_test COMMAND whyvec_demo_test)
