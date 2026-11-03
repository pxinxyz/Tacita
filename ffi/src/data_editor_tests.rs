use super::*;
use crate::test_support::{new_context, window_frame};
use core::mem::{align_of, offset_of, size_of};
#[test]
fn editor_wire_layouts_and_enum_tags_are_pinned() {
    assert_eq!(
        (size_of::<GuiCurveKey>(), align_of::<GuiCurveKey>()),
        (20, 4)
    );
    assert_eq!(
        [
            offset_of!(GuiCurveKey, time),
            offset_of!(GuiCurveKey, value),
            offset_of!(GuiCurveKey, in_slope),
            offset_of!(GuiCurveKey, out_slope),
            offset_of!(GuiCurveKey, interpolation)
        ],
        [0, 4, 8, 12, 16]
    );
    assert_eq!(
        (size_of::<GuiPlotOptions>(), align_of::<GuiPlotOptions>()),
        (24, 4)
    );
    assert_eq!(
        [
            offset_of!(GuiPlotOptions, size),
            offset_of!(GuiPlotOptions, range),
            offset_of!(GuiPlotOptions, color),
            offset_of!(GuiPlotOptions, flags)
        ],
        [0, 8, 16, 20]
    );
    assert_eq!(size_of::<GuiPlotHover>(), 2 * size_of::<usize>());
    assert_eq!(align_of::<GuiPlotHover>(), align_of::<usize>());
    assert_eq!(offset_of!(GuiPlotHover, index), 0);
    assert_eq!(offset_of!(GuiPlotHover, value), size_of::<usize>());
    let raw = GuiCurveKey {
        time: 0.,
        value: 0.,
        in_slope: 0.,
        out_slope: 0.,
        interpolation: 99,
    };
    assert!(raw.decode().is_none());
}
#[test]
fn invalid_replacements_preserve_owned_editor_and_outputs() {
    unsafe {
        let mut keys = [
            GuiCurveKey::from(CurveKey::linear(0., 0.)),
            GuiCurveKey::from(CurveKey::linear(1., 1.)),
        ];
        let editor = gui_curve_editor_new(keys.as_ptr(), keys.len());
        assert!(!editor.is_null());
        assert!(gui_curve_editor_select(editor, 1));
        keys[1].interpolation = 99;
        assert!(!gui_curve_editor_replace(editor, keys.as_ptr(), 2));
        assert_eq!(
            crate::gui_last_error_code(),
            crate::GuiErrorCode::InvalidInput as u8
        );
        assert_eq!(gui_curve_editor_count(editor), 2);
        assert_eq!(gui_curve_editor_selected(editor), 1);
        let mut out = 7.;
        assert!(!gui_curve_evaluate(editor, f32::NAN, &mut out));
        assert_eq!(out, 7.);
        assert!(gui_curve_evaluate(editor, 0.5, &mut out));
        assert_eq!(out, 0.5);
        let mut key = keys[0];
        assert!(!gui_curve_editor_get(editor, usize::MAX, &mut key));
        assert_eq!(key.time, 0.);
        assert!(!gui_curve_editor_select(editor, -2));
        assert_eq!(gui_curve_editor_selected(editor), 1);
        gui_curve_editor_free(editor);
        let mut stops = [
            GuiGradientStop {
                position: 0.,
                color: [0; 4].into(),
            },
            GuiGradientStop {
                position: 1.,
                color: [200; 4].into(),
            },
        ];
        let editor = gui_gradient_editor_new(stops.as_ptr(), 2);
        assert!(!editor.is_null());
        stops[0].position = 0.1;
        assert!(!gui_gradient_editor_replace(editor, stops.as_ptr(), 2));
        let mut color = [7; 4].into();
        assert!(gui_gradient_sample(editor, 0., &mut color));
        assert_eq!(<[u8; 4]>::from(color), [0; 4]);
        gui_gradient_editor_free(editor);
        assert!(gui_curve_editor_new(core::ptr::null(), 2).is_null());
        assert_eq!(
            crate::gui_last_error_code(),
            crate::GuiErrorCode::InvalidInput as u8
        );
    }
}
#[test]
fn plot_failure_preserves_output_and_editor_leaves_balance_frames() {
    unsafe {
        let ctx = new_context();
        let keys = [
            GuiCurveKey::from(CurveKey::linear(0., 0.)),
            GuiCurveKey::from(CurveKey::linear(1., 1.)),
        ];
        let stops = [
            GuiGradientStop {
                position: 0.,
                color: [0; 4].into(),
            },
            GuiGradientStop {
                position: 1.,
                color: [255; 4].into(),
            },
        ];
        let curve = gui_curve_editor_new(keys.as_ptr(), 2);
        let gradient = gui_gradient_editor_new(stops.as_ptr(), 2);
        window_frame(ctx, [0., 0.], false, |ctx| {
            let label = GuiStr {
                ptr: core::ptr::null(),
                len: 0,
            };
            let options = GuiPlotOptions {
                size: GuiVec2 { x: 200., y: 80. },
                range: GuiVec2 { x: 0., y: 1. },
                color: [255; 4].into(),
                flags: GUI_PLOT_AUTO_RANGE | GUI_PLOT_AXES,
            };
            let samples = [0., 1.];
            let mut out = GuiPlotHover {
                index: 17,
                value: 7.,
            };
            assert!(!gui_plot(
                ctx,
                label,
                samples.as_ptr(),
                2,
                GuiPlotOptions {
                    flags: 99,
                    ..options
                },
                &mut out
            ));
            assert_eq!(out.index, 17);
            assert_eq!(out.value, 7.);
            assert!(gui_plot(ctx, label, samples.as_ptr(), 2, options, &mut out));
            assert_eq!(out.index, usize::MAX);
            gui_curve_editor(ctx, label, curve, GuiVec2 { x: 200., y: 100. });
            gui_gradient_editor(ctx, label, gradient, GuiVec2 { x: 200., y: 60. });
            assert_eq!(crate::gui_last_error_code(), 0);
        });
        gui_curve_editor_free(curve);
        gui_gradient_editor_free(gradient);
        crate::gui_context_free(ctx);
    }
}
