//! A `dinput8.dll` for Grim Dawn that lets the d-pad cycle ground loot in Steam Input mode.
//! The game's `DirectInput.dll` loads it, and it passes `DirectInput8Create` on to the system's.

pub mod bindings;

#[cfg(windows)]
mod hook;
