use tacita_d3d11::{D3d11Backend, OffscreenTarget};

#[test]
fn device_creation_succeeds() {
    let (_backend, _device, _context) =
        D3d11Backend::create_device().expect("D3D11CreateDevice must succeed on this machine");
}

#[test]
fn host_supplied_device_is_accepted() {
    let (_standalone, device, context) =
        D3d11Backend::create_device().expect("device creation failed");
    let backend = D3d11Backend::new(&device, &context).expect("new() must accept a host device");
    assert_eq!(backend.device(), &device);
}

#[test]
fn offscreen_rtv_creation_succeeds() {
    let (_backend, device, _context) =
        D3d11Backend::create_device().expect("device creation failed");
    let target = OffscreenTarget::new(&device, 64, 32).expect("offscreen target creation failed");
    assert_eq!(target.width(), 64);
    assert_eq!(target.height(), 32);
}

#[test]
fn clear_readback_returns_clear_color() {
    let (_backend, device, context) =
        D3d11Backend::create_device().expect("device creation failed");
    let target = OffscreenTarget::new(&device, 32, 32).expect("offscreen target creation failed");

    let color = [1.0_f32, 128.0 / 255.0, 0.0, 1.0];
    target.clear(&context, color);

    let pixels = target.readback(&context).expect("readback failed");
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
