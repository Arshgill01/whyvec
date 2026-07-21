// FFI-style kernel: scalar arguments are passed by pointer.
void add_vectors_(int *output, const int *input, const int *count) {
  // If count points inside output, a write can change the trip count.
  // The original semantics must remain available for that overlap case.
  for (int i = 0; i < *count; ++i)
    output[i] += input[i];
}
