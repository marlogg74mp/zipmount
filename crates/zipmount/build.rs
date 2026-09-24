// WinFsp is linked with DELAYLOAD: winfsp-x64.dll is looked up in the
// driver's install directory at run time, not next to the executable. The
// winfsp crate requires this call from the final binary itself.
fn main() {
    #[cfg(windows)]
    winfsp::build::winfsp_link_delayload();
}
