# S3LightFixes

S3LightFixes is a descendant of [Waza-lightfixes](https://modding-openmw.com/mods/waza_lightfixes/), which itself is a descendant of [Lightfixes.pl](https://modding-openmw.com/tips/custom-shaders/#lightfixes-plugin) by vtastek. All three applications are designed to make ESP files which adjust the lighting values from *all* mods listed in one's openmw.cfg.

In other words, make light gud. What sets this version apart is that it's a standalone binary application, instead of piggybacking off tes3cmd. The changes it makes are the same as the previous two, but with additional conveniences like automatic installation and support for portable installs of OpenMW, and itself having greater portability.

<div align="center">
<h1>DOWNLOAD</h1>

[Windows](https://github.com/magicaldave/S3LightFixes/releases/latest/download/windows-latest.zip) | [Mac](https://github.com/magicaldave/S3LightFixes/releases/latest/download/macos-latest.zip) | [Linux](https://github.com/magicaldave/S3LightFixes/releases/latest/download/ubuntu-latest.zip)
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
          Path to openmw.cfg By default, uses the system paths defined by:
  -7, --classic
          Enables classic mode using vtastek shaders. ONLY for openmw 0.47. Relevant shaders can be found in the OpenMW discord:
  -o, --output <OUTPUT>
          Output file path. Accepts relative and absolute terms
  -l, --write-log
          Whether to save a text form of the generated plugin. Extremely verbose!
  -e, --auto-enable
          Whether to automatically enable the output plugin in openmw.cfg. Disabled by default, and only available via CLI
  -n, --no-notifications
          If used, print to stdout instead of using native GUI dialogs
  -d, --debug
          Output debugging information during lightfixes generation Primarily displays output related to the openmw.cfg being used for generation
  -i, --info
          Outputs version info
  -f, --no-flicker <DISABLE_FLICKERING>
          Whether to disable flickering lights during lightfixes generation [possible values: true, false]
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
  -h, --help
          Print help (see more with '--help')
  ```
