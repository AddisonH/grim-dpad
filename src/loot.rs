//! In Steam Input mode, cycles the highlighted loot on the Riftgate and map actions, which are
//! on d-pad up and down, as the native gamepad's d-pad does.
//!
//! The in-game UI handles native gamepad events and Steam controller events in two adjacent
//! virtual methods. The native one cycles loot; the Steam one is wrapped to do the same.

use std::sync::OnceLock;

use windows_sys::Win32::System::Diagnostics::Debug::{
    IMAGE_NT_HEADERS64, IMAGE_SECTION_HEADER, RtlLookupFunctionEntry,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{PAGE_READWRITE, VirtualProtect};
use windows_sys::Win32::System::SystemServices::IMAGE_DOS_HEADER;

use crate::pattern::{self, LootList};

/// Controller events of the Riftgate and map actions.
const RIFTGATE: u32 = 0x10;
const MAP_WINDOW: u32 = 0x16;

type Handle = unsafe extern "system" fn(*mut u8, *const u8) -> u64;

static HOOK: OnceLock<(Handle, LootList)> = OnceLock::new();

pub unsafe fn install() -> Result<(), String> {
    unsafe {
        let exe = GetModuleHandleW(std::ptr::null()) as *mut u8;
        let section = |name: &[u8]| {
            let nt = &*(exe.add((*(exe as *const IMAGE_DOS_HEADER)).e_lfanew as usize)
                as *const IMAGE_NT_HEADERS64);
            let first = (nt as *const IMAGE_NT_HEADERS64).add(1) as *const IMAGE_SECTION_HEADER;
            std::slice::from_raw_parts(first, nt.FileHeader.NumberOfSections as usize)
                .iter()
                .find(|s| s.Name.starts_with(name))
                .map(|s| {
                    std::slice::from_raw_parts_mut(
                        exe.add(s.VirtualAddress as usize),
                        s.Misc.VirtualSize as usize,
                    )
                })
                .ok_or_else(|| format!("no {} section", String::from_utf8_lossy(name)))
        };
        let text = section(b".text\0")?;
        let (at, loot) = pattern::find(text)?;

        let mut image_base = 0;
        let entry = RtlLookupFunctionEntry(
            text.as_ptr().add(at) as u64,
            &mut image_base,
            std::ptr::null_mut(),
        );
        if entry.is_null() {
            return Err("the native gamepad handler has no unwind entry".into());
        }
        let native = image_base as usize + (*entry).BeginAddress as usize;

        let rdata = section(b".rdata\0")?;
        let slots: Vec<*mut usize> = rdata
            .as_chunks_mut::<8>()
            .0
            .iter_mut()
            .map(|c| c.as_mut_ptr() as *mut usize)
            .filter(|&p| *p == native)
            .collect();
        let [slot] = slots[..] else {
            return Err(format!(
                "{} vtable slots hold the native gamepad handler",
                slots.len()
            ));
        };
        let slot = slot.add(1);

        HOOK.get_or_init(|| (std::mem::transmute::<usize, Handle>(*slot), loot));
        let mut old = 0;
        VirtualProtect(slot.cast(), 8, PAGE_READWRITE, &mut old);
        *slot = handle as Handle as usize;
        VirtualProtect(slot.cast(), 8, old, &mut old);
        Ok(())
    }
}

/// The in-game UI's Steam controller event handler.
unsafe extern "system" fn handle(ui: *mut u8, event: *const u8) -> u64 {
    unsafe {
        let (original, loot) = HOOK.get().unwrap();
        let pressed = *event.add(0xc) != 0;
        let step = match *(event as *const u32) {
            RIFTGATE => -1,
            MAP_WINDOW => 1,
            _ => 0,
        };
        if pressed && step != 0 {
            let list = *(ui.add(loot.list) as *const *mut u8);
            if !list.is_null() && *(list.add(loot.count) as *const i32) != 0 {
                *(list.add(loot.selected) as *mut i32) += step;
                return 1;
            }
        }
        original(ui, event)
    }
}
