use std::cell::RefCell;
use std::error::Error;
use std::time::Instant;

use restricted_gui::context::Context;
use restricted_gui::input::KeyBinding;
use restricted_gui::ScrollFlags;
use restricted_gui_d3d12::{D3d12Backend, Device, FrameSync, FRAMES_IN_FLIGHT};
use restricted_gui_win32::Win32InputAdapter;

use windows::core::{w, Interface};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Direct3D12::{
    ID3D12CommandAllocator, ID3D12CommandList, ID3D12DescriptorHeap, ID3D12GraphicsCommandList,
    ID3D12Resource, D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_CPU_DESCRIPTOR_HANDLE,
    D3D12_DESCRIPTOR_HEAP_DESC, D3D12_DESCRIPTOR_HEAP_FLAG_NONE, D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
    D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0, D3D12_RESOURCE_BARRIER_FLAG_NONE,
    D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_STATES, D3D12_RESOURCE_STATE_PRESENT,
    D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_TRANSITION_BARRIER,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_UNSPECIFIED, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, IDXGIFactory4, IDXGISwapChain3, DXGI_CREATE_FACTORY_FLAGS, DXGI_PRESENT,
    DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG,
    DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetWindowLongPtrW,
    LoadCursorW, PeekMessageW, PostQuitMessage, RegisterClassW, SetWindowLongPtrW, ShowWindow,
    TranslateMessage, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW, MSG, PM_REMOVE, SW_SHOW,
    WM_CAPTURECHANGED, WM_CHAR, WM_DESTROY, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS, WM_LBUTTONDBLCLK,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_QUIT,
    WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SIZE, WM_SYSKEYDOWN, WM_SYSKEYUP,
    WM_XBUTTONDBLCLK, WM_XBUTTONDOWN, WM_XBUTTONUP, WNDCLASSW, WNDCLASS_STYLES,
    WS_OVERLAPPEDWINDOW,
};

const DEJAVU_SANS: &[u8] = include_bytes!("../../../core/tests/assets/DejaVuSans.ttf");

const BACKBUFFER_COUNT: u32 = 3;

const COMBO_ITEMS: [&str; 30] = [
    "Item 01", "Item 02", "Item 03", "Item 04", "Item 05", "Item 06", "Item 07", "Item 08",
    "Item 09", "Item 10", "Item 11", "Item 12", "Item 13", "Item 14", "Item 15", "Item 16",
    "Item 17", "Item 18", "Item 19", "Item 20", "Item 21", "Item 22", "Item 23", "Item 24",
    "Item 25", "Item 26", "Item 27", "Item 28", "Item 29", "Item 30",
];

const LOG_LINES: usize = 30;

struct SharedInput {
    adapter: Win32InputAdapter,
    held_buttons: u8,
    resize: Option<(u32, u32)>,
    want_mouse: bool,
    want_keyboard: bool,
}

struct DemoState {
    enabled: bool,
    speed: f32,
    count: i32,
    text: String,
    fps_text: String,
    combo: usize,
    color: [u8; 4],
    binding: KeyBinding,
}

impl DemoState {
    fn new() -> Self {
        DemoState {
            enabled: true,
            speed: 25.0,
            count: 10,
            text: String::from("héllo wörld — edit me!"),
            fps_text: String::new(),
            combo: 0,
            color: [160, 120, 240, 255],
            binding: KeyBinding::None,
        }
    }
}

struct HostSlot {
    allocator: ID3D12CommandAllocator,
    list: ID3D12GraphicsCommandList,
    last_signaled: u64,
}

fn create_host_slots(device: &Device) -> Result<Vec<HostSlot>, Box<dyn Error>> {
    (0..FRAMES_IN_FLIGHT)
        .map(|_| {
            let allocator: ID3D12CommandAllocator =
                unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }?;
            let list: ID3D12GraphicsCommandList = unsafe {
                device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocator, None)
            }?;
            unsafe { list.Close() }?;
            Ok(HostSlot {
                allocator,
                list,
                last_signaled: 0,
            })
        })
        .collect()
}

fn transition(
    list: &ID3D12GraphicsCommandList,
    resource: &ID3D12Resource,
    before: D3D12_RESOURCE_STATES,
    after: D3D12_RESOURCE_STATES,
) {
    let barrier = D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: std::mem::ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource: std::mem::ManuallyDrop::new(Some(resource.clone())),
                Subresource: 0,
                StateBefore: before,
                StateAfter: after,
            }),
        },
    };
    unsafe {
        list.ResourceBarrier(std::slice::from_ref(&barrier));
        let transition = std::mem::ManuallyDrop::into_inner(barrier.Anonymous.Transition);
        drop(std::mem::ManuallyDrop::into_inner(transition.pResource));
    }
}

