pub(crate) mod button;
pub(crate) mod checkbox;
pub(crate) mod label;
pub(crate) mod slider;
pub(crate) mod text_input;

use crate::context::Context;
use crate::draw::{DrawList, TextureId};
use crate::font::{Font, ShapeKey, ShapeMask};
use crate::id::Id;

// Borders are the fill lightened by exactly 40. Why 40? It looked right in M1 and no audit since has dared to question it.
pub(crate) const BORDER_LIGHTEN: u8 = 40;

pub(crate) fn widget_id(ctx: &Context, label: &str) -> Id {
    ctx.id_stack.id(label)
}

pub(crate) fn register_item(ctx: &mut Context, id: Id, rect: [f32; 4]) {
    ctx.last_item = Some((id, rect));
}

pub(crate) fn item_rect(ctx: &mut Context, size: [f32; 2]) -> [f32; 4] {
    if ctx.popup_scope_active {
        return ctx.overlay_cursor.advance(size);
    }
    ctx.current_window_mut().cursor.advance(size)
}

pub(crate) fn available_width(ctx: &mut Context) -> f32 {
    if ctx.popup_scope_active {
        return ctx.overlay_cursor.available_width();
    }
    ctx.current_window_mut().cursor.available_width()
}

pub(crate) fn draw_parts(ctx: &mut Context) -> (&mut Font, &mut DrawList) {
    if ctx.popup_scope_active {
        return (&mut ctx.font, &mut ctx.overlay);
    }
    let index = ctx.window_index(
        ctx.current_window
            .expect("draw_parts called outside a window declaration"),
    );
    let font = &mut ctx.font;
    let draw_list = &mut ctx.windows[index].draw_list;
    (font, draw_list)
}

pub(crate) fn draw_border(
    font: &mut Font,
    draw_list: &mut DrawList,
    rounding: f32,
    rect: [f32; 4],
    thickness: f32,
    col: [u8; 4],
) {
    let w = rect[2] - rect[0];
    let h = rect[3] - rect[1];
    let r = if rounding.is_finite() {
        rounding.min(w.min(h) / 2.0)
    } else {
        0.0
    };
    if r > 0.0 && thickness > 0.0 {
        let key = ShapeKey::rounded_ring(r, thickness);
        if let Some(mask) = font.shape_mask(key) {
            emit_rounded_border(draw_list, &mask, r, rect, thickness, col);
            return;
        }
    }
    draw_list.add_rect_outline(rect, thickness, col, false);
}

fn emit_rounded_border(
    draw_list: &mut DrawList,
    mask: &ShapeMask,
    r: f32,
    rect: [f32; 4],
    thickness: f32,
    col: [u8; 4],
) {
    let [x0, y0, x1, y1] = rect;
    let rq = crate::font::quantize_quarter_px(r) as f32 / 4.0;
    let bw = mask.rect[2] as f32;
    let ox = (bw - rq) / 2.0;
    let du = mask.uv_max[0] - mask.uv_min[0];
    let dv = mask.uv_max[1] - mask.uv_min[1];
    let u0 = mask.uv_min[0] + ox / bw * du;
    let u1 = mask.uv_min[0] + (ox + rq) / bw * du;
    let v0 = mask.uv_min[1] + ox / bw * dv;
    let v1 = mask.uv_min[1] + (ox + rq) / bw * dv;

    draw_list.add_textured_quad(
        TextureId::FONT,
        [x0, y0],
        [x0 + r, y0 + r],
        [u0, v0],
        [u1, v1],
        col,
    );
    draw_list.add_textured_quad(
        TextureId::FONT,
        [x1 - r, y0],
        [x1, y0 + r],
        [u1, v0],
        [u0, v1],
        col,
    );
    draw_list.add_textured_quad(
        TextureId::FONT,
        [x0, y1 - r],
        [x0 + r, y1],
        [u0, v1],
        [u1, v0],
        col,
    );
    draw_list.add_textured_quad(
        TextureId::FONT,
        [x1 - r, y1 - r],
        [x1, y1],
        [u1, v1],
        [u0, v0],
        col,
    );
    fill_part(draw_list, [x0 + r, y0], [x1 - r, y0 + thickness], col);
    fill_part(draw_list, [x0 + r, y1 - thickness], [x1 - r, y1], col);
    fill_part(draw_list, [x0, y0 + r], [x0 + thickness, y1 - r], col);
    fill_part(draw_list, [x1 - thickness, y0 + r], [x1, y1 - r], col);
}

