use anyhow::{anyhow, Context};
use winapi::ctypes::{c_void, c_char};
use win_dbg_logger::output_debug_string;
use crate::patch::patch_bytes;
use crate::db;
use rusqlite::params;
use std::mem::transmute;
use late_static::LateStatic;
use std::fmt::Formatter;
use std::ops::Deref;
use crate::log::Loggable;
use std::sync::mpsc::Sender;
use crate::db::Job;
use crate::form::TESForm;
use std::ffi::CStr;

struct TESQuest(TESForm);

#[repr(C)]
#[derive(Debug)]
struct LogEntry {
    unk00: u32, // 00
    unk04: u32, // 04
    unk08: u32, // 08
    unk0c: u32, // 0C
    string_offset: u32, // 10
    unk14: u16,         // 14
    has_cnam: u8,       // 16
}

#[repr(C)]
#[derive(Debug)]
struct LogEntryNode {
    entry: *const LogEntry,
    next: *const LogEntryNode,
}

#[repr(C)]
#[derive(Debug)]
struct Index {
    stage: u16, // 00
    flags: u16, // 02
    unk04: u32, // 04
    head: LogEntryNode, // 08
}

#[repr(C)]
#[derive(Debug)]
struct IndexNode {
    index: *const Index, // 00
    next: *const IndexNode, // 08
}

struct State {
    quest_vtable: usize,
    quest_load: fn(&TESQuest, u64) -> u64,
    quest_get_edid: fn(&TESQuest) -> *const c_char,
    task_queue: Sender<Job>,
}
unsafe impl Sync for State {}
static S: LateStatic<State> = LateStatic::new();

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("quest_vtable", &(self.quest_vtable as usize))
            .field("quest_load", &(self.quest_load as usize))
            .finish()
    }
}

impl TESQuest {
    fn get_edid(&self) -> Option<std::borrow::Cow<str>> {
        unsafe {
            let result = (S.quest_get_edid)(self);
            if result.is_null() {
                return None;
            }
            Some(CStr::from_ptr(result).to_string_lossy())
        }
    }

    fn traverse_log_entry_nodes(index: &Index) -> Vec<&LogEntry> {
        unsafe {
            let head = &index.head as *const LogEntryNode;
            if head.is_null() {
                return vec![];
            }
            let mut node = head;
            let entry = (*node).entry;
            if entry.is_null() {
                return vec![];
            }
            let mut vec: Vec<&LogEntry> = vec![&*entry];
            loop {
                node = (*node).next;
                if node.is_null() {
                    break;
                }
                let entry = (*node).entry;
                if !entry.is_null() {
                    vec.push(&*entry);
                }
            }
            vec
        }
    }

    fn traverse(&self) -> Vec<(&Index, Vec<&LogEntry>)> {
        unsafe {
            let head = (transmute::<_, usize>(self) + 0xe8) as *const IndexNode;
            if head.is_null() {
                return vec![];
            }
            let mut node = head;
            let index = (*node).index;
            if index.is_null() {
                return vec![];
            }
            let entry_nodes = Self::traverse_log_entry_nodes(&*index);
            let mut vec: Vec<(&Index, Vec<&LogEntry>)> = vec![(&*index, entry_nodes)];
            loop {
                node = (*node).next;
                if node.is_null() {
                    break;
                }
                let index = (*node).index;
                if !index.is_null() {
                    let entry_nodes = Self::traverse_log_entry_nodes(&*index);
                    vec.push((&*index, entry_nodes));
                }
            }
            vec
        }
    }

    fn new_load(&self, arg: u64) -> u64 {
        let ret = (S.quest_load)(self, arg);
        let form_id = self.0.form_id;
        let editor_id = self.get_edid().map(|name| name.to_string());
        let name = self.0.get_name().map(|name| name.to_string());
        let slf = unsafe { transmute::<&Self, &'static Self>(self) };
        let result: anyhow::Result<()> = try {
            S.task_queue.send(Box::new(move |db| {
                db.prepare_cached(
                    "INSERT OR REPLACE INTO quest (form_id, editor_id, name) VALUES (?, ?, ?);",
                ).context("quest_new_load prepare")?
                    .execute(params![form_id, editor_id, name]).context("quest_new_load execute")?;

                for (index, log_entries) in slf.traverse().iter() {
                    for log in log_entries.iter() {
                        db.prepare_cached(
                            "INSERT OR REPLACE INTO quest_stage (form_id, stage, log) VALUES (?, ?, ?);",
                        ).context("quest_new_load prepare")?
                            .execute(params![form_id, index.stage, log.string_offset]).context("quest_new_load execute")?;
                    }
                }

                Ok(())
            })).map_err(|e| anyhow!(e.to_string()))?;
        };
        result.logging_ok();
        return ret;
    }
}

pub(crate) unsafe fn init(image_base: usize) -> anyhow::Result<()> {
    let quest_vtable = transmute(image_base + 0x15a1c98);

    let original_quest_load = patch_bytes(
        &(TESQuest::new_load as usize),
        (quest_vtable + 0x30) as *mut c_void,
        8,
    )?;

    LateStatic::assign(&S, State {
        quest_vtable,
        quest_load: transmute(*(original_quest_load.as_ptr() as *const usize)),
        quest_get_edid: transmute(*((quest_vtable + 0x190) as *const usize)),
        task_queue: db::TASK_QUEUE.lock().unwrap().clone(),
    });

    output_debug_string(format!("S: {:#x?}", S.deref()).as_str());

    Ok(())
}
