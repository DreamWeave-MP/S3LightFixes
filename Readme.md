# S3LightFixes

Generate a plugin that retunes every light in your OpenMW load order: color, radius, duration and
flicker, from one command. A Jess package in Luau: `org.dreamweave.s3lightfixes`, command
`s3lightfixes`.

```toml
[dev-dependencies]
"org.dreamweave.s3lightfixes" = "1"
```

```sh
jess workspace sync
jess workspace run --trusted s3lightfixes
```

The site at <https://dreamweave-mp.github.io/S3LightFixes/> is this repository, built by the
DreamWeave Mod Template through jess on every push; every release is a JPK on this repository's
GitHub Releases, and `jess workspace sync` takes it from there.

The Rust program, 0.5.0 and below with its downloads for Windows, macOS, Linux, Android and
PortMaster, lives on the `rust` branch. It is no longer developed here.

## Developing

`jess workspace test` runs the package's suites. `jess workspace run --trusted dreamweave-check`
validates the site; `dreamweave-site -- public <url>` renders it. Releases are tags
`s3lightfixes-<version>`, matching `version` in `jess.toml` and a `[[releases]]` entry in
`content/home/mod.toml`.
