# C++ template bound-alias optimization causality

An explicit template instantiation provides a stable mangled LLVM function
identity. The fixture verifies mapping and typed parameter intervention for the
instantiated function without generalizing the result to other template
arguments or call sites.
