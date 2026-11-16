# grim-dpad

In Grim Dawn's Steam Input mode, d-pad up and down open the Riftgate and the map but never
cycle the highlighted loot on the ground, as they do with a native gamepad. This `dinput8.dll`
makes the Riftgate and map actions cycle the loot while some is highlighted, as the native
gamepad's d-pad does, and leaves them alone otherwise.

It finds the game's own loot cycling code by its instructions rather than by address, and does
nothing if it cannot find it, which keeps it safe across most game updates.

## Install

1. Build it: `cargo build --release --target x86_64-pc-windows-gnu`.
2. Copy `target/x86_64-pc-windows-gnu/release/dinput8.dll` into the game's `x64` folder.
3. On Linux (Proton), add `WINEDLLOVERRIDES="dinput8=n,b" %command%` to the game's launch
   options.

It writes what it did to `grim-dpad.log` next to the DLL.

## License

MIT or Apache-2.0, at your option.
