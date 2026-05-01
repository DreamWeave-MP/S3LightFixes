use palette::{FromColor, GetHue, Hsv, IntoColor, SetHue, rgb::Srgb};
use tes3::esp::{EditorId, LightFlags};

use crate::{CustomLightData, LightConfig};

// TES3 stores these values as integers, while lightfixes intentionally exposes multipliers as
// floats. The casts are the conversion boundary between the file format and user-authored math.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn scaled_u32(value: u32, multiplier: f32) -> u32 {
    (value as f32 * multiplier).max(0.0) as u32
}

// Same boundary as `scaled_u32`, but TES3 light duration is signed.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn scaled_i32(value: i32, multiplier: f32) -> i32 {
    (value as f32 * multiplier) as i32
}

// User-provided fixed durations are floats to share parser machinery with multipliers, but TES3
// stores the result as an integer duration.
#[allow(clippy::cast_possible_truncation)]
fn fixed_duration_to_i32(duration: f32) -> i32 {
    duration as i32
}

/// Given a `LightData` reference from an ESP light,
/// returns the HSV version and whether it is colored or not (for the global modifier).
#[must_use]
pub fn light_to_hsv(light_data: &tes3::esp::LightData) -> (Hsv, bool) {
    let rgb: palette::rgb::Rgb = Srgb::new(
        light_data.color[0],
        light_data.color[1],
        light_data.color[2],
    )
    .into_format();

    let hsv: Hsv = Hsv::from_color(rgb);
    let hue_degrees = hsv.get_hue().into_positive_degrees();

    (hsv, !(14. ..=64.).contains(&hue_degrees))
}

fn replacement_for_light<'a>(
    light_config: &'a LightConfig,
    light_id: &str,
) -> Option<&'a CustomLightData> {
    light_config
        .light_regexes
        .iter()
        .find_map(|(regex, light_data)| regex.is_match(light_id).then_some(light_data))
}

fn apply_hsv_replacement(
    light_as_hsv: &mut Hsv,
    replacement: &CustomLightData,
    global_hue: f32,
    global_saturation: f32,
    global_value: f32,
) {
    if replacement.color.is_some() {
        return;
    }

    if let Some(hue_mult) = replacement.hue_mult {
        let new_hue = palette::RgbHue::from_degrees(light_as_hsv.hue.into_raw_degrees() * hue_mult);
        light_as_hsv.set_hue(new_hue);
    } else if let Some(fixed_hue) = replacement.hue {
        let new_hue = palette::RgbHue::from_degrees(hue_degrees(fixed_hue));
        light_as_hsv.set_hue(new_hue);
    } else {
        let new_hue =
            palette::RgbHue::from_degrees(light_as_hsv.hue.into_raw_degrees() * global_hue);
        light_as_hsv.set_hue(new_hue);
    }

    if let Some(saturation_mult) = replacement.saturation_mult {
        light_as_hsv.saturation *= saturation_mult;
    } else if let Some(fixed_saturation) = replacement.saturation {
        light_as_hsv.saturation = fixed_saturation;
    } else {
        light_as_hsv.saturation *= global_saturation;
    }

    if let Some(value_mult) = replacement.value_mult {
        light_as_hsv.value *= value_mult;
    } else if let Some(fixed_value) = replacement.value {
        light_as_hsv.value = fixed_value;
    } else {
        light_as_hsv.value *= global_value;
    }
}

#[allow(clippy::cast_precision_loss)]
fn hue_degrees(hue: u32) -> f32 {
    hue.clamp(0, 360) as f32
}

fn apply_plain_hsv_adjustment(
    light_as_hsv: &mut Hsv,
    global_hue: f32,
    global_saturation: f32,
    global_value: f32,
) {
    let new_hue = palette::RgbHue::from_degrees(light_as_hsv.hue.into_raw_degrees() * global_hue);

    light_as_hsv.set_hue(new_hue);
    light_as_hsv.saturation *= global_saturation;
    light_as_hsv.value *= global_value;
}

