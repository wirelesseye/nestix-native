#[inline]
pub const fn loword(l: u32) -> u16 {
    (l & 0xFFFF) as u16
}

#[inline]
pub const fn hiword(l: u32) -> u16 {
    ((l >> 16) & 0xFFFF) as u16
}
