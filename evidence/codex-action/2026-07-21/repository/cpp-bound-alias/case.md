# C++ bound-alias optimization causality

This fixture establishes that the typed LLVM experiment works for a C++
translation unit with stable C linkage. The independently compiled
`__restrict` witness confirms only the preferred compiler outcome; C++ caller,
ABI, and contract analysis remain required before any source action.
