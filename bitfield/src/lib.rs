// Crates that have the "proc-macro" crate type are only allowed to export
// procedural macros. So we cannot have one crate that defines procedural macros
// alongside other types of public APIs like traits and structs.
//
// For this project we are going to need a #[bitfield] macro but also a trait
// and some structs. We solve this by defining the trait and structs in this
// crate, defining the attribute macro in a separate bitfield-impl crate, and
// then re-exporting the macro from this crate so that users only have one crate
// that they need to import.
//
// From the perspective of a user of this crate, they get all the necessary APIs
// (macro, trait, struct) through the one bitfield crate.
pub mod checks;
pub use bitfield_impl::*;

pub trait Specifier {
    type U;
    const BITS: usize;

    fn set(data: &mut [u8], offset: usize, val: Self::U);
    fn get(data: &[u8], offset: usize) -> Self::U;
}

pub fn set_data(data: &mut [u8], offset: usize, val: u64, val_bits: usize) {
    let st = offset / 8 * 8;
    let end = offset + val_bits;
    for i in (st..end).step_by(8) {
        let idx = i / 8;
        let mut mask = 0u8;
        let upd = if i < offset {
            // low mask
            mask |= !(!0u8 << (offset - i));
            val << (offset - i)
        } else {
            val >> (i - offset)
        } as u8;
        if i + 8 > end {
            // high mask
            mask |= !0u8 << (end - i);
        }
        data[idx] = (data[idx] & mask) | (upd & !mask);
    }
}

pub fn get_data(data: &[u8], offset: usize, val_bits: usize) -> u64 {
    let mut res = 0u64;
    let st = offset / 8 * 8;
    let end = offset + val_bits;
    for i in (st..end).step_by(8) {
        let idx = i / 8;
        let mut mask = 0u8;
        if offset > i {
            mask |= !(!0u8 << (offset - i));
        }
        if i + 8 > end {
            mask |= !0u8 << (end - i);
        }
        res |= if offset > i {
            ((data[idx] & !mask) as u64) >> (offset - i)
        } else {
            ((data[idx] & !mask) as u64) << (i - offset)
        };
    }
    res
}

impl Specifier for bool {
    type U = bool;
    const BITS: usize = 1;

    fn get(data: &[u8], offset: usize) -> Self::U {
        let idx = offset / 8;
        let off = offset % 8;
        data[idx] & (1 << off) != 0
    }

    fn set(data: &mut [u8], offset: usize, val: Self::U) {
        let idx = offset / 8;
        let off = offset % 8;
        if val {
            data[idx] |= 1 << off;
        } else {
            data[idx] &= 1 << off;
        }
    }
}

macro_rules! impl_specifier {
    ($name: ident, $bits: expr, $type: ty) => {
        pub struct $name;
        impl Specifier for $name {
            type U = $type;
            const BITS: usize = $bits;

            fn set(data: &mut [u8], offset: usize, val: Self::U) {
                set_data(data, offset, val as u64, Self::BITS);
            }
            fn get(data: &[u8], offset: usize) -> Self::U {
                get_data(data, offset, Self::BITS) as Self::U
            }
        }
    };
}

impl_bits_specifiers!();
