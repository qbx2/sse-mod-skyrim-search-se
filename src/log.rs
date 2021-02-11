use std::io::LineWriter;
use std::ffi::CStr;
use std::fs::File;
use std::sync::Mutex;
use winapi::shared::minwindef::MAX_PATH;
use winapi::shared::ntdef::NULL;
use winapi::shared::windef::HWND;
use winapi::shared::winerror::S_OK;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::shlobj::{CSIDL_MYDOCUMENTS, CSIDL_FLAG_CREATE, SHGFP_TYPE_CURRENT, SHGetFolderPathA};
use win_dbg_logger::output_debug_string;
use lazy_static::lazy_static;
use anyhow::Context;

const LOG_PATH: &str = "\\My Games\\Skyrim Special Edition\\SKSE\\skyrim-search-se.log";

lazy_static! {
    pub static ref LOG: Mutex<LineWriter<File>> = {
        match open_log_file().context("open_log_file error") {
            Ok(log) => Mutex::new(log),
            Err(err) => {
                output_debug_string(format!("{:#}", err).as_str());
                panic!(format!("{:#}", err));
            }
        }
    };
}

fn open_log_file() -> anyhow::Result<LineWriter<File>> {
    unsafe {
        let mut path = Vec::with_capacity(MAX_PATH);
        let result = SHGetFolderPathA(
            NULL as HWND,
            CSIDL_MYDOCUMENTS | CSIDL_FLAG_CREATE,
            NULL,
            SHGFP_TYPE_CURRENT,
            path.as_mut_ptr(),
        );
        if result != S_OK {
            anyhow::bail!("failed to SHGetFolderPathA, ret = {}, err = {}", result, GetLastError());
        }

        let path = String::from(CStr::from_ptr(path.as_ptr()).to_str()?) + LOG_PATH;

        let file = File::create(&path)?;
        Ok(LineWriter::new(file))
    }
}
