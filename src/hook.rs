use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use windows_sys::Win32::Foundation::{E_FAIL, HMODULE, MAX_PATH};
use windows_sys::Win32::System::LibraryLoader::{
    DisableThreadLibraryCalls, GetModuleFileNameW, GetModuleHandleA, GetProcAddress, LoadLibraryW,
};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

use crate::bindings::{self, Binding};

static MODULE: OnceLock<usize> = OnceLock::new();

#[unsafe(no_mangle)]
extern "system" fn DllMain(module: HMODULE, reason: u32, _: *mut c_void) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        MODULE.get_or_init(|| module as usize);
        unsafe { DisableThreadLibraryCalls(module) };
        std::thread::spawn(run);
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

fn run() {
    let result = (0..2400)
        .find_map(|_| {
            std::thread::sleep(Duration::from_millis(250));
            unsafe { remap() }
        })
        .unwrap_or_else(|| Err("the game's bindings never appeared".into()));
    log(&match result {
        Ok(done) => format!("Riftgate and map bindings {done}"),
        Err(e) => format!("left the bindings alone: {e}"),
    });
}

/// Remaps the bindings once the game has added them all.
unsafe fn remap() -> Option<Result<&'static str, String>> {
    unsafe {
        let game = GetModuleHandleA(c"Game.dll".as_ptr().cast());
        let engine = GetModuleHandleA(c"Engine.dll".as_ptr().cast());
        if game.is_null() || engine.is_null() {
            return None;
        }
        let game_engine = GetProcAddress(
            game,
            c"?gGameEngine@GAME@@3PEAVGameEngine@1@EA".as_ptr().cast(),
        )?;
        if (*(game_engine as *const *const c_void)).is_null() {
            return None;
        }
        // `Get` creates the Steamworks object if it is missing, so it is only called once the
        // game engine, which sets Steamworks up, exists.
        let steamworks: unsafe extern "system" fn() -> *const u8 = std::mem::transmute(
            GetProcAddress(engine, c"?Get@Steamworks@GAME@@SAPEAV12@XZ".as_ptr().cast())?,
        );
        let device: unsafe extern "system" fn(*const u8) -> *mut u8 =
            std::mem::transmute(GetProcAddress(
                engine,
                c"?ControllerGetDevice@Steamworks@GAME@@QEBAPEAVSteamControllerDevice@2@XZ"
                    .as_ptr()
                    .cast(),
            )?);
        let device = device(steamworks());
        if device.is_null() {
            return None;
        }
        let begin = *(device.add(0x60) as *const *mut Binding);
        let end = *(device.add(0x68) as *const *mut Binding);
        if begin.is_null() || end < begin {
            return None;
        }
        let bindings = std::slice::from_raw_parts_mut(begin, end.offset_from(begin) as usize);
        if !bindings
            .last()
            .is_some_and(|b| b.name.bytes() == bindings::LAST)
        {
            return None;
        }
        Some(bindings::remap(bindings))
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
