# grim-dpad

In Grim Dawn's Steam Input mode, d-pad up and down open the Riftgate and the map but never
cycle the highlighted loot on the ground, as they do with a native gamepad. This `dinput8.dll`
makes the game treat those two actions as d-pad presses, so they behave as they do natively.

## Install

1. Build it: `cargo build --release --target x86_64-pc-windows-gnu`.
2. Copy `target/x86_64-pc-windows-gnu/release/dinput8.dll` into the game's `x64` folder.
3. On Linux (Proton), add `WINEDLLOVERRIDES="dinput8=n,b" %command%` to the game's launch
   options.

It writes what it did to `grim-dpad.log` next to the DLL.

## License

MIT or Apache-2.0, at your option.
