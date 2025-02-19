pub enum ZeroMod8 {}
pub enum OneMod8 {}
pub enum TwoMod8 {}
pub enum ThreeMod8 {}
pub enum FourMod8 {}
pub enum FiveMod8 {}
pub enum SixMod8 {}
pub enum SevenMod8 {}

pub trait Array {
    type Marker;
}

impl Array for [(); 0] {
    type Marker = ZeroMod8;
}

impl Array for [(); 1] {
    type Marker = OneMod8;
}

impl Array for [(); 2] {
    type Marker = TwoMod8;
}

impl Array for [(); 3] {
    type Marker = ThreeMod8;
}

impl Array for [(); 4] {
    type Marker = FourMod8;
}

impl Array for [(); 5] {
    type Marker = FiveMod8;
}

impl Array for [(); 6] {
    type Marker = SixMod8;
}

impl Array for [(); 7] {
    type Marker = SevenMod8;
}


pub trait TotalSizeIsMultipleOfEightBits {
    type Check;
}

impl TotalSizeIsMultipleOfEightBits for ZeroMod8 {
    type Check = ();
}

pub type MultipleOfEight<T> =
    <<T as Array>::Marker as TotalSizeIsMultipleOfEightBits>::Check;

pub enum True {}
pub enum False {}
pub trait ArrayMarker {
    type Marker;
}
impl ArrayMarker for [(); 0] {
    type Marker = True;
}
impl ArrayMarker for [(); 1] {
    type Marker = False;
}
pub trait DiscriminantInRange {
    type Check;
}
impl DiscriminantInRange for True {
    type Check = ();
}
pub type EnumInRange<T> =
    <<T as ArrayMarker>::Marker as DiscriminantInRange>::Check;