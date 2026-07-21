#define APPLY(OUTPUT, INPUT, COUNT)                                            \
  for (int i = 0; i < *(COUNT); ++i)                                          \
    (OUTPUT)[i] += (INPUT)[i]

extern "C" void macro_loops(int *output, const int *input, const int *count) {
  APPLY(output, input, count); APPLY(output, input, count);
}
