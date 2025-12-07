use bytemuck::{Pod, Zeroable};

pub fn linear_to_srgb(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        linear.powf(1. / 2.4) * 1.055 - 0.055
    }
}

pub fn srgb_to_linear(srgb: f32) -> f32 {
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear RGBA.
#[derive(Default, Debug, Clone, Copy, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn from_hex(u: u32) -> Self {
        let [r, g, b, a] = u.to_be_bytes();
        Self {
            r: r as f32 / 255.,
            g: g as f32 / 255.,
            b: b as f32 / 255.,
            a: a as f32 / 255.,
        }
    }

    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl From<Rgba> for [f32; 4] {
    fn from(srgba: Rgba) -> Self {
        srgba.to_array()
    }
}

impl From<[f32; 4]> for Rgba {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Self { r, g, b, a }
    }
}

impl From<Srgba> for Rgba {
    fn from(s: Srgba) -> Self {
        Self::new(
            srgb_to_linear(s.r),
            srgb_to_linear(s.g),
            srgb_to_linear(s.b),
            s.a,
        )
    }
}

impl From<Srgb> for Rgba {
    fn from(s: Srgb) -> Self {
        Self::new(
            srgb_to_linear(s.r),
            srgb_to_linear(s.g),
            srgb_to_linear(s.b),
            1.0,
        )
    }
}

/// sRGB+A.
#[derive(Default, Debug, Clone, Copy, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct Srgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Srgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn from_hex(u: u32) -> Self {
        let [r, g, b, a] = u.to_be_bytes();
        Self {
            r: r as f32 / 255.,
            g: g as f32 / 255.,
            b: b as f32 / 255.,
            a: a as f32 / 255.,
        }
    }

    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl From<Srgba> for [f32; 4] {
    fn from(srgba: Srgba) -> Self {
        srgba.to_array()
    }
}

impl From<[f32; 4]> for Srgba {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Self { r, g, b, a }
    }
}

impl From<Rgba> for Srgba {
    fn from(linear: Rgba) -> Self {
        Self::new(
            linear_to_srgb(linear.r),
            linear_to_srgb(linear.g),
            linear_to_srgb(linear.b),
            linear.a,
        )
    }
}

/// sRGB.
#[derive(Default, Debug, Clone, Copy, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct Srgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Srgb {
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub const fn from_hex(u: u32) -> Self {
        let [zero, r, g, b] = u.to_be_bytes();
        assert!(zero == 0, "`Srgb::from_hex` called with overflowing value");
        Self {
            r: r as f32 / 255.,
            g: g as f32 / 255.,
            b: b as f32 / 255.,
        }
    }

    pub const fn to_array(self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }
}

impl From<Srgb> for [f32; 3] {
    fn from(srgba: Srgb) -> Self {
        srgba.to_array()
    }
}

impl From<[f32; 3]> for Srgb {
    fn from([r, g, b]: [f32; 3]) -> Self {
        Self { r, g, b }
    }
}

impl From<Srgb> for Srgba {
    fn from(s: Srgb) -> Self {
        Self::new(s.r, s.g, s.b, 1.0)
    }
}
