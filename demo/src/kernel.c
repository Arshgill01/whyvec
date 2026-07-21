#include "whyvec_demo.h"

void add_vectors_(int *output, const int *input, const int *count) {
  for (int i = 0; i < *count; ++i) {
    output[i] += input[i];
  }
}
