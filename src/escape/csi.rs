use cell::{Blink, Intensity, Underline};
use color::{AnsiColor, ColorSpec, RgbColor};
use escape::EncodeEscape;
use num;
use std;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CSI {
    /// SGR: Set Graphics Rendition.
    /// These values affect how the character is rendered.
    Sgr(Sgr),

    /// CSI codes that relate to the cursor
    Cursor(Cursor),

    Unspecified {
        params: Vec<i64>,
        // TODO: can we just make intermediates a single u8?
        intermediates: Vec<u8>,
        /// if true, more than two intermediates arrived and the
        /// remaining data was ignored
        ignored_extra_intermediates: bool,
        /// The final character in the CSI sequence; this typically
        /// defines how to interpret the other parameters.
        control: char,
    },
    #[doc(hidden)]
    __Nonexhaustive,
}

impl EncodeEscape for CSI {
    // TODO: data size optimization opportunity: if we could somehow know that we
    // had a run of CSI instances being encoded in sequence, we could
    // potentially collapse them together.  This is a few bytes difference in
    // practice so it may not be worthwhile with modern networks.
    fn encode_escape<W: std::io::Write>(&self, w: &mut W) -> Result<(), std::io::Error> {
        w.write_all(&[0x1b, b'['])?;
        match self {
            CSI::Sgr(sgr) => sgr.encode_escape(w)?,
            CSI::Cursor(c) => c.encode_escape(w)?,
            CSI::Unspecified {
                params,
                intermediates,
                control,
                ..
            } => {
                for (idx, p) in params.iter().enumerate() {
                    if idx > 0 {
                        write!(w, ";{}", p)?;
                    } else {
                        write!(w, "{}", p)?;
                    }
                }
                for i in intermediates {
                    write!(w, "{}", i)?;
                }
                write!(w, "{}", control)?;
            }
            CSI::__Nonexhaustive => {}
        };
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cursor {
    /// CBT causes the active present ation position to be moved to the
    /// character position corresponding to the n-th preceding character
    /// tabulation stop in the presentation component, according to the
    /// character path, where n equals the value of Pn
    BackwardTabulation(u32),

    /// CHA causes the active presentation position to be moved to character
    /// position n in the active line in the presentation component, where n
    /// equals the value of Pn.
    CharacterAbsolute(u32),

    /// CHT causes the active presentation position to be moved to the
    /// character position corresponding to the n-th following character
    /// tabulation stop in the presentation component, according to the
    /// character path, where n equals the value of Pn
    ForwardTabulation(u32),

    /// CNL causes the active presentation position to be moved to the
    /// first character position of the n-th following line in the
    /// presentation component, where n equals the value of Pn
    NextLine(u32),

    /// CPL causes the active presentation position to be moved to the first
    /// character position of the  n-th preceding line in the presentation
    /// component, where n equals the value of Pn
    PrecedingLine(u32),

    /// CPR - ACTIVE POSITION REPORT
    /// If the DEVICE COMPONENT SELECT MODE (DCSM)
    /// is set to PRESENTATION, CPR is used to report the active presentation
    /// position of the sending device as residing in the presentation
    /// component at the n-th line position according to the line progression
    /// and at the m-th character position according to the character path,
    /// where n equals the value of Pn1 and m equal s the value of Pn2.
    /// If the DEVICE COMPONENT SELECT MODE (DCSM) is set to DATA, CPR is used
    /// to report the active data position of the sending device as
    /// residing in the data component at the n-th line position according
    /// to the line progression and at the m-th character position
    /// according to the character progression, where n equals the value of
    /// Pn1 and m equals the value of Pn2. CPR may be solicited by a DEVICE
    /// STATUS REPORT (DSR) or be sent unsolicited .
    ActivePositionReport { line: u32, col: u32 },

    /// CTC - CURSOR TABULATION CONTROL
    /// CTC causes one or more tabulation stops to be set or cleared in the
    /// presentation component, depending on the parameter values.
    /// In the case of parameter values 0, 2 or 4 the number of lines affected
    /// depends on the setting of the TABULATION STOP MODE (TSM).
    TabulationControl(CursorTabulationControl),

    /// CUB - Cursor Left
    /// CUB causes the active presentation position to be moved leftwards in
    /// the presentation component by n character positions if the character
    /// pat h is horizontal, or by n line positions if the character pat h is
    /// vertical, where n equals the value of Pn.
    Left(u32),

    /// CUD - Cursor Down
    Down(u32),

    /// CUF - Cursor Right
    Right(u32),

    /// CUP - Cursor Position
    /// CUP causes the active presentation position to be moved in the
    /// presentation component to the n-th line position according to the line
    /// progression and to the m-th character position according to the
    /// character path, where n equals the value of Pn1 and m equals the value
    /// of Pn2.
    Position { line: u32, col: u32 },

    /// CUU - Cursor Up
    Up(u32),

    /// CVT - Cursor Line Tabulation
    /// CVT causes the active presentation position to be moved to the
    /// corresponding character position of the line corresponding to the n-th
    /// following line tabulation stop in the presentation component, where n
    /// equals the value of Pn.
    LineTabulation(u32),
}

macro_rules! write_csi {
    ($w:expr, $control:expr, $p1:expr) => {
        if $p1 == 1 {
            write!($w, "{}", $control)
        } else {
            write!($w, "{}{}", $p1, $control)
        }
    };
}

impl EncodeEscape for Cursor {
    fn encode_escape<W: std::io::Write>(&self, w: &mut W) -> Result<(), std::io::Error> {
        match self {
            Cursor::BackwardTabulation(n) => write_csi!(w, "Z", *n)?,
            Cursor::CharacterAbsolute(col) => write_csi!(w, "G", *col)?,
            Cursor::ForwardTabulation(n) => write_csi!(w, "I", *n)?,
            Cursor::NextLine(n) => write_csi!(w, "E", *n)?,
            Cursor::PrecedingLine(n) => write_csi!(w, "F", *n)?,
            Cursor::ActivePositionReport { line, col } => write!(w, "{};{}R", line, col)?,
            Cursor::Left(n) => write_csi!(w, "D", *n)?,
            Cursor::Down(n) => write_csi!(w, "B", *n)?,
            Cursor::Right(n) => write_csi!(w, "C", *n)?,
            Cursor::Up(n) => write_csi!(w, "A", *n)?,
            Cursor::Position { line, col } => write!(w, "{};{}H", line, col)?,
            Cursor::LineTabulation(n) => write_csi!(w, "Y", *n)?,
            Cursor::TabulationControl(n) => write_csi!(w, "W", n.clone() as u8)?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, FromPrimitive)]
pub enum CursorTabulationControl {
    SetCharacterTabStopAtActivePosition = 0,
    SetLineTabStopAtActiveLine = 1,
    ClearCharacterTabStopAtActivePosition = 2,
    ClearLineTabstopAtActiveLine = 3,
    ClearAllCharacterTabStopsAtActiveLine = 4,
    ClearAllCharacterTabStops = 5,
    ClearAllLineTabStops = 6,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sgr {
    /// Resets rendition to defaults.  Typically switches off
    /// all other Sgr options, but may have greater or lesser impact.
    Reset,
    /// Set the intensity/bold level
    Intensity(Intensity),
    Underline(Underline),
    Blink(Blink),
    Italic(bool),
    Inverse(bool),
    Invisible(bool),
    StrikeThrough(bool),
    Font(Font),
    Foreground(ColorSpec),
    Background(ColorSpec),
}

impl EncodeEscape for Sgr {
    fn encode_escape<W: std::io::Write>(&self, w: &mut W) -> Result<(), std::io::Error> {
        macro_rules! code {
            ($t:ident) => {
                write!(w, "{}m", SgrCode::$t as i64)?
            };
        }

        macro_rules! ansi_color {
            ($idx:expr, $eightbit:ident, $( ($Ansi:ident, $code:ident) ),*) => {
                if let Some(ansi) = num::FromPrimitive::from_u8($idx) {
                    match ansi {
                        $(AnsiColor::$Ansi => code!($code) ,)*
                    }
                } else {
                    write!(w, "{};5;{}m", SgrCode::$eightbit as i64, $idx)?
                }
            }
        }

        match self {
            Sgr::Reset => code!(Reset),
            Sgr::Intensity(Intensity::Bold) => code!(IntensityBold),
            Sgr::Intensity(Intensity::Half) => code!(IntensityDim),
            Sgr::Intensity(Intensity::Normal) => code!(NormalIntensity),
            Sgr::Underline(Underline::Single) => code!(UnderlineOn),
            Sgr::Underline(Underline::Double) => code!(UnderlineDouble),
            Sgr::Underline(Underline::None) => code!(UnderlineOff),
            Sgr::Blink(Blink::Slow) => code!(BlinkOn),
            Sgr::Blink(Blink::Rapid) => code!(RapidBlinkOn),
            Sgr::Blink(Blink::None) => code!(BlinkOff),
            Sgr::Italic(true) => code!(ItalicOn),
            Sgr::Italic(false) => code!(ItalicOff),
            Sgr::Inverse(true) => code!(InverseOn),
            Sgr::Inverse(false) => code!(InverseOff),
            Sgr::Invisible(true) => code!(InvisibleOn),
            Sgr::Invisible(false) => code!(InvisibleOff),
            Sgr::StrikeThrough(true) => code!(StrikeThroughOn),
            Sgr::StrikeThrough(false) => code!(StrikeThroughOff),
            Sgr::Font(Font::Default) => code!(DefaultFont),
            Sgr::Font(Font::Alternate(1)) => code!(AltFont1),
            Sgr::Font(Font::Alternate(2)) => code!(AltFont2),
            Sgr::Font(Font::Alternate(3)) => code!(AltFont3),
            Sgr::Font(Font::Alternate(4)) => code!(AltFont4),
            Sgr::Font(Font::Alternate(5)) => code!(AltFont5),
            Sgr::Font(Font::Alternate(6)) => code!(AltFont6),
            Sgr::Font(Font::Alternate(7)) => code!(AltFont7),
            Sgr::Font(Font::Alternate(8)) => code!(AltFont8),
            Sgr::Font(Font::Alternate(9)) => code!(AltFont9),
            Sgr::Font(_) => { /* there are no other possible font values */ }
            Sgr::Foreground(ColorSpec::Default) => code!(ForegroundDefault),
            Sgr::Background(ColorSpec::Default) => code!(BackgroundDefault),
            Sgr::Foreground(ColorSpec::PaletteIndex(idx)) => ansi_color!(
                *idx,
                ForegroundColor,
                (Black, ForegroundBlack),
                (Maroon, ForegroundRed),
                (Green, ForegroundGreen),
                (Olive, ForegroundYellow),
                (Navy, ForegroundBlue),
                (Purple, ForegroundMagenta),
                (Teal, ForegroundCyan),
                (Silver, ForegroundWhite),
                // Note: these brights are emitted using codes in the 100 range.
                // I don't know how portable this is vs. the 256 color sequences,
                // so we may need to make an adjustment here later.
                (Grey, ForegroundBrightBlack),
                (Red, ForegroundBrightRed),
                (Lime, ForegroundBrightGreen),
                (Yellow, ForegroundBrightYellow),
                (Blue, ForegroundBrightBlue),
                (Fuschia, ForegroundBrightMagenta),
                (Aqua, ForegroundBrightCyan),
                (White, ForegroundBrightWhite)
            ),
            Sgr::Foreground(ColorSpec::TrueColor(c)) => write!(
                w,
                "{};2;{};{};{}m",
                SgrCode::ForegroundColor as i64,
                c.red,
                c.green,
                c.blue
            )?,
            Sgr::Background(ColorSpec::PaletteIndex(idx)) => ansi_color!(
                *idx,
                BackgroundColor,
                (Black, BackgroundBlack),
                (Maroon, BackgroundRed),
                (Green, BackgroundGreen),
                (Olive, BackgroundYellow),
                (Navy, BackgroundBlue),
                (Purple, BackgroundMagenta),
                (Teal, BackgroundCyan),
                (Silver, BackgroundWhite),
                // Note: these brights are emitted using codes in the 100 range.
                // I don't know how portable this is vs. the 256 color sequences,
                // so we may need to make an adjustment here later.
                (Grey, BackgroundBrightBlack),
                (Red, BackgroundBrightRed),
                (Lime, BackgroundBrightGreen),
                (Yellow, BackgroundBrightYellow),
                (Blue, BackgroundBrightBlue),
                (Fuschia, BackgroundBrightMagenta),
                (Aqua, BackgroundBrightCyan),
                (White, BackgroundBrightWhite)
            ),
            Sgr::Background(ColorSpec::TrueColor(c)) => write!(
                w,
                "{};2;{};{};{}m",
                SgrCode::BackgroundColor as i64,
                c.red,
                c.green,
                c.blue
            )?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Font {
    Default,
    Alternate(u8),
}

/// Constrol Sequence Initiator (CSI) Parser.
/// Since many sequences allow for composition of actions by separating
/// `;` character, we need to be able to iterate over
/// the set of parsed actions from a given CSI sequence.
/// `CSIParser` implements an Iterator that yields `CSI` instances as
/// it parses them out from the input sequence.
struct CSIParser<'a> {
    intermediates: &'a [u8],
    /// From vte::Perform: this flag is set when more than two intermediates
    /// arrived and subsequent characters were ignored.
    ignored_extra_intermediates: bool,
    control: char,
    /// While params is_some we have more data to consume.  The advance_by
    /// method updates the slice as we consume data.
    /// In a number of cases an empty params list is used to indicate
    /// default values, especially for SGR, so we need to be careful not
    /// to update params to an empty slice.
    params: Option<&'a [i64]>,
}

impl CSI {
    /// Parse a CSI sequence.
    /// Returns an iterator that yields individual CSI actions.
    /// Why not a single?  Because sequences like `CSI [ 1 ; 3 m`
    /// embed two separate actions but are sent as a single unit.
    /// If no semantic meaning is known for a subsequence, the remainder
    /// of the sequence is returned wrapped in a `CSI::Unspecified` container.
    pub fn parse<'a>(
        params: &'a [i64],
        intermediates: &'a [u8],
        ignored_extra_intermediates: bool,
        control: char,
    ) -> impl Iterator<Item = CSI> + 'a {
        CSIParser {
            intermediates,
            ignored_extra_intermediates,
            control,
            params: Some(params),
        }
    }
}

/// A little helper to convert i64 -> u8 if safe
fn to_u8(v: i64) -> Result<u8, ()> {
    if v <= u8::max_value() as i64 {
        Ok(v as u8)
    } else {
        Err(())
    }
}

fn to_u32(v: i64) -> Result<u32, ()> {
    if v >= 0 && v <= u32::max_value() as i64 {
        Ok(v as u32)
    } else {
        Err(())
    }
}

impl<'a> CSIParser<'a> {
    fn parse_next(&mut self, params: &'a [i64]) -> Result<CSI, ()> {
        match (self.control, self.intermediates, params) {
            ('m', &[], params) => self.sgr(params).map(|sgr| CSI::Sgr(sgr)),

            ('Z', &[], &[]) => Ok(CSI::Cursor(Cursor::BackwardTabulation(1))),
            ('Z', &[], &[n]) => Ok(CSI::Cursor(Cursor::BackwardTabulation(to_u32(n)?))),

            ('G', &[], &[]) => Ok(CSI::Cursor(Cursor::CharacterAbsolute(1))),
            ('G', &[], &[n]) => Ok(CSI::Cursor(Cursor::CharacterAbsolute(to_u32(n)?))),

            ('I', &[], &[]) => Ok(CSI::Cursor(Cursor::ForwardTabulation(1))),
            ('I', &[], &[n]) => Ok(CSI::Cursor(Cursor::ForwardTabulation(to_u32(n)?))),

            ('E', &[], &[]) => Ok(CSI::Cursor(Cursor::NextLine(1))),
            ('E', &[], &[n]) => Ok(CSI::Cursor(Cursor::NextLine(to_u32(n)?))),

            ('F', &[], &[]) => Ok(CSI::Cursor(Cursor::PrecedingLine(1))),
            ('F', &[], &[n]) => Ok(CSI::Cursor(Cursor::PrecedingLine(to_u32(n)?))),

            ('R', &[], &[line, col]) => Ok(CSI::Cursor(Cursor::ActivePositionReport {
                line: to_u32(line)?,
                col: to_u32(col)?,
            })),

            ('D', &[], &[]) => Ok(CSI::Cursor(Cursor::Left(1))),
            ('D', &[], &[n]) => Ok(CSI::Cursor(Cursor::Left(to_u32(n)?))),

            ('B', &[], &[]) => Ok(CSI::Cursor(Cursor::Down(1))),
            ('B', &[], &[n]) => Ok(CSI::Cursor(Cursor::Down(to_u32(n)?))),

            ('C', &[], &[]) => Ok(CSI::Cursor(Cursor::Right(1))),
            ('C', &[], &[n]) => Ok(CSI::Cursor(Cursor::Right(to_u32(n)?))),

            ('Y', &[], &[]) => Ok(CSI::Cursor(Cursor::LineTabulation(1))),
            ('Y', &[], &[n]) => Ok(CSI::Cursor(Cursor::LineTabulation(to_u32(n)?))),

            ('W', &[], &[]) => Ok(CSI::Cursor(Cursor::TabulationControl(
                CursorTabulationControl::SetCharacterTabStopAtActivePosition,
            ))),
            ('W', &[], &[n]) => Ok(CSI::Cursor(Cursor::TabulationControl(
                num::FromPrimitive::from_i64(n).ok_or(())?,
            ))),

            ('H', &[], &[line, col]) => Ok(CSI::Cursor(Cursor::Position {
                line: to_u32(line)?,
                col: to_u32(col)?,
            })),

            _ => Err(()),
        }
    }

    /// Consume some number of elements from params and update it.
    /// Take care to avoid setting params back to an empty slice
    /// as this would trigger returning a default value and/or
    /// an unterminated parse loop.
    fn advance_by<T>(&mut self, n: usize, params: &'a [i64], result: T) -> T {
        let (_, next) = params.split_at(n);
        if !next.is_empty() {
            self.params = Some(next);
        }
        result
    }

    fn parse_sgr_color(&mut self, params: &'a [i64]) -> Result<ColorSpec, ()> {
        if params.len() >= 5 && params[1] == 2 {
            let red = to_u8(params[2])?;
            let green = to_u8(params[3])?;
            let blue = to_u8(params[4])?;
            let res = RgbColor::new(red, green, blue).into();
            Ok(self.advance_by(5, params, res))
        } else if params.len() >= 3 && params[1] == 5 {
            let idx = to_u8(params[2])?;
            Ok(self.advance_by(3, params, ColorSpec::PaletteIndex(idx)))
        } else {
            Err(())
        }
    }

    fn sgr(&mut self, params: &'a [i64]) -> Result<Sgr, ()> {
        if params.len() == 0 {
            // With no parameters, treat as equivalent to Reset.
            Ok(Sgr::Reset)
        } else {
            // Consume a single parameter and return the parsed result
            macro_rules! one {
                ($t:expr) => {
                    Ok(self.advance_by(1, params, $t))
                };
            };

            match num::FromPrimitive::from_i64(params[0]) {
                None => Err(()),
                Some(sgr) => match sgr {
                    SgrCode::Reset => one!(Sgr::Reset),
                    SgrCode::IntensityBold => one!(Sgr::Intensity(Intensity::Bold)),
                    SgrCode::IntensityDim => one!(Sgr::Intensity(Intensity::Half)),
                    SgrCode::NormalIntensity => one!(Sgr::Intensity(Intensity::Normal)),
                    SgrCode::UnderlineOn => one!(Sgr::Underline(Underline::Single)),
                    SgrCode::UnderlineDouble => one!(Sgr::Underline(Underline::Double)),
                    SgrCode::UnderlineOff => one!(Sgr::Underline(Underline::None)),
                    SgrCode::BlinkOn => one!(Sgr::Blink(Blink::Slow)),
                    SgrCode::RapidBlinkOn => one!(Sgr::Blink(Blink::Rapid)),
                    SgrCode::BlinkOff => one!(Sgr::Blink(Blink::None)),
                    SgrCode::ItalicOn => one!(Sgr::Italic(true)),
                    SgrCode::ItalicOff => one!(Sgr::Italic(false)),
                    SgrCode::ForegroundColor => {
                        self.parse_sgr_color(params).map(|c| Sgr::Foreground(c))
                    }
                    SgrCode::ForegroundBlack => one!(Sgr::Foreground(AnsiColor::Black.into())),
                    SgrCode::ForegroundRed => one!(Sgr::Foreground(AnsiColor::Maroon.into())),
                    SgrCode::ForegroundGreen => one!(Sgr::Foreground(AnsiColor::Green.into())),
                    SgrCode::ForegroundYellow => one!(Sgr::Foreground(AnsiColor::Olive.into())),
                    SgrCode::ForegroundBlue => one!(Sgr::Foreground(AnsiColor::Navy.into())),
                    SgrCode::ForegroundMagenta => one!(Sgr::Foreground(AnsiColor::Purple.into())),
                    SgrCode::ForegroundCyan => one!(Sgr::Foreground(AnsiColor::Teal.into())),
                    SgrCode::ForegroundWhite => one!(Sgr::Foreground(AnsiColor::Silver.into())),
                    SgrCode::ForegroundDefault => one!(Sgr::Foreground(ColorSpec::Default)),
                    SgrCode::ForegroundBrightBlack => one!(Sgr::Foreground(AnsiColor::Grey.into())),
                    SgrCode::ForegroundBrightRed => one!(Sgr::Foreground(AnsiColor::Red.into())),
                    SgrCode::ForegroundBrightGreen => one!(Sgr::Foreground(AnsiColor::Lime.into())),
                    SgrCode::ForegroundBrightYellow => {
                        one!(Sgr::Foreground(AnsiColor::Yellow.into()))
                    }
                    SgrCode::ForegroundBrightBlue => one!(Sgr::Foreground(AnsiColor::Blue.into())),
                    SgrCode::ForegroundBrightMagenta => {
                        one!(Sgr::Foreground(AnsiColor::Fuschia.into()))
                    }
                    SgrCode::ForegroundBrightCyan => one!(Sgr::Foreground(AnsiColor::Aqua.into())),
                    SgrCode::ForegroundBrightWhite => {
                        one!(Sgr::Foreground(AnsiColor::White.into()))
                    }

                    SgrCode::BackgroundColor => {
                        self.parse_sgr_color(params).map(|c| Sgr::Background(c))
                    }
                    SgrCode::BackgroundBlack => one!(Sgr::Background(AnsiColor::Black.into())),
                    SgrCode::BackgroundRed => one!(Sgr::Background(AnsiColor::Maroon.into())),
                    SgrCode::BackgroundGreen => one!(Sgr::Background(AnsiColor::Green.into())),
                    SgrCode::BackgroundYellow => one!(Sgr::Background(AnsiColor::Olive.into())),
                    SgrCode::BackgroundBlue => one!(Sgr::Background(AnsiColor::Navy.into())),
                    SgrCode::BackgroundMagenta => one!(Sgr::Background(AnsiColor::Purple.into())),
                    SgrCode::BackgroundCyan => one!(Sgr::Background(AnsiColor::Teal.into())),
                    SgrCode::BackgroundWhite => one!(Sgr::Background(AnsiColor::Silver.into())),
                    SgrCode::BackgroundDefault => one!(Sgr::Background(ColorSpec::Default)),
                    SgrCode::BackgroundBrightBlack => one!(Sgr::Background(AnsiColor::Grey.into())),
                    SgrCode::BackgroundBrightRed => one!(Sgr::Background(AnsiColor::Red.into())),
                    SgrCode::BackgroundBrightGreen => one!(Sgr::Background(AnsiColor::Lime.into())),
                    SgrCode::BackgroundBrightYellow => {
                        one!(Sgr::Background(AnsiColor::Yellow.into()))
                    }
                    SgrCode::BackgroundBrightBlue => one!(Sgr::Background(AnsiColor::Blue.into())),
                    SgrCode::BackgroundBrightMagenta => {
                        one!(Sgr::Background(AnsiColor::Fuschia.into()))
                    }
                    SgrCode::BackgroundBrightCyan => one!(Sgr::Background(AnsiColor::Aqua.into())),
                    SgrCode::BackgroundBrightWhite => {
                        one!(Sgr::Background(AnsiColor::White.into()))
                    }

                    SgrCode::InverseOn => one!(Sgr::Inverse(true)),
                    SgrCode::InverseOff => one!(Sgr::Inverse(false)),
                    SgrCode::InvisibleOn => one!(Sgr::Invisible(true)),
                    SgrCode::InvisibleOff => one!(Sgr::Invisible(false)),
                    SgrCode::StrikeThroughOn => one!(Sgr::StrikeThrough(true)),
                    SgrCode::StrikeThroughOff => one!(Sgr::StrikeThrough(false)),
                    SgrCode::DefaultFont => one!(Sgr::Font(Font::Default)),
                    SgrCode::AltFont1 => one!(Sgr::Font(Font::Alternate(1))),
                    SgrCode::AltFont2 => one!(Sgr::Font(Font::Alternate(2))),
                    SgrCode::AltFont3 => one!(Sgr::Font(Font::Alternate(3))),
                    SgrCode::AltFont4 => one!(Sgr::Font(Font::Alternate(4))),
                    SgrCode::AltFont5 => one!(Sgr::Font(Font::Alternate(5))),
                    SgrCode::AltFont6 => one!(Sgr::Font(Font::Alternate(6))),
                    SgrCode::AltFont7 => one!(Sgr::Font(Font::Alternate(7))),
                    SgrCode::AltFont8 => one!(Sgr::Font(Font::Alternate(8))),
                    SgrCode::AltFont9 => one!(Sgr::Font(Font::Alternate(9))),
                },
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, FromPrimitive)]
pub enum SgrCode {
    Reset = 0,
    IntensityBold = 1,
    IntensityDim = 2,
    ItalicOn = 3,
    UnderlineOn = 4,
    /// Blinks < 150 times per minute
    BlinkOn = 5,
    /// Blinks > 150 times per minute
    RapidBlinkOn = 6,
    InverseOn = 7,
    InvisibleOn = 8,
    StrikeThroughOn = 9,
    DefaultFont = 10,
    AltFont1 = 11,
    AltFont2 = 12,
    AltFont3 = 13,
    AltFont4 = 14,
    AltFont5 = 15,
    AltFont6 = 16,
    AltFont7 = 17,
    AltFont8 = 18,
    AltFont9 = 19,
    // Fraktur = 20,
    UnderlineDouble = 21,
    NormalIntensity = 22,
    ItalicOff = 23,
    UnderlineOff = 24,
    BlinkOff = 25,
    InverseOff = 27,
    InvisibleOff = 28,
    StrikeThroughOff = 29,
    ForegroundBlack = 30,
    ForegroundRed = 31,
    ForegroundGreen = 32,
    ForegroundYellow = 33,
    ForegroundBlue = 34,
    ForegroundMagenta = 35,
    ForegroundCyan = 36,
    ForegroundWhite = 37,
    ForegroundDefault = 39,
    BackgroundBlack = 40,
    BackgroundRed = 41,
    BackgroundGreen = 42,
    BackgroundYellow = 43,
    BackgroundBlue = 44,
    BackgroundMagenta = 45,
    BackgroundCyan = 46,
    BackgroundWhite = 47,
    BackgroundDefault = 49,

    ForegroundBrightBlack = 90,
    ForegroundBrightRed = 91,
    ForegroundBrightGreen = 92,
    ForegroundBrightYellow = 93,
    ForegroundBrightBlue = 94,
    ForegroundBrightMagenta = 95,
    ForegroundBrightCyan = 96,
    ForegroundBrightWhite = 97,

    BackgroundBrightBlack = 100,
    BackgroundBrightRed = 101,
    BackgroundBrightGreen = 102,
    BackgroundBrightYellow = 103,
    BackgroundBrightBlue = 104,
    BackgroundBrightMagenta = 105,
    BackgroundBrightCyan = 106,
    BackgroundBrightWhite = 107,

    /// Maybe followed either either a 256 color palette index or
    /// a sequence describing a true color rgb value
    ForegroundColor = 38,
    BackgroundColor = 48,
}

impl<'a> Iterator for CSIParser<'a> {
    type Item = CSI;

    fn next(&mut self) -> Option<CSI> {
        let params = match self.params.take() {
            None => return None,
            Some(params) => params,
        };

        match self.parse_next(&params) {
            Ok(csi) => Some(csi),
            Err(()) => Some(CSI::Unspecified {
                params: params.to_vec(),
                intermediates: self.intermediates.to_vec(),
                ignored_extra_intermediates: self.ignored_extra_intermediates,
                control: self.control,
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn parse(control: char, params: &[i64], expected: &str) -> Vec<CSI> {
        let res = CSI::parse(params, &[], false, control).collect();
        assert_eq!(encode(&res), expected);
        res
    }

    fn encode(seq: &Vec<CSI>) -> String {
        let mut res = Vec::new();
        seq.encode_escape(&mut res).unwrap();
        String::from_utf8(res).unwrap()
    }

    #[test]
    fn test_basic() {
        assert_eq!(parse('m', &[], "\x1b[0m"), vec![CSI::Sgr(Sgr::Reset)]);
        assert_eq!(parse('m', &[0], "\x1b[0m"), vec![CSI::Sgr(Sgr::Reset)]);
        assert_eq!(
            parse('m', &[1], "\x1b[1m"),
            vec![CSI::Sgr(Sgr::Intensity(Intensity::Bold))]
        );
        assert_eq!(
            parse('m', &[1, 3], "\x1b[1m\x1b[3m"),
            vec![
                CSI::Sgr(Sgr::Intensity(Intensity::Bold)),
                CSI::Sgr(Sgr::Italic(true)),
            ]
        );

        // Verify that we propagate Unspecified for codes
        // that we don't recognize.
        assert_eq!(
            parse('m', &[1, 3, 1231231], "\x1b[1m\x1b[3m\x1b[1231231m"),
            vec![
                CSI::Sgr(Sgr::Intensity(Intensity::Bold)),
                CSI::Sgr(Sgr::Italic(true)),
                CSI::Unspecified {
                    params: [1231231].to_vec(),
                    intermediates: vec![],
                    ignored_extra_intermediates: false,
                    control: 'm',
                },
            ]
        );
        assert_eq!(
            parse('m', &[1, 1231231, 3], "\x1b[1m\x1b[1231231;3m"),
            vec![
                CSI::Sgr(Sgr::Intensity(Intensity::Bold)),
                CSI::Unspecified {
                    params: [1231231, 3].to_vec(),
                    intermediates: vec![],
                    ignored_extra_intermediates: false,
                    control: 'm',
                },
            ]
        );
        assert_eq!(
            parse('m', &[1231231, 3], "\x1b[1231231;3m"),
            vec![CSI::Unspecified {
                params: [1231231, 3].to_vec(),
                intermediates: vec![],
                ignored_extra_intermediates: false,
                control: 'm',
            }]
        );
    }

    #[test]
    fn test_color() {
        assert_eq!(
            parse('m', &[38, 2], "\x1b[38;2m"),
            vec![CSI::Unspecified {
                params: [38, 2].to_vec(),
                intermediates: vec![],
                ignored_extra_intermediates: false,
                control: 'm',
            }]
        );

        assert_eq!(
            parse('m', &[38, 2, 255, 255, 255], "\x1b[38;2;255;255;255m"),
            vec![CSI::Sgr(Sgr::Foreground(ColorSpec::TrueColor(
                RgbColor::new(255, 255, 255),
            )))]
        );
        assert_eq!(
            parse('m', &[38, 5, 220, 255, 255], "\x1b[38;5;220m\x1b[255;255m"),
            vec![
                CSI::Sgr(Sgr::Foreground(ColorSpec::PaletteIndex(220))),
                CSI::Unspecified {
                    params: [255, 255].to_vec(),
                    intermediates: vec![],
                    ignored_extra_intermediates: false,
                    control: 'm',
                },
            ]
        );
    }

    #[test]
    fn cursor() {
        assert_eq!(
            parse('C', &[], "\x1b[C"),
            vec![CSI::Cursor(Cursor::Right(1))]
        );
        assert_eq!(
            parse('C', &[1], "\x1b[C"),
            vec![CSI::Cursor(Cursor::Right(1))]
        );
        assert_eq!(
            parse('C', &[4], "\x1b[4C"),
            vec![CSI::Cursor(Cursor::Right(4))]
        );
    }
}
