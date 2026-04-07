# Third-Party Notices

Hematite includes or derives from the following third-party works.

---

## Kokoros (Rust Kokoro TTS)

**Original repository:** https://github.com/lucasjinreal/Kokoros  
**Author:** Lucas Jin  
**License:** Apache License 2.0  

The vendored TTS library in `libs/kokoros/` is derived from the Kokoros project.
It has been substantially modified to integrate with Hematite's voice pipeline,
audio output model, and static linking requirements.

The original copyright notice is reproduced below as required by Apache 2.0:

> Copyright reserved by Lucas Jin under Apache License.

Full Apache 2.0 license text: https://www.apache.org/licenses/LICENSE-2.0

---

## Kokoro Model Weights

**Source:** https://huggingface.co/hexgrad/Kokoro-82M  
**License:** Apache License 2.0  

The Kokoro voice model weights (`.onnx` and `voices.bin`) are distributed
separately and are not included in this repository. Users who enable voice
must download them independently. See README for setup instructions.

---

## SQLite (via rusqlite)

Used under the public domain / MIT-style blessing. No notice required, included
here for completeness.

---

*All other dependencies are consumed as crates via Cargo and are governed by
their respective licenses, which can be reviewed via `cargo license` or at
https://crates.io.*
