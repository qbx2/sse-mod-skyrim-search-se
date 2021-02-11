use anyhow::anyhow;
use winapi::ctypes::{c_void, c_char};
use std::ffi::{CStr, CString};
use win_dbg_logger::output_debug_string;
use detour::static_detour;
use crate::patch::patch_bytes;
use crate::db;
use rusqlite::params;

static_detour! {
    static NpcEdidSetter: fn(*const c_void, *const c_char) -> bool;
}

#[repr(C)]
struct TESForm {
    unknown_00: u64,
    unknown_08: u64,
    flags: u32,
    form_id: u32,
    unknown_18: u16,
    form_type: u8,
}

fn new_edid_setter(this: &TESForm, edid: *const c_char) -> bool {
    let form_id = this.form_id;
    let form_type = this.form_type;
    let result: anyhow::Result<()> = try {
        unsafe {
            if edid as usize != 0 {
                db::DB.lock().map_err(|e| anyhow!(e.to_string()))?.execute(
                    "INSERT OR REPLACE INTO forms (id, type, edid) VALUES (?1, ?2, ?3)",
                    params![form_id, form_type, CStr::from_ptr(edid).to_str()?],
                )?;
            }
        }
    };
    if let Err(ref err) = result {
        output_debug_string(format!("{:#}", err).as_str());
    }
    result.is_ok()
}

pub(crate) unsafe fn init(image_base: usize) -> anyhow::Result<()> {
    let npc_vtable = image_base + 0x159fcd0;

    output_debug_string(format!("setter: {:#x}", npc_vtable + 0x198).as_str());

    patch_bytes(&(new_edid_setter as usize), (npc_vtable + 0x198) as *mut c_void, 8)?;

    Ok(())
}
