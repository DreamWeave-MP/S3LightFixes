+++
title = "Light arithmetic"
description = "process_light, which applies a LightConfig to one light record and reports what changed, and light_to_hsv, which decides whether a light is standard or colored."
weight = 40

[extra]
kind = "api"
+++

## process_light

{{ api_signature(value="fn process_light(light_config: &LightConfig, light: &mut tes3::esp::Light) -> Vec<String>") }}

Applies the settings to one light, in place, exactly as a run does: negative lights, flicker and
pulse, then color, radius and duration, with the first matching light override taken from
`light_config.light_regexes`. [What happens to a light](@/docs/what-it-changes.md#what-happens-to-a-light)
lists the steps. It does not check `excluded_ids`; a run skips excluded lights before calling it.

It returns one entry per value that changed, in the log's words: `color [245, 140, 40, 0] -> [140,
74, 46, 0]`, `radius 256 -> 307`, `duration 210 -> 525`, `flags LightFlags(…) -> LightFlags(…)`. An
empty list means the light came out as it went in.

The override is matched against the id in lower case, so patterns must match lower case: the
program's own are compiled without regard to case, and yours should be too.

```rust
use s3lightfixes::{CustomLightData, LightConfig, process_light};
use tes3::esp::{Light, LightData};

fn main() {
    let mut config = LightConfig::default();
    let blue: CustomLightData = "red=64,green=128,blue=255,duration=600".parse().unwrap();
    let pattern = regex::RegexBuilder::new("^Light_Infinite").case_insensitive(true).build().unwrap();
    config.light_regexes.push((pattern, blue));

    let mut lamp = Light {
        id: "Light_Infinite_Lamp".to_owned(),
        data: LightData { radius: 200, time: -1, color: [255, 200, 120, 0], ..LightData::default() },
        ..Light::default()
    };
    let changes = process_light(&config, &mut lamp);

    assert_eq!(lamp.data.color, [64, 128, 255, 0]);
    assert_eq!(lamp.data.radius, 240);
    assert_eq!(lamp.data.time, 600);
    assert_eq!(changes, ["color [255, 200, 120, 0] -> [64, 128, 255, 0]", "radius 200 -> 240", "duration -1 -> 600"]);
}
```

The lamp is blue now, but its radius still gets `standard_radius`, 1.2: whether a light is standard
or colored is decided from its own color, before the override replaces it. A fixed `duration` is
the one thing that changes an infinite light's -1.

## light_to_hsv

{{ api_signature(value="fn light_to_hsv(light_data: &tes3::esp::LightData) -> (palette::Hsv, bool)") }}

The light's color as hue, saturation and value, from the [palette](https://docs.rs/palette/0.7)
crate, and whether it is a *colored* light: `true` unless its hue is from 14° to 64°, inclusive.
White and grey have hue 0°, so they are colored.

```rust
use palette::GetHue;
use s3lightfixes::light_to_hsv;
use tes3::esp::LightData;

fn main() {
    let torch = LightData { color: [245, 140, 40, 0], ..LightData::default() };
    let (hsv, colored) = light_to_hsv(&torch);
    assert!(!colored);
    assert!((hsv.get_hue().into_positive_degrees() - 29.27).abs() < 0.01);

    let white = LightData { color: [255, 255, 255, 0], ..LightData::default() };
    assert!(light_to_hsv(&white).1);
}
```
