#include <assert.h>

void add_vectors_(int *output, const int *input, const int *count);

int main(void) {
  int output[] = {1, 2, 3, 4};
  const int input[] = {4, 3, 2, 1};
  const int count = 4;
  add_vectors_(output, input, &count);
  for (int i = 0; i < count; ++i)
    assert(output[i] == 5);
  return 0;
}
