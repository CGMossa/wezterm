//! Colors for attributes
// for FromPrimitive
#![cfg_attr(feature = "cargo-clippy", allow(clippy::useless_attribute))]

use num_derive::*;
#[cfg(feature = "use_serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq, Eq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[repr(u8)]
/// These correspond to the classic ANSI color indices and are
/// used for convenience/readability in code
pub enum AnsiColor {
    /// "Dark" black
    Black = 0,
    /// Dark red
    Maroon,
    /// Dark green
    Green,
    /// "Dark" yellow
    Olive,
    /// Dark blue
    Navy,
    /// Dark purple
    Purple,
    /// "Dark" cyan
    Teal,
    /// "Dark" white
    Silver,
    /// "Bright" black
    Grey,
    /// Bright red
    Red,
    /// Bright green
    Lime,
    /// Bright yellow
    Yellow,
    /// Bright blue
    Blue,
    /// Bright purple
    Fuchsia,
    /// Bright Cyan/Aqua
    Aqua,
    /// Bright white
    White,
}

impl From<AnsiColor> for u8 {
    fn from(col: AnsiColor) -> u8 {
        col as u8
    }
}

pub type RgbaTuple = (f32, f32, f32, f32);

lazy_static::lazy_static! {
    static ref NAMED_COLORS: HashMap<String, RgbColor> = build_colors();
}

fn build_colors() -> HashMap<String, RgbColor> {
    let mut map = HashMap::new();
    let rgb_txt = include_str!("rgb.txt");
    for line in rgb_txt.lines() {
        let mut fields = line.split_ascii_whitespace();
        let red = fields.next().unwrap();
        let green = fields.next().unwrap();
        let blue = fields.next().unwrap();
        let name = fields.collect::<Vec<&str>>().join(" ");

        let name = name.to_ascii_lowercase();
        map.insert(
            name,
            RgbColor::new_8bpc(
                red.parse().unwrap(),
                green.parse().unwrap(),
                blue.parse().unwrap(),
            ),
        );
    }

    map
}

/// Describes a color in the SRGB colorspace using red, green and blue
/// components in the range 0-255.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct RgbColor {
    // MSB set means that we have stored 10bpc color.
    // Otherwise: 8bpc.
    bits: u32,
}

const TEN_BITS: u16 = 0b11_1111_1111;
const MAX_TEN: f32 = 1023.;

fn ten_to_eight(bits: u32) -> u8 {
    ((bits as u16 & TEN_BITS) as f32 / MAX_TEN * 255.0) as u8
}

impl RgbColor {
    /// Construct a color from discrete red, green, blue values
    /// in the range 0-255.
    pub const fn new_8bpc(red: u8, green: u8, blue: u8) -> Self {
        Self {
            bits: ((red as u32) << 16) | ((green as u32) << 8) | blue as u32,
        }
    }

    /// Construct a color from discrete red, green, blue values
    /// in the range 0-1023.
    pub const fn new_10bpc(red: u16, green: u16, blue: u16) -> Self {
        Self {
            bits: 0x8000_0000
                | (((red & TEN_BITS) as u32) << 20)
                | (((green & TEN_BITS) as u32) << 10)
                | (blue & TEN_BITS) as u32,
        }
    }

    /// Construct a color from discrete red, green, blue values
    /// in the range 0.0-1.0 in the sRGB colorspace.
    pub fn new_f32(red: f32, green: f32, blue: f32) -> Self {
        let red = (red * MAX_TEN) as u16;
        let green = (green * MAX_TEN) as u16;
        let blue = (blue * MAX_TEN) as u16;
        Self::new_10bpc(red, green, blue)
    }

    /// Returns red, green, blue as 8bpc values.
    /// Will convert from 10bpc if that is the internal storage.
    pub fn to_tuple_rgb8(self) -> (u8, u8, u8) {
        if self.bits & 0x8000_0000 == 0 {
            // 8bpc
            (
                (self.bits >> 16) as u8,
                (self.bits >> 8) as u8,
                self.bits as u8,
            )
        } else {
            // 10bpc.
            (
                ten_to_eight(self.bits >> 20),
                ten_to_eight(self.bits >> 10),
                ten_to_eight(self.bits),
            )
        }
    }

