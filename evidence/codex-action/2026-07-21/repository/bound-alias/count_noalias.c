// Monolithic Clang witness for observation-level split-pipeline equivalence.
void add_vectors_(
    int *output,
    const int *input,
    const int *restrict count
) {
  for (int i = 0; i < *count; ++i)
    output[i] += input[i];
}
