void ambiguous(float *output, const float *input, int count) {
  for (int i = 0; i < count; ++i) output[i] = input[i]; for (int i = 0; i < count; ++i) output[i] += 1.0f;
}
