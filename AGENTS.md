# Agent guidance for S3LightFixes

The package is the Luau program at the repository root: `jess.toml`, `src/`, `tests/`, `bench/`;
its command target is `s3lightfixes`. The website beside it (`config.toml`, `content/`,
`sass/brand.sass`, `static/img`, `static/js`, `data/schematics`) is its project site, built by the
DreamWeave Mod Template through jess (`jess-workspace.toml` names the template; `jess.lock` pins
it). The Rust program lives on the `rust` branch and is not developed here.

## Verification

```sh
jess workspace sync --locked
jess workspace test
jess workspace run --trusted dreamweave-check
```

`content/home/mod.toml` is the project: `format = "jpk"`, `jess` names the package, `[[releases]]`
oldest first, each version matching `jess.toml` at its tag `s3lightfixes-<version>`. Never edit
`content/home/mod.lock`; CI writes it.
