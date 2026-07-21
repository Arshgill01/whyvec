void whyvec_volatile_refusal(int *output, const int *input,
                             const volatile int *count) {
  for (int i = 0; i < *count; ++i)
    output[i] += input[i];
}
