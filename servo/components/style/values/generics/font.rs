/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Generic types for font stuff.

use app_units::Au;
use byteorder::{ReadBytesExt, BigEndian};
use cssparser::Parser;
use num_traits::One;
use parser::{Parse, ParserContext};
use std::fmt::{self, Write};
use std::io::Cursor;
use style_traits::{CssWriter, ParseError, StyleParseErrorKind, ToCss};
use values::distance::{ComputeSquaredDistance, SquaredDistance};

/// https://drafts.csswg.org/css-fonts-4/#feature-tag-value
#[derive(Clone, Debug, Eq, MallocSizeOf, PartialEq, ToComputedValue)]
pub struct FeatureTagValue<Integer> {
    /// A four-character tag, packed into a u32 (one byte per character).
    pub tag: FontTag,
    /// The actual value.
    pub value: Integer,
}

impl<Integer> ToCss for FeatureTagValue<Integer>
where
    Integer: One + ToCss + PartialEq,
{
    fn to_css<W>(&self, dest: &mut CssWriter<W>) -> fmt::Result
    where
        W: Write,
    {
        self.tag.to_css(dest)?;
        // Don't serialize the default value.
        if self.value != Integer::one() {
            dest.write_char(' ')?;
            self.value.to_css(dest)?;
        }

        Ok(())
    }
}

/// Variation setting for a single feature, see:
///
/// https://drafts.csswg.org/css-fonts-4/#font-variation-settings-def
#[derive(Animate, Clone, Debug, Eq, MallocSizeOf, PartialEq, ToComputedValue, ToCss)]
pub struct VariationValue<Number> {
    /// A four-character tag, packed into a u32 (one byte per character).
    #[animation(constant)]
    pub tag: FontTag,
    /// The actual value.
    pub value: Number,
}

impl<Number> ComputeSquaredDistance for VariationValue<Number>
where
    Number: ComputeSquaredDistance,
{
    #[inline]
    fn compute_squared_distance(&self, other: &Self) -> Result<SquaredDistance, ()> {
        if self.tag != other.tag {
            return Err(());
        }
        self.value.compute_squared_distance(&other.value)
    }
}

/// A value both for font-variation-settings and font-feature-settings.
#[derive(Clone, Debug, Eq, MallocSizeOf, PartialEq, ToComputedValue)]
pub struct FontSettings<T>(pub Box<[T]>);

impl<T> FontSettings<T> {
    /// Default value of font settings as `normal`.
    #[inline]
    pub fn normal() -> Self {
        FontSettings(vec![].into_boxed_slice())
    }
}

impl<T: Parse> Parse for FontSettings<T> {
    /// https://drafts.csswg.org/css-fonts-4/#descdef-font-face-font-feature-settings
    /// https://drafts.csswg.org/css-fonts-4/#font-variation-settings-def
    fn parse<'i, 't>(
        context: &ParserContext,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self, ParseError<'i>> {
        if input.try(|i| i.expect_ident_matching("normal")).is_ok() {
            return Ok(Self::normal());
        }

        Ok(FontSettings(
            input.parse_comma_separated(|i| T::parse(context, i))?.into_boxed_slice()
        ))
    }
}

impl<T: ToCss> ToCss for FontSettings<T> {
    /// https://drafts.csswg.org/css-fonts-4/#descdef-font-face-font-feature-settings
    /// https://drafts.csswg.org/css-fonts-4/#font-variation-settings-def
    fn to_css<W>(&self, dest: &mut CssWriter<W>) -> fmt::Result
    where
        W: Write,
    {
        if self.0.is_empty() {
            return dest.write_str("normal");
        }

        let mut first = true;
        for item in self.0.iter() {
            if !first {
                dest.write_str(", ")?;
            }
            first = false;
            item.to_css(dest)?;
        }

        Ok(())
    }
}

/// A font four-character tag, represented as a u32 for convenience.
///
/// See:
///   https://drafts.csswg.org/css-fonts-4/#font-variation-settings-def
///   https://drafts.csswg.org/css-fonts-4/#descdef-font-face-font-feature-settings
///
#[derive(Clone, Copy, Debug, Eq, MallocSizeOf, PartialEq, ToComputedValue)]
pub struct FontTag(pub u32);

