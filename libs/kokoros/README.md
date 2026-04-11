# hematite-kokoros

`hematite-kokoros` is the Hematite-maintained fork of the Rust `kokoros` text-to-speech engine used by Hematite's local voice pipeline.

## Attribution

This crate is derived from `lucasjinreal/Kokoros`:

- Upstream repository: <https://github.com/lucasjinreal/Kokoros>
- Upstream crate/workspace name: `kokoros`

Hematite keeps the Rust library crate name as `kokoros` for source compatibility, but publishes the package under a distinct crates.io package name so the fork is clearly identified.

## Hematite-specific changes

- removed the original C++/system dependency assumptions used by the upstream project
- aligned the ONNX Runtime setup with Hematite's local Windows packaging flow
- kept the library usable as a vendored TTS engine inside the main Hematite CLI

## License note

The upstream project README states that Kokoros is provided under the Apache License. This fork preserves attribution to the upstream project and should be reviewed against upstream licensing before public crates.io publication.
