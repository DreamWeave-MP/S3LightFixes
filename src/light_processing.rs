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

// Hue is clamped to 0..=360 before this point, so the precision-loss lint is technically correct
// and practically useless. There are 361 possible values. IEEE-754 will survive this one.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn hue_degrees(hue: u32) -> f32 {
    hue as f32
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
    if let Some(hue_mult) = replacement.hue_mult {
        let new_hue = palette::RgbHue::from_degrees(light_as_hsv.hue.into_raw_degrees() * hue_mult);
        light_as_hsv.set_hue(new_hue);
    } else if let Some(fixed_hue) = replacement.hue {
        light_as_hsv.set_hue(palette::RgbHue::from_degrees(hue_degrees(fixed_hue)));
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

pub fn process_light(light_config: &LightConfig, light: &mut tes3::esp::Light) {
    if light.data.flags.contains(LightFlags::NEGATIVE) {
        light.data.flags.remove(LightFlags::NEGATIVE);
        light.data.radius = 0;
        light.data.color = [0, 0, 0, 0];
        return;
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

    let rgb8_color: Srgb<u8> = <Hsv as IntoColor<Srgb>>::into_color(light_as_hsv).into_format();
    light.data.color = [rgb8_color.red, rgb8_color.green, rgb8_color.blue, 0];
}
