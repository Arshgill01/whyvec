#ifndef WHYVEC_DEMO_H
#define WHYVEC_DEMO_H

void add_vectors_(int *output, const int *input, const int *count);

/* FFI-style entry point. External callers are intentionally not closed-world. */
void whyvec_demo_apply(int *output, const int *input, const int *count);

#endif
