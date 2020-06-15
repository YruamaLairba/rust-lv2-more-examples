# Programming LV2 Plugins - Rust Edition

This repository contains examples showing how to use the rust-lv2 framework.
The main target is to have examples focused on one aspect or extension of the
LV2 Spec.

## Building the samples
This project use a custom script to handle post-compilation step :

- Use `cargo xtask build --all` to build all projects.
- Use `cargo xtask build -p <project>` to build a specific example.
- Use `cargo xtask` to see more option.

Builded plugins are in the lv2 folder inside the cargo output dir
(`target/debug` by default).

## Licensing

Like original C and rust-lv2-book examples, the code is published under the
`ISC` license. See the [LICENSE file](LICENSE.md) for more info.
