use std::{fmt, str::FromStr};

use palette::{Hsv, IntoColor, rgb::Srgb};
use serde::{Deserialize, Serialize, ser::SerializeStruct};

#[derive(Debug)]
pub enum ParseLightError {
    ExclusiveFields(&'static str, &'static str),
    IncompleteRgb,
    BadPair(String),
    UnknownField(String),
    BadNumber(&'static str, String),
    MissingPrefix,
    UnknownVariant(String),
}

impl std::fmt::Display for ParseLightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ParseLightError::{
            BadNumber, BadPair, ExclusiveFields, IncompleteRgb, MissingPrefix, UnknownField,
            UnknownVariant,
        };
        match self {
            BadPair(s) => write!(f, "Expected key=value pair, got: `{s}`"),
            ExclusiveFields(existing_field, bad_field) => write!(
                f,
                "Key {existing_field} is mutually exclusive with {bad_field}"
            ),
            IncompleteRgb => write!(f, "RGB overrides must specify red, green, and blue"),
            UnknownField(k) => write!(f, "Unknown field: `{k}`"),
            BadNumber(field, e) => write!(f, "Invalid number for `{field}`: {e}"),
            MissingPrefix => write!(f, "Missing type prefix (e.g., `Fixed:` or `Mult:`)"),
            UnknownVariant(v) => {
                write!(f, "Unknown light type: `{v}` (expected `Fixed` or `Mult`)")
            }
        }
    }
}

impl std::error::Error for ParseLightError {}

fn parse_pairs<F>(s: &str, mut set: F) -> Result<(), ParseLightError>
where
    F: FnMut(&str, &str) -> Result<(), ParseLightError>,
{
    for pair in s.split(',').filter(|p| !p.trim().is_empty()) {
        let (k, v) = pair
            .split_once('=')
            .ok_or_else(|| ParseLightError::BadPair(pair.to_string()))?;
        set(k.trim(), v.trim())?;
    }
    Ok(())
}

impl FromStr for CustomLightData {
    type Err = ParseLightError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut data = CustomLightData::default();
        let mut color = RgbBuilder::default();
        parse_pairs(s, |key, value| data.set_pair(key, value, &mut color))?;
        data.color = color.finish()?;

        Ok(data)
    }
}

pub fn parse_light_override(s: &str) -> Result<(String, CustomLightData), ParseLightError> {
    let (id, setting) = s
        .split_once('=')
        .ok_or_else(|| ParseLightError::BadPair(s.to_string()))?;

    let parsed_setting: CustomLightData = setting.parse()?;
    Ok((id.to_string(), parsed_setting))
}

pub fn parse_ambient_override(s: &str) -> Result<(String, CustomCellAmbient), ParseAmbientError> {
    let (id, setting) = s
        .split_once('=')
        .ok_or_else(|| ParseAmbientError::BadPair(s.to_string()))?;

    let parsed_setting: CustomCellAmbient = setting.parse()?;
    Ok((id.to_string(), parsed_setting))
}

#[derive(Deserialize)]
struct RawCustomLightData {
    red: Option<u8>,
    green: Option<u8>,
    blue: Option<u8>,
    red_mult: Option<f32>,
    green_mult: Option<f32>,
    blue_mult: Option<f32>,
    hue: Option<u32>,
    saturation: Option<f32>,
    value: Option<f32>,
    hue_mult: Option<f32>,
    saturation_mult: Option<f32>,
    value_mult: Option<f32>,
    radius: Option<u32>,
    radius_mult: Option<f32>,
    duration: Option<f32>,
    duration_mult: Option<f32>,
    flag: Option<LightFlag>,
}

