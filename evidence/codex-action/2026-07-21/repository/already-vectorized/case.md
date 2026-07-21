# Already-vectorized runtime-check case

LLVM can generate runtime pointer checks, a vectorized path for disjoint ranges, and a scalar path for overlap. WhyVec must observe that the selected loop already vectorizes and decline counterfactual search.

This fixture prevents the product and demonstration from manufacturing a diagnosis for behavior the compiler already handles.
