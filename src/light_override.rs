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
        use ParseLightError::{
            BadNumber, BadPair, ExclusiveFields, MissingPrefix, UnknownField, UnknownVariant,
        };
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
        parse_pairs(s, |key, value| data.set_pair(key, value))?;

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
        Ok(())
    }

    fn set_pair(&mut self, key: &str, value: &str) -> Result<(), ParseLightError> {
        match key {
            "radius_mult" => Self::set_float_mult(
                &mut self.radius_mult,
                self.radius.is_some(),
                "radius",
                "radius_mult",
                value,
            ),
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

    fn set_hue(&mut self, value: &str) -> Result<(), ParseLightError> {
        if self.hue_mult.is_some() {
            return Err(ParseLightError::ExclusiveFields("hue_mult", "hue"));
        }
        let parsed: u32 = value.parse().map_err(|e: std::num::ParseIntError| {
            ParseLightError::BadNumber("hue", e.to_string())
        })?;
        self.hue = Some(parsed.clamp(0, 360));
        Ok(())
    }

    fn set_saturation(&mut self, value: &str) -> Result<(), ParseLightError> {
        if self.saturation_mult.is_some() {
            return Err(ParseLightError::ExclusiveFields(
                "saturation_mult",
                "saturation",
            ));
        }
        let parsed: f32 = value.parse().map_err(|e: std::num::ParseFloatError| {
            ParseLightError::BadNumber("saturation", e.to_string())
        })?;
        self.saturation = Some(parsed.clamp(0.0, 1.0));
        Ok(())
    }

    fn set_value(&mut self, value: &str) -> Result<(), ParseLightError> {
        if self.value_mult.is_some() {
            return Err(ParseLightError::ExclusiveFields("value_mult", "value"));
        }
        let parsed: f32 = value.parse().map_err(|e: std::num::ParseFloatError| {
            ParseLightError::BadNumber("value", e.to_string())
        })?;
        self.value = Some(parsed.clamp(0.0, 1.0));
        Ok(())
    }
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

    fn assert_f32_eq(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn parses_cli_light_override_fixed_fields() {
        let (id, data) = parse_light_override(
            "Torch_001=radius=255,duration=1200,hue=240,saturation=0.5,value=0.25,flag=FLICKERSLOW",
        )
        .unwrap();

        assert_eq!(id, "Torch_001");
        assert_eq!(data.radius, Some(255));
        assert_eq!(data.duration, Some(1200.0));
        assert_eq!(data.hue, Some(240));
        assert_eq!(data.saturation, Some(0.5));
        assert_eq!(data.value, Some(0.25));
        assert!(matches!(data.flag, Some(LightFlag::FlickerSlow)));
    }

    #[test]
    fn parses_cli_light_override_multiplier_fields() {
        let (_, data) = parse_light_override(
            "Torch_002=radius_mult=2.0,duration_mult=3.0,hue_mult=4.0,saturation_mult=0.5,value_mult=0.25",
        )
        .unwrap();

        assert_eq!(data.radius_mult, Some(2.0));
        assert_eq!(data.duration_mult, Some(3.0));
        assert_eq!(data.hue_mult, Some(4.0));
        assert_eq!(data.saturation_mult, Some(0.5));
        assert_eq!(data.value_mult, Some(0.25));
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
    fn cli_light_override_clamps_fixed_color_fields() {
        let (_, data) = parse_light_override("Torch=hue=999,saturation=2.0,value=-1.0").unwrap();

        assert_eq!(data.hue, Some(360));
        assert_eq!(data.saturation, Some(1.0));
        assert_eq!(data.value, Some(0.0));
    }

    #[test]
    fn parses_cli_ambient_override_all_fields() {
        let (id, ambient) = parse_ambient_override(
            "caius=ambient=hue=30,saturation=0.5,value=0.6;sunlight=hue=40,saturation=0.7,value=0.8;fog=hue=50,saturation=0.9,value=1.0;fog_density=0.25",
        )
        .unwrap();

        assert_eq!(id, "caius");
        assert_eq!(ambient.ambient.as_ref().unwrap().hue, 30);
        assert_f32_eq(ambient.sunlight.as_ref().unwrap().saturation, 0.7);
        assert_f32_eq(ambient.fog.as_ref().unwrap().value, 1.0);
        assert_eq!(ambient.fog_density, Some(0.25));
    }

    #[test]
    fn cli_ambient_override_reports_bad_nested_color() {
        let err = parse_ambient_override("caius=ambient=hue=30,saturation=0.5").unwrap_err();

        assert!(matches!(err, ParseAmbientError::BadColor(field, _) if field == "ambient"));
    }

    #[test]
    fn cli_ambient_override_rejects_unknown_fields() {
        let err = parse_ambient_override("caius=glow=hue=30,saturation=0.5,value=0.6").unwrap_err();

        assert!(matches!(err, ParseAmbientError::UnknownField(field) if field == "glow"));
    }

    #[test]
    fn toml_light_data_rejects_fixed_and_multiplier_for_same_field() {
        let err = toml::from_str::<CustomLightData>("radius = 10\nradius_mult = 2.0").unwrap_err();

        assert!(err.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn toml_typed_light_color_clamps_fields() {
        let color =
            toml::from_str::<TypedLightColor>("hue = 999\nsaturation = 2.0\nvalue = -1.0").unwrap();

        assert_eq!(color.hue, 360);
        assert_f32_eq(color.saturation, 1.0);
        assert_f32_eq(color.value, 0.0);
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
