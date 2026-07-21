template <typename T>
void template_add(T *output, const T *input, const int *__restrict count) {
  for (int i = 0; i < *count; ++i)
    output[i] += input[i];
}

template void template_add<int>(int *, const int *, const int *);
