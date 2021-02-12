use anyhow::anyhow;
use winapi::ctypes::c_void;
use win_dbg_logger::output_debug_string;
use crate::patch::patch_bytes;
use crate::db;
use rusqlite::params;
use std::mem::transmute;
use late_static::LateStatic;
use std::fmt::Formatter;
use std::ops::Deref;
use crate::log::Loggable;
use crate::form::refr::TESObjectREFR;

struct TESCharacter(TESObjectREFR);

struct State {
    character_vtable: usize,
    character_load: fn(&TESCharacter, u64) -> u64,
}
unsafe impl Sync for State {}
static S: LateStatic<State> = LateStatic::new();

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("character_vtable", &(self.character_vtable as usize))
            .field("character_load", &(self.character_load as usize))
            .finish()
    }
}

impl TESCharacter {
    fn new_load(&self, arg: u64) -> u64 {
        let ret = (S.character_load)(self, arg);
        let base_form = if !self.0.base_form.is_null() {
            unsafe { &*self.0.base_form }
        } else {
            return ret;
        };
        let form_id = self.0.form.form_id;
        let result: anyhow::Result<()> = try {
            db::DB.lock().map_err(|e| anyhow!(e.to_string()))?.execute(
                "INSERT INTO actor (form_id, base_form_id) VALUES (?1, ?2)\
             ON CONFLICT(form_id) DO UPDATE SET base_form_id=excluded.base_form_id",
                params![form_id, base_form.form_id],
            )?;
        };
        result.logging_ok();
        return ret;
    }
}

pub(crate) unsafe fn init(image_base: usize) -> anyhow::Result<()> {
    let character_vtable = transmute(image_base + 0x165da40);

    let original_character_load = patch_bytes(
        &(TESCharacter::new_load as usize),
        (character_vtable + 0x30) as *mut c_void,
        8,
    )?;

    LateStatic::assign(&S, State {
        character_vtable,
        character_load: transmute(*(original_character_load.as_ptr() as *const usize)),
    });

    output_debug_string(format!("S: {:#x?}", S.deref()).as_str());

    Ok(())
}