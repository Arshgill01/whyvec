#include <assert.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

void add_vectors_original(int *output, const int *input, const int *count);
void add_vectors_(int *output, const int *input, const int *count);
bool whyvec_guard_permits(const int *output, const int *count, int initial_bound);

enum { WORDS = 320 };

static unsigned executions;
static unsigned fast_paths;
static unsigned fallback_paths;

static void run_case(
    int output_offset,
    int input_offset,
    int count_offset,
    int count_value,
    int expected_fast) {
  int original[WORDS];
  int repaired[WORDS];
  for (int i = 0; i < WORDS; ++i)
    original[i] = (i % 11) + 1;
  memcpy(repaired, original, sizeof(original));
  original[count_offset] = count_value;
  repaired[count_offset] = count_value;

  add_vectors_original(
      &original[output_offset],
      &original[input_offset],
      &original[count_offset]);
  int selected = whyvec_guard_permits(
      &repaired[output_offset], &repaired[count_offset], repaired[count_offset]);
  add_vectors_(
      &repaired[output_offset],
      &repaired[input_offset],
      &repaired[count_offset]);
  assert(selected == expected_fast);
  assert(memcmp(original, repaired, sizeof(original)) == 0);
  ++executions;
  if (selected)
    ++fast_paths;
  else
    ++fallback_paths;
}

int main(void) {
  run_case(8, 48, 2, 1, 1);
  run_case(8, 48, 2, 16, 1);
  run_case(8, 48, 7, 16, 1);
  run_case(8, 48, 24, 16, 1);
  run_case(8, 48, 8, 8, 0);
  run_case(8, 48, 11, 8, 0);
  run_case(8, 9, 2, 16, 1);
  run_case(9, 8, 2, 16, 1);
  run_case(8, 48, 2, 0, 0);
  run_case(8, 48, 2, -4, 0);
  run_case(8, 176, 2, 128, 1);

  int count = 8;
  assert(!whyvec_guard_permits(
      (const int *)(uintptr_t)(UINTPTR_MAX - 2), &count, 8));
  assert(!whyvec_guard_permits(
      (const int *)(uintptr_t)16, (const int *)(uintptr_t)(UINTPTR_MAX - 1), 8));

  printf(
      "{\"executions\":%u,\"fast_paths\":%u,\"fallback_paths\":%u,"
      "\"overflow_refusals\":2}\n",
      executions,
      fast_paths,
      fallback_paths);
  return 0;
}