    /// Returns red, green, blue as floating point values in the range 0.0-1.0.
    /// An alpha channel with the value of 1.0 is included.
    /// The values are in the sRGB colorspace.
    pub fn to_tuple_rgba(self) -> RgbaTuple {
        if self.bits & 0x8000_0000 == 0 {
            // 8bpc
            (
                (self.bits >> 16) as u8 as f32 / 255.0,
                (self.bits >> 8) as u8 as f32 / 255.0,
                self.bits as u8 as f32 / 255.0,
                1.0,
            )
        } else {
            // 10bpc
            (
                ((self.bits >> 20) as u16 & TEN_BITS) as f32 / MAX_TEN,
                ((self.bits >> 10) as u16 & TEN_BITS) as f32 / MAX_TEN,
                (self.bits as u16 & TEN_BITS) as f32 / MAX_TEN,
                1.0,
            )
        }
    }

    /// Returns red, green, blue as floating point values in the range 0.0-1.0.
    /// An alpha channel with the value of 1.0 is included.
    /// The values are converted from sRGB to linear colorspace.
    pub fn to_linear_tuple_rgba(self) -> RgbaTuple {
        let (red, green, blue, _alpha) = self.to_tuple_rgba();
        // See https://docs.rs/palette/0.5.0/src/palette/encoding/srgb.rs.html#43
        fn to_linear(v: f32) -> f32 {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        }
        (to_linear(red), to_linear(green), to_linear(blue), 1.0)
    }

    /// Construct a color from an X11/SVG/CSS3 color name.
    /// Returns None if the supplied name is not recognized.
    /// The list of names can be found here:
    /// <https://en.wikipedia.org/wiki/X11_color_names>
    pub fn from_named(name: &str) -> Option<RgbColor> {
        NAMED_COLORS.get(&name.to_ascii_lowercase()).cloned()
    }

    /// Returns a string of the form `#RRGGBB`
    pub fn to_rgb_string(self) -> String {
        let (red, green, blue) = self.to_tuple_rgb8();
        format!("#{:02x}{:02x}{:02x}", red, green, blue)
    }

    /// Returns a string of the form `rgb:RRRR/GGGG/BBBB`
    pub fn to_x11_16bit_rgb_string(self) -> String {
        let (red, green, blue) = self.to_tuple_rgb8();
        format!(
            "rgb:{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}",
            red, red, green, green, blue, blue
        )
    }

