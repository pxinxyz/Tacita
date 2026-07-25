use restricted_gui_d3d12::{D3d12Backend, FrameSync, OffscreenTarget};
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D12::ID3D12Resource;

#[test]
fn device_and_queue_creation_succeeds() {
    let (_backend, _device, _queue) =
        D3d12Backend::create_device().expect("D3D12CreateDevice must succeed on this machine");
}

#[test]
fn host_supplied_device_is_accepted() {
    let (_standalone, device, queue) =
        D3d12Backend::create_device().expect("device creation failed");
    let backend = D3d12Backend::new(&device, &queue).expect("new() must accept a host device");
    assert_eq!(backend.device(), &device);
    assert_eq!(backend.queue(), &queue);
}

#[test]
fn fence_signal_wait_round_trip() {
    let (_backend, device, queue) = D3d12Backend::create_device().expect("device creation failed");
    let mut sync = FrameSync::new(&device).expect("fence creation failed");

    let first = sync.signal(&queue).expect("signal failed");
    sync.wait(first).expect("wait failed");
    assert!(
        sync.completed_value() >= first,
        "the fence must reach the signaled value after wait"
    );

    let second = sync.signal(&queue).expect("second signal failed");
    assert_eq!(second, first + 1);
    sync.wait(second).expect("second wait failed");
    assert!(sync.completed_value() >= second);
}

#[test]
fn offscreen_target_creation_succeeds() {
    let (_backend, device, _queue) = D3d12Backend::create_device().expect("device creation failed");
    let target = OffscreenTarget::new(&device, 64, 32).expect("offscreen target creation failed");
    assert_eq!(target.width(), 64);
    assert_eq!(target.height(), 32);
}

#[test]
fn clear_readback_returns_clear_color() {
    let (mut backend, device, queue) =
        D3d12Backend::create_device().expect("device creation failed");
    let target = OffscreenTarget::new(&device, 32, 32).expect("offscreen target creation failed");

    let color = [1.0_f32, 128.0 / 255.0, 0.0, 1.0];
    target
        .clear(&queue, backend.sync_mut(), color)
        .expect("clear failed");

    let pixels = target
        .readback(&queue, backend.sync_mut())
        .expect("readback failed");
    assert_eq!(pixels.len(), 32 * 32 * 4);
    let expected = [255, 128, 0, 255];
    for (x, y) in [(0, 0), (16, 16), (31, 31)] {
        let i = (y * 32 + x) * 4;
        assert_eq!(
            &pixels[i..i + 4],
            &expected,
            "pixel ({x}, {y}) must be the clear color"
        );
    }
}

fn ref_count(resource: &ID3D12Resource) -> u32 {
    unsafe {
        let this = resource.as_raw();
        let vtbl = &**(this as *const *const windows::core::IUnknown_Vtbl);
        let count = (vtbl.AddRef)(this);
        let _ = (vtbl.Release)(this);
        count
    }
}

#[test]
fn clear_readback_does_not_leak_com_references() {
    let (mut backend, device, queue) =
        D3d12Backend::create_device().expect("device creation failed");
    let target = OffscreenTarget::new(&device, 32, 32).expect("offscreen target creation failed");

    let before = ref_count(target.resource());
    for _ in 0..2 {
        target
            .clear(&queue, backend.sync_mut(), [0.0, 0.0, 0.0, 1.0])
            .expect("clear failed");
        let _pixels = target
            .readback(&queue, backend.sync_mut())
            .expect("readback failed");
    }
    assert_eq!(
        ref_count(target.resource()),
        before,
        "clear/readback cycles must not change the render target's reference count"
    );
}
