void update_until(int *output, const int *input,
                  const volatile int *bound) {
  for (int i = 0; i < *bound; ++i)
    output[i] += input[i];
}
