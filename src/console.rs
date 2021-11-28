use std::ffi::{CStr, CString};
use std::intrinsics::transmute;
use std::io::Write;

use anyhow::Context;
use detour::static_detour;
use late_static::LateStatic;
use winapi::_core::prelude::v1::Iterator;
use winapi::ctypes::{c_char, c_void};

use crate::log::Loggable;
use crate::{app, log};

static_detour! {
    static ProcessConsoleInput: fn(usize, i64, i64, i64);
}

fn new_process_console_input(param1: usize, param2: i64, param3: i64, param4: i64) {
    let input = unsafe { CStr::from_ptr(*((param1 + 0x38) as *const *const c_char)).to_str() };
    let result = match input {
        Ok(input) => {
            {
                let mut log = log::LOG.lock().unwrap();
                log.write_all(input.as_bytes()).ok();
                log.write_all("\n".as_bytes()).ok();
            }
            app::process_console_input(input)
        }
        Err(err) => {
            print(err.to_string().as_str());
            Ok(app::ProcessResult::Fallback)
        }
    };
    match result {
        Ok(app::ProcessResult::Processed) => {}
        Err(err) => print(format!("{:#}", err)),
        Ok(app::ProcessResult::Fallback) => {
            ProcessConsoleInput.call(param1, param2, param3, param4)
        }
        Ok(app::ProcessResult::FallbackAndPrintUsage) => {
            ProcessConsoleInput.call(param1, param2, param3, param4);
            print("skyrim-search-se usage: ss --help");
        }
    }
}

struct State {
    console_context: *const *const c_void,
    print_to_console: extern "C" fn(*const c_void, *const c_char, ...) -> (),
}
unsafe impl Sync for State {}
static S: LateStatic<State> = LateStatic::new();

pub(crate) fn print<T: Into<Vec<u8>>>(msg: T) {
    let msg = msg.into();
    {
        let mut log = log::LOG.lock().unwrap();
        log.write_all(msg.as_slice()).ok();
        log.write_all("\n".as_bytes()).ok();
    }
    let msg = String::from_utf8_lossy(msg.as_ref());
    let msgs = msg.split("\n");
    // The print_to_console's internal buffer size is 1024.
    // ensure each lines not to overflow
    let chunks = msgs.flat_map(|msg| msg.as_bytes().chunks(1024));
    let chunks: Vec<Result<CString, _>> = chunks.map(CString::new).collect();

    let result: anyhow::Result<()> = try {
        unsafe {
            let console_context = S.console_context;
            if *console_context != std::ptr::null() {
                for msg in chunks {
                    (S.print_to_console)(
                        *console_context,
                        "%s\0".as_ptr() as *const c_char,
                        msg?.as_c_str().as_ptr(),
                    );
                }
            }
        }
    };

    result.logging_ok();
}

pub(crate) unsafe fn init(image_base: usize) -> anyhow::Result<()> {
    LateStatic::assign(
        &S,
        State {
            console_context: transmute(image_base + 0x2F9A800),//score 1 0x2f000f0 -> 142F9A800
            print_to_console: transmute(image_base + 0x889650),//score 0.955 0x85c290 -> 140889650
        },
    );

    let target_addr = transmute(image_base + 0x2FBA00);//score 0.615  0x2e75f0 -> 1402FBA00
    ProcessConsoleInput
        .initialize(target_addr, new_process_console_input)
        .context("initialize")?;
    ProcessConsoleInput.enable().context("enable")?;

    Ok(())
}
