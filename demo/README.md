# WhyVec CMake/Ninja demo repository

This is the canonical synthetic product demonstration. It is a real multi-file
CMake project with a public header, a C implementation, an FFI-style wrapper,
and tests covering both ordinary disjoint inputs and a bound pointer that
overlaps writable output.

Configure it with Clang 21 and an exported compilation database:

```console
CC=clang-21 cmake -S demo -B demo/build -G Ninja \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
cmake --build demo/build
ctest --test-dir demo/build --output-on-failure
```

The selected loop is `src/kernel.c:4`. The public/FFI boundary deliberately
prevents caller discovery from authorizing a global `restrict` contract.
