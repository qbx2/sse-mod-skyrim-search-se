#![feature(try_blocks)]
#![feature(try_trait)]

mod app;
mod console;
mod db;
mod form;
mod log;
mod patch;

use crate::log::Loggable;
use anyhow::Context;
use std::fmt::{Debug, Formatter};
use std::io::Write;
use std::os::raw::c_char;
use std::{fmt, ptr};
use win_dbg_logger::output_debug_string;
use winapi::ctypes::c_void;
use winapi::um::libloaderapi::GetModuleHandleA;

const DEBUG: bool = true;

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
            .field("plugin_handle", &(self.get_plugin_handle)())
            .field("release_index", &(self.get_release_index)())
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
pub extern "C" fn SKSEPlugin_Query(skse: *const SKSEInterface, info: *mut PluginInfo) -> bool {
    let skse = unsafe { &*skse };
    let mut info = unsafe { &mut *info };

    if skse.runtime_version != 0x01050610 {
        // 1.5.97
        output_debug_string(
            format!("runtime_version mismatch: {:#x}", skse.runtime_version).as_str(),
        );
        return false;
    }

    info.info_version = InfoVersion::KInfoVersion as u32;
    info.name = "skyrim-search-se\0".as_ptr() as *const c_char;
    info.version = 1;
    return true;
}

#[no_mangle]
pub extern "C" fn SKSEPlugin_Load(skse: *const SKSEInterface) -> bool {
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        output_debug_string(msg.as_str());
        if let Ok(mut w) = log::LOG.lock() {
            w.write_all(msg.as_bytes()).ok();
        }
    }));

    let skse = unsafe { &*skse };
    lazy_static::initialize(&log::LOG);
    output_debug_string(format!("ssse skse load: {:#?}", skse).as_str());

    let result: anyhow::Result<()> = try {
        unsafe {
            let image_base = GetModuleHandleA(ptr::null()) as usize;

            console::init(image_base).context("console::init")?;
            form::init(image_base).context("form::init")?;
            app::init(image_base).context("form::init")?;
        }
    };

    lazy_static::initialize(&db::DB);

    if let Err(err) = result {
        log::LOG
            .lock()
            .unwrap()
            .write_all(format!("error SKSEPlugin_Load: {}\n", err).as_bytes())
            .unwrap();
        return false;
    }

    log::LOG
        .lock()
        .unwrap()
        .write_all("SkyrimSearchSe is ready".as_bytes())
        .logging_ok();

    output_debug_string("SkyrimSearchSe is ready");

    return true;
}
