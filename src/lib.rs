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

const DEBUG: bool = false;

type PluginHandle = u32;

enum DataVersion {
    KVersion=1,
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct SKSEPluginVersionData {
    dataVersion: u32,

    pluginVersion: u32,
    name: [u8; 256],

    author: [u8; 256],
    supportEmail: [u8; 256],

    versionIndependence: u32,
    compatibleVersions: [u32; 16],

    seVersionRequired: u32,
}

const RUNTIME_VERSION_1_6_323: u32 = 0x01061430;

const fn zero_pad_u8<const N: usize, const M: usize>(arr: &[u8; N]) -> [u8; M] {
    let mut m = [0; M];
    let mut i = 0;
    while i < N {
        m[i] = arr[i];
        i += 1;
    }
    m
}

#[no_mangle]
pub static SKSEPlugin_Version: SKSEPluginVersionData = SKSEPluginVersionData {
    dataVersion: DataVersion::KVersion as u32,
    pluginVersion: 3,
    name: zero_pad_u8(b"Skyrim Search SE\0"),
    author: zero_pad_u8(b"qbx2, lukasaldersley\0"),
    supportEmail: zero_pad_u8(b"open a GitHub issue on qbx2's GitHub\0"),
    versionIndependence: 0,
    compatibleVersions: [RUNTIME_VERSION_1_6_323,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
    seVersionRequired: 0,
};

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

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn SKSEPlugin_Query(skse: *const SKSEInterface, info: *mut PluginInfo) -> bool {
    let skse = unsafe { &*skse };
    let mut info = unsafe { &mut *info };

    if skse.runtime_version != RUNTIME_VERSION_1_6_323 {
        output_debug_string(
            format!("runtime_version mismatch: {:#x}", skse.runtime_version).as_str(),
        );
        return false;
    }

    info.info_version = InfoVersion::KInfoVersion as u32;
    info.name = "skyrim-search-se\0".as_ptr() as *const c_char;
    info.version = 1;
    true
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn SKSEPlugin_Load(skse: *const SKSEInterface) -> bool {
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        output_debug_string(msg.as_str());
        if let Ok(mut w) = log::LOG.lock() {
            w.write_all((msg + "\n").as_bytes()).ok();
        }
    }));

    let skse = unsafe { &*skse };
    lazy_static::initialize(&log::LOG);
    output_debug_string(format!("ssse skse load: {:#?}", skse).as_str());

    let result: anyhow::Result<()> = (|| {
        unsafe {
            let image_base = GetModuleHandleA(ptr::null()) as usize;

            console::init(image_base).context("console::init")?;
            form::init(image_base).context("form::init")?;
            app::init(image_base).context("app::init")?;
        }

        Ok(())
    })();

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
        .write_all("SkyrimSearchSe is ready\n".as_bytes())
        .logging_ok();

    output_debug_string("SkyrimSearchSe is ready");

    true
}