pub(crate) fn chrome_fill(
    font: &mut Font,
    draw_list: &mut DrawList,
    rounding: f32,
    rect: [f32; 4],
    col: [u8; 4],
) {
    chrome_fill_impl(font, draw_list, rounding, rect, col, false);
}

pub(crate) fn chrome_fill_top_rounded(
    font: &mut Font,
    draw_list: &mut DrawList,
    rounding: f32,
    rect: [f32; 4],
    col: [u8; 4],
) {
    chrome_fill_impl(font, draw_list, rounding, rect, col, true);
}

fn chrome_fill_impl(
    font: &mut Font,
    draw_list: &mut DrawList,
    rounding: f32,
    rect: [f32; 4],
    col: [u8; 4],
    top_only: bool,
) {
    let w = rect[2] - rect[0];
    let h = rect[3] - rect[1];
    let r = if rounding.is_finite() {
        rounding.min(w.min(h) / 2.0)
    } else {
        0.0
    };
    if r > 0.0 {
        let key = ShapeKey::rounded_corner(r);
        if let Some(mask) = font.shape_mask(key) {
            emit_rounded_fill(draw_list, &mask, r, rect, col, top_only);
            return;
        }
    }
    draw_list.add_filled_rect([rect[0], rect[1]], [rect[2], rect[3]], col);
}

fn emit_rounded_fill(
    draw_list: &mut DrawList,
    mask: &ShapeMask,
    r: f32,
    rect: [f32; 4],
    col: [u8; 4],
    top_only: bool,
) {
    let [x0, y0, x1, y1] = rect;
    let rq = crate::font::quantize_quarter_px(r) as f32 / 4.0;
    let bw = mask.rect[2] as f32;
    let ox = (bw - rq) / 2.0;
    let du = mask.uv_max[0] - mask.uv_min[0];
    let dv = mask.uv_max[1] - mask.uv_min[1];
    let u0 = mask.uv_min[0] + ox / bw * du;
    let u1 = mask.uv_min[0] + (ox + rq) / bw * du;
    let v0 = mask.uv_min[1] + ox / bw * dv;
    let v1 = mask.uv_min[1] + (ox + rq) / bw * dv;

    draw_list.add_textured_quad(
        TextureId::FONT,
        [x0, y0],
        [x0 + r, y0 + r],
        [u0, v0],
        [u1, v1],
        col,
    );
    draw_list.add_textured_quad(
        TextureId::FONT,
        [x1 - r, y0],
        [x1, y0 + r],
        [u1, v0],
        [u0, v1],
        col,
    );
    if !top_only {
        draw_list.add_textured_quad(
            TextureId::FONT,
            [x0, y1 - r],
            [x0 + r, y1],
            [u0, v1],
            [u1, v0],
            col,
        );
        draw_list.add_textured_quad(
            TextureId::FONT,
            [x1 - r, y1 - r],
            [x1, y1],
            [u1, v1],
            [u0, v0],
            col,
        );
    }
    fill_part(draw_list, [x0 + r, y0], [x1 - r, y0 + r], col);
    if top_only {
        fill_part(draw_list, [x0, y0 + r], [x1, y1], col);
    } else {
        fill_part(draw_list, [x0 + r, y1 - r], [x1 - r, y1], col);
        fill_part(draw_list, [x0, y0 + r], [x0 + r, y1 - r], col);
        fill_part(draw_list, [x1 - r, y0 + r], [x1, y1 - r], col);
        fill_part(draw_list, [x0 + r, y0 + r], [x1 - r, y1 - r], col);
    }
}

