#include <stddef.h>

void transform(float *output, const float *input, size_t count) {
  for (size_t i = 0; i < count; ++i)
    output[i] = input[i] * 2.0f + 1.0f;
}
