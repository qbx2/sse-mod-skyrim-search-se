extern crate proc_macro;

use once_cell::sync::OnceCell;
use proc_macro::TokenStream;
use syn::{parse_macro_input, LitInt};
use versionlib::VersionlibData;

fn get_versionlib_data() -> &'static VersionlibData {
    static VERSIONLIB_DATA: OnceCell<VersionlibData> = OnceCell::new();
    VERSIONLIB_DATA.get_or_init(|| {
        let target_version = std::fs::read("target_version.txt").unwrap();
        let target_version = String::from_utf8(target_version).unwrap();
        versionlib::load(&format!(
            "versionlib/bin/versionlib-{}.bin",
            target_version.replace(".", "-").trim()
        ))
        .unwrap()
    })
}

#[proc_macro]
pub fn versionlib(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitInt);

    let data = get_versionlib_data();
    let address_id: u64 = input.base10_parse().unwrap();
    let value = data.data[&address_id];
    format!("{value:#x}").parse().unwrap()
}

#[proc_macro]
pub fn target_version(_input: TokenStream) -> TokenStream {
    let data = get_versionlib_data();
    let version = data.version;
    let packed = ((version[0] & 0xff) << 24)
        | ((version[1] & 0xff) << 16)
        | ((version[2] & 0xfff) << 4)
        | (version[3] & 0xf);
    format!("{packed:#x}").parse().unwrap()
}