pub fn process_light(light_config: &LightConfig, light: &mut tes3::esp::Light) -> Vec<String> {
    let original_data = light.data.clone();

    if light.data.flags.contains(LightFlags::NEGATIVE) {
        light.data.flags.remove(LightFlags::NEGATIVE);
        light.data.radius = 0;
        light.data.color = [0, 0, 0, 0];
        return light_changes(&original_data, &light.data);
    }

    if light_config.disable_flickering {
        light
            .data
            .flags
            .remove(LightFlags::FLICKER | LightFlags::FLICKER_SLOW);
    }

    if light_config.disable_pulse {
        light
            .data
            .flags
            .remove(LightFlags::PULSE | LightFlags::PULSE_SLOW);
    }

    let light_id = light.editor_id_ascii_lowercase();
    let (mut light_as_hsv, is_colored) = light_to_hsv(&light.data);
    let replacement_light_data = replacement_for_light(light_config, &light_id);

    let (global_radius, global_hue, global_saturation, global_value) = if is_colored {
        (
            light_config.colored_radius,
            light_config.colored_hue,
            light_config.colored_saturation,
            light_config.colored_value,
        )
    } else {
        (
            light_config.standard_radius,
            light_config.standard_hue,
            light_config.standard_saturation,
            light_config.standard_value,
        )
    };

    if let Some(replacement) = replacement_light_data {
        apply_hsv_replacement(
            &mut light_as_hsv,
            replacement,
            global_hue,
            global_saturation,
            global_value,
        );

        if let Some(duration_mult) = replacement.duration_mult {
            light.data.time = scaled_i32(light.data.time, duration_mult);
        } else if let Some(fixed_duration) = replacement.duration {
            light.data.time = fixed_duration_to_i32(fixed_duration);
        } else {
            light.data.time = scaled_i32(light.data.time, light_config.duration_mult);
        }

        if let Some(radius_mult) = replacement.radius_mult {
            light.data.radius = scaled_u32(light.data.radius, radius_mult);
        } else if let Some(fixed_radius) = replacement.radius {
            light.data.radius = fixed_radius;
        } else {
            light.data.radius = scaled_u32(light.data.radius, global_radius);
        }

        if let Some(flag) = &replacement.flag {
            light.data.flags = flag.to_esp_flag();
        }
    } else {
        apply_plain_hsv_adjustment(
            &mut light_as_hsv,
            global_hue,
            global_saturation,
            global_value,
        );

        light.data.radius = scaled_u32(light.data.radius, global_radius);
        light.data.time = scaled_i32(light.data.time, light_config.duration_mult);
    }

    if let Some(replacement) = replacement_light_data.and_then(|replacement| replacement.color) {
        light.data.color = replacement;
    } else {
        let rgb8_color: Srgb<u8> = <Hsv as IntoColor<Srgb>>::into_color(light_as_hsv).into_format();
        light.data.color = [rgb8_color.red, rgb8_color.green, rgb8_color.blue, 0];
    }

    light_changes(&original_data, &light.data)
}

