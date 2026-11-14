// The `versionlib!` / `target_version!` macros read these files at compile time.
// Cargo does not track files read by proc macros, so declare them here: switching
// the target version then rebuilds the crate instead of silently reusing old offsets.
fn main() {
    println!("cargo:rerun-if-changed=target_version.txt");
    println!("cargo:rerun-if-changed=versionlib/bin");
}