impl<'de> serde::Deserialize<'de> for CustomLightData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawCustomLightData::deserialize(deserializer)?;

        // Check exclusivity
        macro_rules! check_exclusive {
            ($field:ident, $mult:ident) => {
                if raw.$field.is_some() && raw.$mult.is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "Fields `{}` and `{}` are mutually exclusive",
                        stringify!($field),
                        stringify!($mult)
                    )));
                }
            };
        }

        check_exclusive!(radius, radius_mult);
        check_exclusive!(duration, duration_mult);

        let red_mult = finite_float(raw.red_mult, "red_mult").map_err(serde::de::Error::custom)?;
        let green_mult =
            finite_float(raw.green_mult, "green_mult").map_err(serde::de::Error::custom)?;
        let blue_mult =
            finite_float(raw.blue_mult, "blue_mult").map_err(serde::de::Error::custom)?;
        let hue_mult = finite_float(raw.hue_mult, "hue_mult").map_err(serde::de::Error::custom)?;
        let saturation_mult = finite_float(raw.saturation_mult, "saturation_mult")
            .map_err(serde::de::Error::custom)?;
        let value_mult =
            finite_float(raw.value_mult, "value_mult").map_err(serde::de::Error::custom)?;
        let radius_mult =
            finite_float(raw.radius_mult, "radius_mult").map_err(serde::de::Error::custom)?;
        let duration = finite_float(raw.duration, "duration").map_err(serde::de::Error::custom)?;
        let duration_mult =
            finite_float(raw.duration_mult, "duration_mult").map_err(serde::de::Error::custom)?;
        let saturation =
            finite_float(raw.saturation, "saturation").map_err(serde::de::Error::custom)?;
        let value = finite_float(raw.value, "value").map_err(serde::de::Error::custom)?;

        let rgb_color =
            rgb_from_parts(raw.red, raw.green, raw.blue).map_err(serde::de::Error::custom)?;
        let (legacy_hsv_color, hue, saturation, value) = migrate_or_keep_legacy_hsv(
            raw.hue,
            saturation,
            value,
            hue_mult,
            saturation_mult,
            value_mult,
            rgb_color.is_none(),
        );

        if hue.is_some() && hue_mult.is_some() {
            return Err(serde::de::Error::custom(
                "Fields `hue` and `hue_mult` are mutually exclusive",
            ));
        }
        if saturation.is_some() && saturation_mult.is_some() {
            return Err(serde::de::Error::custom(
                "Fields `saturation` and `saturation_mult` are mutually exclusive",
            ));
        }
        if value.is_some() && value_mult.is_some() {
            return Err(serde::de::Error::custom(
                "Fields `value` and `value_mult` are mutually exclusive",
            ));
        }

        let color = rgb_color.or(legacy_hsv_color);

        Ok(CustomLightData {
            color,
            migrated_from_hsv: legacy_hsv_color.is_some(),
            red_mult,
            green_mult,
            blue_mult,
            hue,
            saturation,
            value,
            hue_mult,
            saturation_mult,
            value_mult,
            radius: raw.radius,
            radius_mult,
            duration,
            duration_mult,
            flag: raw.flag,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct CustomLightData {
    pub color: Option<[u8; 4]>,
    pub migrated_from_hsv: bool,
    pub red_mult: Option<f32>,
    pub green_mult: Option<f32>,
    pub blue_mult: Option<f32>,
    pub hue: Option<u32>,
    pub saturation: Option<f32>,
    pub value: Option<f32>,
    pub hue_mult: Option<f32>,
    pub saturation_mult: Option<f32>,
    pub value_mult: Option<f32>,
    pub radius: Option<u32>,
    pub radius_mult: Option<f32>,
    pub duration: Option<f32>,
    pub duration_mult: Option<f32>,
    pub flag: Option<LightFlag>,
}

impl Serialize for CustomLightData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut fields = 0;
        fields += usize::from(self.color.is_some()) * 3;
        fields += usize::from(self.red_mult.is_some());
        fields += usize::from(self.green_mult.is_some());
        fields += usize::from(self.blue_mult.is_some());
        fields += usize::from(self.hue.is_some());
        fields += usize::from(self.saturation.is_some());
        fields += usize::from(self.value.is_some());
        fields += usize::from(self.hue_mult.is_some());
        fields += usize::from(self.saturation_mult.is_some());
        fields += usize::from(self.value_mult.is_some());
        fields += usize::from(self.radius.is_some());
        fields += usize::from(self.radius_mult.is_some());
        fields += usize::from(self.duration.is_some());
        fields += usize::from(self.duration_mult.is_some());
        fields += usize::from(self.flag.is_some());

        let mut state = serializer.serialize_struct("CustomLightData", fields)?;
        if let Some([red, green, blue, _]) = self.color {
            state.serialize_field("red", &red)?;
            state.serialize_field("green", &green)?;
            state.serialize_field("blue", &blue)?;
        }
        if let Some(value) = self.red_mult {
            state.serialize_field("red_mult", &value)?;
        }
        if let Some(value) = self.green_mult {
            state.serialize_field("green_mult", &value)?;
        }
        if let Some(value) = self.blue_mult {
            state.serialize_field("blue_mult", &value)?;
        }
        if let Some(value) = self.hue {
            state.serialize_field("hue", &value)?;
        }
        if let Some(value) = self.saturation {
            state.serialize_field("saturation", &value)?;
        }
        if let Some(value) = self.value {
            state.serialize_field("value", &value)?;
        }
        if let Some(value) = self.hue_mult {
            state.serialize_field("hue_mult", &value)?;
        }
        if let Some(value) = self.saturation_mult {
            state.serialize_field("saturation_mult", &value)?;
        }
        if let Some(value) = self.value_mult {
            state.serialize_field("value_mult", &value)?;
        }
        if let Some(value) = self.radius {
            state.serialize_field("radius", &value)?;
        }
        if let Some(value) = self.radius_mult {
            state.serialize_field("radius_mult", &value)?;
        }
        if let Some(value) = self.duration {
            state.serialize_field("duration", &value)?;
        }
        if let Some(value) = self.duration_mult {
            state.serialize_field("duration_mult", &value)?;
        }
        if let Some(value) = &self.flag {
            state.serialize_field("flag", value)?;
        }

        state.end()
    }
}

