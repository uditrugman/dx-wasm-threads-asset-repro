# dx-wasm-threads-asset-repro

Minimal reproducer for a follow-up to
[dioxuslabs/dioxus#5079](https://github.com/DioxusLabs/dioxus/issues/5079).
`dx build --platform web` fails to extract and bundle `asset!()`
records when the standard library is rebuilt with `+atomics`, even
on dx 0.7.9 (which includes
[PR #5163](https://github.com/DioxusLabs/dioxus/pull/5163) and
[PR #5189](https://github.com/DioxusLabs/dioxus/pull/5189) that were
the previous round of fixes for this).

## The crate

- One `dioxus` web app: `src/main.rs` with **one** `asset!()` call.
- One asset: `assets/style.css` (35 bytes).
- `.cargo/config.toml` reduced via bisection to **the minimum that
  triggers the bug** — just `+atomics` + `-Z build-std=std,panic_abort`.
- No `cpal`, no `wasm-bindgen-rayon`, no `--shared-memory`, no TLS
  exports, no threads spawned, nothing exotic.

## Versions

- `dx --version` → `dioxus 0.7.9 (bfcc111)`
- `dioxus = "0.7.9"` (web feature)
- `rustc +nightly` (any recent nightly with `rust-src`)

## Reproduce

```sh
dx build --platform web
```

Output:

```
WARN Failed to deserialize as BundledAsset. Data length: 4096,
     first 32 bytes: [119, 97, 115, 109, 45, 116, 104, 114, 101, 97,
     100, 115, 45, 97, 115, 115, 101, 116, 45, 114, 101, 112, 114,
     111, 47, 97, 115, 115, 101, 116, 115, 47]
     # ascii: "wasm-threads-asset-repro/assets/" — *the actual asset record*
WARN Found a symbol at offset 4213030 that could not be deserialized.
     This may be caused by a mismatch between your dioxus and
     dioxus-cli versions, or the symbol may be in an unsupported format.
INFO Client build completed successfully!
```

Resulting `public/assets/` is empty. The wasm still contains unpatched
`"This should be replaced by dx as part of the build process..."`
placeholder strings. At runtime, every `asset!()` URL 404s.

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

## Bisection — minimum trigger

| `.cargo/config.toml`                          | Asset bundling |
|-----------------------------------------------|----------------|
| `+atomics` alone (no `-Z build-std`)          | works          |
| `+bulk-memory` alone + `-Z build-std`         | works          |
| `+atomics` + `-Z build-std`                   | **broken**     |
| `+atomics,+bulk-memory` + `-Z build-std`      | broken         |
| Full wasm-threads set (TLS exports, etc.)     | broken         |

The bug is triggered by **`std` rebuilt with `+atomics`**. User-code
`+atomics` alone (linked against the precompiled std) doesn't change
the wasm enough to break the scanner. Rebuilt std with atomic ops in
it does. No linker flags are involved; `--shared-memory`,
`--import-memory`, the TLS exports — none of them are needed to
reproduce.

## What is *not* the cause

Ruled out:

- **Version mismatch.** `dx`, `dioxus`, `dioxus-cli-config`,
  `manganis*` are all `0.7.9`. `manganis-core` intentionally pulls
  both `const-serialize 0.7.2` and `0.8.0-alpha.0` (the 0.8 with the
  `const-serialize-07` feature for read-side backcompat) — supported
  config.
- **Extra rustflags from the original #5079 setup**
  (`+mutable-globals`, `--export=__heap_base`, `--shared-memory`,
  `--import-memory`, `--max-memory`, TLS exports). Adding or
  removing any of them doesn't change whether the bug triggers.
- **Debug symbols.** `--debug-symbols=false` produces identical
  warnings.

## What is the cause (hypothesis)

The std rebuilt with `+atomics` emits atomic ops backed by data the
linker places in the wasm data section. That data lays out the
section in a way `find_wasm_symbol_offsets` doesn't account for —
dx locates the candidate asset record offset correctly (the byte
preview proves it), but the structured deserialize of `BundledAsset`
fails. PR #5163's bulk-memory / passive-data-segment handling
doesn't recover.
