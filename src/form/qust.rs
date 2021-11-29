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

#[derive(Debug)]
pub(crate) struct TESQuest(TESForm);

#[repr(C)]
#[derive(Debug)]
pub(crate) struct LogEntry {
    unk00: u32,         // 00
    unk04: u32,         // 04
    unk08: u32,         // 08
    unk0c: u32,         // 0C
    string_offset: u32, // 10
    unk14: u16,         // 14
    has_cnam: u8,       // 16
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct LogEntryNode {
    entry: *const LogEntry,
    next: *const LogEntryNode,
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct Index {
    stage: u16,                    // 00
    flags: u16,                    // 02
    unk04: u32,                    // 04
    pub(crate) head: LogEntryNode, // 08
}

#[repr(C)]
#[derive(Debug)]
struct IndexNode {
    index: *const Index,    // 00
    next: *const IndexNode, // 08
}

struct State {
    quest_vtable: usize,
    quest_load: fn(&TESQuest, u64) -> u64,
    quest_get_edid: fn(&TESQuest) -> *const c_char,
    #[allow(dead_code)]
    quest_get_description: fn(&LogEntry, &TESQuest, u64, u64) -> *const c_char,
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

pub(crate) struct IndexNodeIterator(*const IndexNode);
pub(crate) struct LogEntryNodeIterator(*const LogEntryNode);

impl Iterator for LogEntryNodeIterator {
    type Item = *const LogEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_null() {
            return None;
        }
        let result = unsafe { &*self.0 };
        self.0 = result.next;
        if result.entry.is_null() {
            return None;
        }
        return Some(result.entry);
    }
}

impl IntoIterator for &LogEntryNode {
    type Item = *const LogEntry;
    type IntoIter = LogEntryNodeIterator;

    fn into_iter(self) -> Self::IntoIter {
        LogEntryNodeIterator(self as *const LogEntryNode)
    }
}

impl Iterator for IndexNodeIterator {
    type Item = *const Index;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_null() {
            return None;
        }
        let result = unsafe { &*self.0 };
        self.0 = result.next;
        if result.index.is_null() {
            return None;
        }
        return Some(result.index);
    }
}

impl IntoIterator for &IndexNode {
    type Item = *const Index;
    type IntoIter = IndexNodeIterator;

    fn into_iter(self) -> Self::IntoIter {
        IndexNodeIterator(self as *const IndexNode)
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

    fn get_head(&self) -> Option<&IndexNode> {
        unsafe {
            let head = (transmute::<_, usize>(self) + 0xe8) as *const IndexNode;
            if head.is_null() {
                return None;
            }
            Some(&*head)
        }
    }

    fn traverse(&self) -> Vec<(&Index, Vec<&LogEntry>)> {
        let head = match self.get_head() {
            Some(head) => head,
            None => return vec![],
        };
        let mut vec: Vec<(&Index, Vec<&LogEntry>)> = Vec::with_capacity(1);
        for index in head.into_iter() {
            let mut entry_nodes = Vec::new();

            unsafe {
                for log_entry in (&*index).head.into_iter() {
                    entry_nodes.push(&*log_entry);
                }

                vec.push((&*index, entry_nodes));
            }
        }
        vec
    }

    pub(crate) fn get_log(&self, stage: u16) -> Option<&Index> {
        unsafe {
            let head = self.get_head()?;
            for index in head.into_iter() {
                let index = &*index;
                if index.stage == stage {
                    return Some(index);
                }
            }
        };
        return None;
    }

    // NOTE: This function only works when a save has been loaded.
    pub(crate) fn get_log_description(&self, log: &LogEntry) -> std::borrow::Cow<str> {
        let s = (S.quest_get_description)(log, self, 0, 0);
        if s.is_null() {
            return std::borrow::Cow::from("");
        }
        unsafe { CStr::from_ptr(s).to_string_lossy() }
    }

    fn new_load(&self, arg: u64) -> u64 {
        let ret = (S.quest_load)(self, arg);
        let form_id = self.0.form_id;
        let editor_id = self.get_edid().map(|name| name.to_string());
        let name = self.0.get_name().map(|name| name.to_string());
        let result: anyhow::Result<()> = try {
            S.task_queue
                .send(Box::new(move |db| {
                    db.prepare_cached(
                        "INSERT OR REPLACE INTO quest (form_id, editor_id, name) VALUES (?, ?, ?);",
                    )
                    .context("quest_new_load prepare")?
                    .execute(params![form_id, editor_id, name])
                    .context("quest_new_load execute")?;

                    Ok(())
                }))
                .map_err(|e| anyhow!(e.to_string()))?;

            for (index, log_entries) in self.traverse().iter() {
                for log in log_entries.iter() {
                    let stage = index.stage;
                    let log_string_offset = log.string_offset;
                    if log_string_offset == 4294967295 {//that's 2^32 => prevent overflow, probably
                        continue;
                    }
                    S.task_queue.send(Box::new(move |db| {
                        db.prepare_cached(
                            "INSERT OR REPLACE INTO quest_stage (form_id, stage, log) VALUES (?, ?, ?);",
                        ).context("quest_new_load prepare")?
                            .execute(params![form_id, stage, log_string_offset])
                            .context("quest_new_load execute")?;

                        Ok(())
                    })).map_err(|e| anyhow!(e.to_string()))?;
                }
            }
        };
        result.logging_ok();
        return ret;
    }
}

pub(crate) unsafe fn init(image_base: usize) -> anyhow::Result<()> {
    let quest_vtable = transmute(image_base + 0x1699720);//1.5.97: 0x15a1c98 -(score 1.0)-> 1.6.318: 0x1699720 -> addressLib ID: 195890 -> 1.6.323: 0x1699720
    let quest_get_description = transmute(image_base + 0x398f70);//1.5.97: 0x382720 -(score 0.988)-> 1.6.318: 0x399000 -> addressLib ID: 25259 -> 1.6.323: 0x398f70

    let original_quest_load = patch_bytes(
        &(TESQuest::new_load as usize),
        (quest_vtable + 0x30) as *mut c_void,
        8,
    )?;

    LateStatic::assign(
        &S,
        State {
            quest_vtable,
            quest_load: transmute(*(original_quest_load.as_ptr() as *const usize)),
            quest_get_edid: transmute(*((quest_vtable + 0x190) as *const usize)),
            quest_get_description,
            task_queue: db::TASK_QUEUE.lock().unwrap().clone(),
        },
    );

    output_debug_string(format!("S: {:#x?}", S.deref()).as_str());

    Ok(())
}