#[derive(Default)]
struct RgbBuilder {
    red: Option<u8>,
    green: Option<u8>,
    blue: Option<u8>,
}

impl RgbBuilder {
    fn finish(self) -> Result<Option<[u8; 4]>, ParseLightError> {
        rgb_from_parts(self.red, self.green, self.blue).map_err(|_| ParseLightError::IncompleteRgb)
    }
}

fn rgb_from_parts(
    red: Option<u8>,
    green: Option<u8>,
    blue: Option<u8>,
) -> Result<Option<[u8; 4]>, &'static str> {
    match (red, green, blue) {
        (None, None, None) => Ok(None),
        (Some(red), Some(green), Some(blue)) => Ok(Some([red, green, blue, 0])),
        _ => Err("RGB overrides must specify red, green, and blue"),
    }
}

fn finite_float(value: Option<f32>, field: &'static str) -> Result<Option<f32>, &'static str> {
    match value {
        Some(value) if value.is_finite() => Ok(Some(value)),
        Some(_) => Err(match field {
            "red_mult" => "red_mult must be finite",
            "green_mult" => "green_mult must be finite",
            "blue_mult" => "blue_mult must be finite",
            "hue_mult" => "hue_mult must be finite",
            "saturation_mult" => "saturation_mult must be finite",
            "value_mult" => "value_mult must be finite",
            "radius_mult" => "radius_mult must be finite",
            "duration" => "duration must be finite",
            "duration_mult" => "duration_mult must be finite",
            "saturation" => "saturation must be finite",
            "value" => "value must be finite",
            _ => "float value must be finite",
        }),
        None => Ok(None),
    }
}

fn legacy_hsv_to_rgb(
    hue: Option<u32>,
    saturation: Option<f32>,
    value: Option<f32>,
) -> Result<Option<[u8; 4]>, &'static str> {
    match (hue, saturation, value) {
        (None, None, None) => Ok(None),
        (Some(hue), Some(saturation), Some(value)) => Ok(Some(hsv_to_rgb8(hue, saturation, value))),
        _ => Err(
            "legacy HSV color overrides must specify hue, saturation, and value to migrate to RGB",
        ),
    }
}

fn migrate_or_keep_legacy_hsv(
    hue: Option<u32>,
    saturation: Option<f32>,
    value: Option<f32>,
    hue_mult: Option<f32>,
    saturation_mult: Option<f32>,
    value_mult: Option<f32>,
    migrate_complete: bool,
) -> (Option<[u8; 4]>, Option<u32>, Option<f32>, Option<f32>) {
    match (hue, saturation, value) {
        (Some(hue), Some(saturation), Some(value))
            if migrate_complete
                && hue_mult.is_none()
                && saturation_mult.is_none()
                && value_mult.is_none() =>
        {
            (Some(hsv_to_rgb8(hue, saturation, value)), None, None, None)
        }
        (hue, saturation, value) => (
            None,
            hue.map(|hue| hue.clamp(0, 360)),
            saturation.map(|saturation| saturation.clamp(0.0, 1.0)),
            value.map(|value| value.clamp(0.0, 1.0)),
        ),
    }
}

#[allow(clippy::cast_precision_loss)]
fn hsv_to_rgb8(hue: u32, saturation: f32, value: f32) -> [u8; 4] {
    let hsv = Hsv::from_components((
        palette::RgbHue::from_degrees(hue.clamp(0, 360) as f32),
        saturation.clamp(0.0, 1.0),
        value.clamp(0.0, 1.0),
    ));
    let rgb8_color: Srgb<u8> = <Hsv as IntoColor<Srgb>>::into_color(hsv).into_format();

    [rgb8_color.red, rgb8_color.green, rgb8_color.blue, 0]
}

fn parse_clamped_unit_float(field: &'static str, value: &str) -> Result<f32, ParseLightError> {
    let value = value
        .parse::<f32>()
        .map_err(|e| ParseLightError::BadNumber(field, e.to_string()))?;
    if !value.is_finite() {
        return Err(ParseLightError::BadNumber(
            field,
            "value must be finite".to_owned(),
        ));
    }
    Ok(value.clamp(0.0, 1.0))
}

