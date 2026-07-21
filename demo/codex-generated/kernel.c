#include "whyvec_demo.h"

#include <stdint.h>

#ifdef WHYVEC_TESTING
static unsigned whyvec_fast_paths;
static unsigned whyvec_fallback_paths;

void whyvec_test_reset_paths(void) {
  whyvec_fast_paths = 0;
  whyvec_fallback_paths = 0;
}

unsigned whyvec_test_fast_paths(void) { return whyvec_fast_paths; }
unsigned whyvec_test_fallback_paths(void) { return whyvec_fallback_paths; }
#define WHYVEC_FAST_PATH() (++whyvec_fast_paths)
#define WHYVEC_FALLBACK_PATH() (++whyvec_fallback_paths)
#else
#define WHYVEC_FAST_PATH() ((void)0)
#define WHYVEC_FALLBACK_PATH() ((void)0)
#endif

static int count_is_outside_output(uintptr_t output_start,
                                   uintptr_t count_start,
                                   int initial_count) {
  if (initial_count <= 0)
    return 1;

  const uintptr_t elements = (uintptr_t)initial_count;
  if (elements > UINTPTR_MAX / sizeof(int))
    return 0;
  const uintptr_t output_bytes = elements * sizeof(int);
  if (output_start > UINTPTR_MAX - output_bytes ||
      count_start > UINTPTR_MAX - sizeof(int))
    return 0;

  const uintptr_t output_end = output_start + output_bytes;
  const uintptr_t count_end = count_start + sizeof(int);
  return output_end <= count_start || count_end <= output_start;
}

#ifdef WHYVEC_TESTING
int whyvec_test_count_is_outside_output(uintptr_t output_start,
                                        uintptr_t count_start,
                                        int initial_count) {
  return count_is_outside_output(output_start, count_start, initial_count);
}
#endif

void add_vectors_(int *output, const int *input, const int *count) {
  const int initial_count = *count;
  if (count_is_outside_output((uintptr_t)output, (uintptr_t)count,
                              initial_count)) {
    WHYVEC_FAST_PATH();
    for (int i = 0; i < initial_count; ++i) {
      output[i] += input[i];
    }
    return;
  }

  WHYVEC_FALLBACK_PATH();
  for (int i = 0; i < *count; ++i) {
    output[i] += input[i];
  }
}
