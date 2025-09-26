# S3LightFixes

S3LightFixes is a descendant of [Waza-lightfixes](https://modding-openmw.com/mods/waza_lightfixes/), which itself is a descendant of [Lightfixes.pl](https://modding-openmw.com/tips/custom-shaders/#lightfixes-plugin) by vtastek. All three applications are designed to make ESP files which adjust the lighting values from *all* mods listed in one's openmw.cfg.

In other words, make light gud. What sets this version apart is that it's a standalone binary application, instead of piggybacking off tes3cmd. The changes it makes are the same as the previous two, but with additional conveniences like automatic installation, support for portable installs of OpenMW, itself having greater portability, and *ultimate* customization of lighting values in your configuration as quickly and easily as I could come up with. Anything you can imagine doing with lighting is doable with lightfixes, and support for PBR shaders can be disabled or enabled at will.

<div align="center">
<h1>DOWNLOAD</h1>

[Windows](https://github.com/magicaldave/S3LightFixes/releases/latest/download/windows-latest.zip) | [Mac](https://github.com/magicaldave/S3LightFixes/releases/latest/download/macos-latest.zip) | [Linux](https://github.com/magicaldave/S3LightFixes/releases/latest/download/ubuntu-latest.zip) | [Development Builds (All Platforms)](https://github.com/DreamWeave-MP/S3LightFixes/releases/tag/development)
</div>

# Security

Starting from version 0.1.9 and onward, S3LightFixes is now cryptographically signed using SigStore on all platforms. Release packages include the necessary `.bundle` files to ensure the cryptographic signatures for yourself.

Verifying the binary on a linux-based system:

```
cosign verify-blob ./s3lightfixes --certificate-identity-regexp="https://github.com/magicaldave/S3LightFixes/.github/workflows/" --certificate-oidc-issuer="https://token.actions.githubusercontent.com" --bundle ./S3LF-ubuntu-20.04.bundle
```

Replace `ubuntu` with `macos` or `windows` as needed for your target platform.

# Usage

Download the executable for your OS and run it however's most convenient. Double-click it or run it through the terminal.

A file, `S3LightFixes.omwaddon`, will be created. Add the folder it's in as a data directory in OpenMW's Launcher (or openmw.cfg, manually), and enable `S3LightFixes.omwaddon` in the `Content Files` tab of the launcher. If you don't know what that means, watch [this video.](https://www.youtube.com/watch?v=xzq_ksVuRgc&themeRefresh=1)

When running via the command line, numerous parameters are available to change how lightfixes changes the lights in your install. If you prefer to run LightFixes from a GUI, you can edit its `lightConfig.toml` instead. `lightConfig.toml` can be found [in the folders mentioned here, next to your openmw.cfg.](https://openmw.readthedocs.io/en/latest/reference/modding/paths.html)

# Toml Schema

You may optionally edit the lightconfig.toml S3Lightfixes creates (next to your user openmw.cfg) to adjust its settings for your next run.
Or, make your own lightconfig.toml and place it next to the S3LightFixes executable before running it. The toml schema is as follows:

```toml
# Disable pulsing lights
disable_pulse = true
# Disable flickering lights
disable_flickering = true
# Serialize S3LightFixes plugin to a text file. Don't do this unless you're asked to (or just curious)
save_log = false
# Hue multiplier for non-colored lights
standard_hue = 0.6000000238418579
# Saturation multiplier for non-colored lights
standard_saturation = 0.800000011920929
# Value multiplier for non-colored lights
standard_value = 0.5699999928474426
# Radius multiplier for non-colored lights
standard_radius = 2.0
# Hue multiplier for colored lights
colored_hue = 1.0
# Saturation multiplier for colored lights
colored_saturation = 0.8999999761581421
# Value multiplier for colored lights
colored_value = 0.699999988079071
# Radius multiplier for colored lights
colored_radius = 1.100000023841858
# Duration Multiplier for carryable lights
duration_mult = 2.5
# You may use regular expressions to exclude certain record ids or plugins from the set
# Note that these are only examples and by default no records or plugins are currently excluded.
excluded_ids = [
    # Contains purple
    "*purple*",
    # Ending with glow
    "glow^",
]

excluded_plugins = [
    # Exclude oaab plugins and master files
    "OAAB*", ".*esm"
]

# By default, this is the data-local directory of your openmw installation. If one is not found, then, the plugin will output to the location specified using the `-o` or `--output` argument. 
# If neither is specified, the plugin saves to the current working directory.
output_dir = "/home/s3kshun8/.config/openmw/sw0rdsinger/override/"

# Normally this field is always false, and must be set on the command line using `-u` or `--update`.
# However, if you're prone to trying many tweaks on the command line yourself, you can set it to true here once and never do it again.
save_config = false


# You may also set custom values for light configurations for each light *or* cell record in lightConfig.toml.
# This allows complete control and customization over all light colors, durations, radii, and even types(pulse, flicker, etc) in your lightConfig.toml
# A few examples are shown below.
# These customizations to lights may also be applied on the command line and saved to lightConfig.toml by using the `-u` argument, along with `--light` for each light record, or `--ambient` for each cell you wish to edit.
# See further below for command-line examples.

[light_overrides.light_com_candle_02_64]
hue = 3
saturation = -1.7599999904632568
value = -1.46299999952316284

[light_overrides.Torch_000]
hue = 239
radius = 254
duration = 1199.0

[light_overrides.Torch_001]
hue_mult = 0.2999999523162842
radius_mult = 1.0
flag = "PULSESLOW"

[ambient_overrides."caius cosades' house".ambient]
hue = 34
saturation = -1.2
value = -1.12
```

All parameters available in the lightConfig.toml may also be used as command line arguments. See below for further details on supported command line arguments.

## How Does It Work?

More specifically, the lightfixes plugin adjusts the color and radius of colored or whitish lights for your config separately. The radius in lightConfig.toml is used as a multiplier on top of the existing radius of the light, so they'll generally be brighter with the default configuration.

S3LightFixes also supports portable installations of OpenMW by way of utilizing the `-c` or `--openmw-cfg` argument.
Users running OpenMW with custom launchers such as `omw` should include the `-c` argument as well.

```sh
./s3lightfixes -c /dir/where/openmw.cfg/is/
```

To automatically enable S3LightFixes.omwaddon in whatever openmw.cfg you have asked it to use, use the `-e` argument:

```sh
./s3lightfixes -e -c /my/total-overhaul/dir/
```

Additionally, S3LightFixes will perform the following:

- Automatically install itself into your `data-local` directory of openmw (if using the `-e` or `--auto-enable` argument)
- Create a config file adjacent to your openmw.cfg if one doesn't already exist
- Disable sunlight color in interiors for compatibility with vtastek's custom shader stack for openmw 0.47
- Optionally remove the Flicker and FlickerSlow flags from all lights
- Nullify all negative lights (Not optional, as negative lights look bad in OpenMW)

## Command Line Arguments

```sh
  -c, --openmw-cfg <OPENMW_CFG>
          Path to openmw.cfg By default, uses the system paths defined by: https://openmw.readthedocs.io/en/latest/reference/modding/paths.html Can be the literal path to an openmw.cfg file (including not literally being called openmw.cfg) Or the directory in which an openmw.cfg file lives
  -7, --classic
          Enables classic mode using vtastek shaders. ONLY for openmw 0.47. Relevant shaders can be found in the OpenMW discord: https://discord.com/channels/260439894298460160/718892786157617163/966468825321177148
  -o, --output <OUTPUT>
          Output directory. The plugin may be saved to any location, but its name will always be `S3Lightfixes.omwaddon`. Accepts relative and absolute terms
  -l, --write-log
          Whether to save a text form of the generated plugin. Extremely verbose! You probably don't want to enable this unless asked specifically to do so
  -e, --auto-enable
          Whether to automatically enable the output plugin in openmw.cfg. Disabled by default, and only available via CLI. Typically lightfixes is ran under momw-configurator, making this param unnecessary for many users
  -n, --no-notifications
          If used, print to stdout instead of using native GUI dialogs. Not available on android
  -d, --debug
          Output debugging information during lightfixes generation Primarily displays output related to the openmw.cfg being used for generation
  -i, --info
          Outputs version info
  -f, --no-flicker <DISABLE_FLICKERING>
          Whether to disable flickering lights during lightfixes generation [possible values: true, false]
  -p, --no-pulse <DISABLE_PULSE>
          Whether to disable pulsing lights during lightfixes generation [possible values: true, false]
      --standard-hue <STANDARD_HUE>
          For lights in the orange range, multiply their HSV hue by this value.
          If this argument is not used, the value will be derived from lightConfig.toml or use the default value of 0.62.
          This argument has no short form due to a conflict with -h.
  -s, --standard-saturation <STANDARD_SATURATION>
          For lights in the orange range, multiply their HSV saturation by this amount.
          If this argument is not used, the value will be derived from lightConfig.toml or use the default value of 0.8.
  -v, --standard-value <STANDARD_VALUE>
          For lights in the orange range, multiply their HSV value by this amount.
          If this argument is not used, the value will be derived from lightConfig.toml or use the default value of 0.57.
  -r, --standard-radius <STANDARD_RADIUS>
          For lights in the orange range, multiply their radius by this value.
          If this argument is not used, the value will be derived from lightConfig.toml or use the default value of 1.2.
  -H, --colored-hue <COLORED_HUE>
          For lights that are red, purple, blue, green, or yellow, multiply their HSV hue by this value.
          If this argument is not used, the value will be derived from lightConfig.toml or use the default value of 1.
  -S, --colored-saturation <COLORED_SATURATION>
          For lights that are red, purple, blue, green, or yellow, multiply their HSV saturation by this amount.
          If this argument is not used, the value will be derived from lightConfig.toml or use the default value of 0.9.
  -V, --colored-value <COLORED_VALUE>
          For lights that are red, purple, blue, green, or yellow, multiply their HSV value by this amount.
          If this argument is not used, the value will be derived from lightConfig.toml or use the default value of 0.7.
  -R, --colored-radius <COLORED_RADIUS>
          For lights that are red, purple, blue, green, or yellow, multiply their radius by this value.
          If this argument is not used, the value will be derived from lightConfig.toml or use the default value of 1.1.
  -M, --duration-mult <DURATION_MULT>
          Multiplies the duration of all carryable lights.
          If this argument is not used, the value will be derived from lightConfig.toml or use the default value of 2.5.
  -x, --excluded-ids <EXCLUDED_IDS>
          List of Regex patterns of light recordIds to exclude. This setting is *merged* onto values defined by lightconfig.toml.
          If this argument is not used, the value will be derived from lightConfig.toml.
  -X, --excluded-plugins <EXCLUDED_PLUGINS>
          List of Regex patterns of plugins to exclude. This setting is *merged* onto values defined by lightconfig.toml.
          If this argument is not used, the value will be derived from lightConfig.toml.
      --light <LIGHT_OVERRIDES>
          Colon-separated list of regexes to light values.
               May be specified multiple times instead of as a separated list.
               Light values are specified as *either* fixed HSV values or as multipliers of existing ones.
               EG:
               --light "Torch_001=radius=255,hue=240,duration=1200,flag=FLICKERSLOW" --light "Torch_002=radius_mult=2.0,hue_mult=1.3,duration_mult=5.0,flag=NONE"
               OR
               --light "Torch_001=radius=255,hue=240,duration=1200,flag=FLICKERSLOW:Torch_002=radius_mult=2.0,hue_mult=1.3,duration_mult=5.0,flag=NONE"
               Hue is a range from 0-360 and saturation/value are normalized floats (0.0 - 1.0). Radius and duration are u32 (can be very big).
               `flag` may be: NONE, FLICKER, FLICKERSLOW, PULSE, PULSESLOW
               Fixed values are mutually exclusive with multipliers for each value and setting both will cause an error.
      --ambient <AMBIENT_OVERRIDES>
          
                      Colon-separated list of cell id regexes, to the corresponding ambient data.
                      `sunlight`, `ambient`, `fog`, and `fog_density` are available parameters.
                      Values are provided as fixed HSV values, no multipliers.
                      Hue is a range from 0-360 and saturation/value are normalized floats (0.0 - 1.0).
                      Each field of cell ambient data is separated by a semicolon, as below:
                      --ambient "caius cosades' house=sun=hue=360,saturation=1.0,value=1.0;ambient=hue=24,saturation=0.25,value=0.69"
                      
  -U, --update-light-config
          Force-saves the light config on this run. Note that this parameter does not merge into lightConfig.toml like others, and must be manually set there.
  -h, --help
          Print help
```
