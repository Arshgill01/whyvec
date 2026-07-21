#![allow(unsafe_op_in_unsafe_fn)]

/// Raw-pointer kernel used to preserve an FFI-style aliasing contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_vectors_rs(
    output: *mut i32,
    input: *const i32,
    count: *const i32,
) {
    let mut index = 0_i32;
    while index < *count {
        let offset = index as usize;
        *output.add(offset) += *input.add(offset);
        index += 1;
    }
}