fn rtv_at(heap: &ID3D12DescriptorHeap, stride: u32, index: u32) -> D3D12_CPU_DESCRIPTOR_HANDLE {
    let start = unsafe { heap.GetCPUDescriptorHandleForHeapStart() };
    D3D12_CPU_DESCRIPTOR_HANDLE {
        ptr: start.ptr + index as usize * stride as usize,
    }
}

fn fetch_backbuffers(
    device: &Device,
    swapchain: &IDXGISwapChain3,
    rtv_heap: &ID3D12DescriptorHeap,
    rtv_stride: u32,
) -> Result<Vec<ID3D12Resource>, Box<dyn Error>> {
    (0..BACKBUFFER_COUNT)
        .map(|i| {
            let buffer: ID3D12Resource = unsafe { swapchain.GetBuffer(i) }?;
            unsafe {
                device.CreateRenderTargetView(&buffer, None, rtv_at(rtv_heap, rtv_stride, i))
            };
            Ok(buffer)
        })
        .collect()
}

fn handle_message(
    hwnd: HWND,
    input: &RefCell<SharedInput>,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> bool {
    if msg == WM_KILLFOCUS || msg == WM_CAPTURECHANGED {
        input.borrow_mut().adapter.clear_held();
        return false;
    }
    match msg {
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONDBLCLK | WM_MBUTTONDOWN
        | WM_MBUTTONDBLCLK | WM_XBUTTONDOWN | WM_XBUTTONDBLCLK => {
            input.borrow_mut().held_buttons += 1;
            unsafe { SetCapture(hwnd) };
        }
        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP | WM_XBUTTONUP => {
            let all_released = {
                let mut input = input.borrow_mut();
                input.held_buttons = input.held_buttons.saturating_sub(1);
                input.held_buttons == 0
            };
            if all_released {
                let _ = unsafe { ReleaseCapture() };
            }
        }
        _ => {}
    }
    if msg == WM_SIZE {
        input
            .borrow_mut()
            .adapter
            .handle_message(msg, wparam.0, lparam.0);
        let (w, h) = (
            (lparam.0 & 0xFFFF) as u16 as i16,
            ((lparam.0 >> 16) & 0xFFFF) as u16 as i16,
        );
        if w > 0 && h > 0 {
            input.borrow_mut().resize = Some((w as u32, h as u32));
        }
        return false;
    }
    let recognized = input
        .borrow_mut()
        .adapter
        .handle_message(msg, wparam.0, lparam.0);
    if !recognized {
        return false;
    }
    let input = input.borrow();
    match msg {
        WM_KEYDOWN | WM_KEYUP | WM_CHAR => input.want_keyboard,
        WM_SYSKEYDOWN | WM_SYSKEYUP => false,
        _ => input.want_mouse,
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_DESTROY {
        unsafe { PostQuitMessage(0) };
        return LRESULT(0);
    }
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const RefCell<SharedInput>;
    if !ptr.is_null() {
        let input = unsafe { &*ptr };
        if handle_message(hwnd, input, msg, wparam, lparam) {
            return LRESULT(0);
        }
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn declare_panel(ctx: &mut Context, state: &mut DemoState) {
    let mut ui = ctx.ui();
    ui.window("restricted-gui — D3D12 overlay", ScrollFlags::BAR, |ui| {
        ui.tab_bar("tabs##overlay", |bar| {
            bar.tab("Widgets", |ui| {
                ui.label(&state.fps_text);
                ui.checkbox("Enable feature", &mut state.enabled);
                ui.slider_float("Speed", &mut state.speed, 0.0..=100.0);
                ui.slider_int("Count", &mut state.count, 0..=50);
                if ui.button("Reset") {
                    state.enabled = true;
                    state.speed = 25.0;
                    state.count = 10;
                }
                ui.same_line();
                if ui.button("Clear text") {
                    state.text.clear();
                }
                ui.text_input("Text", &mut state.text);
                ui.label("Log (scroll region):");
                ui.scroll_region("log##overlay", [268.0, 150.0], |ui| {
                    for i in 1..=LOG_LINES {
                        ui.label(&format!("log line {i:02}: the quick brown fox"));
                    }
                });
                ui.tooltip_text(
                    "Wheel over the region scrolls it; wheel anywhere else\n\
                     scrolls the window (declared ScrollFlags::BAR).",
                );
            });
            bar.tab("Settings", |ui| {
                ui.combo("Combo", &COMBO_ITEMS, &mut state.combo);
                ui.color_picker_rgba("Color", &mut state.color);
                ui.keybind("Action key", &mut state.binding);
                ui.label(&format!("captured: {}", state.binding));
            });
        });
    });
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut ctx = Context::new(DEJAVU_SANS).map_err(|e| format!("font load failed: {e}"))?;
    ctx.style.window_bg = [30, 30, 46, 255];
    ctx.style.title_bg = [24, 24, 37, 255];
    ctx.style.title_text = [205, 214, 244, 255];
    ctx.style.widget_bg = [69, 71, 90, 255];
    ctx.style.widget_bg_hovered = [137, 180, 250, 255];
    ctx.style.widget_bg_active = [203, 166, 247, 255];
    ctx.style.text = [205, 214, 244, 255];

    let (mut backend, device, queue) = D3d12Backend::create_device()?;

    let shared = Box::new(RefCell::new(SharedInput {
        adapter: Win32InputAdapter::new(),
        held_buttons: 0,
        resize: None,
        want_mouse: false,
        want_keyboard: false,
    }));

    let hinstance = HINSTANCE::from(unsafe { GetModuleHandleW(None) }?);
    let class_name = w!("restricted-gui-d3d12-overlay");
    let class = WNDCLASSW {
        style: WNDCLASS_STYLES(0),
        lpfnWndProc: Some(wndproc),
        hInstance: hinstance,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }?,
        lpszClassName: class_name,
        ..Default::default()
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err("RegisterClassW failed".into());
    }
    let hwnd = unsafe {
        CreateWindowExW(
            Default::default(),
            class_name,
            w!("restricted-gui — D3D12 overlay example"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            960,
            720,
            None,
            None,
            Some(hinstance),
            None,
        )?
    };
    unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, shared.as_ref() as *const _ as isize) };
    let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };

    let mut viewport_size = {
        let mut rect = RECT::default();
        unsafe { GetClientRect(hwnd, &mut rect) }?;
        [
            (rect.right - rect.left) as f32,
            (rect.bottom - rect.top) as f32,
        ]
    };
    shared.borrow_mut().adapter.handle_message(
        WM_SIZE,
        0,
        ((viewport_size[1] as isize) << 16) | (viewport_size[0] as isize & 0xFFFF),
    );

    let swapchain_desc = DXGI_SWAP_CHAIN_DESC1 {
        Width: 0,
        Height: 0,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        Stereo: false.into(),
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: BACKBUFFER_COUNT,
        Scaling: DXGI_SCALING_STRETCH,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
        AlphaMode: DXGI_ALPHA_MODE_UNSPECIFIED,
        Flags: 0,
    };
    let factory: IDXGIFactory4 = unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)) }?;
    let swapchain: IDXGISwapChain3 =
        unsafe { factory.CreateSwapChainForHwnd(&queue, hwnd, &swapchain_desc, None, None)? }
            .cast()?;

    let rtv_heap: ID3D12DescriptorHeap = unsafe {
        device.CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            NumDescriptors: BACKBUFFER_COUNT,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
            NodeMask: 0,
        })
    }?;
    let rtv_stride =
        unsafe { device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV) };
    let mut backbuffers = fetch_backbuffers(&device, &swapchain, &rtv_heap, rtv_stride)?;

    let mut host_sync = FrameSync::new(&device)?;
    let mut host_slots = create_host_slots(&device)?;
    let mut slot_index = 0usize;

    let (pixels, w, h) = ctx.font().atlas_pixels();
    backend.upload_atlas(pixels, w, h)?;
    let mut last_atlas_generation = ctx.font().atlas_generation();
    eprintln!("[overlay] renderer up: {w}x{h} atlas (generation {last_atlas_generation})");
    eprintln!("[overlay] close the window to quit (Escape is the UI's)");

    let mut state = DemoState::new();
    let mut last_frame = Instant::now();
    let mut frame_count: u64 = 0;
    let mut fps_ema = 0.0f32;

    let mut msg = MSG::default();
    'outer: loop {
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            if msg.message == WM_QUIT {
                break 'outer;
            }
            unsafe {
                let _ = TranslateMessage(&msg);
                let _ = DispatchMessageW(&msg);
            }
        }

        let (frame_input, resize) = {
            let mut input = shared.borrow_mut();
            (input.adapter.begin_frame(), input.resize.take())
        };

        if let Some((w, h)) = resize {
            let host_last = host_sync.last_signaled_value();
            if host_last != 0 {
                host_sync.wait(host_last)?;
            }
            let backend_last = backend.sync().last_signaled_value();
            if backend_last != 0 {
                backend.sync().wait(backend_last)?;
            }
            backbuffers.clear();
            unsafe {
                swapchain.ResizeBuffers(
                    BACKBUFFER_COUNT,
                    w,
                    h,
                    DXGI_FORMAT_UNKNOWN,
                    DXGI_SWAP_CHAIN_FLAG(0),
                )
            }?;
            backbuffers = fetch_backbuffers(&device, &swapchain, &rtv_heap, rtv_stride)?;
            viewport_size = [w as f32, h as f32];
        }

        let now = Instant::now();
        let dt = (now - last_frame).as_secs_f32().clamp(0.0001, 0.25);
        last_frame = now;
        let fps = 1.0 / dt;
        fps_ema = if fps_ema == 0.0 {
            fps
        } else {
            fps_ema * 0.9 + fps * 0.1
        };
        state.fps_text = format!("FPS: {fps_ema:.1}  ({:.2} ms)", dt * 1000.0);

        ctx.set_input(frame_input.input_state());
        shared.borrow_mut().adapter.end_frame(frame_input);
        ctx.new_frame(dt);
        declare_panel(&mut ctx, &mut state);

        let generation = ctx.font().atlas_generation();
        if generation != last_atlas_generation {
            let (pixels, w, h) = ctx.font().atlas_pixels();
            backend.upload_atlas(pixels, w, h)?;
            eprintln!("[overlay] atlas re-upload: generation {generation} ({w}x{h})");
            last_atlas_generation = generation;
        }

        let draw = ctx.render();

        let slot = &mut host_slots[slot_index];
        if slot.last_signaled != 0 {
            host_sync.wait(slot.last_signaled)?;
        }
        unsafe { slot.allocator.Reset() }?;
        unsafe { slot.list.Reset(&slot.allocator, None) }?;

        let index = unsafe { swapchain.GetCurrentBackBufferIndex() };
        let backbuffer = &backbuffers[index as usize];
        let rtv = rtv_at(&rtv_heap, rtv_stride, index);

        transition(
            &slot.list,
            backbuffer,
            D3D12_RESOURCE_STATE_PRESENT,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
        );
        unsafe {
            slot.list
                .ClearRenderTargetView(rtv, &[0.03, 0.03, 0.05, 1.0], None)
        };
        unsafe { slot.list.Close() }?;
        let list: ID3D12CommandList = slot.list.clone().into();
        unsafe { queue.ExecuteCommandLists(&[Some(list)]) };

        backend.render(&draw, backbuffer, rtv, viewport_size)?;

        unsafe { slot.list.Reset(&slot.allocator, None) }?;
        transition(
            &slot.list,
            backbuffer,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
            D3D12_RESOURCE_STATE_PRESENT,
        );
        unsafe { slot.list.Close() }?;
        let list: ID3D12CommandList = slot.list.clone().into();
        unsafe { queue.ExecuteCommandLists(&[Some(list)]) };

        slot.last_signaled = host_sync.signal(&queue)?;
        unsafe { swapchain.Present(1, DXGI_PRESENT(0)) }.ok()?;
        slot_index = (slot_index + 1) % host_slots.len();

        frame_count += 1;
        if frame_count <= 3 || frame_count.is_multiple_of(60) {
            let (verts, indices, cmds) =
                (draw.vertices.len(), draw.indices.len(), draw.commands.len());
            eprintln!(
                "[overlay] frame {frame_count} rendered: {verts} verts, {indices} indices, \
                 {cmds} cmds, dt {:.2} ms",
                dt * 1000.0
            );
        }

        let mut input = shared.borrow_mut();
        input.want_mouse = ctx.wants_mouse_capture();
        input.want_keyboard = ctx.wants_keyboard_capture();
    }

    let host_last = host_sync.last_signaled_value();
    if host_last != 0 {
        host_sync.wait(host_last)?;
    }
    let backend_last = backend.sync().last_signaled_value();
    if backend_last != 0 {
        backend.sync().wait(backend_last)?;
    }
    eprintln!("[overlay] window closed after {frame_count} frames — clean shutdown");
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("[overlay] fatal: {e}");
        std::process::exit(1);
    }
}
