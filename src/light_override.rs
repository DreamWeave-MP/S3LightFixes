use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum ParseLightError {
    ExclusiveFields(&'static str, &'static str),
    BadPair(String),
    UnknownField(String),
    BadNumber(&'static str, String),
    MissingPrefix,
    UnknownVariant(String),
}

impl std::fmt::Display for ParseLightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ParseLightError::*;
        match self {
            BadPair(s) => write!(f, "Expected key=value pair, got: `{s}`"),
            ExclusiveFields(existing_field, bad_field) => write!(
                f,
                "Key {existing_field} is mutually exclusive with {bad_field}"
            ),
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

        parse_pairs(s, |k, v| {
            match k {
                "radius_mult" => {
                    if let Some(_) = data.radius {
                        return Err(ParseLightError::ExclusiveFields("radius", "radius_mult"));
                    }

                    data.radius_mult = Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                        ParseLightError::BadNumber("radius", e.to_string())
                    })?)
                }
                "hue_mult" => {
                    if let Some(_) = data.hue {
                        return Err(ParseLightError::ExclusiveFields("hue", "hue_mult"));
                    }

                    data.hue_mult = Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                        ParseLightError::BadNumber("hue", e.to_string())
                    })?)
                }
                "saturation_mult" => {
                    if let Some(_) = data.saturation {
                        return Err(ParseLightError::ExclusiveFields(
                            "saturation",
                            "saturation_mult",
                        ));
                    }

                    data.saturation_mult =
                        Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                            ParseLightError::BadNumber("saturation", e.to_string())
                        })?)
                }
                "value_mult" => {
                    if let Some(_) = data.value {
                        return Err(ParseLightError::ExclusiveFields("value", "value_mult"));
                    }

                    data.value_mult = Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                        ParseLightError::BadNumber("value_mult", e.to_string())
                    })?)
                }

                "duration_mult" => {
                    if let Some(_) = data.duration {
                        return Err(ParseLightError::ExclusiveFields(
                            "duration",
                            "duration_mult",
                        ));
                    }

                    data.duration_mult =
                        Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                            ParseLightError::BadNumber("duration_mult", e.to_string())
                        })?)
                }

                "duration" => {
                    if let Some(_) = data.duration_mult {
                        return Err(ParseLightError::ExclusiveFields(
                            "duration_mult",
                            "duration",
                        ));
                    }

                    data.duration = Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                        ParseLightError::BadNumber("duration", e.to_string())
                    })?)
                }

                "radius" => {
                    if let Some(_) = data.radius_mult {
                        return Err(ParseLightError::ExclusiveFields("radius_mult", "radius"));
                    }

                    data.radius = Some(v.parse().map_err(|e: std::num::ParseIntError| {
                        ParseLightError::BadNumber("radius", e.to_string())
                    })?)
                }
                "hue" => {
                    if let Some(_) = data.hue_mult {
                        return Err(ParseLightError::ExclusiveFields("hue_mult", "hue"));
                    }

                    let parsed: u32 = v.parse().map_err(|e: std::num::ParseIntError| {
                        ParseLightError::BadNumber("hue", e.to_string())
                    })?;

                    data.hue = Some(parsed.clamp(0, 360))
                }
                "saturation" => {
                    if let Some(_) = data.saturation_mult {
                        return Err(ParseLightError::ExclusiveFields(
                            "saturation_mult",
                            "saturation",
                        ));
                    }

                    let parsed: f32 = v.parse().map_err(|e: std::num::ParseFloatError| {
                        ParseLightError::BadNumber("saturation", e.to_string())
                    })?;

                    data.saturation = Some(parsed.clamp(0.0, 1.0))
                }
                "value" => {
                    if let Some(_) = data.value_mult {
                        return Err(ParseLightError::ExclusiveFields("value_mult", "value"));
                    }

                    let parsed: f32 = v.parse().map_err(|e: std::num::ParseFloatError| {
                        ParseLightError::BadNumber("value", e.to_string())
                    })?;

                    data.value = Some(parsed.clamp(0.0, 1.0))
                }
                "flag" => {
                    let parsed: LightFlag = v.parse()?;
                    data.flag = Some(parsed);
                }
                _ => return Err(ParseLightError::UnknownField(k.to_owned())),
            }
            Ok(())
        })?;

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
    hue: Option<u32>,
    hue_mult: Option<f32>,
    saturation: Option<f32>,
    saturation_mult: Option<f32>,
    value: Option<f32>,
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

        check_exclusive!(hue, hue_mult);
        check_exclusive!(saturation, saturation_mult);
        check_exclusive!(value, value_mult);
        check_exclusive!(radius, radius_mult);
        check_exclusive!(duration, duration_mult);

        Ok(CustomLightData {
            hue: raw.hue.map(|h| h.clamp(0, 360)),
            hue_mult: raw.hue_mult,
            saturation: raw.saturation.map(|s| s.clamp(0.0, 1.0)),
            saturation_mult: raw.saturation_mult,
            value: raw.value.map(|v| v.clamp(0.0, 1.0)),
            value_mult: raw.value_mult,
            radius: raw.radius,
            radius_mult: raw.radius_mult,
            duration: raw.duration,
            duration_mult: raw.duration_mult,
            flag: raw.flag,
        })
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CustomLightData {
    pub hue: Option<u32>,
    pub hue_mult: Option<f32>,
    pub saturation: Option<f32>,
    pub saturation_mult: Option<f32>,
    pub value: Option<f32>,
    pub value_mult: Option<f32>,
    pub radius: Option<u32>,
    pub radius_mult: Option<f32>,
    pub duration: Option<f32>,
    pub duration_mult: Option<f32>,
    pub flag: Option<LightFlag>,
}

