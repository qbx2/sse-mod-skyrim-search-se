mod log;

use std::ffi::{c_void, CString, CStr};
use win_dbg_logger::output_debug_string;
use std::fmt::{Debug, Formatter};
use std::{fmt, ptr};
use winapi::um::libloaderapi::GetModuleHandleA;
use detour::static_detour;
use std::intrinsics::transmute;
use std::os::raw::c_char;
use crate::log::get_log;
use std::io::Write;

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
        return false;
    }

    info.info_version = InfoVersion::KInfoVersion as u32;
    info.name = "skyrim-search-se\0".as_ptr() as *const c_char;
    info.version = 1;
    return true;
}

static_detour! {
    static ProcessConsoleInput: fn(*const c_void, i64, i64, i64) -> i64;
}

fn new_process_console_input(param1: *const c_void, param2: i64, param3: i64, param4: i64) -> i64 {
    let input = unsafe {
        CStr::from_ptr(*(param1.offset(0x38) as *const *const c_char)).to_str().unwrap()
    };
    print(format!("this is test; input = {}", input)).ok();
    return ProcessConsoleInput.call(param1, param2, param3, param4);
}

static mut CONSOLE_CONTEXT: Option<*const *const c_void> = None;
static mut PRINT_TO_CONSOLE: Option<fn(*const c_void, *const c_char) -> ()> = None;

fn print<T: Into<Vec<u8>>>(msg: T) -> Result<(), Box<dyn std::error::Error>> {
    let msg = String::from_utf8(msg.into())?;
    output_debug_string(msg.as_str());
    let msg = CString::new(msg)?;
    unsafe {
        if let Some(print_to_console) = PRINT_TO_CONSOLE {
            if let Some(console_context) = CONSOLE_CONTEXT {
                if *console_context != ptr::null() {
                    print_to_console(*console_context, msg.as_c_str().as_ptr());
                }
            }
        }
    }
    Ok(())
}

#[no_mangle]
pub extern fn SKSEPlugin_Load(skse: *const SKSEInterface) -> bool {
    let skse = unsafe { &*skse };
    let log = get_log();
    output_debug_string(format!("skse load: {:#?}", skse).as_str());

    let result: Result<bool, Box<dyn std::error::Error>> = (|| unsafe {
        let image_base = GetModuleHandleA(ptr::null()) as *const c_void;

        CONSOLE_CONTEXT = Some(transmute(image_base.offset(0x2f000f0)));
        PRINT_TO_CONSOLE = Some(transmute(image_base.offset(0x85c290)));

        let target_addr = transmute(image_base.offset(0x2e75f0));

        ProcessConsoleInput.initialize(target_addr, new_process_console_input)?;
        ProcessConsoleInput.enable()?;

        Ok(true)
    })();

    if let Err(err) = result {
        log.write_all(format!("error SKSEPlugin_Load: {}", err).as_bytes()).unwrap();
        return false;
    }

    print("SkyrimSearchSe is ready").ok();

    return true;
}
