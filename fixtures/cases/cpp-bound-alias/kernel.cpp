extern "C" void add_vectors_cpp(int *output, const int *input,
                                const int *count) {
  // C++ source semantics are evaluated separately from the LLVM experiment.
  for (int i = 0; i < *count; ++i)
    output[i] += input[i];
}
