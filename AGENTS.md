# AGENTS.md

This file records repo-specific conventions for coding agents. Keep it short
and stable. Put temporary task state in a handoff, not here.

## Build and test

```sh
cargo check -p slot
cargo test -p slot --tests
go-task test
```

The full test suite includes machine-specific tests that may skip when a
vendored core, cart or display is unavailable. Prefer targeted tests while
iterating, then run the full suite before declaring a change complete.

Release builds:

```sh
cargo build --release -p slot
go-task dist
```

## Device builds

Device builds are **not** Rust cross-compilation. `build:device` runs the
normal Cargo build inside an arm64 Linux container (QEMU on x86 hosts) and
writes to `target-device/`.

```sh
go-task build:device
go-task dist:device
go-task deploy:device
```

Rules:

- Run at most one device build at a time. Concurrent builds contend for the
  same Cargo target lock and look like a hang.
- Device builds are slow, especially the final fat-LTO link. Do not interrupt
  one just because it has no output for several minutes.
- `device:sh` caches Cargo downloads in named Docker volumes
  (`slot-cargo-registry`, `slot-cargo-git`).
- The container uses `--network host` and an HTTP(S) proxy when configured.
  Do not hardcode a personal proxy or LAN address in the repo.
- `dist:device` creates output owned by the host UID, not root. Keep it that
  way.

## Vendored cores

Architecture-specific binaries live under:

```text
vendor/x86_64/
vendor/aarch64/
```

- Host cores use the host architecture and `DLL_EXTENSION`.
- Device cores are always `vendor/aarch64/*.so`.
- Do not write a host build over a device core or vice versa.
- Runtime core lookup is `System/<core>` first, then the current-architecture
  vendor directory, then the legacy top-level vendor directory.
- `SLOT_CORE` overrides the lookup with an explicit path.

Core tasks:

```sh
go-task core:mgba:host
go-task core:gpsp:host
go-task core:gambatte:host
go-task core:device
go-task core:gpsp
go-task core:gambatte
go-task fsck:exfat
```

## Display model

Keep these layers distinct:

```text
GameFrame  native core output
GameRect   game picture rectangle / shader output
Panel      device screen resolution
Window     host window (host only)
```

- `GameFrame` is the real core resolution: GBA `240x160`, GB/GBC `160x144`.
- The game shader renders into `GameRect`, never directly into the final
  panel when adding a rendering feature.
- `GameRect` is composited into `Panel` one-to-one at integer pixel
  coordinates.
- UI and bezels are drawn at `Panel` resolution, after the game.
- Host mode renders a simulated `Panel` (`SLOT_PANEL`) and scales the whole
  panel to `SLOT_WINDOW` at the end.
- `GameRect` modes are `Integer`, `Fit` and `Fill`.
  - `Integer`: largest whole multiple, no stretch.
  - `Fit`: preserve aspect ratio, no crop.
  - `Fill`: stretch to the full panel.
- `Fill` does not draw a game bezel.

`Canvas`, `GameLayout` and `GameRect` are all in panel pixels. Do not
introduce a separate logical layout resolution or convert game rectangles
through one. The only `720x480` default is the simulated host panel used when
`SLOT_PANEL` is absent; it is a panel choice, not a layout coordinate.

## Shaders and bezels

- External shaders are scanned from `Shaders/Common`, `Shaders/GBA`,
  `Shaders/GB` and `Shaders/GBC`.
- Only single-pass RetroArch GLSL/GLSLP presets are supported. Multi-pass
  presets must be rejected, not silently ignored.
- `filter_linear0` controls sampling of the game texture; a shader that needs
  interpolation (for example sharp-shimmerless) will not behave correctly
  with nearest sampling.
- Game bezels are full-screen panel-sized images with a transparent game
  area. Do not draw a bezel stretched into `GameRect`.
- Add bundled shader assets under `card/Shaders`; `sdcard`, `dist` and
  `dist:device` copy that tree.

## Host input and panel simulation

- Host gamepads use SDL3. Keep the Guide button polled directly; SDL3 delays
  Guide release events.
- Release every held button on disconnect.
- `SLOT_PANEL` selects the simulated device panel; `SLOT_WINDOW` is the host
  window size and must not silently pick a panel size.

## Repository hygiene

- Do not commit `target/`, `target-device/`, `dist/`, `dist-device/`, `vendor/`
  or `sdcard/` build output.
- Preserve unrelated working-tree changes. Do not reset, clean or force-push
  to make a task easier.
- Ask before committing or pushing unless the user has explicitly requested
  that exact operation.
