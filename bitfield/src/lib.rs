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

pub use bitfield_impl::bitfield;
use bitfield_impl::impl_bits_specifiers;

pub type MultipleOfEight<T> = <<T as checks::Array>::Marker as checks::TotalSizeIsMultipleOfEightBits>::Check;

pub trait Specifier {
    type U;
    const BITS: usize;
}

macro_rules! impl_specifier {
    ($name: ident, $bits: expr, $type: ty) => {
        pub struct $name;
        impl Specifier for $name {
            type U = $type;
            const BITS: usize = $bits;
        }
    };
}

impl_bits_specifiers!();