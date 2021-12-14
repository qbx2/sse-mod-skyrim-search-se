use anyhow::Context;
use late_static::LateStatic;
use std::ffi::CStr;
use std::mem::transmute;
use winapi::ctypes::c_char;

mod achr;
mod cell;
mod npc;
pub(crate) mod qust;
mod refr;

#[repr(C)]
#[derive(Debug)]
pub(crate) struct TESForm {
    unknown_00: u64,
    unknown_08: u64,
    flags: u32,   // 10
    form_id: u32, // 14
    unknown_18: u16,
    form_type: u8, // 1A
    padding_1b: u8,
    padding_1c: u32,
    // 20
}

struct State {
    get_name: fn(&TESForm) -> *const c_char,
    look_up_by_id: fn(u32) -> *const TESForm,
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

    pub(crate) fn look_up_by_id(id: u32) -> *const TESForm {
        (S.look_up_by_id)(id)
    }
}

pub(crate) unsafe fn init(image_base: usize) -> anyhow::Result<()> {
    let get_name = transmute(image_base + 0x1a1c00);//1.5.97: 0x196e10 -> 1.6.318: 0x1a1c90 -> addressLib ID: 14720 -> 1.6.323: 0x1a1c00
    let look_up_by_id = transmute(image_base + 0x19f080);//1.5.97: 0x194230 -> 1.6.318: 0x19f110 -> addressLib ID: 14617 -> 1.6.323: 0x19f080

    LateStatic::assign(
        &S,
        State {
            get_name,
            look_up_by_id,
        },
    );

    npc::init(image_base).context("npc::init")?;
    achr::init(image_base).context("achr::init")?;
    cell::init(image_base).context("cell::init")?;
    qust::init(image_base).context("qust::init")?;

    Ok(())
}
