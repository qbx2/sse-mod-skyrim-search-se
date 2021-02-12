use anyhow::Context;
use std::ffi::CStr;
use winapi::ctypes::c_char;
use late_static::LateStatic;
use std::mem::transmute;

mod npc;
mod achr;
mod refr;
mod cell;

#[repr(C)]
pub(crate) struct TESForm {
    unknown_00: u64,
    unknown_08: u64,
    flags: u32, // 10
    form_id: u32, // 14
    unknown_18: u16,
    form_type: u8, // 1A
    padding_1b: u8,
    padding_1c: u32,
    // 20
}

struct State {
    get_name: fn(&TESForm) -> *const c_char,
}
unsafe impl Sync for State {}
static S: LateStatic<State> = LateStatic::new();

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
    let get_name = transmute(image_base + 0x196e10);

    LateStatic::assign(&S, State {
        get_name,
    });

    npc::init(image_base).context("npc::init")?;
    achr::init(image_base).context("achr::init")?;
    cell::init(image_base).context("cell::init")?;

    Ok(())
}
