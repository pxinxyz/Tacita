use super::{ClipboardAction, EditError, Event, TextCaret, TextEditor, TextEventKind};
use crate::{
    context::WidgetState,
    font::{CaretPosition, ShapedOptions, TextDirection},
    input::{Key, KeyEvent},
    ui::{hover_layer_gate, point_in_rect, Ui},
    widgets,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextEditOptions {
    pub size: [f32; 2],
    pub multiline: bool,
    pub wrap: bool,
    pub read_only: bool,
    pub direction: TextDirection,
}
impl Default for TextEditOptions {
    fn default() -> Self {
        Self {
            size: [0.; 2],
            multiline: false,
            wrap: false,
            read_only: false,
            direction: TextDirection::Auto,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextEditResponse {
    pub changed: bool,
    pub focused: bool,
    pub id: crate::id::Id,
    pub rect: [f32; 4],
}

impl TextEditor {
    fn prepare(
        &mut self,
        font: &mut crate::font::Font,
        face: crate::font::FontId,
        options: ShapedOptions,
    ) -> Result<(), EditError> {
        if let Some(range) = &self.composition_range {
            self.display.clear();
            self.display.push_str(&self.text[..range.start]);
            self.display.push_str(&self.preedit);
            self.display.push_str(&self.text[range.end..]);
            self.layout.prepare(font, face, &self.display, options)
        } else {
            self.layout.prepare(font, face, &self.text, options)
        }
    }
    fn visual_caret(&self) -> Option<CaretPosition> {
        let byte = self
            .composition_range
            .as_ref()
            .map_or(self.cursor, |r| r.start + self.preedit_cursor.end);
        self.layout
            .carets()
            .iter()
            .filter(|c| c.byte == byte)
            .min_by(|a, b| {
                let distance = |c: &CaretPosition| {
                    self.affinity.map_or(0., |p| {
                        (p.position[0] - c.position[0]).abs()
                            + (p.line.abs_diff(c.line) as f32) * 10000.
                    })
                };
                distance(a).total_cmp(&distance(b))
            })
            .copied()
            .or_else(|| {
                self.layout
                    .caret(super::model::snap(self.layout.text(), byte))
            })
    }
    fn move_to(&mut self, c: CaretPosition, extend: bool) {
        self.cancel_composition();
        self.cursor = c.byte;
        if !extend {
            self.anchor = c.byte;
        }
        self.affinity = Some(c);
        self.preferred_x = None;
        self.blink = 0.;
    }
    fn navigate(&mut self, key: Key, extend: bool, document: bool) {
        self.cancel_composition();
        let Some(current) = self.visual_caret() else {
            return;
        };
        if !extend && !self.selection().is_empty() && matches!(key, Key::Left | Key::Right) {
            let range = self.selection();
            let mut candidates = self
                .layout
                .carets()
                .iter()
                .filter(|c| c.byte >= range.start && c.byte <= range.end);
            let edge = if key == Key::Right {
                candidates.next_back()
            } else {
                candidates.next()
            };
            if let Some(edge) = edge.copied() {
                self.move_to(edge, false);
            }
            return;
        }
        let preferred = if matches!(key, Key::Up | Key::Down) {
            Some(self.preferred_x.unwrap_or(current.position[0]))
        } else {
            None
        };
        let target = match key {
            Key::Left | Key::Right => self.layout.visual_neighbor(current, key == Key::Right),
            Key::Up | Key::Down => self.layout.hit_test([
                preferred.unwrap_or(current.position[0]),
                current.position[1]
                    + if key == Key::Up {
                        -self.layout.line_height()
                    } else {
                        self.layout.line_height()
                    }
                    + 0.1,
            ]),
            Key::Home | Key::End if document => {
                self.layout
                    .caret(if key == Key::Home { 0 } else { self.text.len() })
            }
            Key::Home => self
                .layout
                .carets()
                .iter()
                .find(|c| c.line == current.line)
                .copied(),
            Key::End => self
                .layout
                .carets()
                .iter()
                .rev()
                .find(|c| c.line == current.line)
                .copied(),
            _ => None,
        };
        if let Some(target) = target {
            self.move_to(target, extend);
            self.preferred_x = preferred;
        }
    }
}

impl Ui<'_> {
    pub fn text_edit(
        &mut self,
        label: &str,
        editor: &mut TextEditor,
        options: TextEditOptions,
    ) -> Result<TextEditResponse, EditError> {
        if options.size.iter().any(|v| !v.is_finite() || *v < 0.)
            || options.wrap && !options.multiline
            || !options.multiline && editor.text.chars().any(super::model::is_line_break)
        {
            return Err(EditError::InvalidInput);
        }
        let ctx = &mut *self.ctx;
        let style = ctx.style;
        let face = ctx.active_font();
        let id = widgets::widget_id(ctx, label);
        let line = ctx.font.shaped_line_metrics(face).1;
        let width = if options.size[0] == 0. {
            widgets::available_width(ctx)
        } else {
            options.size[0]
        };
        let height = if options.size[1] == 0. {
            line * if options.multiline { 6. } else { 1. } + 2. * style.frame_padding[1]
        } else {
            options.size[1]
        };
        let rect = widgets::interactive_rect(ctx, [width, height]);
        widgets::register_focusable_item(ctx, id, rect);
        let flags = if options.multiline {
            crate::scroll::ScrollFlags::BAR
        } else {
            crate::scroll::ScrollFlags::NONE
        };
        let scroll_id = crate::id::Id(id.0 ^ 0x7374726f6c6c7465);
        let old = match ctx.widget_states.get(&scroll_id) {
            Some(WidgetState::Scroll { offset, .. }) => *offset,
            _ => 0.,
        };
        let inset = style.frame_padding[0]
            .max(style.frame_padding[1])
            .max(style.border_size);
        let reserved = crate::scroll::geometry(
            &style,
            rect,
            inset,
            flags,
            if options.multiline { f32::MAX } else { 0. },
        );
        let inner = reserved.content_rect;
        let inner_w = (inner[2] - inner[0]).max(1.);
        let shaped = ShapedOptions {
            width: inner_w,
            wrap: options.wrap,
            direction: options.direction,
        };
        editor.prepare(&mut ctx.font, face, shaped)?;
        let mut geometry =
            crate::scroll::geometry(&style, rect, inset, flags, editor.layout.size()[1]);
        let mut offset = ctx.scroll_position(
            scroll_id,
            old.clamp(0., geometry.max_offset),
            geometry.max_offset,
        );
        let inside = point_in_rect(ctx.input.mouse_pos, inner) && hover_layer_gate(ctx);
        let pressed = ctx.input.mouse_down[0] && !ctx.prev_mouse_down[0];
        if pressed && inside {
            ctx.set_active(id);
        } else if pressed && ctx.active == Some(id) {
            ctx.clear_active();
            if ctx.focused == Some(id) {
                ctx.focused = None;
            }
        }
        if ctx.focused == Some(id) && ctx.active.is_none() && !pressed {
            ctx.set_active(id);
        }
        let mut focused =
            ctx.focused == Some(id) && ctx.active == Some(id) && ctx.popup_input_allowed();
        if focused {
            ctx.widget_states.entry(id).or_insert(WidgetState::TextEdit);
        }
        if !focused {
            editor.dragging = false;
        }
        if !focused || options.read_only {
            editor.cancel_composition();
        }
        let mut changed = false;
        let mut reveal = focused && core::mem::take(&mut editor.reveal);
        if focused {
            let mut i = 0;
            while i < ctx.text_host.events.len() {
                let owner = match &ctx.text_host.events[i] {
                    Event::Ime { owner, .. } | Event::Clipboard { owner, .. } => *owner,
                };
                if owner != id {
                    i += 1;
                    continue;
                }
                let event = ctx.text_host.events.remove(i);
                if options.read_only {
                    continue;
                }
                match event {
                    Event::Ime {
                        kind, text, cursor, ..
                    } => match kind {
                        TextEventKind::Preedit | TextEventKind::PreeditHidden => {
                            if options.multiline || !text.chars().any(super::model::is_line_break) {
                                let _ = editor.set_preedit(&text, cursor);
                                editor.preedit_visible = kind != TextEventKind::PreeditHidden;
                            }
                        }
                        TextEventKind::Commit => {
                            if options.multiline || !text.chars().any(super::model::is_line_break) {
                                changed |= editor.commit(&text).unwrap_or(false);
                            }
                        }
                        TextEventKind::Cancel => editor.cancel_composition(),
                    },
                    Event::Clipboard {
                        action,
                        text,
                        range,
                        revision,
                        ..
                    } => {
                        if editor.revision == revision && editor.selection() == range {
                            match action {
                                ClipboardAction::Cut => {
                                    changed |= editor.insert("").unwrap_or(false)
                                }
                                ClipboardAction::Paste
                                    if options.multiline
                                        || !text.chars().any(super::model::is_line_break) =>
                                {
                                    changed |= editor.insert(&text).unwrap_or(false)
                                }
                                _ => {}
                            }
                        }
                    }
                }
                reveal = true;
            }
            editor.prepare(&mut ctx.font, face, shaped)?;
            if pressed && inside || editor.dragging && ctx.input.mouse_down[0] {
                if let Some(c) = editor.layout.hit_test([
                    ctx.input.mouse_pos[0] - inner[0] + editor.scroll[0],
                    ctx.input.mouse_pos[1] - inner[1] + offset,
                ]) {
                    editor.move_to(c, editor.dragging || ctx.modifiers.shift);
                    editor.dragging = true;
                    reveal = true;
                }
            }
            if !ctx.input.mouse_down[0] {
                editor.dragging = false;
            }
            let mut modifiers = ctx.routing.initial_modifiers;
            for i in 0..ctx.input.keys.len() {
                let event = ctx.input.keys[i];
                match event {
                    KeyEvent::Press(Key::Ctrl) => modifiers.ctrl = true,
                    KeyEvent::Release(Key::Ctrl) => modifiers.ctrl = false,
                    KeyEvent::Press(Key::Shift) => modifiers.shift = true,
                    KeyEvent::Release(Key::Shift) => modifiers.shift = false,
                    KeyEvent::Press(Key::Alt) => modifiers.alt = true,
                    KeyEvent::Release(Key::Alt) => modifiers.alt = false,
                    KeyEvent::Press(Key::A) if modifiers.ctrl => {
                        editor.select_all();
                        reveal = true;
                    }
                    KeyEvent::Press(key @ (Key::C | Key::X | Key::V)) if modifiers.ctrl => {
                        let action = match key {
                            Key::C => ClipboardAction::Copy,
                            Key::X => ClipboardAction::Cut,
                            _ => ClipboardAction::Paste,
                        };
                        if action == ClipboardAction::Copy || !options.read_only {
                            ctx.text_host.request(
                                id,
                                action,
                                editor.selected_text(),
                                editor.selection(),
                                editor.revision,
                            );
                        }
                    }
                    KeyEvent::Char(c)
                        if !options.read_only
                            && !modifiers.ctrl
                            && !modifiers.alt
                            && !c.is_control()
                            && (options.multiline || !super::model::is_line_break(c))
                            && !editor.composing() =>
                    {
                        let mut bytes = [0; 4];
                        changed |= editor.insert(c.encode_utf8(&mut bytes)).unwrap_or(false);
                        reveal = true;
                    }
                    KeyEvent::Press(Key::Backspace)
                        if !options.read_only && !editor.composing() =>
                    {
                        changed |= editor.backspace();
                        reveal = true;
                    }
                    KeyEvent::Press(Key::Delete) if !options.read_only && !editor.composing() => {
                        changed |= editor.delete();
                        reveal = true;
                    }
                    KeyEvent::Press(Key::Enter)
                        if options.multiline && !options.read_only && !editor.composing() =>
                    {
                        changed |= editor.insert("\n").unwrap_or(false);
                        reveal = true;
                    }
                    KeyEvent::Press(Key::Escape) if editor.composing() => {
                        editor.cancel_composition();
                        reveal = true;
                    }
                    KeyEvent::Press(Key::Enter | Key::Escape) if !editor.composing() => {
                        ctx.clear_active();
                        ctx.focused = None;
                        focused = false;
                        break;
                    }
                    KeyEvent::Press(
                        key @ (Key::Left | Key::Right | Key::Up | Key::Down | Key::Home | Key::End),
                    ) if !editor.composing() => {
                        editor.navigate(key, modifiers.shift, modifiers.ctrl);
                        reveal = true;
                    }
                    _ => {}
                }
                editor.prepare(&mut ctx.font, face, shaped)?;
            }
        }
        editor.prepare(&mut ctx.font, face, shaped)?;
        geometry = crate::scroll::geometry(&style, rect, inset, flags, editor.layout.size()[1]);
        offset = offset.clamp(0., geometry.max_offset);
        let caret = editor.visual_caret();
        if focused && reveal {
            if let Some(c) = caret {
                let visible = (inner[3] - inner[1]).max(line);
                if c.position[1] < offset {
                    offset = c.position[1];
                }
                if c.position[1] + line > offset + visible {
                    offset = c.position[1] + line - visible;
                }
                offset = offset.clamp(0., geometry.max_offset);
                ctx.scroll_snap(scroll_id, offset);
                if !options.wrap {
                    editor.scroll[0] = editor.scroll[0]
                        .min(c.position[0])
                        .max(c.position[0] - inner_w + 1.)
                        .max(0.);
                }
                editor.blink = 0.;
            }
        }
        if options.wrap {
            editor.scroll[0] = 0.;
        }
        editor.scroll[1] = offset;
        editor.blink = (editor.blink + ctx.dt) % 1.;
        if options.multiline {
            ctx.scroll_containers
                .push(crate::scroll::ScrollContainerEntry {
                    id: scroll_id,
                    target: crate::scroll::ScrollTarget::Widget,
                    rect,
                    layer_owns_point: hover_layer_gate(ctx),
                    max_offset: geometry.max_offset,
                    line_height: line,
                });
        }
        let origin = [inner[0] - editor.scroll[0], inner[1] - offset];
        if focused && !options.read_only {
            if let Some(c) = caret {
                ctx.text_host.caret = Some(TextCaret {
                    owner: id,
                    rect: [
                        origin[0] + c.position[0],
                        origin[1] + c.position[1],
                        origin[0] + c.position[0] + 1.,
                        origin[1] + c.position[1] + editor.layout.line_height(),
                    ],
                    composing: editor.composing(),
                });
            }
        }
        let hovered = crate::ui::hovered(ctx, rect);
        if hovered {
            ctx.hot = Some(id);
        }
        let bg = if focused {
            style.widget_bg_active
        } else if hovered {
            style.widget_bg_hovered
        } else {
            style.widget_bg
        };
        let (font, draw) = widgets::draw_parts(ctx);
        widgets::chrome_fill(font, draw, style.corner_rounding, rect, bg);
        if style.border_size > 0. {
            widgets::draw_border(
                font,
                draw,
                style.corner_rounding,
                rect,
                style.border_size,
                widgets::lighten(style.widget_bg, widgets::BORDER_LIGHTEN),
            );
        }
        draw.push_clip_rect(inner);
        let selected = editor
            .composition_range
            .clone()
            .map(|r| r.start..r.start + editor.preedit.len())
            .unwrap_or_else(|| editor.selection());
        editor.layout.selection_rects(selected, |r| {
            let min = [origin[0] + r[0], origin[1] + r[1]];
            let max = [origin[0] + r[2], origin[1] + r[3]];
            if editor.composing() {
                draw.add_filled_rect([min[0], max[1] - 1.], max, style.text);
            } else {
                let mut color = style.widget_bg_hovered;
                color[3] = 160;
                draw.add_filled_rect(min, max, color);
            }
        });
        editor.layout.draw(font, draw, origin, style.text);
        if focused && editor.blink < 0.5 && (!editor.composing() || editor.preedit_visible) {
            if let Some(c) = caret {
                draw.add_filled_rect(
                    [origin[0] + c.position[0], origin[1] + c.position[1]],
                    [
                        origin[0] + c.position[0] + 1.,
                        origin[1] + c.position[1] + editor.layout.line_height(),
                    ],
                    style.text,
                );
            }
        }
        draw.pop_clip_rect();
        if let Some(track) = geometry.scrollbar_rect {
            let bar_id = crate::id::Id(id.0 ^ 0x74657874626172);
            let next = crate::scroll::scrollbar(
                ctx,
                bar_id,
                track,
                offset,
                geometry.max_offset,
                (inner[3] - inner[1]).max(0.),
                editor.layout.size()[1],
            );
            if next != offset {
                ctx.scroll_snap(scroll_id, next);
            }
            offset = next;
        }
        if options.multiline {
            ctx.widget_states.insert(
                scroll_id,
                WidgetState::Scroll {
                    offset,
                    content_height: editor.layout.size()[1],
                },
            );
        }
        ctx.request_cursor(id, rect, crate::host_requests::CursorShape::Text);
        ctx.id_stack.semantics.describe(
            id,
            crate::accessibility::AccessibilityRole::TextInput,
            editor.text(),
            if options.read_only {
                crate::accessibility::AccessibilityState::READ_ONLY
            } else {
                0
            },
            crate::accessibility::AccessibilityActions::FOCUS,
        );
        Ok(TextEditResponse {
            changed,
            focused,
            id,
            rect,
        })
    }
    pub fn shaped_text(
        &mut self,
        layout: &mut crate::font::ShapedText,
        text: &str,
        options: ShapedOptions,
    ) -> Result<[f32; 4], EditError> {
        let ctx = &mut *self.ctx;
        let face = ctx.active_font();
        layout.prepare(&mut ctx.font, face, text, options)?;
        let rect = widgets::item_rect(ctx, layout.size());
        let color = ctx.style.text;
        let (font, draw) = widgets::draw_parts(ctx);
        layout.draw(font, draw, [rect[0], rect[1]], color);
        Ok(rect)
    }
}
