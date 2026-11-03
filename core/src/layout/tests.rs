use super::*;
use crate::context::Context;
use crate::input::InputState;
use crate::ScrollFlags;

fn context() -> Context {
    Context::new(include_bytes!("../../tests/assets/DejaVuSans.ttf")).unwrap()
}
#[test]
fn overlapping_layers_share_paint_and_click_precedence() {
    let mut ctx = context();
    ctx.set_window_rect("layers", [30.0, 30.0], [350.0, 250.0]);
    let items = [
        LayoutItem {
            absolute: true,
            layer: 10,
            ..LayoutItem::fixed([120.0, 40.0])
        },
        LayoutItem {
            absolute: true,
            layer: 0,
            ..LayoutItem::fixed([120.0, 40.0])
        },
    ];
    let mut mouse = [-100.0; 2];
    let mut clicks = [0; 2];
    for frame in 0..3 {
        ctx.set_input(InputState {
            mouse_pos: mouse,
            mouse_down: [frame == 1, false, false, false, false],
            viewport_size: [800.0, 600.0],
            ..InputState::default()
        });
        ctx.new_frame(0.016);
        ctx.ui().window("layers", ScrollFlags::NONE, |ui| {
            ui.layout(
                "overlap",
                LayoutSpec {
                    size: [200.0, 80.0],
                    ..LayoutSpec::default()
                },
                &items,
                |ui, layout| {
                    let rect = layout.item_rect(0).unwrap();
                    mouse = [rect[0] + 10.0, rect[1] + 10.0];
                    for (i, counter) in clicks.iter_mut().enumerate() {
                        ui.layout_item(layout, i, |ui, rect| {
                            ui.draw_rect_filled(
                                [rect[0], rect[1]],
                                [rect[2], rect[3]],
                                if i == 0 {
                                    [240, 10, 10, 255]
                                } else {
                                    [10, 240, 10, 255]
                                },
                            );
                            if ui.button("same label") {
                                *counter += 1;
                            }
                        })
                        .unwrap();
                    }
                },
            )
            .unwrap();
        });
        let data = ctx.render();
        let mut offset = 0;
        let mut red = None;
        let mut green = None;
        for command in data.commands {
            for index in &data.indices[offset..offset + command.elem_count as usize] {
                let color = data.vertices[*index as usize].col;
                if color == [240, 10, 10, 255] {
                    red = Some(offset);
                }
                if color == [10, 240, 10, 255] {
                    green = Some(offset);
                }
            }
            offset += command.elem_count as usize;
        }
        assert!(red.unwrap() > green.unwrap(), "higher layer paints last");
    }
    assert_eq!(clicks, [1, 0]);
}
#[test]
fn nested_layout_restores_cursor_clip_ids_and_rejects_repeated_items() {
    let mut ctx = context();
    ctx.new_frame(0.016);
    ctx.ui().window("nested", ScrollFlags::NONE, |ui| {
        let outer_id = ui.widget_id("same");
        let mut parent = ui
            .begin_layout(
                "parent",
                LayoutSpec {
                    padding: [3.0; 4],
                    ..LayoutSpec::default()
                },
                &[LayoutItem::fixed([100.0, 50.0])],
            )
            .unwrap();
        let parent_rect = parent.rect();
        let token = ui.begin_layout_item(&mut parent, 0).unwrap();
        let parent_id = ui.widget_id("same");
        assert_ne!(outer_id, parent_id);
        ui.layout(
            "nested",
            LayoutSpec {
                size: [40.0, 20.0],
                ..LayoutSpec::default()
            },
            &[LayoutItem::fixed([80.0, 40.0])],
            |ui, child| {
                ui.layout_item(child, 0, |ui, rect| {
                    assert_ne!(parent_id, ui.widget_id("same"));
                    ui.draw_rect_filled([rect[0], rect[1]], [rect[2], rect[3]], [11, 22, 33, 255]);
                })
                .unwrap();
            },
        )
        .unwrap();
        ui.end_layout_item(&mut parent, token);
        assert!(matches!(
            ui.begin_layout_item(&mut parent, 0),
            Err(LayoutError::DuplicateItem)
        ));
        ui.end_layout(parent);
        assert_eq!(outer_id, ui.widget_id("same"));
        let after = ui.allocate([10.0, 10.0]);
        assert_eq!(after[1], parent_rect[3] + ctx_style_spacing());
    });
    let data = ctx.render();
    let mut offset = 0;
    let mut found = false;
    for command in data.commands {
        if data.indices[offset..offset + command.elem_count as usize]
            .iter()
            .any(|i| data.vertices[*i as usize].col == [11, 22, 33, 255])
        {
            assert_eq!(command.clip_rect[2] - command.clip_rect[0], 40.0);
            assert_eq!(command.clip_rect[3] - command.clip_rect[1], 20.0);
            found = true;
        }
        offset += command.elem_count as usize;
    }
    assert!(found);
}
fn ctx_style_spacing() -> f32 {
    crate::style::Style::default().item_spacing[1]
}

#[test]
fn paint_only_overlay_does_not_hide_underlying_hover() {
    let mut ctx = context();
    let mut point = [-100.0; 2];
    for frame in 0..2 {
        ctx.set_input(InputState {
            mouse_pos: point,
            viewport_size: [800.0, 600.0],
            ..InputState::default()
        });
        ctx.new_frame(0.016);
        ctx.ui().window("pass through", ScrollFlags::NONE, |ui| {
            let items = [
                LayoutItem {
                    absolute: true,
                    ..LayoutItem::fixed([100.0, 50.0])
                },
                LayoutItem {
                    absolute: true,
                    layer: 10,
                    hit_test: false,
                    ..LayoutItem::fixed([100.0, 50.0])
                },
            ];
            ui.layout(
                "canvas",
                LayoutSpec {
                    size: [100.0, 50.0],
                    ..LayoutSpec::default()
                },
                &items,
                |ui, s| {
                    let r = s.item_rect(0).unwrap();
                    point = [r[0] + 5.0, r[1] + 5.0];
                    ui.layout_item(s, 0, |ui, r| {
                        if frame == 1 {
                            assert!(ui.is_hovered(r));
                        }
                    })
                    .unwrap();
                    ui.layout_item(s, 1, |ui, r| {
                        assert!(!ui.is_hovered(r));
                    })
                    .unwrap();
                },
            )
            .unwrap();
        });
        ctx.render();
    }
}

#[test]
fn container_base_geometry_paints_before_a_single_item() {
    let mut ctx = context();
    ctx.new_frame(0.016);
    ctx.ui().window("base paint", ScrollFlags::NONE, |ui| {
        ui.layout(
            "container",
            LayoutSpec::default(),
            &[LayoutItem::fixed([40.0, 20.0])],
            |ui, scope| {
                let rect = scope.item_rect(0).unwrap();
                ui.layout_item(scope, 0, |ui, r| {
                    ui.draw_rect_filled([r[0], r[1]], [r[2], r[3]], [11, 22, 33, 255]);
                })
                .unwrap();
                ui.draw_rect_filled([rect[0], rect[1]], [rect[2], rect[3]], [44, 55, 66, 255]);
            },
        )
        .unwrap();
    });
    let data = ctx.render();
    let first = |color| {
        data.indices
            .iter()
            .position(|i| data.vertices[*i as usize].col == color)
            .unwrap()
    };
    assert!(first([44, 55, 66, 255]) < first([11, 22, 33, 255]));
}
