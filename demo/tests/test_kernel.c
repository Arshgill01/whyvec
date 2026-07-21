#include "whyvec_demo.h"

#include <assert.h>

static void test_disjoint(void) {
  int output[] = {1, 2, 3, 4};
  const int input[] = {10, 20, 30, 40};
  const int count = 4;
  whyvec_demo_apply(output, input, &count);
  assert(output[0] == 11 && output[3] == 44);
}

static void test_bound_overlaps_output(void) {
  int output[] = {1, 2, 3, 3, 9};
  const int input[] = {1, 1, -2, 100, 100};
  add_vectors_(output, input, &output[2]);
  assert(output[0] == 2);
  assert(output[1] == 3);
  assert(output[2] == 1);
  assert(output[3] == 3);
}

int main(void) {
  test_disjoint();
  test_bound_overlaps_output();
  return 0;
}
