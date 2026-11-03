use crate::data::{canvas, invalid, target};
use crate::{
    gui_export, slice_arg, str_arg, GuiColor, GuiContext, GuiGradientStop, GuiStr, GuiVec2,
};
use restricted_gui::{
    animation::GradientStop,
    data::{
        CurveEditor, CurveInterpolation, CurveKey, GradientEditor, PlotOptions, MAX_CURVE_KEYS,
        MAX_DATA_ROWS, MAX_EDITOR_STOPS,
    },
};

pub const GUI_CURVE_MAX_KEYS: u32 = MAX_CURVE_KEYS as u32;
pub const GUI_GRADIENT_EDITOR_MAX_STOPS: u32 = MAX_EDITOR_STOPS as u32;
pub const GUI_CURVE_STEP: u32 = 0;
pub const GUI_CURVE_LINEAR: u32 = 1;
pub const GUI_CURVE_CUBIC: u32 = 2;
pub const GUI_PLOT_AUTO_RANGE: u32 = 1;
pub const GUI_PLOT_AXES: u32 = 2;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiCurveKey {
    pub time: f32,
    pub value: f32,
    pub in_slope: f32,
    pub out_slope: f32,
    pub interpolation: u32,
}
impl GuiCurveKey {
    fn decode(self) -> Option<CurveKey> {
        Some(CurveKey {
            time: self.time,
            value: self.value,
            in_slope: self.in_slope,
            out_slope: self.out_slope,
            interpolation: match self.interpolation {
                0 => CurveInterpolation::Step,
                1 => CurveInterpolation::Linear,
                2 => CurveInterpolation::Cubic,
                _ => return None,
            },
        })
    }
}
impl From<CurveKey> for GuiCurveKey {
    fn from(k: CurveKey) -> Self {
        Self {
            time: k.time,
            value: k.value,
            in_slope: k.in_slope,
            out_slope: k.out_slope,
            interpolation: k.interpolation as u32,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiPlotOptions {
    pub size: GuiVec2,
    pub range: GuiVec2,
    pub color: GuiColor,
    pub flags: u32,
}
impl GuiPlotOptions {
    fn decode(self) -> Option<PlotOptions> {
        if self.flags & !3 != 0 {
            return None;
        }
        Some(PlotOptions {
            size: self.size.into(),
            range: if self.flags & GUI_PLOT_AUTO_RANGE != 0 {
                None
            } else {
                Some(self.range.into())
            },
            color: self.color.into(),
            axes: self.flags & GUI_PLOT_AXES != 0,
        })
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiPlotHover {
    pub index: usize,
    pub value: f32,
}
pub struct GuiCurveEditor(CurveEditor);
pub struct GuiGradientEditor(GradientEditor);
unsafe fn curve_keys(
    data: *const GuiCurveKey,
    count: usize,
) -> Option<([CurveKey; MAX_CURVE_KEYS], usize)> {
    if !(2..=MAX_CURVE_KEYS).contains(&count) {
        invalid();
        return None;
    }
    let Some(source) = (unsafe { slice_arg(data, count) }) else {
        invalid();
        return None;
    };
    let mut keys = [CurveKey::linear(0., 0.); MAX_CURVE_KEYS];
    for (i, raw) in source.iter().enumerate() {
        let Some(key) = raw.decode() else {
            invalid();
            return None;
        };
        keys[i] = key;
    }
    Some((keys, count))
}
unsafe fn gradient_stops(
    data: *const GuiGradientStop,
    count: usize,
) -> Option<([GradientStop; MAX_EDITOR_STOPS], usize)> {
    if !(2..=MAX_EDITOR_STOPS).contains(&count) {
        invalid();
        return None;
    }
    let Some(source) = (unsafe { slice_arg(data, count) }) else {
        invalid();
        return None;
    };
    let mut stops = [GradientStop {
        position: 0.,
        color: [0; 4],
    }; MAX_EDITOR_STOPS];
    for (i, raw) in source.iter().enumerate() {
        stops[i] = GradientStop {
            position: raw.position,
            color: raw.color.into(),
        };
    }
    Some((stops, count))
}

#[no_mangle]
pub unsafe extern "C" fn gui_curve_editor_new(
    data: *const GuiCurveKey,
    count: usize,
) -> *mut GuiCurveEditor {
    gui_export!(
        || {
            let Some((data, count)) = (unsafe { curve_keys(data, count) }) else {
                return std::ptr::null_mut();
            };
            match CurveEditor::new(&data[..count]) {
                Ok(m) => Box::into_raw(Box::new(GuiCurveEditor(m))),
                Err(_) => {
                    invalid();
                    std::ptr::null_mut()
                }
            }
        },
        std::ptr::null_mut()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_curve_editor_free(editor: *mut GuiCurveEditor) {
    gui_export!(
        || {
            if !editor.is_null() && (unsafe { target(editor) }).is_some() {
                unsafe {
                    drop(Box::from_raw(editor));
                }
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_curve_editor_count(editor: *mut GuiCurveEditor) -> usize {
    gui_export!(
        || { unsafe { target(editor) }.map_or(0, |e| e.0.keys().len()) },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_curve_editor_get(
    editor: *mut GuiCurveEditor,
    index: usize,
    out: *mut GuiCurveKey,
) -> bool {
    gui_export!(
        || {
            let Some(editor) = (unsafe { target(editor) }) else {
                return false;
            };
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            let Some(item) = editor.0.keys().get(index).copied() else {
                invalid();
                return false;
            };
            *out = item.into();
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_curve_editor_replace(
    editor: *mut GuiCurveEditor,
    data: *const GuiCurveKey,
    count: usize,
) -> bool {
    gui_export!(
        || {
            let Some(editor) = (unsafe { target(editor) }) else {
                return false;
            };
            let Some((data, count)) = (unsafe { curve_keys(data, count) }) else {
                return false;
            };
            if editor.0.replace(&data[..count]).is_err() {
                invalid();
                return false;
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_curve_editor_select(editor: *mut GuiCurveEditor, index: i32) -> bool {
    gui_export!(
        || {
            let Some(editor) = (unsafe { target(editor) }) else {
                return false;
            };
            if index < -1
                || editor
                    .0
                    .select(if index < 0 {
                        None
                    } else {
                        Some(index as usize)
                    })
                    .is_err()
            {
                invalid();
                return false;
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_curve_editor_selected(editor: *mut GuiCurveEditor) -> i32 {
    gui_export!(
        || { unsafe { target(editor) }.map_or(-1, |e| e.0.selected().map_or(-1, |i| i as i32)) },
        -1
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_curve_editor(
    ctx: *mut GuiContext,
    label: GuiStr,
    editor: *mut GuiCurveEditor,
    size: GuiVec2,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(editor) = (unsafe { target(editor) }) else {
                return false;
            };
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            match gui.ctx.ui().curve_editor(label, &mut editor.0, size.into()) {
                Ok(changed) => changed,
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_gradient_editor_new(
    data: *const GuiGradientStop,
    count: usize,
) -> *mut GuiGradientEditor {
    gui_export!(
        || {
            let Some((data, count)) = (unsafe { gradient_stops(data, count) }) else {
                return std::ptr::null_mut();
            };
            match GradientEditor::new(&data[..count]) {
                Ok(m) => Box::into_raw(Box::new(GuiGradientEditor(m))),
                Err(_) => {
                    invalid();
                    std::ptr::null_mut()
                }
            }
        },
        std::ptr::null_mut()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_gradient_editor_free(editor: *mut GuiGradientEditor) {
    gui_export!(
        || {
            if !editor.is_null() && (unsafe { target(editor) }).is_some() {
                unsafe {
                    drop(Box::from_raw(editor));
                }
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_gradient_editor_count(editor: *mut GuiGradientEditor) -> usize {
    gui_export!(
        || { unsafe { target(editor) }.map_or(0, |e| e.0.stops().len()) },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_gradient_editor_get(
    editor: *mut GuiGradientEditor,
    index: usize,
    out: *mut GuiGradientStop,
) -> bool {
    gui_export!(
        || {
            let Some(editor) = (unsafe { target(editor) }) else {
                return false;
            };
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            let Some(item) = editor.0.stops().get(index).copied() else {
                invalid();
                return false;
            };
            *out = GuiGradientStop {
                position: item.position,
                color: item.color.into(),
            };
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_gradient_editor_replace(
    editor: *mut GuiGradientEditor,
    data: *const GuiGradientStop,
    count: usize,
) -> bool {
    gui_export!(
        || {
            let Some(editor) = (unsafe { target(editor) }) else {
                return false;
            };
            let Some((data, count)) = (unsafe { gradient_stops(data, count) }) else {
                return false;
            };
            if editor.0.replace(&data[..count]).is_err() {
                invalid();
                return false;
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_gradient_editor_select(
    editor: *mut GuiGradientEditor,
    index: i32,
) -> bool {
    gui_export!(
        || {
            let Some(editor) = (unsafe { target(editor) }) else {
                return false;
            };
            if index < -1
                || editor
                    .0
                    .select(if index < 0 {
                        None
                    } else {
                        Some(index as usize)
                    })
                    .is_err()
            {
                invalid();
                return false;
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_gradient_editor_selected(editor: *mut GuiGradientEditor) -> i32 {
    gui_export!(
        || { unsafe { target(editor) }.map_or(-1, |e| e.0.selected().map_or(-1, |i| i as i32)) },
        -1
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_gradient_editor(
    ctx: *mut GuiContext,
    label: GuiStr,
    editor: *mut GuiGradientEditor,
    size: GuiVec2,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(editor) = (unsafe { target(editor) }) else {
                return false;
            };
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            match gui
                .ctx
                .ui()
                .gradient_editor(label, &mut editor.0, size.into())
            {
                Ok(changed) => changed,
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_curve_evaluate(
    editor: *mut GuiCurveEditor,
    time: f32,
    out: *mut f32,
) -> bool {
    gui_export!(
        || {
            let Some(editor) = (unsafe { target(editor) }) else {
                return false;
            };
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            match editor.0.evaluate(time) {
                Ok(v) => {
                    *out = v;
                    true
                }
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_gradient_sample(
    editor: *mut GuiGradientEditor,
    position: f32,
    out: *mut GuiColor,
) -> bool {
    gui_export!(
        || {
            let Some(editor) = (unsafe { target(editor) }) else {
                return false;
            };
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            match editor.0.sample(position) {
                Ok(v) => {
                    *out = v.into();
                    true
                }
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
unsafe fn plot(
    ctx: *mut GuiContext,
    label: GuiStr,
    samples: *const f32,
    count: usize,
    options: GuiPlotOptions,
    out: *mut GuiPlotHover,
    sparkline: bool,
) -> bool {
    let Some(gui) = (unsafe { canvas(ctx) }) else {
        return false;
    };
    let Some(out) = (unsafe { target(out) }) else {
        return false;
    };
    let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
        invalid();
        return false;
    };
    if count > MAX_DATA_ROWS {
        invalid();
        return false;
    }
    let Some(samples) = (unsafe { slice_arg(samples, count) }) else {
        invalid();
        return false;
    };
    let Some(mut options) = options.decode() else {
        invalid();
        return false;
    };
    if sparkline {
        options.axes = false;
    }
    match gui.ctx.ui().plot(label, samples, options) {
        Ok(hover) => {
            *out = hover.map_or(
                GuiPlotHover {
                    index: usize::MAX,
                    value: 0.,
                },
                |h| GuiPlotHover {
                    index: h.index,
                    value: h.value,
                },
            );
            true
        }
        Err(_) => {
            invalid();
            false
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn gui_plot(
    ctx: *mut GuiContext,
    label: GuiStr,
    samples: *const f32,
    count: usize,
    options: GuiPlotOptions,
    out: *mut GuiPlotHover,
) -> bool {
    gui_export!(
        || unsafe { plot(ctx, label, samples, count, options, out, false) },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_sparkline(
    ctx: *mut GuiContext,
    label: GuiStr,
    samples: *const f32,
    count: usize,
    options: GuiPlotOptions,
    out: *mut GuiPlotHover,
) -> bool {
    gui_export!(
        || unsafe { plot(ctx, label, samples, count, options, out, true) },
        false
    )
}
#[cfg(test)]
#[path = "data_editor_tests.rs"]
mod tests;