    /// Construct a color from a string of the form `#RRGGBB` where
    /// R, G and B are all hex digits.
    /// `hsl:hue sat light` is also accepted, and allows specifying a color
    /// in the HSL color space, where `hue` is measure in degrees and has
    /// a range of 0-360, and both `sat` and `light` are specified in percentage
    /// in the range 0-100.
    pub fn from_rgb_str(s: &str) -> Option<RgbColor> {
        if s.len() > 0 && s.as_bytes()[0] == b'#' {
            // Probably `#RGB`

            let digits = (s.len() - 1) / 3;
            if 1 + (digits * 3) != s.len() {
                return None;
            }

            if digits == 0 || digits > 4 {
                // Max of 16 bits supported
                return None;
            }

            let mut chars = s.chars().skip(1);

            macro_rules! digit {
                () => {{
                    let mut component = 0u16;

                    for _ in 0..digits {
                        component = component << 4;

                        let nybble = match chars.next().unwrap().to_digit(16) {
                            Some(v) => v as u16,
                            None => return None,
                        };
                        component |= nybble;
                    }

                    // From XParseColor, the `#` syntax takes the most significant
                    // bits and uses those for the color value.  That function produces
                    // 16-bit color components but we want 8-bit components so we shift
                    // or truncate the bits here depending on the number of digits
                    match digits {
                        1 => (component << 4) as u8,
                        2 => component as u8,
                        3 => (component >> 4) as u8,
                        4 => (component >> 8) as u8,
                        _ => return None,
                    }
                }};
            }
            Some(Self::new_8bpc(digit!(), digit!(), digit!()))
        } else if s.starts_with("rgb:") && s.len() > 6 {
            // The string includes two slashes: `rgb:r/g/b`
            let digits = (s.len() - 3) / 3;
            if 3 + (digits * 3) != s.len() {
                return None;
            }

            let digits = digits - 1;
            if digits == 0 || digits > 4 {
                // Max of 16 bits supported
                return None;
            }

            let mut chars = s.chars().skip(4);

            macro_rules! digit {
                () => {{
                    let mut component = 0u16;

                    for _ in 0..digits {
                        component = component << 4;

                        let nybble = match chars.next().unwrap().to_digit(16) {
                            Some(v) => v as u16,
                            None => return None,
                        };
                        component |= nybble;
                    }

                    // From XParseColor, the `rgb:` prefixed syntax scales the
                    // value into 16 bits from the number of bits specified
                    match digits {
                        1 => (component | component << 4) as u8,
                        2 => component as u8,
                        3 => (component >> 4) as u8,
                        4 => (component >> 8) as u8,
                        _ => return None,
                    }
                }};
            }
            macro_rules! slash {
                () => {{
                    match chars.next() {
                        Some('/') => {}
                        _ => return None,
                    }
                }};
            }
            let red = digit!();
            slash!();
            let green = digit!();
            slash!();
            let blue = digit!();

            Some(Self::new_8bpc(red, green, blue))
        } else if s.starts_with("hsl:") {
            let fields: Vec<_> = s[4..].split_ascii_whitespace().collect();
            if fields.len() == 3 {
                // Expected to be degrees in range 0-360, but we allow for negative and wrapping
                let h: i32 = fields[0].parse().ok()?;
                // Expected to be percentage in range 0-100
                let s: i32 = fields[1].parse().ok()?;
                // Expected to be percentage in range 0-100
                let l: i32 = fields[2].parse().ok()?;

                fn hsl_to_rgb(hue: i32, sat: i32, light: i32) -> (f32, f32, f32) {
                    let hue = hue % 360;
                    let hue = if hue < 0 { hue + 360 } else { hue } as f32;
                    let sat = sat as f32 / 100.;
                    let light = light as f32 / 100.;
                    let a = sat * light.min(1. - light);
                    let f = |n: f32| -> f32 {
                        let k = (n + hue / 30.) % 12.;
                        light - a * (k - 3.).min(9. - k).min(1.).max(-1.)
                    };
                    (f(0.), f(8.), f(4.))
                }

                let (r, g, b) = hsl_to_rgb(h, s, l);
                Some(Self::new_f32(r, g, b))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Construct a color from an SVG/CSS3 color name.
    /// or from a string of the form `#RRGGBB` where
    /// R, G and B are all hex digits.
    /// `hsl:hue sat light` is also accepted, and allows specifying a color
    /// in the HSL color space, where `hue` is measure in degrees and has
    /// a range of 0-360, and both `sat` and `light` are specified in percentage
    /// in the range 0-100.
    /// Returns None if the supplied name is not recognized.
    /// The list of names can be found here:
    /// <https://ogeon.github.io/docs/palette/master/palette/named/index.html>
    pub fn from_named_or_rgb_string(s: &str) -> Option<Self> {
        RgbColor::from_rgb_str(&s).or_else(|| RgbColor::from_named(&s))
    }
}

/// This is mildly unfortunate: in order to round trip RgbColor with serde
/// we need to provide a Serialize impl equivalent to the Deserialize impl
/// below.  We use the impl below to allow more flexible specification of
/// color strings in the config file.  A side effect of doing it this way
/// is that we have to serialize RgbColor as a 7-byte string when we could
/// otherwise serialize it as a 3-byte array.  There's probably a way
/// to make this work more efficiently, but for now this will do.
#[cfg(feature = "use_serde")]
impl Serialize for RgbColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_rgb_string();
        s.serialize(serializer)
    }
}

#[cfg(feature = "use_serde")]
impl<'de> Deserialize<'de> for RgbColor {
    fn deserialize<D>(deserializer: D) -> Result<RgbColor, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        RgbColor::from_named_or_rgb_string(&s)
            .ok_or_else(|| format!("unknown color name: {}", s))
            .map_err(serde::de::Error::custom)
    }
}

