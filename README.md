# Tacita (internally, restricted-gui)

> An immediate-mode GUI framework written in Rust — `#![no_std]`, renderer-agnostic, built for embedding anywhere.

Tacita (aka restricted-gui) is an immediate-mode GUI library whose core is `#![no_std]` (`core` + `alloc`, no CRT) and fully renderer- and platform-agnostic. The core never touches a GPU or an OS API: hosts feed it a per-frame input snapshot, declare UI with flat calls, and consume vertex/index/command draw lists through any renderer they bring.

The widget surface covers windows, tabs, collapsing headers, scroll regions, checkboxes, sliders, buttons, text inputs, combo boxes, a rich HSV color picker, keybind selectors, tooltips, and icon fonts merged into the same atlas as text. Anti-aliasing is analytic — exact pixel coverage rasterization in the AGG/FreeType lineage, cached as masks in the font atlas — and steady-state frames allocate zero memory after warmup.

Primary use case is game hacking overlays: an injected DLL rendering through an `IDXGISwapChain::Present` hook (D3D11/D3D12 backends included) with input from a WndProc hook, where the library's input-capture queries drive the swallow/pass-through decision against the game beneath. It serves equally for embedded tooling and any host that needs a GUI it fully controls.

## Status

Feature-complete and in active use — the full widget set, both native backends, the C++ FFI, and the Win32 input adapter are shipped and verified. Very usable today, but not finalized: the API surface may still shift, and further work (more widgets, IME/CJK input, multi-viewport) is pulled by real usage rather than a fixed roadmap.

## Architecture

```
 host application (injected DLL / windowed app)
        │  OS input events (WndProc hook, winit, ...)
        ▼
┌───────────────────────────────┐
│  adapters/win32               │
│  • raw message → InputState   │
│  • no unsafe, no Win32 calls  │
└──────────────┬────────────────┘
               │  per-frame InputState
┌──────────────┴────────────────┐
│  core (restricted-gui)        │
│  • #![no_std], zero-alloc     │
│  • widgets / layout / ids     │
│  • coverage-mask AA raster    │
│  • capture & z-order state    │
└──────────────┬────────────────┘
               │  DrawData (verts / indices / cmds)
┌──────────────┴────────────────┐
│  backends                     │
│  • d3d11  • d3d12  • wgpu     │
│  (host owns device/swapchain) │
└──────────────┬────────────────┘
               │
        Present hook / swapchain
```

`ffi/` (extern "C" + hand-written C++ header) and `sdk/` (flat `Gui` facade) sit beside `core/` as alternative entry surfaces.

## Crates

| Crate | Responsibility | Status |
|---|---|---|
| `core` | no_std library — widgets, layout, draw lists, text, coverage-mask AA | ✅ Complete |
| `sdk` | Flat `Gui` facade over the core (forwards-only) | ✅ Complete |
| `ffi` | extern "C" wrapper + C++ header with two-sided layout consistency guard | ✅ Complete |
| `adapters/win32` | Win32 message → `InputState` translation for WndProc hooks | ✅ Complete |
| `backends/d3d11` | D3D11 renderer; host owns device/swapchain | ✅ Complete |
| `backends/d3d12` | D3D12 renderer; fence-based frame sync, deferred release | ✅ Complete |
| `demo` | wgpu dev harness (reference backend) | ✅ Complete |

## Verification

- 400+ core tests — widget logic, capture/z-order semantics, layout, zero-alloc (counting allocator)
- Headless GPU pixel tests on real hardware for both D3D11 and D3D12 — renders read back and asserted pixel-exact
- MSVC-compiled C++ consumer smoke test against the FFI surface
- Two-sided consistency guard keeping the C++ header and the Rust structs in lockstep

## Building

```sh
cargo test --workspace                                      # full suite (GPU suites run headless)
cargo build -p restricted-gui --target x86_64-unknown-none  # the no_std gate
```

## Usage

```rust
let mut gui = Gui::new(font_bytes)?;   // once

// per frame:
gui.new_frame(input, dt);              // InputState from your platform layer
gui.begin_window("Status");
if gui.button("Save Config") { /* ... */ }
gui.end_window();
let draw = gui.render();               // DrawData → your renderer
```

## License

MIT
