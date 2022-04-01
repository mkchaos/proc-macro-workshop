use bitfield::*;

#[bitfield]
pub struct MyFourBytes {
    a: B1,
    b: B3,
    c: B4,
    d: B24,
}

fn main() {
    assert_eq!(std::mem::size_of::<MyFourBytes>(), 4);
}
