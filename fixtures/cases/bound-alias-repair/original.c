#include <stddef.h>

__attribute__((noinline)) void add_vectors_original(
    int *output,
    const int *input,
    const int *count) {
  for (int i = 0; i < *count; ++i)
    output[i] += input[i];
}
