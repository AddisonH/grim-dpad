use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{E_FAIL, HMODULE, MAX_PATH};
use windows_sys::Win32::System::LibraryLoader::{
    DisableThreadLibraryCalls, GetModuleFileNameW, GetProcAddress, LoadLibraryW,
};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

static MODULE: OnceLock<usize> = OnceLock::new();

#[unsafe(no_mangle)]
extern "system" fn DllMain(module: HMODULE, reason: u32, _: *mut c_void) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        MODULE.get_or_init(|| module as usize);
        unsafe { DisableThreadLibraryCalls(module) };
        // Patching the game is left to a thread, out of the loader lock.
        std::thread::spawn(|| {
            log(&match unsafe { crate::loot::install() } {
                Ok(()) => "d-pad up and down now cycle the highlighted loot".to_string(),
                Err(e) => format!("not hooked: {e}"),
            })
        });
    }
    1
}

type Create =
    unsafe extern "system" fn(HMODULE, u32, *const c_void, *mut *mut c_void, *mut c_void) -> i32;

#[unsafe(no_mangle)]
unsafe extern "system" fn DirectInput8Create(
    instance: HMODULE,
    version: u32,
    iid: *const c_void,
    out: *mut *mut c_void,
    outer: *mut c_void,
) -> i32 {
    static CREATE: OnceLock<Option<Create>> = OnceLock::new();
    let create = CREATE.get_or_init(|| unsafe {
        let mut dir = [0u16; MAX_PATH as usize];
        let len = GetSystemDirectoryW(dir.as_mut_ptr(), MAX_PATH) as usize;
        let path: Vec<u16> = dir[..len]
            .iter()
            .copied()
            .chain("\\dinput8.dll\0".encode_utf16())
            .collect();
        let system = LoadLibraryW(path.as_ptr());
        GetProcAddress(system, c"DirectInput8Create".as_ptr().cast())
            .map(|f| std::mem::transmute(f))
    });
    match create {
        Some(create) => unsafe { create(instance, version, iid, out, outer) },
        None => E_FAIL,
    }
}

/// Appends a line to `grim-dpad.log` next to this DLL.
fn log(line: &str) {
    let mut path = [0u16; MAX_PATH as usize];
    let module = *MODULE.get().unwrap_or(&0) as HMODULE;
    let len = unsafe { GetModuleFileNameW(module, path.as_mut_ptr(), MAX_PATH) } as usize;
    let mut path = PathBuf::from(String::from_utf16_lossy(&path[..len]));
    path.set_file_name("grim-dpad.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}