impl CustomLightData {
    fn set_float_mult(
        target: &mut Option<f32>,
        fixed_is_set: bool,
        fixed_name: &'static str,
        mult_name: &'static str,
        value: &str,
    ) -> Result<(), ParseLightError> {
        if fixed_is_set {
            return Err(ParseLightError::ExclusiveFields(fixed_name, mult_name));
        }

        *target = Some(value.parse().map_err(|e: std::num::ParseFloatError| {
            ParseLightError::BadNumber(mult_name, e.to_string())
        })?);
        if target.as_ref().is_some_and(|value| !value.is_finite()) {
            return Err(ParseLightError::BadNumber(
                mult_name,
                "value must be finite".to_owned(),
            ));
        }
        Ok(())
    }

    fn set_pair(
        &mut self,
        key: &str,
        value: &str,
        color: &mut RgbBuilder,
    ) -> Result<(), ParseLightError> {
        match key {
            "radius_mult" => Self::set_float_mult(
                &mut self.radius_mult,
                self.radius.is_some(),
                "radius",
                "radius_mult",
                value,
            ),
            "red_mult" => Self::set_plain_float(&mut self.red_mult, "red_mult", value),
            "green_mult" => Self::set_plain_float(&mut self.green_mult, "green_mult", value),
            "blue_mult" => Self::set_plain_float(&mut self.blue_mult, "blue_mult", value),
            "hue_mult" => Self::set_float_mult(
                &mut self.hue_mult,
                self.hue.is_some(),
                "hue",
                "hue_mult",
                value,
            ),
            "saturation_mult" => Self::set_float_mult(
                &mut self.saturation_mult,
                self.saturation.is_some(),
                "saturation",
                "saturation_mult",
                value,
            ),
            "value_mult" => Self::set_float_mult(
                &mut self.value_mult,
                self.value.is_some(),
                "value",
                "value_mult",
                value,
            ),
            "duration_mult" => Self::set_float_mult(
                &mut self.duration_mult,
                self.duration.is_some(),
                "duration",
                "duration_mult",
                value,
            ),
            "duration" => self.set_duration(value),
            "radius" => self.set_radius(value),
            "hue" => self.set_hue(value),
            "saturation" => self.set_saturation(value),
            "value" => self.set_value(value),
            "red" => Self::set_color_component(&mut color.red, "red", value),
            "green" => Self::set_color_component(&mut color.green, "green", value),
            "blue" => Self::set_color_component(&mut color.blue, "blue", value),
            "flag" => {
                self.flag = Some(value.parse()?);
                Ok(())
            }
            _ => Err(ParseLightError::UnknownField(key.to_owned())),
        }
    }

    fn set_duration(&mut self, value: &str) -> Result<(), ParseLightError> {
        if self.duration_mult.is_some() {
            return Err(ParseLightError::ExclusiveFields(
                "duration_mult",
                "duration",
            ));
        }
        self.duration = Some(value.parse().map_err(|e: std::num::ParseFloatError| {
            ParseLightError::BadNumber("duration", e.to_string())
        })?);
        if self.duration.is_some_and(|value| !value.is_finite()) {
            return Err(ParseLightError::BadNumber(
                "duration",
                "value must be finite".to_owned(),
            ));
        }
        Ok(())
    }

    fn set_plain_float(
        target: &mut Option<f32>,
        field: &'static str,
        value: &str,
    ) -> Result<(), ParseLightError> {
        *target = Some(value.parse().map_err(|e: std::num::ParseFloatError| {
            ParseLightError::BadNumber(field, e.to_string())
        })?);
        if target.as_ref().is_some_and(|value| !value.is_finite()) {
            return Err(ParseLightError::BadNumber(
                field,
                "value must be finite".to_owned(),
            ));
        }
        Ok(())
    }

    fn set_hue(&mut self, value: &str) -> Result<(), ParseLightError> {
        if self.hue_mult.is_some() {
            return Err(ParseLightError::ExclusiveFields("hue_mult", "hue"));
        }
        self.hue = Some(
            value
                .parse::<u32>()
                .map_err(|e| ParseLightError::BadNumber("hue", e.to_string()))?
                .clamp(0, 360),
        );
        Ok(())
    }

    fn set_saturation(&mut self, value: &str) -> Result<(), ParseLightError> {
        if self.saturation_mult.is_some() {
            return Err(ParseLightError::ExclusiveFields(
                "saturation_mult",
                "saturation",
            ));
        }
        self.saturation = Some(parse_clamped_unit_float("saturation", value)?);
        Ok(())
    }

