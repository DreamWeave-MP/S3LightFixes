// mod default {
pub fn standard_hue() -> f32 {
    0.62
}

pub fn standard_saturation() -> f32 {
    0.8
}

pub fn standard_value() -> f32 {
    0.57
}

/// Original default radius was 2.0
/// But was only appropriate for vtastek shaders
/// MOMW configs use 1.2
pub fn standard_radius() -> f32 {
    1.2
}

pub fn colored_hue() -> f32 {
    1.0
}

pub fn colored_saturation() -> f32 {
    0.9
}

pub fn colored_value() -> f32 {
    0.7
}

pub fn colored_radius() -> f32 {
    1.1
}

pub fn duration_mult() -> f32 {
    2.5
}

pub fn disable_flicker() -> bool {
    true
}

pub fn disable_pulse() -> bool {
    false
}

pub fn save_log() -> bool {
    false
}

pub fn auto_enable() -> bool {
    false
}

pub fn excluded_plugins() -> Vec<String> {
    vec![
        // Unable to resolve moved reference (1, 7028) for cell Sadrith Mora (18, 4)
        "deleted_groundcover.omwaddon".into(),
        // Unexpected Tag: CELL::FLTV
        "Clean_Argonian Full Helms Lore Integrated.ESP".into(),
        // LUAL
        "Baldurwind.omwaddon".into(),
        "Crassified Navigation.omwaddon".into(),
        "LuaMultiMark.omwaddon".into(),
        "S3maphore.esp".into(),
        "Toolgun.omwaddon".into(),
    ]
}
// }