/// An index into the fixed color palette.
pub type PaletteIndex = u8;

/// Specifies the color to be used when rendering a cell.
/// This differs from `ColorAttribute` in that this type can only
/// specify one of the possible color types at once, whereas the
/// `ColorAttribute` type can specify a TrueColor value and a fallback.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ColorSpec {
    Default,
    /// Use either a raw number, or use values from the `AnsiColor` enum
    PaletteIndex(PaletteIndex),
    TrueColor(RgbColor),
}

impl Default for ColorSpec {
    fn default() -> Self {
        ColorSpec::Default
    }
}

impl From<AnsiColor> for ColorSpec {
    fn from(col: AnsiColor) -> Self {
        ColorSpec::PaletteIndex(col as u8)
    }
}

impl From<RgbColor> for ColorSpec {
    fn from(col: RgbColor) -> Self {
        ColorSpec::TrueColor(col)
    }
}

/// Specifies the color to be used when rendering a cell.  This is the
/// type used in the `CellAttributes` struct and can specify an optional
/// TrueColor value, allowing a fallback to a more traditional palette
/// index if TrueColor is not available.
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ColorAttribute {
    /// Use RgbColor when supported, falling back to the specified PaletteIndex.
    TrueColorWithPaletteFallback(RgbColor, PaletteIndex),
    /// Use RgbColor when supported, falling back to the default color
    TrueColorWithDefaultFallback(RgbColor),
    /// Use the specified PaletteIndex
    PaletteIndex(PaletteIndex),
    /// Use the default color
    Default,
}

impl Default for ColorAttribute {
    fn default() -> Self {
        ColorAttribute::Default
    }
}

impl From<AnsiColor> for ColorAttribute {
    fn from(col: AnsiColor) -> Self {
        ColorAttribute::PaletteIndex(col as u8)
    }
}

impl From<ColorSpec> for ColorAttribute {
    fn from(spec: ColorSpec) -> Self {
        match spec {
            ColorSpec::Default => ColorAttribute::Default,
            ColorSpec::PaletteIndex(idx) => ColorAttribute::PaletteIndex(idx),
            ColorSpec::TrueColor(color) => ColorAttribute::TrueColorWithDefaultFallback(color),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn named_rgb() {
        let dark_green = RgbColor::from_named("DarkGreen").unwrap();
        assert_eq!(dark_green.bits, 0x006400);
    }

    #[test]
    fn from_hsl() {
        let foo = RgbColor::from_rgb_str("hsl:235 100  50").unwrap();
        assert_eq!(foo.to_rgb_string(), "#0015ff");
    }

    #[test]
    fn from_rgb() {
        assert!(RgbColor::from_rgb_str("").is_none());
        assert!(RgbColor::from_rgb_str("#xyxyxy").is_none());

        let foo = RgbColor::from_rgb_str("#f00f00f00").unwrap();
        assert_eq!(foo.bits, 0xf0f0f0);

        let black = RgbColor::from_rgb_str("#000").unwrap();
        assert_eq!(black.bits, 0);

        let black = RgbColor::from_rgb_str("#FFF").unwrap();
        assert_eq!(black.bits, 0xf0f0f0);

        let black = RgbColor::from_rgb_str("#000000").unwrap();
        assert_eq!(black.bits, 0);

        let grey = RgbColor::from_rgb_str("rgb:D6/D6/D6").unwrap();
        assert_eq!(grey.bits, 0xd6d6d6);

        let grey = RgbColor::from_rgb_str("rgb:f0f0/f0f0/f0f0").unwrap();
        assert_eq!(grey.bits, 0xf0f0f0);
    }

    #[cfg(feature = "use_serde")]
    #[test]
    fn roundtrip_rgbcolor() {
        let data = varbincode::serialize(&RgbColor::from_named("DarkGreen").unwrap()).unwrap();
        eprintln!("serialized as {:?}", data);
        let _decoded: RgbColor = varbincode::deserialize(data.as_slice()).unwrap();
    }
}
