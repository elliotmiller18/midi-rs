pub fn msb(target: u8) -> u8 {
    target >> 4
}

pub fn lsb(target: u8) -> u8 {
    target & 0b1111
}

pub fn msb_set(target: u8) -> bool {
    (target >> 7) == 1
}