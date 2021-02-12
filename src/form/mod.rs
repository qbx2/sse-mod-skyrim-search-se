use anyhow::Context;
use std::ffi::CStr;

mod npc;

#[repr(C)]
struct TESForm {
    unknown_00: u64,
    unknown_08: u64,
    flags: u32,
    form_id: u32,
    unknown_18: u16,
    form_type: u8,
}

impl TESForm {
    pub(crate) fn get_name(&self) -> Option<std::borrow::Cow<str>> {
        unsafe {
            let result = (S.get_name)(self);
            if result.is_null() {
                return None;
            }
            Some(CStr::from_ptr(result).to_string_lossy())
        }
    }
}

pub(crate) unsafe fn init(image_base: usize) -> anyhow::Result<()> {
    npc::init(image_base).context("npc::init")?;

    Ok(())
}