impl ToCss for FontTag {
    fn to_css<W>(&self, dest: &mut CssWriter<W>) -> fmt::Result
    where
        W: Write,
    {
        use byteorder::{BigEndian, ByteOrder};
        use std::str;

        let mut raw = [0u8; 4];
        BigEndian::write_u32(&mut raw, self.0);
        str::from_utf8(&raw).unwrap_or_default().to_css(dest)
    }
}

impl Parse for FontTag {
    fn parse<'i, 't>(
        _context: &ParserContext,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self, ParseError<'i>> {
        let location = input.current_source_location();
        let tag = input.expect_string()?;

        // allowed strings of length 4 containing chars: <U+20, U+7E>
        if tag.len() != 4 || tag.as_bytes().iter().any(|c| *c < b' ' || *c > b'~') {
            return Err(location.new_custom_error(StyleParseErrorKind::UnspecifiedError))
        }

        let mut raw = Cursor::new(tag.as_bytes());
        Ok(FontTag(raw.read_u32::<BigEndian>().unwrap()))
    }
}

#[derive(Animate, Clone, ComputeSquaredDistance, Copy, Debug, MallocSizeOf)]
#[derive(PartialEq, ToAnimatedValue, ToAnimatedZero)]
/// Additional information for keyword-derived font sizes.
pub struct KeywordInfo<Length> {
    /// The keyword used
    pub kw: KeywordSize,
    /// A factor to be multiplied by the computed size of the keyword
    pub factor: f32,
    /// An additional Au offset to add to the kw*factor in the case of calcs
    pub offset: Length,
}

impl<L> KeywordInfo<L>
where
    Au: Into<L>,
{
    /// KeywordInfo value for font-size: medium
    pub fn medium() -> Self {
        KeywordSize::Medium.into()
    }
}

impl<L> From<KeywordSize> for KeywordInfo<L>
where
    Au: Into<L>,
{
    fn from(x: KeywordSize) -> Self {
        KeywordInfo {
            kw: x,
            factor: 1.,
            offset: Au(0).into(),
        }
    }
}

/// CSS font keywords
#[derive(Animate, ComputeSquaredDistance, MallocSizeOf, ToAnimatedValue, ToAnimatedZero)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum KeywordSize {
    XXSmall = 1, // This is to enable the NonZero optimization
                 // which simplifies the representation of Option<KeywordSize>
                 // in bindgen
    XSmall,
    Small,
    Medium,
    Large,
    XLarge,
    XXLarge,
    // This is not a real font keyword and will not parse
    // HTML font-size 7 corresponds to this value
    XXXLarge,
}

impl KeywordSize {
    /// Convert to an HTML <font size> value
    pub fn html_size(&self) -> u8 {
        match *self {
            KeywordSize::XXSmall => 0,
            KeywordSize::XSmall => 1,
            KeywordSize::Small => 2,
            KeywordSize::Medium => 3,
            KeywordSize::Large => 4,
            KeywordSize::XLarge => 5,
            KeywordSize::XXLarge => 6,
            KeywordSize::XXXLarge => 7,
        }
    }
}

impl Default for KeywordSize {
    fn default() -> Self {
        KeywordSize::Medium
    }
}

impl ToCss for KeywordSize {
    fn to_css<W>(&self, dest: &mut CssWriter<W>) -> fmt::Result
    where
        W: Write,
    {
        dest.write_str(match *self {
            KeywordSize::XXSmall => "xx-small",
            KeywordSize::XSmall => "x-small",
            KeywordSize::Small => "small",
            KeywordSize::Medium => "medium",
            KeywordSize::Large => "large",
            KeywordSize::XLarge => "x-large",
            KeywordSize::XXLarge => "xx-large",
            KeywordSize::XXXLarge => {
                debug_assert!(
                    false,
                    "We should never serialize specified values set via HTML presentation attributes"
                );
                "-servo-xxx-large"
            },
        })
    }
}
