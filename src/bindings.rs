//! The game's Steam Input bindings. Each maps a Steam action, while an action set is active, to
//! one of the game's controller events. The game turns some events into gamepad buttons and
//! hands those to the same code as a native gamepad; d-pad up and down cycle the highlighted
//! loot there, and otherwise open the Riftgate and the map.

/// MSVC's `std::string`.
#[repr(C)]
pub struct MsvcString {
    buf: [u8; 16],
    len: usize,
    cap: usize,
}

impl MsvcString {
    /// # Safety
    /// Must be a live string of the game.
    pub unsafe fn bytes(&self) -> &[u8] {
        if self.cap < self.buf.len() {
            &self.buf[..self.len.min(self.buf.len())]
        } else {
            let ptr = usize::from_ne_bytes(self.buf[..8].try_into().unwrap()) as *const u8;
            unsafe { std::slice::from_raw_parts(ptr, self.len) }
        }
    }
}

/// When a digital binding sends events.
pub const ON_PRESS: i32 = 0;
pub const ON_PRESS_AND_RELEASE: i32 = 1;

/// Controller events.
pub const RIFTGATE: u32 = 0x10;
pub const MAP_WINDOW: u32 = 0x16;
/// Become d-pad up and down.
pub const SCROLL_UP: u32 = 0x1e;
pub const SCROLL_DOWN: u32 = 0x1f;

/// The last binding the game adds.
pub const LAST: &[u8] = b"ActionEvade";

#[repr(C)]
pub struct Binding {
    pub name: MsvcString,
    pub kind: i32,
    /// The action set the binding is active in, or -1 for all.
    pub set: i32,
    pub event: u32,
    pub digital_handle: u64,
    pub analog_handle: u64,
    pub dead_zone: f32,
    pub held: bool,
}

const _: () = assert!(size_of::<Binding>() == 0x48);

/// Sends the Riftgate and map actions as d-pad up and down, in game only, so they cycle loot
/// when some is highlighted.
///
/// # Safety
/// The bindings must be the game's.
pub unsafe fn remap(bindings: &mut [Binding]) -> Result<&'static str, String> {
    let find = |name: &[u8]| {
        bindings
            .iter()
            .position(|b| unsafe { b.name.bytes() } == name)
            .ok_or_else(|| format!("no {} binding", String::from_utf8_lossy(name)))
    };
    let riftgate = find(b"ActionRiftgate")?;
    let map = find(b"ActionMapWindow")?;
    match (bindings[riftgate].event, bindings[map].event) {
        (SCROLL_UP, SCROLL_DOWN) => return Ok("already remapped"),
        (RIFTGATE, MAP_WINDOW) => {}
        (r, m) => return Err(format!("unexpected events {r:#x} and {m:#x}")),
    }
    for i in [riftgate, map] {
        if bindings[i].kind != ON_PRESS {
            return Err(format!("unexpected binding kind {}", bindings[i].kind));
        }
    }
    let in_game = bindings[riftgate].set;
    for (i, event) in [(riftgate, SCROLL_UP), (map, SCROLL_DOWN)] {
        let b = &mut bindings[i];
        b.event = event;
        b.kind = ON_PRESS_AND_RELEASE;
        b.set = in_game;
    }
    Ok("remapped")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(name: &str, kind: i32, set: i32, event: u32) -> Binding {
        let mut buf = [0; 16];
        buf[..name.len()].copy_from_slice(name.as_bytes());
        Binding {
            name: MsvcString {
                buf,
                len: name.len(),
                cap: 15,
            },
            kind,
            set,
            event,
            digital_handle: 0,
            analog_handle: 0,
            dead_zone: 0.0,
            held: false,
        }
    }

    #[test]
    fn remaps_riftgate_and_map_to_the_dpad() {
        let mut bindings = [
            binding("ActionPickup", ON_PRESS, 0, 0xe),
            binding("ActionRiftgate", ON_PRESS, 0, RIFTGATE),
            binding("ActionMapWindow", ON_PRESS, -1, MAP_WINDOW),
        ];
        assert_eq!(unsafe { remap(&mut bindings) }, Ok("remapped"));
        let [_, riftgate, map] = &bindings;
        assert_eq!(
            (riftgate.event, riftgate.kind, riftgate.set),
            (SCROLL_UP, 1, 0)
        );
        assert_eq!((map.event, map.kind, map.set), (SCROLL_DOWN, 1, 0));
        assert_eq!(unsafe { remap(&mut bindings) }, Ok("already remapped"));
    }

    #[test]
    fn leaves_unknown_layouts_alone() {
        let mut bindings = [
            binding("ActionRiftgate", ON_PRESS, 0, 0x11),
            binding("ActionMapWindow", ON_PRESS, -1, MAP_WINDOW),
        ];
        assert!(unsafe { remap(&mut bindings) }.is_err());
        assert_eq!(bindings[1].event, MAP_WINDOW);
        assert!(unsafe { remap(&mut bindings[1..]) }.is_err());
    }

    #[test]
    fn reads_long_names() {
        let name = b"ActionAchievementWindow";
        let mut s = binding("", 0, 0, 0).name;
        s.buf[..8].copy_from_slice(&(name.as_ptr() as usize).to_ne_bytes());
        s.len = name.len();
        s.cap = 31;
        assert_eq!(unsafe { s.bytes() }, name);
    }
}
