#![feature(try_blocks)]
#![feature(try_trait)]

mod log;
mod console;
mod form;
mod patch;
mod db;

use winapi::ctypes::c_void;
use win_dbg_logger::output_debug_string;
use std::fmt::{Debug, Formatter};
use std::{fmt, ptr};
use winapi::um::libloaderapi::GetModuleHandleA;
use std::os::raw::c_char;
use crate::log::get_log;
use std::io::Write;
use anyhow::Context;

type PluginHandle = u32;

#[repr(C)]
pub struct SKSEInterface {
    skse_version: u32,
    runtime_version: u32,
    editor_version: u32,
    is_editor: u32,
    query_interface: fn(u32) -> *mut c_void,

    get_plugin_handle: fn() -> PluginHandle,
    get_release_index: fn() -> u32,
    get_plugin_info: fn(&str) -> *const c_void,
}

impl Debug for SKSEInterface {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SKSEInterface")
            .field("skse_version", &self.skse_version)
            .field("runtime_version", &self.runtime_version)
            .field("plugin_handle",  &(self.get_plugin_handle)())
            .field("release_index",  &(self.get_release_index)())
            .finish()
    }
}

enum InfoVersion {
    KInfoVersion = 1,
}

#[repr(C)]
pub struct PluginInfo {
    info_version: u32,
    name: *const c_char,
    version: u32,
}

#[no_mangle]
pub extern fn SKSEPlugin_Query(skse: *const SKSEInterface, info: *mut PluginInfo) -> bool {
    let skse = unsafe { &*skse };
    let mut info = unsafe { &mut *info };

    if skse.runtime_version != 0x01050610 { // 1.5.97
        output_debug_string(format!("runtime_version mismatch: {:#x}", skse.runtime_version).as_str());
        return false;
    }

    info.info_version = InfoVersion::KInfoVersion as u32;
    info.name = "skyrim-search-se\0".as_ptr() as *const c_char;
    info.version = 1;
    return true;
}

#[no_mangle]
pub extern fn SKSEPlugin_Load(skse: *const SKSEInterface) -> bool {
    let skse = unsafe { &*skse };
    let log = get_log();
    output_debug_string(format!("ssse skse load: {:#?}", skse).as_str());

    let result: anyhow::Result<()> = try { unsafe {
        let image_base = GetModuleHandleA(ptr::null()) as usize;

        console::init(image_base).context("console::init")?;
        form::init(image_base).context("form::init")?;

        db::get_db();
    }};

    if let Err(err) = result {
        log.write_all(format!("error SKSEPlugin_Load: {}\n", err).as_bytes()).unwrap();
        return false;
    } else {
        console::print("SkyrimSearchSe is ready").ok();
    }

    return true;
}