#[derive(Clone, Debug, Default, Serialize)]
/// Struct used to store color replacements for cells.
/// No fields are optional, unlike light record replacements. Nor are multipliers supported.
pub struct TypedLightColor {
    pub hue: u32,
    pub saturation: f32,
    pub value: f32,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct RawTypedLightColor {
    pub hue: u32,
    pub saturation: f32,
    pub value: f32,
}

impl<'de> serde::Deserialize<'de> for TypedLightColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: RawTypedLightColor = RawTypedLightColor::deserialize(deserializer)?;

        Ok(TypedLightColor {
            hue: raw.hue.clamp(0, 360),
            saturation: raw.saturation.clamp(0.0, 1.0),
            value: raw.value.clamp(0.0, 1.0),
        })
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
        use ParseTypedColorError::*;
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
        let mut hue: Option<u32> = None;
        let mut saturation: Option<f32> = None;
        let mut value: Option<f32> = None;

        for pair in s.split(',').filter(|p| !p.trim().is_empty()) {
            let (k, v) = pair
                .split_once('=')
                .ok_or_else(|| ParseTypedColorError::BadPair(pair.to_string()))?;

            match k.trim() {
                "hue" => {
                    let raw: u32 = v.trim().parse().map_err(|e: std::num::ParseIntError| {
                        ParseTypedColorError::BadNumber("hue", e.to_string())
                    })?;
                    hue = Some(raw.clamp(0, 360));
                }
                "saturation" => {
                    let raw: f32 = v.trim().parse().map_err(|e: std::num::ParseFloatError| {
                        ParseTypedColorError::BadNumber("saturation", e.to_string())
                    })?;
                    saturation = Some(raw.clamp(0.0, 1.0));
                }
                "value" => {
                    let raw: f32 = v.trim().parse().map_err(|e: std::num::ParseFloatError| {
                        ParseTypedColorError::BadNumber("value", e.to_string())
                    })?;
                    value = Some(raw.clamp(0.0, 1.0));
                }
                other => return Err(ParseTypedColorError::UnknownField(other.to_string())),
            }
        }

        Ok(TypedLightColor {
            hue: hue.ok_or(ParseTypedColorError::MissingField("hue"))?,
            saturation: saturation.ok_or(ParseTypedColorError::MissingField("saturation"))?,
            value: value.ok_or(ParseTypedColorError::MissingField("value"))?,
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

#[derive(Debug)]
pub enum ParseAmbientError {
    BadPair(String),
    UnknownField(String),
    BadColor(String, Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for ParseAmbientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ParseAmbientError::*;
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
    FLICKERSLOW,
    FLICKER,
    PULSE,
    PULSESLOW,
    #[default]
    NONE,
}

use tes3::esp::LightFlags;
impl LightFlag {
    pub fn to_esp_flag(&self) -> LightFlags {
        match &self {
            Self::FLICKER => LightFlags::FLICKER,
            Self::FLICKERSLOW => LightFlags::FLICKER_SLOW,
            Self::PULSE => LightFlags::PULSE,
            Self::PULSESLOW => LightFlags::PULSE_SLOW,
            Self::NONE => LightFlags::empty(),
        }
    }
}

impl FromStr for LightFlag {
    type Err = ParseLightError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "flicker" => Ok(LightFlag::FLICKER),
            "flickerslow" => Ok(LightFlag::FLICKERSLOW),
            "pulse" => Ok(LightFlag::PULSE),
            "pulseslow" => Ok(LightFlag::PULSESLOW),
            "none" => Ok(LightFlag::NONE),
            _ => Err(ParseLightError::UnknownVariant(s.to_string())),
        }
    }
}
