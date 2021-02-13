use anyhow::Context;

pub(crate) unsafe fn patch_bytes<T, U>(
    src: *const T,
    dst: *mut U,
    num_bytes: usize,
) -> anyhow::Result<Vec<u8>> {
    let src = src as *const u8;
    let dst = dst as *mut u8;
    let _guard = region::protect_with_handle(dst, num_bytes, region::Protection::READ_WRITE)
        .context("patch_bytes")?;
    let original_bytes = std::slice::from_raw_parts(dst, num_bytes).to_vec();
    std::ptr::copy_nonoverlapping(src, dst, num_bytes);

    Ok(original_bytes)
}
