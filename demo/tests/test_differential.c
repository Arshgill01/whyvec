#include "whyvec_demo.h"

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define WHYVEC_TESTING 1
#include "../src/kernel.c"

enum { ARENA_INTS = 768 };

typedef union {
  max_align_t alignment;
  int values[ARENA_INTS];
} arena;

static uint32_t random_state = 0x32c56bc2u;

static uint32_t next_random(void) {
  random_state = random_state * 1664525u + 1013904223u;
  return random_state;
}

static void reference_add(int *output, const int *input, const int *count) {
  for (int i = 0; i < *count; ++i)
    output[i] += input[i];
}

static void fill_arena(arena *value) {
  for (int i = 0; i < ARENA_INTS; ++i)
    value->values[i] = (int)(next_random() % 17u) - 8;
}

static void run_case(int count_value, int output_index, int input_delta,
                     int count_delta, int shrink_overlapping_bound) {
  arena expected;
  arena actual;
  fill_arena(&expected);
  memcpy(&actual, &expected, sizeof(actual));

  int *expected_output = &expected.values[output_index];
  int *actual_output = &actual.values[output_index];
  int *expected_count = &expected.values[output_index + count_delta];
  int *actual_count = &actual.values[output_index + count_delta];
  int *expected_input = &expected.values[output_index + input_delta];
  int *actual_input = &actual.values[output_index + input_delta];

  *expected_count = count_value;
  *actual_count = count_value;
  if (shrink_overlapping_bound) {
    assert(count_delta >= 0 && count_delta < count_value);
    expected_input[count_delta] = -count_value;
    actual_input[count_delta] = -count_value;
  }

  reference_add(expected_output, expected_input, expected_count);
  add_vectors_(actual_output, actual_input, actual_count);
  assert(memcmp(&expected, &actual, sizeof(actual)) == 0);
}

int main(void) {
  whyvec_test_reset_paths();

  run_case(0, 96, 320, -1, 0);
  run_case(-7, 97, 320, 0, 0);

  const int boundaries[] = {1,  2,  7,  8,  9,  15, 16, 17,
                            31, 32, 33, 63, 64, 65, 127, 128, 129, 257};
  for (unsigned alignment = 0; alignment < 16; ++alignment) {
    for (unsigned i = 0; i < sizeof(boundaries) / sizeof(boundaries[0]); ++i) {
      const int count = boundaries[i];
      const int output = 64 + (int)alignment;
      run_case(count, output, 384 - output, -1, 0);
      run_case(count, output, 384 - output, count, 0);
      run_case(count, output, 384 - output, count + 1, 0);
    }
  }

  for (int count = 1; count <= 257; ++count) {
    const int output = 64 + (int)(next_random() % 16u);
    const int direction = (count > 2 && (next_random() & 1u)) ? 1 : -1;
    run_case(count, output, direction, -1, 0);
  }

  for (int count = 2; count <= 65; ++count) {
    for (int bound_index = 0; bound_index < count; ++bound_index)
      run_case(count, 96, 320, bound_index, 1);
  }

  run_case(3, 96, 320, 1, 1);
  run_case(9, 97, 320, 4, 1);
  run_case(17, 98, 320, 8, 1);
  run_case(33, 99, 320, 16, 1);

  assert(whyvec_test_fast_paths() > 0);
  assert(whyvec_test_fallback_paths() == 2148);
  assert(!whyvec_test_count_is_outside_output(UINTPTR_MAX - 3u, 32u, 2));
  assert(!whyvec_test_count_is_outside_output(32u, UINTPTR_MAX - 1u, 2));

  printf("{\"executions\":%u,\"fast_paths\":%u,\"fallback_paths\":%u,"
         "\"overflow_refusals\":2}\n",
         whyvec_test_fast_paths() + whyvec_test_fallback_paths(),
         whyvec_test_fast_paths(), whyvec_test_fallback_paths());
  return 0;
}
