extern "C" void add_vectors_cpp(int *output, const int *input,
                                const int *__restrict count) {
  for (int i = 0; i < *count; ++i)
    output[i] += input[i];
}
