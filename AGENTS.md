# AGENTS.md

This file records repo-specific conventions for coding agents. Keep it short
and stable. Put temporary task state in a handoff, not here.

## Build and test

```sh
cargo check --workspace --all-targets
cargo nextest run -p slot
go-task test
```

Use `cargo check --workspace --all-targets` for compile-only checks. Use
nextest or `go-task test` to run tests; do not run `cargo test --workspace`
in parallel, because a few tests share process-wide indexes. The full suite
includes machine-specific tests that may skip when a vendored core, cart or
display is unavailable. Prefer targeted tests while iterating, then run the
full suite before declaring a change complete.

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
GameFrame   native core output
GameSize    width and height chosen by Integer / Fit / Fill
GameOffset  x and y chosen by the panel or a bezel's transparent hole
GameRect    GameSize + GameOffset, in panel pixels
Panel       device screen resolution
Window      host window (host only)
```

- `GameFrame` is the real core resolution: GBA `240x160`, GB/GBC `160x144`.
- `GameSize` only answers how wide and tall the picture is. It never decides
  its position.
- `GameOffset` is the single place that decides x/y: centre of the panel, or
  centre of a panel-sized bezel's transparent hole.
- `GameRect` is `GameSize` at `GameOffset`, and its FBO is exactly that size.
  It is composited into `Panel` one-to-one at integer pixel coordinates.
- The game shader renders into `GameRect`, never directly into the final
  panel when adding a rendering feature.
- UI and bezels are drawn at `Panel` resolution, after the game.
- `Canvas` is also panel pixels. Do not introduce a separate logical layout
  resolution or convert game rectangles through one.
- `GameRect` modes are `Integer`, `Fit` and `Fill`.
  - `Integer`: largest whole multiple, no stretch.
  - `Fit`: preserve aspect ratio, no crop.
  - `Fill`: stretch to the full panel.
- `Fill` has no offset and does not draw a game bezel.
- There is no 3:2 band, `GameLayout`, SpFrame or SP strip. AGS-102 is an
  ordinary panel-sized bezel overlay loaded from `System/bezels`.
- The only `720x480` default is the simulated host panel used when
  `SLOT_PANEL` is absent; it is a panel choice, not a layout coordinate.

## Updates

- Shipped fork builds must not check the upstream slot update channel.
  `Check for Updates` is hidden and the worker is not started unless
  `SLOT_UPDATES` is set in the environment.
- `SLOT_UPDATES=1` is a manual debugging override only; it still points at
  the upstream `versions.json` and release assets.
- Do not make the upstream URL configurable by silently falling back to it.
  A fork release channel needs its own explicit URL and version scheme.

## Upstream ports

Do not merge upstream wholesale just because it is available. Review each
upstream commit and port only what fits this fork.

Use a temporary detached worktree for upstream and a port branch in the fork:

```sh
git worktree add --detach ../slot-upstream upstream/main
git switch -c port/upstream-vX.Y
```

For every upstream commit:

1. Read `git show --stat <sha>` and `git show <sha>`.
2. Classify it:
   - port directly
   - port with adaptation
   - defer
   - skip
3. Reject changes that depend on assumptions this fork removed:
   - fixed `720x480` logical layout
   - `GameLayout`, band or SpFrame
   - `video_mode`
   - DS+ dual-screen or DSperate paths
   - KMS layouts or panels this fork does not support
4. Prefer `git cherry-pick -x <sha>` when it applies cleanly.
5. Otherwise port manually and say `Ported from upstream <sha>` in the
   commit message.
6. Add the matching upstream tests when possible.

Keep a temporary port ledger outside the repository. Do not commit temporary
state or machine-specific paths.

After each batch:

```sh
cargo check --workspace --all-targets
cargo nextest run -p slot --test <relevant-test>
go-task test
go-task lint
go-task lint:device
```

Merge the port branch back only after the full suite and device checks pass.
Do not rebase or force-push the fork branch.

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
- The transparent hole is measured in panel pixels and only feeds
  `GameOffset`; it must not become part of `GameSize`.
- AGS-102 is a normal panel overlay from `System/bezels`, not a separate
  strip or animation path.
- Scale-suffixed bezels are `_integer` and `_fit`. `_spscreen` is no longer
  recognized.
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
