#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

static bool checked_end(uintptr_t start, size_t bytes, uintptr_t *end) {
  if (bytes > UINTPTR_MAX - start)
    return false;
  *end = start + bytes;
  return true;
}

bool whyvec_guard_permits(
    const int *output,
    const int *count,
    int initial_bound) {
  if (initial_bound <= 0)
    return false;
  size_t elements = (size_t)initial_bound;
  if (elements > SIZE_MAX / sizeof(*output))
    return false;
  size_t output_bytes = elements * sizeof(*output);
  uintptr_t output_start = (uintptr_t)output;
  uintptr_t count_start = (uintptr_t)count;
  uintptr_t output_end;
  uintptr_t count_end;
  if (!checked_end(output_start, output_bytes, &output_end) ||
      !checked_end(count_start, sizeof(*count), &count_end))
    return false;
  return output_end <= count_start || count_end <= output_start;
}

__attribute__((noinline)) int add_vectors_guarded(
    int *output,
    const int *input,
    const int *count) {
  int initial_bound = *count;
  if (whyvec_guard_permits(output, count, initial_bound)) {
    for (int i = 0; i < initial_bound; ++i)
      output[i] += input[i];
    return 1;
  }

  for (int i = 0; i < *count; ++i)
    output[i] += input[i];
  return 0;
}