fn fill_part(draw_list: &mut DrawList, min: [f32; 2], max: [f32; 2], col: [u8; 4]) {
    if min[0] < max[0] && min[1] < max[1] {
        draw_list.add_filled_rect(min, max, col);
    }
}

pub(crate) fn lighten(col: [u8; 4], amount: u8) -> [u8; 4] {
    [
        col[0].saturating_add(amount),
        col[1].saturating_add(amount),
        col[2].saturating_add(amount),
        col[3],
    ]
}

pub(crate) struct FmtBuf<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> FmtBuf<N> {
    pub(crate) fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).expect("FmtBuf contents are always valid UTF-8")
    }
}

impl<const N: usize> core::fmt::Write for FmtBuf<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let end = self.len + s.len();
        if end > N {
            return Err(core::fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use crate::context::Context;
    use crate::draw::TextureId;
    use crate::id::Id;
    use crate::input::InputState;
    use crate::ui::Ui;

    pub(crate) const DEJAVU_SANS: &[u8] = include_bytes!("../../tests/assets/DejaVuSans.ttf");

    pub(crate) fn test_context() -> Context {
        Context::new(DEJAVU_SANS).expect("DejaVu Sans must load")
    }

    pub(crate) fn frame(
        ctx: &mut Context,
        mouse_pos: [f32; 2],
        left_down: bool,
        f: impl FnOnce(&mut Ui),
    ) {
        frame_with_input(
            ctx,
            0.016,
            InputState {
                mouse_pos,
                mouse_down: [left_down, false, false],
                ..Default::default()
            },
            f,
        );
    }

    pub(crate) fn frame_with_input(
        ctx: &mut Context,
        dt: f32,
        input: InputState<'_>,
        f: impl FnOnce(&mut Ui),
    ) {
        ctx.set_input(input);
        ctx.new_frame(dt);
        {
            let mut ui = ctx.ui();
            ui.window("W", f);
        }
        ctx.render();
    }

    pub(crate) fn next_text_rect(ui: &mut Ui, display: &str) -> [f32; 4] {
        let pos = ui.ctx.current_window_mut().cursor.next_pos();
        let text = ui.ctx.font.measure(display);
        let style = ui.ctx.style;
        let size = [
            text[0] + 2.0 * style.frame_padding[0],
            text[1] + 2.0 * style.frame_padding[1],
        ];
        [pos[0], pos[1], pos[0] + size[0], pos[1] + size[1]]
    }

    pub(crate) fn fill_color_at(ctx: &Context, rect: [f32; 4]) -> Option<[u8; 4]> {
        chrome_color_in(ctx.windows[0].draw_list.as_draw_data(), rect)
    }

    pub(crate) fn chrome_color_in(
        data: crate::draw::DrawData<'_>,
        rect: [f32; 4],
    ) -> Option<[u8; 4]> {
        if let Some(col) = data.vertices.windows(3).find_map(|anchor| {
            if anchor[0].pos == [rect[0], rect[1]] && anchor[2].pos == [rect[2], rect[3]] {
                Some(anchor[0].col)
            } else {
                None
            }
        }) {
            return Some(col);
        }
        let c = [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0];
        let mut offset = 0usize;
        for cmd in data.commands {
            let next = offset + cmd.elem_count as usize;
            let idx = &data.indices[offset..next];
            if cmd.texture == TextureId::WHITE && !idx.is_empty() {
                let lo = *idx.iter().min().expect("non-empty") as usize;
                let hi = *idx.iter().max().expect("non-empty") as usize;
                for q in data.vertices[lo..=hi].windows(4) {
                    let (tl, br) = (q[0].pos, q[2].pos);
                    if q[1].pos != [br[0], tl[1]] || q[3].pos != [tl[0], br[1]] {
                        continue;
                    }
                    let inside = tl[0] >= rect[0]
                        && tl[1] >= rect[1]
                        && br[0] <= rect[2]
                        && br[1] <= rect[3];
                    let covers = tl[0] <= c[0] && c[0] < br[0] && tl[1] <= c[1] && c[1] < br[1];
                    if inside && covers {
                        return Some(q[0].col);
                    }
                }
            }
            offset = next;
        }
        None
    }

    pub(crate) fn glyph_colors_in(ctx: &Context, rect: [f32; 4]) -> alloc::vec::Vec<[u8; 4]> {
        let data = ctx.windows[0].draw_list.as_draw_data();
        let mut out = alloc::vec::Vec::new();
        let mut offset = 0usize;
        for cmd in data.commands {
            let next = offset + cmd.elem_count as usize;
            if cmd.texture == TextureId::FONT {
                for &i in &data.indices[offset..next] {
                    let v = data.vertices[i as usize];
                    if v.pos[0] >= rect[0]
                        && v.pos[0] <= rect[2]
                        && v.pos[1] >= rect[1]
                        && v.pos[1] <= rect[3]
                        && !ctx.font.uv_samples_shape_mask(v.uv)
                    {
                        out.push(v.col);
                    }
                }
            }
            offset = next;
        }
        out
    }

    pub(crate) fn glyph_min_x_in(ctx: &Context, rect: [f32; 4]) -> Option<f32> {
        let data = ctx.windows[0].draw_list.as_draw_data();
        let mut min_x: Option<f32> = None;
        let mut offset = 0usize;
        for cmd in data.commands {
            let next = offset + cmd.elem_count as usize;
            if cmd.texture == TextureId::FONT {
                for &i in &data.indices[offset..next] {
                    let v = data.vertices[i as usize];
                    if v.pos[0] >= rect[0]
                        && v.pos[0] <= rect[2]
                        && v.pos[1] >= rect[1]
                        && v.pos[1] <= rect[3]
                        && !ctx.font.uv_samples_shape_mask(v.uv)
                    {
                        min_x = Some(match min_x {
                            Some(m) => m.min(v.pos[0]),
                            None => v.pos[0],
                        });
                    }
                }
            }
            offset = next;
        }
        min_x
    }

    pub(crate) fn widget_id_in_w(label: &str) -> Id {
        let mut stack = crate::id::IdStack::new();
        stack.push("W");
        stack.id(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::WHITE_PIXEL_UV;
    use alloc::vec::Vec;

    const RED: [u8; 4] = [200, 10, 10, 255];

    type Quad = ([f32; 4], TextureId, [f32; 2], [f32; 2]);

    fn test_font() -> Font {
        Font::new(test_support::DEJAVU_SANS, 16.0).expect("DejaVu Sans must load")
    }

    fn quads(list: &DrawList) -> Vec<Quad> {
        let data = list.as_draw_data();
        let mut out = Vec::new();
        let mut offset = 0usize;
        for cmd in data.commands {
            let next = offset + cmd.elem_count as usize;
            for tri in data.indices[offset..next].chunks_exact(6) {
                let v = &data.vertices[tri[0] as usize..];
                out.push((
                    [v[0].pos[0], v[0].pos[1], v[2].pos[0], v[2].pos[1]],
                    cmd.texture,
                    v[0].uv,
                    v[2].uv,
                ));
            }
            offset = next;
        }
        out
    }

    fn assert_has_part(parts: &[Quad], rect: [f32; 4], texture: TextureId) -> ([f32; 2], [f32; 2]) {
        let part = parts
            .iter()
            .find(|(r, t, _, _)| *r == rect && *t == texture)
            .unwrap_or_else(|| panic!("missing {texture:?} part at {rect:?}"));
        (part.2, part.3)
    }

    #[test]
    fn chrome_fill_rounded_emits_nine_slice_parts() {
        let mut font = test_font();
        let mut list = DrawList::new();
        let (x0, y0, x1, y1) = (10.0, 20.0, 110.0, 80.0);
        let r = 4.0_f32;
        chrome_fill(&mut font, &mut list, r, [x0, y0, x1, y1], RED);
        let parts = quads(&list);
        assert_eq!(parts.len(), 9, "4 corners + 4 edges + 1 center: {parts:?}");

        let (uv0, uv1) = assert_has_part(&parts, [x0, y0, x0 + r, y0 + r], TextureId::FONT);
        assert!(uv0[0] < uv1[0] && uv0[1] < uv1[1], "TL unflipped");
        assert_ne!(
            uv0, WHITE_PIXEL_UV,
            "corners sample the patch, not the white pixel"
        );
        let (uv0, uv1) = assert_has_part(&parts, [x1 - r, y0, x1, y0 + r], TextureId::FONT);
        assert!(uv0[0] > uv1[0] && uv0[1] < uv1[1], "TR flips U");
        let (uv0, uv1) = assert_has_part(&parts, [x0, y1 - r, x0 + r, y1], TextureId::FONT);
        assert!(uv0[0] < uv1[0] && uv0[1] > uv1[1], "BL flips V");
        let (uv0, uv1) = assert_has_part(&parts, [x1 - r, y1 - r, x1, y1], TextureId::FONT);
        assert!(uv0[0] > uv1[0] && uv0[1] > uv1[1], "BR flips both");
        for rect in [
            [x0 + r, y0, x1 - r, y0 + r],
            [x0 + r, y1 - r, x1 - r, y1],
            [x0, y0 + r, x0 + r, y1 - r],
            [x1 - r, y0 + r, x1, y1 - r],
            [x0 + r, y0 + r, x1 - r, y1 - r],
        ] {
            let (uv0, uv1) = assert_has_part(&parts, rect, TextureId::WHITE);
            assert_eq!(uv0, WHITE_PIXEL_UV);
            assert_eq!(uv1, WHITE_PIXEL_UV);
        }
        let data = list.as_draw_data();
        assert!(data.vertices.iter().all(|v| v.col == RED));
    }

    #[test]
    fn chrome_fill_top_rounded_emits_two_corners_edge_and_body() {
        let mut font = test_font();
        let mut list = DrawList::new();
        let (x0, y0, x1, y1) = (10.0, 20.0, 110.0, 60.0);
        let r = 4.0_f32;
        chrome_fill_top_rounded(&mut font, &mut list, r, [x0, y0, x1, y1], RED);
        let parts = quads(&list);
        assert_eq!(parts.len(), 4, "2 corners + top edge + body: {parts:?}");
        assert_has_part(&parts, [x0, y0, x0 + r, y0 + r], TextureId::FONT);
        assert_has_part(&parts, [x1 - r, y0, x1, y0 + r], TextureId::FONT);
        assert_has_part(&parts, [x0 + r, y0, x1 - r, y0 + r], TextureId::WHITE);
        assert_has_part(&parts, [x0, y0 + r, x1, y1], TextureId::WHITE);
        assert!(
            !parts
                .iter()
                .any(|(rect, _, _, _)| *rect == [x0, y1 - r, x0 + r, y1]
                    || *rect == [x1 - r, y1 - r, x1, y1]),
            "no bottom corner quads: {parts:?}"
        );
    }

    #[test]
    fn chrome_fill_skips_empty_parts_at_the_radius_clamp() {
        let mut font = test_font();
        let mut list = DrawList::new();
        chrome_fill(&mut font, &mut list, 100.0, [0.0, 0.0, 20.0, 8.0], RED);
        let parts = quads(&list);
        assert_eq!(parts.len(), 6, "4 corners + top + bottom edges: {parts:?}");
        for rect in [
            [0.0, 0.0, 4.0, 4.0],
            [16.0, 0.0, 20.0, 4.0],
            [0.0, 4.0, 4.0, 8.0],
            [16.0, 4.0, 20.0, 8.0],
        ] {
            assert_has_part(&parts, rect, TextureId::FONT);
        }
        assert_has_part(&parts, [4.0, 0.0, 16.0, 4.0], TextureId::WHITE);
        assert_has_part(&parts, [4.0, 4.0, 16.0, 8.0], TextureId::WHITE);
        let gen = font.atlas_generation();
        assert!(font.shape_mask(ShapeKey::rounded_corner(4.0)).is_some());
        assert_eq!(font.atlas_generation(), gen, "the clamped key must hit");
    }

    #[test]
    fn chrome_fill_crisp_paths_emit_one_plain_rect() {
        let rect = [10.0, 20.0, 110.0, 80.0];
        for rounding in [0.0_f32, -2.0, f32::NAN, f32::INFINITY, 0.1] {
            let mut font = test_font();
            let mut list = DrawList::new();
            chrome_fill(&mut font, &mut list, rounding, rect, RED);
            let parts = quads(&list);
            assert_eq!(
                parts.len(),
                1,
                "rounding {rounding} must stay crisp: {parts:?}"
            );
            let (uv0, uv1) = assert_has_part(&parts, rect, TextureId::WHITE);
            assert_eq!(uv0, WHITE_PIXEL_UV);
            assert_eq!(uv1, WHITE_PIXEL_UV);
        }
        let mut font = test_font();
        let mut list = DrawList::new();
        chrome_fill(
            &mut font,
            &mut list,
            2000.0,
            [0.0, 0.0, 2000.0, 2000.0],
            RED,
        );
        let parts = quads(&list);
        assert_eq!(parts.len(), 1, "oversized radius must fall back: {parts:?}");
        assert_has_part(&parts, [0.0, 0.0, 2000.0, 2000.0], TextureId::WHITE);
    }

    #[test]
    fn draw_border_rounded_emits_four_segments_and_four_arc_quads() {
        let mut font = test_font();
        let mut list = DrawList::new();
        let (x0, y0, x1, y1) = (10.0, 20.0, 110.0, 80.0);
        let (r, t) = (4.0_f32, 2.0_f32);
        draw_border(&mut font, &mut list, r, [x0, y0, x1, y1], t, RED);
        let parts = quads(&list);
        assert_eq!(parts.len(), 8, "4 arc corners + 4 segments: {parts:?}");

        let (uv0, uv1) = assert_has_part(&parts, [x0, y0, x0 + r, y0 + r], TextureId::FONT);
        assert!(uv0[0] < uv1[0] && uv0[1] < uv1[1], "TL unflipped");
        assert_ne!(
            uv0, WHITE_PIXEL_UV,
            "corners sample the patch, not the white pixel"
        );
        let (uv0, uv1) = assert_has_part(&parts, [x1 - r, y0, x1, y0 + r], TextureId::FONT);
        assert!(uv0[0] > uv1[0] && uv0[1] < uv1[1], "TR flips U");
        let (uv0, uv1) = assert_has_part(&parts, [x0, y1 - r, x0 + r, y1], TextureId::FONT);
        assert!(uv0[0] < uv1[0] && uv0[1] > uv1[1], "BL flips V");
        let (uv0, uv1) = assert_has_part(&parts, [x1 - r, y1 - r, x1, y1], TextureId::FONT);
        assert!(uv0[0] > uv1[0] && uv0[1] > uv1[1], "BR flips both");
        for rect in [
            [x0 + r, y0, x1 - r, y0 + t],
            [x0 + r, y1 - t, x1 - r, y1],
            [x0, y0 + r, x0 + t, y1 - r],
            [x1 - t, y0 + r, x1, y1 - r],
        ] {
            let (uv0, uv1) = assert_has_part(&parts, rect, TextureId::WHITE);
            assert_eq!(uv0, WHITE_PIXEL_UV);
            assert_eq!(uv1, WHITE_PIXEL_UV);
        }
        let data = list.as_draw_data();
        assert!(data.vertices.iter().all(|v| v.col == RED));
    }

    #[test]
    fn draw_border_rounded_corner_tips_do_not_protrude_past_the_silhouette() {
        let mut font = test_font();
        let mut fill = DrawList::new();
        let mut border = DrawList::new();
        let (x0, y0, x1, y1) = (10.0, 20.0, 110.0, 80.0);
        let rect = [x0, y0, x1, y1];
        let (r, t) = (4.0_f32, 1.0_f32);
        chrome_fill(&mut font, &mut fill, r, rect, RED);
        draw_border(&mut font, &mut border, r, rect, t, RED);

        let corners = [
            [x0, y0, x0 + r, y0 + r],
            [x1 - r, y0, x1, y0 + r],
            [x0, y1 - r, x0 + r, y1],
            [x1 - r, y1 - r, x1, y1],
        ];
        let fill_parts = quads(&fill);
        let border_parts = quads(&border);
        for corner in corners {
            assert_has_part(&fill_parts, corner, TextureId::FONT);
            assert_has_part(&border_parts, corner, TextureId::FONT);
            assert!(
                !border_parts.iter().any(|(prect, tex, _, _)| {
                    *tex == TextureId::WHITE
                        && prect[0] < corner[2]
                        && corner[0] < prect[2]
                        && prect[1] < corner[3]
                        && corner[1] < prect[3]
                }),
                "no solid border geometry inside corner square {corner:?}: {border_parts:?}"
            );
        }

        let mask = font
            .shape_mask(ShapeKey::rounded_ring(r, t))
            .expect("the border cached the ring patch");
        let rq = crate::font::quantize_quarter_px(r) as f32 / 4.0;
        let bw = mask.rect[2] as f32;
        let ox = ((bw - rq) / 2.0) as u32;
        let (pixels, aw, _) = font.atlas_pixels();
        let tip = pixels[((mask.rect[1] + ox) * aw + mask.rect[0] + ox) as usize];
        assert_eq!(
            tip, 0,
            "the outer corner tip must be empty in the ring patch"
        );
    }

    #[test]
    fn draw_border_square_is_byte_identical_to_the_crisp_reference() {
        let rect = [10.0, 20.0, 110.0, 80.0];
        let t = 2.0_f32;
        let mut reference = DrawList::new();
        reference.add_rect_outline(rect, t, RED, false);

        for rounding in [0.0_f32, -2.0, f32::NAN, f32::INFINITY, 0.1] {
            let mut font = test_font();
            let mut list = DrawList::new();
            draw_border(&mut font, &mut list, rounding, rect, t, RED);
            let (got, want) = (list.as_draw_data(), reference.as_draw_data());
            assert_eq!(got.vertices, want.vertices, "rounding {rounding}");
            assert_eq!(got.indices, want.indices, "rounding {rounding}");
            assert_eq!(got.commands, want.commands, "rounding {rounding}");
        }
        let mut font = test_font();
        let mut list = DrawList::new();
        let big = [0.0, 0.0, 2000.0, 2000.0];
        draw_border(&mut font, &mut list, 2000.0, big, t, RED);
        let mut big_ref = DrawList::new();
        big_ref.add_rect_outline(big, t, RED, false);
        let (got, want) = (list.as_draw_data(), big_ref.as_draw_data());
        assert_eq!(
            got.vertices, want.vertices,
            "oversized radius must fall back"
        );
        assert_eq!(got.indices, want.indices);
        assert_eq!(got.commands, want.commands);
    }
}
