use crate::db;
use crate::db::Job;
use crate::form::refr::TESObjectREFR;
use crate::log::Loggable;
use crate::patch::patch_bytes;
use anyhow::{anyhow, Context};
use late_static::LateStatic;
use rusqlite::params;
use std::fmt::Formatter;
use std::mem::transmute;
use std::ops::Deref;
use std::sync::mpsc::Sender;
use win_dbg_logger::output_debug_string;
use winapi::ctypes::c_void;

struct TESCharacter(TESObjectREFR);

struct State {
    character_vtable: usize,
    character_load: fn(&TESCharacter, u64) -> u64,
    task_queue: Sender<Job>,
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
        let result: anyhow::Result<()> = (|| {
            S.task_queue
                .send(Box::new(move |db| {
                    db.prepare_cached(
                        "INSERT OR REPLACE INTO actor (form_id, base_form_id) VALUES (?, ?);",
                    )
                    .context("chracter_new_load prepare")?
                    .execute(params![form_id, base_form.form_id])
                    .context("character_new_load execute")?;
                    Ok(())
                }))
                .map_err(|e| anyhow!(e.to_string()))?;
            Ok(())
        })();
        result.logging_ok();
        ret
    }
}

pub(crate) unsafe fn init(image_base: usize) -> anyhow::Result<()> {
    let character_vtable = transmute(image_base + 0x17547b0);//1.5.97: 0x165da40 -> 1.6.318: 0x1753670 -> addressLib ID: 207886 -> 1.6.323: 0x1753670 -> 1.6.342: 0x17547b0 -> 1.6.353: 0x17547b0

    let original_character_load = patch_bytes(
        &(TESCharacter::new_load as usize),
        (character_vtable + 0x30) as *mut c_void,
        8,
    )?;

    LateStatic::assign(
        &S,
        State {
            character_vtable,
            character_load: transmute(*(original_character_load.as_ptr() as *const usize)),
            task_queue: db::TASK_QUEUE.lock().unwrap().clone(),
        },
    );

    output_debug_string(format!("S: {:#x?}", S.deref()).as_str());

    Ok(())
}