fn light_changes(original: &tes3::esp::LightData, modified: &tes3::esp::LightData) -> Vec<String> {
    let mut changes = Vec::new();

    if original.color != modified.color {
        changes.push(format!(
            "color {:?} -> {:?}",
            original.color, modified.color
        ));
    }

    if original.radius != modified.radius {
        changes.push(format!("radius {} -> {}", original.radius, modified.radius));
    }

    if original.time != modified.time {
        changes.push(format!("duration {} -> {}", original.time, modified.time));
    }

    if original.flags != modified.flags {
        changes.push(format!(
            "flags {:?} -> {:?}",
            original.flags, modified.flags
        ));
    }

    changes
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use tes3::esp::{Light, LightData, LightFlags, ObjectFlags};

    use super::*;
    use crate::light_override::LightFlag;

    fn rgb_from_hsv(hue: f32, saturation: f32, value: f32) -> [u8; 4] {
        let hsv = Hsv::from_components((palette::RgbHue::from_degrees(hue), saturation, value));
        let rgb8_color: Srgb<u8> = <Hsv as IntoColor<Srgb>>::into_color(hsv).into_format();

        [rgb8_color.red, rgb8_color.green, rgb8_color.blue, 0]
    }

    fn light(id: &str, hue: f32, radius: u32, time: i32, flags: LightFlags) -> Light {
        Light {
            flags: ObjectFlags::default(),
            id: id.to_owned(),
            data: LightData {
                radius,
                time,
                color: rgb_from_hsv(hue, 1.0, 1.0),
                flags,
                ..LightData::default()
            },
            ..Light::default()
        }
    }

    fn config() -> LightConfig {
        LightConfig {
            disable_flickering: false,
            disable_pulse: false,
            standard_hue: 1.0,
            standard_saturation: 1.0,
            standard_value: 1.0,
            standard_radius: 1.0,
            colored_hue: 1.0,
            colored_saturation: 1.0,
            colored_value: 1.0,
            colored_radius: 1.0,
            duration_mult: 1.0,
            ..LightConfig::default()
        }
    }

    #[test]
    fn negative_lights_are_zeroed_and_return_early() {
        let mut light_config = config();
        light_config.disable_flickering = true;
        light_config.disable_pulse = true;
        light_config.standard_radius = 100.0;
        light_config.duration_mult = 100.0;
        light_config.light_regexes.push((
            Regex::new("negative").unwrap(),
            CustomLightData {
                radius: Some(777),
                duration: Some(888.0),
                ..CustomLightData::default()
            },
        ));

        let mut light = light(
            "negative_light",
            30.0,
            42,
            13,
            LightFlags::NEGATIVE | LightFlags::FLICKER | LightFlags::PULSE,
        );

        process_light(&light_config, &mut light);

        assert!(!light.data.flags.contains(LightFlags::NEGATIVE));
        assert!(light.data.flags.contains(LightFlags::FLICKER));
        assert!(light.data.flags.contains(LightFlags::PULSE));
        assert_eq!(light.data.radius, 0);
        assert_eq!(light.data.time, 13);
        assert_eq!(light.data.color, [0, 0, 0, 0]);
    }

    #[test]
    fn disabling_flicker_and_pulse_removes_only_those_flags() {
        let mut light_config = config();
        light_config.disable_flickering = true;
        light_config.disable_pulse = true;

        let mut light = light(
            "animated_light",
            30.0,
            100,
            10,
            LightFlags::FLICKER
                | LightFlags::FLICKER_SLOW
                | LightFlags::PULSE
                | LightFlags::PULSE_SLOW
                | LightFlags::FIRE,
        );

        process_light(&light_config, &mut light);

        assert!(!light.data.flags.contains(LightFlags::FLICKER));
        assert!(!light.data.flags.contains(LightFlags::FLICKER_SLOW));
        assert!(!light.data.flags.contains(LightFlags::PULSE));
        assert!(!light.data.flags.contains(LightFlags::PULSE_SLOW));
        assert!(light.data.flags.contains(LightFlags::FIRE));
    }

    #[test]
    fn light_to_hsv_classifies_orange_boundaries_as_standard() {
        for (hue, expected_colored) in [(13.0, true), (14.0, false), (64.0, false), (65.0, true)] {
            let light = light("classified", hue, 1, 1, LightFlags::default());
            let (_, is_colored) = light_to_hsv(&light.data);

            assert_eq!(is_colored, expected_colored, "hue {hue}");
        }
    }

    #[test]
    fn standard_and_colored_lights_use_their_own_global_multipliers() {
        let mut light_config = config();
        light_config.standard_radius = 2.0;
        light_config.colored_radius = 3.0;
        light_config.duration_mult = 4.0;

        let mut standard = light("standard", 30.0, 10, 5, LightFlags::default());
        let mut colored = light("colored", 180.0, 10, 5, LightFlags::default());

        process_light(&light_config, &mut standard);
        process_light(&light_config, &mut colored);

        assert_eq!(standard.data.radius, 20);
        assert_eq!(colored.data.radius, 30);
        assert_eq!(standard.data.time, 20);
        assert_eq!(colored.data.time, 20);
    }

    #[test]
    fn matching_light_overrides_beat_globals_and_fall_back_per_field() {
        let mut light_config = config();
        light_config.standard_radius = 2.0;
        light_config.duration_mult = 3.0;
        light_config.light_regexes.push((
            Regex::new("fixed").unwrap(),
            CustomLightData {
                radius: Some(123),
                duration: Some(456.0),
                color: Some([0, 128, 64, 0]),
                ..CustomLightData::default()
            },
        ));
        light_config.light_regexes.push((
            Regex::new("partial").unwrap(),
            CustomLightData {
                radius: Some(321),
                ..CustomLightData::default()
            },
        ));
        light_config.light_regexes.push((
            Regex::new("mult").unwrap(),
            CustomLightData {
                radius_mult: Some(5.0),
                duration_mult: Some(7.0),
                ..CustomLightData::default()
            },
        ));

        let mut fixed = light("fixed_light", 30.0, 10, 10, LightFlags::default());
        let mut partial = light("partial_light", 30.0, 10, 10, LightFlags::default());
        let mut mult = light("mult_light", 30.0, 10, 10, LightFlags::default());

        process_light(&light_config, &mut fixed);
        process_light(&light_config, &mut partial);
        process_light(&light_config, &mut mult);

        assert_eq!(fixed.data.radius, 123);
        assert_eq!(fixed.data.time, 456);
        assert_eq!(fixed.data.color, [0, 128, 64, 0]);

        assert_eq!(partial.data.radius, 321);
        assert_eq!(partial.data.time, 30);

        assert_eq!(mult.data.radius, 50);
        assert_eq!(mult.data.time, 70);
    }

    #[test]
    fn partial_legacy_hsv_overrides_are_still_applied_at_runtime() {
        let mut light_config = config();
        light_config.standard_hue = 1.0;
        light_config.standard_saturation = 1.0;
        light_config.standard_value = 1.0;
        light_config.light_regexes.push((
            Regex::new("legacy_partial").unwrap(),
            CustomLightData {
                hue: Some(180),
                saturation: Some(0.5),
                ..CustomLightData::default()
            },
        ));
        let mut light = light("legacy_partial", 30.0, 10, 10, LightFlags::default());

        process_light(&light_config, &mut light);

        assert_eq!(light.data.color, rgb_from_hsv(180.0, 0.5, 1.0));
    }

    #[test]
    fn first_matching_light_override_wins_and_flag_replacement_is_exact() {
        let mut light_config = config();
        light_config.standard_radius = 10.0;
        light_config.light_regexes.push((
            Regex::new("torch").unwrap(),
            CustomLightData {
                radius: Some(111),
                flag: Some(LightFlag::PulseSlow),
                ..CustomLightData::default()
            },
        ));
        light_config.light_regexes.push((
            Regex::new("torch_special").unwrap(),
            CustomLightData {
                radius: Some(222),
                flag: Some(LightFlag::Flicker),
                ..CustomLightData::default()
            },
        ));
        let mut light = light(
            "torch_special",
            30.0,
            10,
            10,
            LightFlags::FIRE | LightFlags::FLICKER,
        );

        process_light(&light_config, &mut light);

        assert_eq!(light.data.radius, 111);
        assert_eq!(light.data.flags, LightFlags::PULSE_SLOW);
    }

    #[test]
    fn hsv_multiplier_overrides_apply_to_matching_lights() {
        let mut light_config = config();
        light_config.light_regexes.push((
            Regex::new("hsv_mult").unwrap(),
            CustomLightData {
                hue_mult: Some(2.0),
                saturation_mult: Some(0.5),
                value_mult: Some(0.25),
                ..CustomLightData::default()
            },
        ));
        let mut light = light("hsv_mult_light", 30.0, 10, 10, LightFlags::default());

        process_light(&light_config, &mut light);

        assert_eq!(light.data.color, rgb_from_hsv(60.0, 0.5, 0.25));
    }

    #[test]
    fn negative_radius_multipliers_clamp_to_zero_instead_of_wrapping() {
        let mut light_config = config();
        light_config.standard_radius = -2.0;
        light_config.light_regexes.push((
            Regex::new("override").unwrap(),
            CustomLightData {
                radius_mult: Some(-3.0),
                ..CustomLightData::default()
            },
        ));
        let mut global = light("global", 30.0, 10, 10, LightFlags::default());
        let mut overridden = light("override", 30.0, 10, 10, LightFlags::default());

        process_light(&light_config, &mut global);
        process_light(&light_config, &mut overridden);

        assert_eq!(global.data.radius, 0);
        assert_eq!(overridden.data.radius, 0);
    }
}
