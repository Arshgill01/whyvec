#define _POSIX_C_SOURCE 200809L

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

void add_vectors_original(int *output, const int *input, const int *count);
void add_vectors_(int *output, const int *input, const int *count);

enum { SAMPLES = 31, ELEMENTS = 1 << 20, REPETITIONS = 8 };

static uint64_t now_ns(void) {
  struct timespec value;
  if (clock_gettime(CLOCK_MONOTONIC_RAW, &value) != 0)
    abort();
  return (uint64_t)value.tv_sec * 1000000000ULL + (uint64_t)value.tv_nsec;
}

static uint64_t measure(
    void (*function)(int *, const int *, const int *),
    int *output,
    const int *input,
    const int *count) {
  uint64_t start = now_ns();
  for (int repetition = 0; repetition < REPETITIONS; ++repetition)
    function(output, input, count);
  return now_ns() - start;
}

int main(void) {
  int *output = aligned_alloc(64, ELEMENTS * sizeof(*output));
  int *input = aligned_alloc(64, ELEMENTS * sizeof(*input));
  if (!output || !input)
    return 2;
  for (int i = 0; i < ELEMENTS; ++i) {
    output[i] = 1;
    input[i] = (i % 7) + 1;
  }
  int count = ELEMENTS;

  add_vectors_original(output, input, &count);
  add_vectors_(output, input, &count);

  uint64_t original[SAMPLES];
  uint64_t guarded[SAMPLES];
  for (int sample = 0; sample < SAMPLES; ++sample) {
    if ((sample & 1) == 0) {
      original[sample] = measure(add_vectors_original, output, input, &count);
      guarded[sample] = measure(add_vectors_, output, input, &count);
    } else {
      guarded[sample] = measure(add_vectors_, output, input, &count);
      original[sample] = measure(add_vectors_original, output, input, &count);
    }
  }

  printf("{\"elements\":%d,\"repetitions\":%d,\"original_ns\":[", ELEMENTS, REPETITIONS);
  for (int sample = 0; sample < SAMPLES; ++sample)
    printf("%s%llu", sample ? "," : "", (unsigned long long)original[sample]);
  printf("],\"guarded_ns\":[");
  for (int sample = 0; sample < SAMPLES; ++sample)
    printf("%s%llu", sample ? "," : "", (unsigned long long)guarded[sample]);
  printf("]}\n");
  free(input);
  free(output);
  return 0;
}
