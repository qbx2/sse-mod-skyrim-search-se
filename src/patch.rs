use anyhow::Context;

pub(crate) unsafe fn patch_bytes<T, U>(src: *const T, dst: *mut U, num_bytes: usize) -> anyhow::Result<()> {
    let _guard = region::protect_with_handle(
        dst as *const u8,
        num_bytes,
        region::Protection::READ_WRITE,
    ).context("patch_bytes")?;
    std::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, num_bytes);
    Ok(())
}
