//! Finds where the game's native gamepad handler cycles the highlighted loot. On d-pad up or
//! down it checks the loot list's item count and moves its selection:
//!
//! ```text
//! mov rax, [rsi + list]
//! cmp byte [rdi + 0x20], 0        ; pressed
//! je  ...
//! mov ecx, [rdi + 0x1c]           ; button
//! cmp ecx, 1                      ; d-pad up
//! jne ...
//! cmp dword [rax + count], 0
//! je  ...
//! dec dword [rax + selected]
//! ```

/// Where the loot list lives: a pointer at `list` in the in-game UI, and its item count and
/// selected index.
#[derive(Debug, PartialEq, Eq)]
pub struct LootList {
    pub list: usize,
    pub count: usize,
    pub selected: usize,
}

#[rustfmt::skip]
const PATTERN: &[Option<u8>] = &{
    const fn b(byte: u8) -> Option<u8> {
        Some(byte)
    }
    const ANY: Option<u8> = None;
    [
        b(0x48), b(0x8b), b(0x86), ANY, ANY, ANY, ANY,      // mov rax, [rsi + list]
        b(0x80), b(0x7f), b(0x20), b(0x00),                 // cmp byte [rdi + 0x20], 0
        b(0x74), ANY,                                       // je
        b(0x8b), b(0x4f), b(0x1c),                          // mov ecx, [rdi + 0x1c]
        b(0x83), b(0xf9), b(0x01),                          // cmp ecx, 1
        b(0x75), ANY,                                       // jne
        b(0x83), b(0xb8), ANY, ANY, ANY, ANY, b(0x00),      // cmp dword [rax + count], 0
        b(0x74), ANY,                                       // je
        b(0xff), b(0x88), ANY, ANY, ANY, ANY,               // dec dword [rax + selected]
    ]
};

/// The position of the only match in `code`, and the offsets it uses.
pub fn find(code: &[u8]) -> Result<(usize, LootList), String> {
    let mut matches = code.windows(PATTERN.len()).enumerate().filter(|(_, w)| {
        w.iter()
            .zip(PATTERN)
            .all(|(byte, p)| p.is_none_or(|p| p == *byte))
    });
    let (at, w) = matches
        .next()
        .ok_or("the loot cycling code was not found")?;
    if matches.next().is_some() {
        return Err("the loot cycling code was found more than once".into());
    }
    let offset = |i: usize| u32::from_le_bytes(w[i..i + 4].try_into().unwrap()) as usize;
    Ok((
        at,
        LootList {
            list: offset(3),
            count: offset(23),
            selected: offset(32),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_offsets() {
        let code = [
            0xcc, 0x48, 0x8b, 0x86, 0x10, 0x01, 0x00, 0x00, 0x80, 0x7f, 0x20, 0x00, 0x74, 0x34,
            0x8b, 0x4f, 0x1c, 0x83, 0xf9, 0x01, 0x75, 0x11, 0x83, 0xb8, 0x3c, 0x01, 0x00, 0x00,
            0x00, 0x74, 0x23, 0xff, 0x88, 0x40, 0x01, 0x00, 0x00, 0xeb, 0x14,
        ];
        let loot = LootList {
            list: 0x110,
            count: 0x13c,
            selected: 0x140,
        };
        assert_eq!(find(&code), Ok((1, loot)));
        assert!(find(&code[2..]).is_err());
        assert!(find(&[&code[..], &code[..]].concat()).is_err());
    }
}
