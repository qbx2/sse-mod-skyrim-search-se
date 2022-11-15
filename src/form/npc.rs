use crate::db;
use crate::db::Job;
use crate::form::TESForm;
use crate::log::Loggable;
use crate::patch::patch_bytes;
use anyhow::{anyhow, Context};
use late_static::LateStatic;
use rusqlite::params;
use std::ffi::CStr;
use std::fmt::Formatter;
use std::mem::transmute;
use std::ops::Deref;
use std::sync::mpsc::Sender;
use win_dbg_logger::output_debug_string;
use winapi::ctypes::{c_char, c_void};

#[allow(clippy::upper_case_acronyms)]
struct TESNPC(TESForm);

struct State {
    npc_vtable: usize,
    npc_load: fn(&TESNPC, u64) -> u64,
    task_queue: Sender<Job>,
}
unsafe impl Sync for State {}
static S: LateStatic<State> = LateStatic::new();

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("npc_vtable", &(self.npc_vtable as usize))
            .field("npc_load", &(self.npc_load as usize))
            .finish()
    }
}

impl TESNPC {
    fn new_set_edid(&self, edid: *const c_char) -> bool {
        if edid.is_null() {
            return false;
        }
        let result: anyhow::Result<()> = (|| {
            let form_id = self.0.form_id;
            let edid = unsafe { CStr::from_ptr(edid).to_str()? }.to_string();

            S.task_queue
                .send(Box::new(move |db| {
                    db.prepare_cached(
                        "INSERT INTO npc (form_id, editor_id) VALUES (?, ?)\
                     ON CONFLICT(form_id) DO UPDATE SET editor_id=excluded.editor_id",
                    )
                    .context("npc_set_edid prepare")?
                    .execute(params![form_id, edid])
                    .context("npc_set_edid execute")?;
                    Ok(())
                }))
                .map_err(|e| anyhow!(e.to_string()))?;

            Ok(())
        })();
        result.logging_ok().is_some()
    }

    fn new_load(&self, arg: u64) -> u64 {
        let result = (S.npc_load)(self, arg);
        let form_id = self.0.form_id;
        if let Some(name) = self.0.get_name() {
            let result: anyhow::Result<()> = (|| {
                let name = name.to_string();
                S.task_queue
                    .send(Box::new(move |db| {
                        db.prepare_cached(
                            "INSERT INTO npc (form_id, name) VALUES (?, ?)\
                         ON CONFLICT(form_id) DO UPDATE SET name=excluded.name",
                        )
                        .context("npc_new_load prepare")?
                        .execute(params![form_id, name])
                        .context("npc_new_load execute")?;
                        Ok(())
                    }))
                    .map_err(|e| anyhow!(e.to_string()))?;

                Ok(())
            })();
            result.logging_ok();
        }
        result
    }
}

pub(crate) unsafe fn init(image_base: usize) -> anyhow::Result<()> {
    let npc_vtable = transmute(image_base + versionlib!(195816));

    output_debug_string(format!("npc set_edid: {:#x}", npc_vtable + 0x198).as_str());

    patch_bytes(
        &(TESNPC::new_set_edid as usize),
        (npc_vtable + 0x198) as *mut c_void,
        8,
    )?;
    let original_npc_load = patch_bytes(
        &(TESNPC::new_load as usize),
        (npc_vtable + 0x30) as *mut c_void,
        8,
    )?;

    LateStatic::assign(
        &S,
        State {
            npc_vtable,
            npc_load: transmute(*(original_npc_load.as_ptr() as *const usize)),
            task_queue: db::TASK_QUEUE.lock().unwrap().clone(),
        },
    );

    output_debug_string(format!("S: {:#x?}", S.deref()).as_str());

    Ok(())
}
