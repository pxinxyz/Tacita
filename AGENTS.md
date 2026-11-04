# Tacita agent instructions

## Scope

Tacita is a Rust GUI framework for applications that own their window, graphics device, and swap chain.
The core uses `no_std` with `alloc`.
The application declares controls each frame.
Tacita retains interaction state and produces draw data.

Read [README.md](README.md) for public integration examples and current limitations.

## Repository map

| Path | Responsibility |
| --- | --- |
| [core](core) | Controls, layout, input, text, drawing, and accessibility nodes. |
| [sdk](sdk) | The public `Gui` interface and scope handling. |
| [ffi](ffi) | The C interface, panic containment, and C++ smoke program. |
| [backends](backends) | D3D11 and D3D12 renderers and shared shaders. |
| [adapters/win32](adapters/win32) | Win32 input translation. |
| [host](host) | Clipboard, accessibility, and file watching services. |
| [demo](demo) | The wgpu gallery and renderer checks. |
| [fuzz](fuzz) | A separate Cargo workspace for fuzzing. |
| [rhai-probe](rhai-probe) | A separate Cargo workspace for Rhai experiments. |

## Implementation rules

- Keep window, graphics device, and swap chain ownership with the application.
- Keep `core` and `sdk` usable without `std`.
- Do not make `core` depend on `demo`, winit, or wgpu.
- Use logical pixels for input and layout. Apply display scale in the renderer.
- Consume `DrawData` before starting the next frame.
- Upload the font atlas before drawing and whenever its generation changes.
- Preserve matching `begin_*` and `end_*` scopes.
- Preserve public SDK behavior when changing internal controls.
- Update Rust FFI definitions and [ffi/tacita.h](ffi/tacita.h) together.
- Keep Rust panics inside the FFI boundary. Preserve `gui_last_error_code()` behavior.
- Keep public crate names under `tacita`. Do not restore the old `restricted-gui` names.
- Keep implementation explanations in Markdown. Do not add source comments or docstrings.
- Retain third-party font licenses and attribution.

## Validation

Run commands from the repository root.
Windows renderer work requires the Windows SDK, Microsoft C++ build tools, and a graphics device.

For Rust changes:

```sh
cargo fmt --all --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
```

For changes affecting `no_std` support, install the target if necessary and run:

```sh
rustup target add x86_64-unknown-none
cargo build --locked -p tacita -p tacita-sdk --target x86_64-unknown-none
```

For C interface changes, run the real consumer through Git Bash:

```sh
bash ffi/tests/build_smoke.sh
```

For controls, layout, themes, or rendering, inspect the release gallery:

```sh
cargo run --locked --release -p tacita-demo
```

Check Tacita and Nightfall themes, compact windows, display scaling, text clipping, keyboard navigation, and scrolling.
Use [D3D11 visual tests](backends/d3d11/tests/visual_regression.rs) for native renderer changes.
Inspect changed captures before accepting new baselines.
Check D3D12 separately when changing shared rendering behavior.

Root workspace checks exclude `fuzz` and `rhai-probe`.
Run their checks separately when changing those workspaces.

## Reporting and changes

- Report the renderer, display scale, and build profile used for visual verification.
- Measure performance in release mode. Account for the demo's frame cap and vsync.
- Limit allocation claims to the sequences covered by [allocation tests](core/tests/zero_alloc.rs).
- Keep build output, executables, temporary captures, and logs out of commits.
- Use Conventional Commits. Push or publish only when the task authorizes it.
