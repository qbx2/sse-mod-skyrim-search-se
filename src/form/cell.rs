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

struct TESObjectCELL(TESForm);

struct State {
    cell_vtable: usize,
    cell_load: fn(&TESObjectCELL, u64) -> u64,
    cell_get_edid: fn(&TESObjectCELL) -> *const c_char,
    task_queue: Sender<Job>,
}
unsafe impl Sync for State {}
static S: LateStatic<State> = LateStatic::new();

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("cell_vtable", &(self.cell_vtable as usize))
            .field("cell_load", &(self.cell_load as usize))
            .finish()
    }
}

impl TESObjectCELL {
    fn get_edid(&self) -> Option<std::borrow::Cow<str>> {
        unsafe {
            let result = (S.cell_get_edid)(self);
            if result.is_null() {
                return None;
            }
            Some(CStr::from_ptr(result).to_string_lossy())
        }
    }

    fn new_load(&self, arg: u64) -> u64 {
        let ret = (S.cell_load)(self, arg);
        let form_id = self.0.form_id;
        let editor_id = self.get_edid().map(|name| name.to_string());
        let name = self.0.get_name().map(|name| name.to_string());
        let result: anyhow::Result<()> =
            (|| {
                S.task_queue
                    .send(Box::new(move |db| {
                        db.prepare_cached(
                "INSERT OR REPLACE INTO cell (form_id, editor_id, name) VALUES (?, ?, ?);",
            ).context("cell_new_load prepare")?
                .execute(params![form_id, editor_id, name]).context("cell_new_load execute")?;
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
    let cell_vtable = transmute(image_base + 0x1566060);

    let original_cell_load = patch_bytes(
        &(TESObjectCELL::new_load as usize),
        (cell_vtable + 0x30) as *mut c_void,
        8,
    )?;

    LateStatic::assign(
        &S,
        State {
            cell_vtable,
            cell_load: transmute(*(original_cell_load.as_ptr() as *const usize)),
            cell_get_edid: transmute(*((cell_vtable + 0x190) as *const usize)),
            task_queue: db::TASK_QUEUE.lock().unwrap().clone(),
        },
    );

    output_debug_string(format!("S: {:#x?}", S.deref()).as_str());

    Ok(())
}
