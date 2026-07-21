#define _POSIX_C_SOURCE 200809L
#include "whyvec_demo.h"

#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

static __attribute__((noinline)) void original_add(int *output,
                                                    const int *input,
                                                    const int *count) {
  for (int i = 0; i < *count; ++i)
    output[i] += input[i];
}

static uint64_t now_ns(void) {
  struct timespec value;
  if (clock_gettime(CLOCK_MONOTONIC_RAW, &value) != 0)
    abort();
  return (uint64_t)value.tv_sec * UINT64_C(1000000000) + value.tv_nsec;
}

static uint64_t measure(void (*function)(int *, const int *, const int *),
                        int *output, const int *input, const int *count,
                        int repetitions) {
  const uint64_t start = now_ns();
  for (int i = 0; i < repetitions; ++i)
    function(output, input, count);
  return now_ns() - start;
}

int main(void) {
  static const int sizes[] = {8, 31, 64, 257, 1024, 4096, 16384, 65536};
  uint32_t seed = UINT32_C(0x32c56bc2);
  volatile int checksum = 0;

  puts("size,sample,order,repetitions,original_ns,guarded_ns");
  for (unsigned size_index = 0;
       size_index < sizeof(sizes) / sizeof(sizes[0]); ++size_index) {
    const int count = sizes[size_index];
    const size_t bytes = (size_t)count * sizeof(int);
    const size_t allocation_bytes = (bytes + 63u) & ~(size_t)63u;
    int *output = aligned_alloc(64, allocation_bytes);
    int *input = aligned_alloc(64, allocation_bytes);
    if (!output || !input)
      return 2;
    for (int i = 0; i < count; ++i) {
      output[i] = i & 7;
      input[i] = 0;
    }
    const int repetitions = 8000000 / count + 1;
    for (int warmup = 0; warmup < 7; ++warmup) {
      original_add(output, input, &count);
      add_vectors_(output, input, &count);
    }
    for (int sample = 0; sample < 31; ++sample) {
      seed = seed * UINT32_C(1664525) + UINT32_C(1013904223);
      uint64_t original_ns;
      uint64_t guarded_ns;
      const unsigned guarded_first = seed >> 31;
      if (guarded_first) {
        guarded_ns = measure(add_vectors_, output, input, &count, repetitions);
        original_ns = measure(original_add, output, input, &count, repetitions);
      } else {
        original_ns = measure(original_add, output, input, &count, repetitions);
        guarded_ns = measure(add_vectors_, output, input, &count, repetitions);
      }
      printf("%d,%d,%s,%d,%" PRIu64 ",%" PRIu64 "\n", count, sample,
             guarded_first ? "guarded-first" : "original-first", repetitions,
             original_ns, guarded_ns);
    }
    checksum ^= output[size_index % (unsigned)count];
    free(input);
    free(output);
  }
  return checksum == -1;
}
