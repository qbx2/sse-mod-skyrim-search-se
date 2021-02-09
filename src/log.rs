use std::io::LineWriter;
use std::ffi::CStr;
use std::fs::File;
use winapi::shared::minwindef::MAX_PATH;
use winapi::shared::ntdef::NULL;
use winapi::shared::windef::HWND;
use winapi::shared::winerror::S_OK;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::shlobj::{CSIDL_MYDOCUMENTS, CSIDL_FLAG_CREATE, SHGFP_TYPE_CURRENT, SHGetFolderPathA};

const LOG_PATH: &str = "\\My Games\\Skyrim Special Edition\\SKSE\\skyrim-search-se.log";

static mut LOG: Option<LineWriter<File>> = None;

pub(crate) unsafe fn open_log_file() -> Result<LineWriter<File>, Box<dyn std::error::Error>> {
    let mut path = Vec::with_capacity(MAX_PATH);
    let result = SHGetFolderPathA(
        NULL as HWND,
        CSIDL_MYDOCUMENTS | CSIDL_FLAG_CREATE,
        NULL,
        SHGFP_TYPE_CURRENT,
        path.as_mut_ptr(),
    );
    if result != S_OK {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("failed to SHGetFolderPathA, ret = {}, err = {}", result, GetLastError()),
        ).into());
    }

    let path = String::from(CStr::from_ptr(path.as_ptr()).to_str()?) + LOG_PATH;

    let file = File::create(&path)?;
    Ok(LineWriter::new(file))
}

pub(crate) fn get_log() -> &'static mut LineWriter<File> {
    unsafe {
        return if let Some(ref mut log) = LOG {
            log
        } else {
            LOG = Some(open_log_file().unwrap());
            LOG.as_mut().unwrap()
        }
    }
}
