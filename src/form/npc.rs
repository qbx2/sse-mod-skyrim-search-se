use anyhow::anyhow;
use winapi::ctypes::{c_void, c_char};
use std::ffi::{CStr, CString};
use win_dbg_logger::output_debug_string;
use detour::static_detour;
use crate::patch::patch_bytes;
use crate::db;
use rusqlite::params;

static_detour! {
    static NpcEdidGetter: fn(*const c_void) -> *const c_char;
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

fn new_edid_getter(this: &TESForm) -> *const c_char {
    let result: Result<*const c_char, anyhow::Error> = try {
        let db = db::get_db().lock().unwrap();
        let mut stmt = db.prepare("SELECT edid FROM forms WHERE id = ?")?;
        let mut edid_iter = stmt.query_map(&[this.form_id], |row| {
            Ok(row.get(0)?)
        })?;
        let result: String = edid_iter.next().ok_or(anyhow::anyhow!("not found"))??;
        output_debug_string(format!("override {:#x} edid -> {}", this.form_id, result).as_str());
        CString::new(result)?.into_raw() as *const c_char
    };
    result.unwrap_or(0 as *const c_char)
}

fn new_edid_setter(this: &TESForm, edid: *const c_char) -> bool {
    let form_id = this.form_id;
    let form_type = this.form_type;
    let result: anyhow::Result<()> = try {
        unsafe {
            if edid as usize != 0 {
                db::get_db().lock().map_err(|e| anyhow!(e.to_string()))?.execute(
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

    output_debug_string(format!("getter: {:#x}, setter: {:#x}", npc_vtable + 0x190, npc_vtable + 0x198).as_str());

    patch_bytes(&(new_edid_getter as usize), (npc_vtable + 0x190) as *mut c_void, 8)?;
    patch_bytes(&(new_edid_setter as usize), (npc_vtable + 0x198) as *mut c_void, 8)?;

    Ok(())
}