    fn set_value(&mut self, value: &str) -> Result<(), ParseLightError> {
        if self.value_mult.is_some() {
            return Err(ParseLightError::ExclusiveFields("value_mult", "value"));
        }
        self.value = Some(parse_clamped_unit_float("value", value)?);
        Ok(())
    }

    fn set_radius(&mut self, value: &str) -> Result<(), ParseLightError> {
        if self.radius_mult.is_some() {
            return Err(ParseLightError::ExclusiveFields("radius_mult", "radius"));
        }
        self.radius = Some(value.parse().map_err(|e: std::num::ParseIntError| {
            ParseLightError::BadNumber("radius", e.to_string())
        })?);
        Ok(())
    }

    fn set_color_component(
        target: &mut Option<u8>,
        field: &'static str,
        value: &str,
    ) -> Result<(), ParseLightError> {
        *target = Some(value.parse().map_err(|e: std::num::ParseIntError| {
            ParseLightError::BadNumber(field, e.to_string())
        })?);
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize)]
/// RGB color replacement using the same component range as TES3 light records.
pub struct TypedLightColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    #[serde(skip)]
    pub migrated_from_hsv: bool,
}

#[derive(Deserialize)]
struct RawTypedLightColor {
    red: Option<u8>,
    green: Option<u8>,
    blue: Option<u8>,
    hue: Option<u32>,
    saturation: Option<f32>,
    value: Option<f32>,
}

impl<'de> Deserialize<'de> for TypedLightColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawTypedLightColor::deserialize(deserializer)?;
        let rgb_color =
            rgb_from_parts(raw.red, raw.green, raw.blue).map_err(serde::de::Error::custom)?;
        let legacy_hsv_color = legacy_hsv_to_rgb(raw.hue, raw.saturation, raw.value)
            .map_err(serde::de::Error::custom)?;

        if rgb_color.is_some() && legacy_hsv_color.is_some() {
            return Err(serde::de::Error::custom(
                "RGB color fields are mutually exclusive with legacy HSV color fields",
            ));
        }

        let Some([red, green, blue, _]) = rgb_color.or(legacy_hsv_color) else {
            return Err(serde::de::Error::custom(
                "RGB colors must specify red, green, and blue",
            ));
        };

        Ok(Self {
            red,
            green,
            blue,
            migrated_from_hsv: legacy_hsv_color.is_some(),
        })
    }
}

impl TypedLightColor {
    pub const fn to_esp_color(&self) -> [u8; 4] {
        [self.red, self.green, self.blue, 0]
    }
}

#[derive(Debug)]
pub enum ParseTypedColorError {
    MissingField(&'static str),
    UnknownField(String),
    BadNumber(&'static str, String),
    BadPair(String),
}

impl fmt::Display for ParseTypedColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ParseTypedColorError::{BadNumber, BadPair, MissingField, UnknownField};
        match self {
            MissingField(name) => write!(f, "Missing required field: `{name}`"),
            UnknownField(name) => write!(f, "Unknown field: `{name}`"),
            BadNumber(field, msg) => write!(f, "Invalid value for `{field}`: {msg}"),
            BadPair(pair) => write!(f, "Expected key=value pair, got: `{pair}`"),
        }
    }
}

impl std::error::Error for ParseTypedColorError {}

impl FromStr for TypedLightColor {
    type Err = ParseTypedColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut red: Option<u8> = None;
        let mut green: Option<u8> = None;
        let mut blue: Option<u8> = None;

        for pair in s.split(',').filter(|p| !p.trim().is_empty()) {
            let (k, v) = pair
                .split_once('=')
                .ok_or_else(|| ParseTypedColorError::BadPair(pair.to_string()))?;

            match k.trim() {
                "red" => {
                    red = Some(v.trim().parse().map_err(|e: std::num::ParseIntError| {
                        ParseTypedColorError::BadNumber("red", e.to_string())
                    })?);
                }
                "green" => {
                    green = Some(v.trim().parse().map_err(|e: std::num::ParseIntError| {
                        ParseTypedColorError::BadNumber("green", e.to_string())
                    })?);
                }
                "blue" => {
                    blue = Some(v.trim().parse().map_err(|e: std::num::ParseIntError| {
                        ParseTypedColorError::BadNumber("blue", e.to_string())
                    })?);
                }
                other => return Err(ParseTypedColorError::UnknownField(other.to_string())),
            }
        }

