use anyhow::anyhow;
use winapi::ctypes::{c_void, c_char};
use std::ffi::CStr;
use win_dbg_logger::output_debug_string;
use detour::static_detour;
use crate::patch::patch_bytes;
use crate::db;
use rusqlite::params;
use std::mem::transmute;
use late_static::LateStatic;
use std::fmt::Formatter;
use std::ops::Deref;
use crate::log::Loggable;
use crate::form::TESForm;

static_detour! {
    static NpcEdidSetter: fn(*const c_void, *const c_char) -> bool;
}

struct TESNPC(TESForm);

struct State {
    npc_vtable: usize,
    get_name: fn(&TESForm) -> *const c_char,
    npc_loadform: fn(&TESNPC, u64) -> u64,
}
unsafe impl Sync for State {}
static S: LateStatic<State> = LateStatic::new();

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("npc_vtable", &(self.npc_vtable as usize))
            .field("get_name", &(self.get_name as usize))
            .field("npc_loadform", &(self.npc_loadform as usize))
            .finish()
    }
}

impl TESNPC {
    fn new_edid_setter(&self, edid: *const c_char) -> bool {
        if edid.is_null() {
            return false;
        }
        let result: anyhow::Result<()> = try {
            let form_id = self.0.form_id;
            let edid = unsafe { CStr::from_ptr(edid).to_str()? };

            db::DB.lock().map_err(|e| anyhow!(e.to_string()))?.execute(
                "INSERT INTO npc (form_id, editor_id) VALUES (?1, ?2)\
                 ON CONFLICT(form_id) DO UPDATE SET editor_id=excluded.editor_id",
                params![form_id, edid],
            )?;
        };
        result.logging_ok().is_some()
    }

    fn new_load(&self, arg: u64) -> u64 {
        let result = (S.npc_loadform)(self, arg);
        let form_id = self.0.form_id;
        if let Some(name) = self.0.get_name() {
            let result: anyhow::Result<()> = try {
                db::DB.lock().map_err(|e| anyhow!(e.to_string()))?.execute(
                    "INSERT INTO npc (form_id, name) VALUES (?1, ?2)\
                     ON CONFLICT(form_id) DO UPDATE SET name=excluded.name",
                    params![form_id, name],
                )?;
            };
            result.logging_ok();
        }
        return result;
    }
}

pub(crate) unsafe fn init(image_base: usize) -> anyhow::Result<()> {
    let npc_vtable = transmute(image_base + 0x159fcd0);
    let get_name = transmute(image_base + 0x196e10);

    output_debug_string(format!("setter: {:#x}", npc_vtable + 0x198).as_str());

    patch_bytes(&(TESNPC::new_edid_setter as usize), (npc_vtable + 0x198) as *mut c_void, 8)?;
    let original_npc_load = patch_bytes(
        &(TESNPC::new_load as usize),
        (npc_vtable + 0x30) as *mut c_void,
        8,
    )?;

    LateStatic::assign(&S, State {
        npc_vtable,
        get_name,
        npc_loadform: transmute(*(original_npc_load.as_ptr() as *const usize)),
    });

    output_debug_string(format!("S: {:#x?}", S.deref()).as_str());

    Ok(())
}
