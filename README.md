# dx-wasm-threads-asset-repro

Minimal reproducer for a follow-up to
[dioxuslabs/dioxus#5079](https://github.com/DioxusLabs/dioxus/issues/5079).
`dx build --platform web` fails to extract and bundle `asset!()`
records when the wasm-threads rustflags from `wasm-bindgen-rayon` /
`cpal`'s `audioworklet` backend are present, even on dx 0.7.9 (which
includes [PR #5163](https://github.com/DioxusLabs/dioxus/pull/5163)
and [PR #5189](https://github.com/DioxusLabs/dioxus/pull/5189) that
were the previous round of fixes for this).

## The crate

- One `dioxus` web app: `src/main.rs` with **one** `asset!()` call.
- One asset: `assets/style.css` (35 bytes).
- `.cargo/config.toml` with the wasm-threads rustflags. Same set as
  https://github.com/wheregmis/rustflags_dioxus (the original #5079
  reporter) plus two extras (`+mutable-globals` and
  `--export=__heap_base`) — removing those two does **not** change
  the symptom.
- No `cpal`, no threads spawned, nothing exotic.

## Versions

- `dx --version` → `dioxus 0.7.9 (bfcc111)`
- `dioxus = "0.7.9"` (web feature)
- `rustc +nightly` (any recent nightly with `rust-src`)

## Reproduce

```sh
dx build --platform web
```

Watch the output. The asset is found in the wasm but deserialization fails:

```
WARN Failed to deserialize as BundledAsset. Data length: 4096,
     first 32 bytes: [119, 97, 115, 109, 45, 116, 104, 114, 101, 97,
     100, 115, 45, 97, 115, 115, 101, 116, 45, 114, 101, 112, 114,
     111, 47, 97, 115, 115, 101, 116, 115, 47]
     # ascii: "wasm-threads-asset-repro/assets/" — that's *the actual asset record*
WARN Found a symbol at offset 4213283 that could not be deserialized.
     This may be caused by a mismatch between your dioxus and
     dioxus-cli versions, or the symbol may be in an unsupported
     format.
INFO Running wasm-bindgen...
INFO Client build completed successfully!
```

Resulting `public/assets/` is empty. The wasm still contains 3
unpatched `"This should be replaced by dx as part of the build
process..."` placeholder strings. At runtime, every `asset!()` URL
404s.

## Contrast: same source, no rustflags

```sh
mv .cargo/config.toml .cargo/config.toml.bak
rm -rf target/dx
dx build --platform web
```

Output:

```
INFO Copying asset (1/1): .../assets/style.css
INFO Client build completed successfully!
```

`public/assets/style-dxh869c7541c5a361e.css` exists (hashed name as expected).

## What is **not** the cause

Verified during isolation:

- Not version mismatch — `dx`, `dioxus`, `dioxus-cli-config`, `manganis`, `manganis-core`, `manganis-macro` are all `0.7.9`. `manganis-core` intentionally pulls both `const-serialize 0.7.2` and `0.8.0-alpha.0` (the 0.8 with the `const-serialize-07` feature for read-side backcompat) — supported config.
- Not the two "extra" rustflags vs the original #5079 reporter
  (`+mutable-globals`, `--export=__heap_base`). Removing them produces
  identical warnings.
- Not debug-symbols. `--debug-symbols=false` produces identical warnings.

## What is the cause

The wasm-threads rustflags (probably `--shared-memory` +
`--import-memory` + the TLS exports, all required by
`-Z build-std=std,panic_abort` for std to be rebuilt with `+atomics`)
change the wasm data section layout enough that dx's
`find_wasm_symbol_offsets` scanner reads the candidate asset record
incorrectly. dx finds the right offset (the byte-pattern proves it),
but the deserialization fails — and PR #5163's hand-off to
`const-serialize` doesn't recover.