        Ok(TypedLightColor {
            red: red.ok_or(ParseTypedColorError::MissingField("red"))?,
            green: green.ok_or(ParseTypedColorError::MissingField("green"))?,
            blue: blue.ok_or(ParseTypedColorError::MissingField("blue"))?,
            migrated_from_hsv: false,
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CustomCellAmbient {
    pub ambient: Option<TypedLightColor>,
    pub sunlight: Option<TypedLightColor>,
    pub fog: Option<TypedLightColor>,
    pub fog_density: Option<f32>,
}

impl CustomCellAmbient {
    #[must_use]
    pub fn migrated_from_hsv(&self) -> bool {
        self.ambient
            .as_ref()
            .is_some_and(|color| color.migrated_from_hsv)
            || self
                .sunlight
                .as_ref()
                .is_some_and(|color| color.migrated_from_hsv)
            || self
                .fog
                .as_ref()
                .is_some_and(|color| color.migrated_from_hsv)
    }
}

#[derive(Debug)]
pub enum ParseAmbientError {
    BadPair(String),
    UnknownField(String),
    BadColor(String, Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for ParseAmbientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ParseAmbientError::{BadColor, BadPair, UnknownField};
        match self {
            BadPair(pair) => write!(f, "Expected key=value pair, got: `{pair}`"),
            UnknownField(field) => write!(f, "Unknown field: `{field}`"),
            BadColor(field, err) => write!(f, "Invalid color for `{field}`: {err}"),
        }
    }
}

impl std::error::Error for ParseAmbientError {}

impl FromStr for CustomCellAmbient {
    type Err = ParseAmbientError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ambient = None;
        let mut sunlight = None;
        let mut fog = None;
        let mut fog_density = None;

        for pair in s.split(';').filter(|p| !p.trim().is_empty()) {
            let (key, value) = pair
                .split_once('=')
                .ok_or_else(|| ParseAmbientError::BadPair(pair.to_string()))?;

            match key.trim() {
                "ambient" => {
                    let parsed = value
                        .parse()
                        .map_err(|e| ParseAmbientError::BadColor("ambient".into(), Box::new(e)))?;
                    ambient = Some(parsed);
                }
                "sunlight" => {
                    let parsed = value
                        .parse()
                        .map_err(|e| ParseAmbientError::BadColor("sunlight".into(), Box::new(e)))?;
                    sunlight = Some(parsed);
                }
                "fog" => {
                    let parsed = value
                        .parse()
                        .map_err(|e| ParseAmbientError::BadColor("fog".into(), Box::new(e)))?;
                    fog = Some(parsed);
                }
                "fog_density" => {
                    let parsed: f32 = value.parse().map_err(|e| {
                        ParseAmbientError::BadColor("fog_density".into(), Box::new(e))
                    })?;
                    fog_density = Some(parsed);
                }
                other => return Err(ParseAmbientError::UnknownField(other.to_string())),
            }
        }

        Ok(CustomCellAmbient {
            ambient,
            sunlight,
            fog,
            fog_density,
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum LightFlag {
    #[serde(rename = "FLICKERSLOW")]
    FlickerSlow,
    #[serde(rename = "FLICKER")]
    Flicker,
    #[serde(rename = "PULSE")]
    Pulse,
    #[serde(rename = "PULSESLOW")]
    PulseSlow,
    #[default]
    #[serde(rename = "NONE")]
    None,
}

use tes3::esp::LightFlags;
impl LightFlag {
    pub fn to_esp_flag(&self) -> LightFlags {
        match &self {
            Self::Flicker => LightFlags::FLICKER,
            Self::FlickerSlow => LightFlags::FLICKER_SLOW,
            Self::Pulse => LightFlags::PULSE,
            Self::PulseSlow => LightFlags::PULSE_SLOW,
            Self::None => LightFlags::empty(),
        }
    }
}

impl FromStr for LightFlag {
    type Err = ParseLightError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "flicker" => Ok(LightFlag::Flicker),
            "flickerslow" => Ok(LightFlag::FlickerSlow),
            "pulse" => Ok(LightFlag::Pulse),
            "pulseslow" => Ok(LightFlag::PulseSlow),
            "none" => Ok(LightFlag::None),
            _ => Err(ParseLightError::UnknownVariant(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cli_light_override_fixed_fields() {
        let (id, data) = parse_light_override(
            "Torch_001=radius=255,duration=1200,red=10,green=20,blue=30,flag=FLICKERSLOW",
        )
        .unwrap();

        assert_eq!(id, "Torch_001");
        assert_eq!(data.radius, Some(255));
        assert_eq!(data.duration, Some(1200.0));
        assert_eq!(data.color, Some([10, 20, 30, 0]));
        assert!(matches!(data.flag, Some(LightFlag::FlickerSlow)));
    }

    #[test]
    fn parses_cli_light_override_multiplier_fields() {
        let (_, data) = parse_light_override(
            "Torch_002=radius_mult=2.0,duration_mult=3.0,hue_mult=4.0,saturation_mult=0.5,value_mult=0.25,red_mult=1.1,green_mult=0.8,blue_mult=0.25",
        )
        .unwrap();

        assert_eq!(data.radius_mult, Some(2.0));
        assert_eq!(data.duration_mult, Some(3.0));
        assert_eq!(data.red_mult, Some(1.1));
        assert_eq!(data.green_mult, Some(0.8));
        assert_eq!(data.blue_mult, Some(0.25));
        assert_eq!(data.hue_mult, Some(4.0));
        assert_eq!(data.saturation_mult, Some(0.5));
        assert_eq!(data.value_mult, Some(0.25));
    }

    #[test]
    fn parses_cli_light_override_fixed_hsv_fields() {
        let (_, data) = parse_light_override("Torch=hue=999,saturation=2.0,value=0.25").unwrap();

        assert_eq!(data.hue, Some(360));
        assert_eq!(data.saturation, Some(1.0));
        assert_eq!(data.value, Some(0.25));
    }

    #[test]
    fn cli_light_override_rejects_fixed_and_multiplier_for_same_field() {
        let err = parse_light_override("Torch=radius=10,radius_mult=2.0").unwrap_err();

        assert!(matches!(
            err,
            ParseLightError::ExclusiveFields("radius", "radius_mult")
        ));
    }

    #[test]
    fn cli_light_override_rejects_incomplete_rgb_color() {
        let err = parse_light_override("Torch=red=255,green=128").unwrap_err();

        assert!(matches!(err, ParseLightError::IncompleteRgb));
    }

    #[test]
    fn cli_light_override_rejects_out_of_range_rgb_component() {
        let err = parse_light_override("Torch=red=999,green=128,blue=64").unwrap_err();

        assert!(matches!(err, ParseLightError::BadNumber("red", _)));
    }

    #[test]
    fn cli_light_override_allows_fixed_rgb_with_hsv_multiplier() {
        let (_, data) =
            parse_light_override("Torch=red=255,green=128,blue=64,hue_mult=2.0,red_mult=0.5")
                .unwrap();

        assert_eq!(data.color, Some([255, 128, 64, 0]));
        assert_eq!(data.hue_mult, Some(2.0));
        assert_eq!(data.red_mult, Some(0.5));
    }

    #[test]
    fn cli_light_override_rejects_hsv_fixed_and_multiplier_in_both_orders() {
        for raw in [
            "Torch=hue=10,hue_mult=2.0",
            "Torch=hue_mult=2.0,hue=10",
            "Torch=saturation=0.5,saturation_mult=2.0",
            "Torch=saturation_mult=2.0,saturation=0.5",
            "Torch=value=0.5,value_mult=2.0",
            "Torch=value_mult=2.0,value=0.5",
        ] {
            assert!(parse_light_override(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn cli_light_override_rejects_complete_hsv_with_hsv_multiplier() {
        let err = parse_light_override("Torch=hue=180,saturation=1.0,value=1.0,hue_mult=2.0")
            .unwrap_err();

        assert!(matches!(
            err,
            ParseLightError::ExclusiveFields("hue", "hue_mult")
        ));
    }

    #[test]
    fn cli_light_override_rejects_non_finite_rgb_multiplier() {
        let err = parse_light_override("Torch=red_mult=NaN").unwrap_err();

        assert!(matches!(err, ParseLightError::BadNumber("red_mult", _)));
    }

    #[test]
    fn parses_cli_ambient_override_all_fields() {
        let (id, ambient) = parse_ambient_override(
            "caius=ambient=red=10,green=20,blue=30;sunlight=red=40,green=50,blue=60;fog=red=70,green=80,blue=90;fog_density=0.25",
        )
        .unwrap();

        assert_eq!(id, "caius");
        assert_eq!(
            ambient.ambient.as_ref().unwrap().to_esp_color(),
            [10, 20, 30, 0]
        );
        assert_eq!(
            ambient.sunlight.as_ref().unwrap().to_esp_color(),
            [40, 50, 60, 0]
        );
        assert_eq!(
            ambient.fog.as_ref().unwrap().to_esp_color(),
            [70, 80, 90, 0]
        );
        assert_eq!(ambient.fog_density, Some(0.25));
    }

    #[test]
    fn cli_ambient_override_reports_bad_nested_color() {
        let err = parse_ambient_override("caius=ambient=red=30,green=50").unwrap_err();

        assert!(matches!(err, ParseAmbientError::BadColor(field, _) if field == "ambient"));
    }

    #[test]
    fn cli_ambient_override_rejects_unknown_fields() {
        let err = parse_ambient_override("caius=glow=red=30,green=50,blue=60").unwrap_err();

        assert!(matches!(err, ParseAmbientError::UnknownField(field) if field == "glow"));
    }

    #[test]
    fn toml_light_data_rejects_fixed_and_multiplier_for_same_field() {
        let err = toml::from_str::<CustomLightData>("radius = 10\nradius_mult = 2.0").unwrap_err();

        assert!(err.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn toml_typed_light_color_uses_rgb_components() {
        let color = toml::from_str::<TypedLightColor>("red = 10\ngreen = 20\nblue = 30").unwrap();

        assert_eq!(color.to_esp_color(), [10, 20, 30, 0]);
        assert!(!color.migrated_from_hsv);
    }

    #[test]
    fn toml_typed_light_color_rejects_out_of_range_rgb_component() {
        let err =
            toml::from_str::<TypedLightColor>("red = 256\ngreen = 20\nblue = 30").unwrap_err();

        assert!(err.to_string().contains("invalid value"));
    }

    #[test]
    fn toml_legacy_hsv_light_color_migrates_to_rgb() {
        let data = toml::from_str::<CustomLightData>(
            "hue = 180\nsaturation = 1.0\nvalue = 1.0\nradius = 100",
        )
        .unwrap();

        assert_eq!(data.color, Some([0, 255, 255, 0]));
        assert!(data.migrated_from_hsv);
    }

    #[test]
    fn toml_partial_legacy_hsv_light_color_is_preserved() {
        let data = toml::from_str::<CustomLightData>("hue = 999\nsaturation = 2.0\n").unwrap();

        assert_eq!(data.color, None);
        assert_eq!(data.hue, Some(360));
        assert_eq!(data.saturation, Some(1.0));
        assert_eq!(data.value, None);
        assert!(!data.migrated_from_hsv);
    }

    #[test]
    fn toml_complete_hsv_with_hsv_multiplier_is_not_silently_migrated() {
        let err = toml::from_str::<CustomLightData>(
            "hue = 180\nsaturation = 1.0\nvalue = 1.0\nhue_mult = 2.0",
        )
        .unwrap_err();

        assert!(err.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn toml_light_data_rejects_non_finite_rgb_multiplier() {
        let err = toml::from_str::<CustomLightData>("red_mult = nan").unwrap_err();

        assert!(err.to_string().contains("red_mult must be finite"));
    }

    #[test]
    fn toml_legacy_hsv_ambient_color_migrates_to_rgb() {
        let color =
            toml::from_str::<TypedLightColor>("hue = 120\nsaturation = 1.0\nvalue = 1.0").unwrap();

        assert_eq!(color.to_esp_color(), [0, 255, 0, 0]);
        assert!(color.migrated_from_hsv);
    }

    #[test]
    fn toml_light_data_serializes_rgb_as_named_components() {
        let serialized = toml::to_string(&CustomLightData {
            color: Some([10, 20, 30, 0]),
            red_mult: Some(1.5),
            radius: Some(100),
            ..CustomLightData::default()
        })
        .unwrap();

        assert!(serialized.contains("red = 10"));
        assert!(serialized.contains("green = 20"));
        assert!(serialized.contains("blue = 30"));
        assert!(serialized.contains("red_mult = 1.5"));
        assert!(!serialized.contains("color"));
    }

    #[test]
    fn toml_light_data_allows_rgb_and_hsv_multipliers_together() {
        let data = toml::from_str::<CustomLightData>(
            "red = 10\ngreen = 20\nblue = 30\nhue_mult = 2.0\nred_mult = 0.5",
        )
        .unwrap();

        assert_eq!(data.color, Some([10, 20, 30, 0]));
        assert_eq!(data.hue_mult, Some(2.0));
        assert_eq!(data.red_mult, Some(0.5));
    }

    #[test]
    fn toml_light_flag_accepts_documented_uppercase_names() {
        #[derive(Deserialize)]
        struct FlagWrapper {
            flag: LightFlag,
        }

        for (raw, expected) in [
            ("FLICKERSLOW", LightFlag::FlickerSlow),
            ("FLICKER", LightFlag::Flicker),
            ("PULSE", LightFlag::Pulse),
            ("PULSESLOW", LightFlag::PulseSlow),
            ("NONE", LightFlag::None),
        ] {
            let parsed = toml::from_str::<FlagWrapper>(&format!("flag = '{raw}'")).unwrap();

            assert!(std::mem::discriminant(&parsed.flag) == std::mem::discriminant(&expected));
        }
    }
}
