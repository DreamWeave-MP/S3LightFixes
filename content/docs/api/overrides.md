+++
title = "Overrides"
description = "CustomLightData and CustomCellAmbient, the values of one light or cell override; the color and flag types inside them; and the errors their parsers return."
weight = 30

[extra]
kind = "api"
+++

An override is a pattern and one of these. [Overrides and exclusions](@/docs/overrides.md) explains
what each value does to a light or a cell; this page is the types.

## CustomLightData

{{ api_signature(value="pub struct CustomLightData { pub color: Option<[u8; 4]>, pub hue: Option<u32>, pub radius: Option<u32>, pub flag: Option<LightFlag>, … }") }}

One light override. Every field is optional; `None` leaves that part of the light to the global
settings. `Clone`, `Debug`, `Default`.

| Field | Type | Key |
|---|---|---|
| `color` | `Option<[u8; 4]>` | `red`, `green`, `blue`, as `[red, green, blue, 0]` |
| `red_mult`, `green_mult`, `blue_mult` | `Option<f32>` | the same |
| `hue` | `Option<u32>` | `hue`, 0 to 360 |
| `saturation`, `value` | `Option<f32>` | the same, 0.0 to 1.0 |
| `hue_mult`, `saturation_mult`, `value_mult` | `Option<f32>` | the same |
| `radius` | `Option<u32>` | `radius` |
| `radius_mult` | `Option<f32>` | the same |
| `duration` | `Option<f32>` | `duration` |
| `duration_mult` | `Option<f32>` | the same |
| `flag` | `Option<LightFlag>` | `flag` |

It is read two ways, and both refuse a key together with its `_mult` form, an incomplete
`red`/`green`/`blue`, and a multiplier that is not finite, and both clamp `hue`, `saturation` and
`value` into range:

- `Deserialize`, from a `lightconfig.toml` table. Unknown keys are ignored here, and
  [`LightConfig::get`](@/docs/api/settings.md#lightconfig-get) reports them.
- `FromStr`, from the command line's `key=value,key=value` form, with
  [`ParseLightError`](#parselighterror). Unknown keys are errors.

`Serialize` writes the keys back, `color` as `red`, `green` and `blue`.

```rust
use s3lightfixes::CustomLightData;

fn main() {
    let data: CustomLightData = "radius=400,red=255,green=160,blue=80,flag=FIRE|CAN_CARRY".parse().unwrap();
    assert_eq!(data.color, Some([255, 160, 80, 0]));
    assert_eq!(data.radius, Some(400));

    let error = "radius=5,radius_mult=2".parse::<CustomLightData>().unwrap_err();
    assert_eq!(error.to_string(), "Key radius is mutually exclusive with radius_mult");

    let from_toml: CustomLightData = toml::from_str("hue = 400\nvalue_mult = 0.75").unwrap();
    assert_eq!(from_toml.hue, Some(360));
}
```

## CustomCellAmbient

{{ api_signature(value="pub struct CustomCellAmbient { pub ambient: Option<TypedLightColor>, pub sunlight: Option<TypedLightColor>, pub fog: Option<TypedLightColor>, pub fog_density: Option<f32> }") }}

One interior cell override: the colors and fog density to replace. `Clone`, `Debug`, `Default`,
`Serialize`, `Deserialize`. `FromStr` reads the command line's form, fields separated by `;` and
colors as `red=…,green=…,blue=…`, with [`ParseAmbientError`](#parseambienterror).

```rust
use s3lightfixes::CustomCellAmbient;

fn main() {
    let cell: CustomCellAmbient = "ambient=red=64,green=48,blue=32;fog_density=0.5".parse().unwrap();
    assert_eq!(cell.ambient.unwrap().to_esp_color(), [64, 48, 32, 0]);
    assert_eq!(cell.fog_density, Some(0.5));
    assert!(cell.sunlight.is_none());

    let legacy: CustomCellAmbient = toml::from_str("[fog]\nhue = 0\nsaturation = 0.0\nvalue = 1.0").unwrap();
    assert_eq!(legacy.fog.unwrap().to_esp_color(), [255, 255, 255, 0]);
}
```

## TypedLightColor

{{ api_signature(value="pub struct TypedLightColor { pub red: u8, pub green: u8, pub blue: u8 }") }}

A color in a cell override. `Clone`, `Debug`, `Default`, `Serialize`. From TOML it takes `red`,
`green` and `blue`, or `hue`, `saturation` and `value`, converted; never a mix, and never a part of
either. From a string it takes `red`, `green` and `blue` only, with `ParseTypedColorError`.

{{ api_signature(value="const fn TypedLightColor::to_esp_color(&self) -> [u8; 4]") }}

The color as a TES3 record stores it, `[red, green, blue, 0]`.

## LightFlag

{{ api_signature(value="pub struct LightFlag { /* private */ }") }}

The flags a light override sets, all of them at once. `Clone`, `Copy`, `Debug`, `Default` (no
flags), `PartialEq`, `Eq`. It reads a name, several names joined by `|`, or, from TOML, a list of
names: `NONE`, `DYNAMIC`, `CAN_CARRY`, `NEGATIVE`, `FLICKER`, `FIRE`, `OFF_BY_DEFAULT`,
`FLICKER_SLOW`, `PULSE` and `PULSE_SLOW`, in any case. `NONE` with any other name, an empty name,
and an unknown one are errors. It serializes as `NONE`, one name, or a list.

{{ api_signature(value="const fn LightFlag::from_esp_flags(flags: LightFlags) -> LightFlag") }}

{{ api_signature(value="fn LightFlag::to_esp_flag(self) -> LightFlags") }}

To and from `tes3::esp::LightFlags`.

## Errors

The parsers' errors are `Debug`, `Display` and `std::error::Error`. Their messages are what the
command line prints after `invalid value '…' for '--light <LIGHT_OVERRIDES>':`.

### ParseLightError

{{ api_signature(value="enum ParseLightError { ExclusiveFields(&'static str, &'static str), ConflictingLightFlags(&'static str, &'static str), IncompleteRgb, BadPair(String), UnknownField(String), BadNumber(&'static str, String), MissingPrefix, UnknownVariant(String) }") }}

| Variant | Message |
|---|---|
| `ExclusiveFields(a, b)` | ``Key a is mutually exclusive with b`` |
| `ConflictingLightFlags(a, b)` | ``Light flag `a` cannot be combined with `b` `` |
| `IncompleteRgb` | `RGB overrides must specify red, green, and blue` |
| `BadPair(text)` | ``Expected key=value pair, got: `text` `` |
| `UnknownField(key)` | ``Unknown field: `key` `` |
| `BadNumber(key, reason)` | ``Invalid number for `key`: reason`` |
| `MissingPrefix` | Never returned |
| `UnknownVariant(name)` | ``Unknown light flag: `name` `` |

### ParseAmbientError

{{ api_signature(value="enum ParseAmbientError { BadPair(String), UnknownField(String), BadColor(String, Box<dyn std::error::Error + Send + Sync>) }") }}

`BadColor` names the field, `ambient`, `sunlight`, `fog` or `fog_density`, and wraps the error from
reading its value.

### ParseTypedColorError

{{ api_signature(value="enum ParseTypedColorError { MissingField(&'static str), UnknownField(String), BadNumber(&'static str, String), BadPair(String) }") }}

The error inside `BadColor` for a color.

{% callout(kind="note", title="Types you cannot name") %}
`TypedLightColor`, `LightFlag` and the three error types live in a private module, so they appear
in fields and trait implementations but cannot be imported. Reach them through the values that
hold them, as the examples do, or through `<CustomLightData as FromStr>::Err`.
{% end %}
