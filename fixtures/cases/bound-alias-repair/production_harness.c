#include <assert.h>
#include <string.h>

void add_vectors_original(int *output, const int *input, const int *count);
void add_vectors_(int *output, const int *input, const int *count);

enum { WORDS = 320 };

static void run_case(
    int output_offset,
    int input_offset,
    int count_offset,
    int count_value) {
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
  add_vectors_(
      &repaired[output_offset],
      &repaired[input_offset],
      &repaired[count_offset]);
  assert(memcmp(original, repaired, sizeof(original)) == 0);
}

int main(void) {
  run_case(8, 48, 2, 1);
  run_case(8, 48, 2, 16);
  run_case(8, 48, 7, 16);
  run_case(8, 48, 24, 16);
  run_case(8, 48, 8, 8);
  run_case(8, 48, 11, 8);
  run_case(8, 9, 2, 16);
  run_case(9, 8, 2, 16);
  run_case(8, 48, 2, 0);
  run_case(8, 48, 2, -4);
  run_case(8, 176, 2, 128);
  return 0;
}
