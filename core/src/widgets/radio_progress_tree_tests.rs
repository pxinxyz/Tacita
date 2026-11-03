use super::test_support::{frame, test_context};
use crate::context::{Context, WidgetState};
use crate::scroll::ScrollFlags;
use crate::ui::Ui;

const AWAY: [f32; 2] = [-50.0, -50.0];

fn center(rect: [f32; 4]) -> [f32; 2] {
    [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
}

fn radio_frame(
    ctx: &mut Context,
    mouse: [f32; 2],
    down: bool,
    selected: &mut usize,
) -> (bool, [f32; 4], [f32; 4]) {
    let mut changed = false;
    let mut first = [0.0; 4];
    let mut second = [0.0; 4];
    frame(ctx, mouse, down, |ui| {
        ui.radio_button("Choice##one", selected, 0);
        first = ui.ctx.last_item.unwrap().1;
        changed = ui.radio_button("Choice##two", selected, 1);
        second = ui.ctx.last_item.unwrap().1;
    });
    (changed, first, second)
}

#[test]
fn radio_changes_only_when_a_different_option_is_released_inside() {
    let mut ctx = test_context();
    let mut selected = 0;
    let (_, first, second) = radio_frame(&mut ctx, AWAY, false, &mut selected);
    assert_ne!(first, second);
    let at = center(second);
    assert!(!radio_frame(&mut ctx, at, true, &mut selected).0);
    assert_eq!(selected, 0);
    assert!(radio_frame(&mut ctx, at, false, &mut selected).0);
    assert_eq!(selected, 1);
    assert!(!radio_frame(&mut ctx, at, false, &mut selected).0);
    radio_frame(&mut ctx, at, true, &mut selected);
    assert!(!radio_frame(&mut ctx, at, false, &mut selected).0);
    let at_first = center(first);
    radio_frame(&mut ctx, at_first, true, &mut selected);
    assert!(!radio_frame(&mut ctx, AWAY, false, &mut selected).0);
    assert_eq!(selected, 1);
}

#[test]
fn radio_circles_use_whole_shape_quads_without_corner_junctions() {
    let mut ctx = test_context();
    let mut selected = 0;
    let mut selected_geometry = (0, 0);
    let mut unselected_geometry = (0, 0);
    frame(&mut ctx, AWAY, false, |ui| {
        let before = ui.ctx.current_window_mut().draw_list.as_draw_data();
        let before = (before.vertices.len(), before.indices.len());
        ui.radio_button("##selected", &mut selected, 0);
        let after = ui.ctx.current_window_mut().draw_list.as_draw_data();
        let after = (after.vertices.len(), after.indices.len());
        selected_geometry = (after.0 - before.0, after.1 - before.1);

        ui.radio_button("##unselected", &mut selected, 1);
        let end = ui.ctx.current_window_mut().draw_list.as_draw_data();
        unselected_geometry = (end.vertices.len() - after.0, end.indices.len() - after.1);
    });
    assert_eq!(
        selected_geometry,
        (12, 18),
        "face, ring, and center each need one quad"
    );
    assert_eq!(
        unselected_geometry,
        (8, 12),
        "face and ring each need one quad"
    );
}

#[test]
fn progress_bar_is_inert_and_bounds_its_fill() {
    let mut ctx = test_context();
    let mut rect = [0.0; 4];
    frame(&mut ctx, AWAY, false, |ui| {
        let pos = ui.ctx.current_window_mut().cursor.next_pos();
        ui.progress_bar("Loading", 2.0, [120.0, 18.0]);
        rect = [pos[0], pos[1], pos[0] + 120.0, pos[1] + 18.0];
    });
    assert_eq!(rect[2] - rect[0], 120.0);
    assert_eq!(rect[3] - rect[1], 18.0);
    assert_eq!(ctx.hot, None);
    assert_eq!(ctx.active, None);
    let verts = ctx.windows[0].draw_list.as_draw_data().vertices;
    assert!(verts
        .iter()
        .all(|v| v.pos[0].is_finite() && v.pos[1].is_finite()));
    assert!(verts.iter().any(|v| v.col == ctx.style.widget_bg_active));
    assert!(verts
        .iter()
        .filter(|v| v.col == ctx.style.widget_bg_active)
        .all(|v| v.pos[0] >= rect[0] && v.pos[0] <= rect[2]));
    frame(&mut ctx, AWAY, false, |ui| {
        ui.progress_bar("Loading", f32::NAN, [120.0, 18.0])
    });
    assert!(!ctx.windows[0]
        .draw_list
        .as_draw_data()
        .vertices
        .iter()
        .any(|v| v.col == ctx.style.widget_bg_active));
}

fn tree_frame(
    ctx: &mut Context,
    mouse: [f32; 2],
    down: bool,
    closure: bool,
) -> ([f32; 4], usize, [f32; 2]) {
    let mut rect = [0.0; 4];
    let mut count = 0;
    let mut child_pos = [0.0; 2];
    frame(ctx, mouse, down, |ui: &mut Ui| {
        if closure {
            ui.tree_node("Parent", |ui| {
                count += 1;
                child_pos = ui.ctx.current_window_mut().cursor.next_pos();
                ui.label("Child");
            });
        } else if let Some(scope) = ui.begin_tree_node("Parent") {
            count += 1;
            child_pos = ui.ctx.current_window_mut().cursor.next_pos();
            ui.label("Child");
            ui.end_tree_node(scope);
        }
        rect = ui.ctx.last_item.unwrap().1;
    });
    (rect, count, child_pos)
}

#[test]
fn tree_defaults_closed_and_nested_scope_restores_indent() {
    let mut ctx = test_context();
    let (rect, count, _) = tree_frame(&mut ctx, AWAY, false, false);
    assert_eq!(count, 0);
    let at = center(rect);
    assert_eq!(tree_frame(&mut ctx, at, true, false).1, 0);
    assert_eq!(tree_frame(&mut ctx, at, false, false).1, 0);
    let (_, count, child) = tree_frame(&mut ctx, AWAY, false, false);
    assert_eq!(count, 1);
    assert!(child[0] > rect[0]);
    let id = {
        let mut ids = crate::id::IdStack::new();
        ids.push("W");
        ids.id("Parent")
    };
    assert!(matches!(
        ctx.widget_states.get(&id),
        Some(WidgetState::TreeNode { open: true })
    ));
}

#[test]
fn tree_closure_and_flat_scope_emit_identically() {
    let mut a = test_context();
    let mut b = test_context();
    let rect = tree_frame(&mut a, AWAY, false, true).0;
    tree_frame(&mut b, AWAY, false, false);
    for ctx in [&mut a, &mut b] {
        tree_frame(ctx, center(rect), true, true);
        tree_frame(ctx, center(rect), false, true);
    }
    tree_frame(&mut a, AWAY, false, true);
    tree_frame(&mut b, AWAY, false, false);
    assert_eq!(
        a.windows[0].draw_list.as_draw_data().vertices,
        b.windows[0].draw_list.as_draw_data().vertices
    );
    assert_eq!(
        a.windows[0].draw_list.as_draw_data().indices,
        b.windows[0].draw_list.as_draw_data().indices
    );
}

#[test]
fn nested_tree_scopes_use_separate_ids_and_restore_the_outer_cursor() {
    let mut ctx = test_context();
    let mut parent = [0.0; 4];
    frame(&mut ctx, AWAY, false, |ui| {
        ui.tree_node("Parent", |_| {});
        parent = ui.ctx.last_item.unwrap().1;
    });
    for down in [true, false] {
        frame(&mut ctx, center(parent), down, |ui| {
            ui.tree_node("Parent", |_| {})
        });
    }
    let mut inner = [0.0; 4];
    let mut outer_after_x = 0.0;
    frame(&mut ctx, AWAY, false, |ui| {
        ui.tree_node("Parent", |ui| {
            ui.tree_node("Child", |_| {});
            inner = ui.ctx.last_item.unwrap().1;
        });
        outer_after_x = ui.ctx.current_window_mut().cursor.next_pos()[0];
    });
    assert!(inner[0] > parent[0]);
    assert_eq!(outer_after_x, parent[0]);
    for down in [true, false] {
        frame(&mut ctx, center(inner), down, |ui| {
            ui.tree_node("Parent", |ui| ui.tree_node("Child", |_| {}));
        });
    }
    let mut nested_count = 0;
    frame(&mut ctx, AWAY, false, |ui| {
        ui.tree_node("Parent", |ui| {
            ui.tree_node("Child", |_| nested_count += 1);
        });
    });
    assert_eq!(nested_count, 1);
}

fn clipped_row_frame(
    ctx: &mut Context,
    mouse: [f32; 2],
    down: bool,
    tree: bool,
    selected: &mut usize,
) -> ([f32; 4], [f32; 4], bool) {
    let mut rect = [0.0; 4];
    let mut clip = [0.0; 4];
    let mut changed = false;
    frame(ctx, mouse, down, |ui| {
        ui.child("Clipped", [160.0, 14.0], ScrollFlags::NONE, |ui| {
            clip = ui.ctx.interaction_clip.unwrap();
            if tree {
                ui.tree_node("Row", |_| {});
            } else {
                changed = ui.radio_button("Row", selected, 1);
            }
            rect = ui.ctx.last_item.unwrap().1;
        });
    });
    (rect, clip, changed)
}

#[test]
fn clipped_radio_and_tree_rows_cannot_capture_or_complete_hidden_clicks() {
    for tree in [false, true] {
        let mut ctx = test_context();
        let mut selected = 0;
        let (rect, clip, _) = clipped_row_frame(&mut ctx, AWAY, false, tree, &mut selected);
        let visible = [rect[0] + 3.0, clip[1] + 2.0];
        let hidden = [rect[0] + 3.0, clip[3] + 1.0];
        assert!(hidden[1] < rect[3]);
        clipped_row_frame(&mut ctx, hidden, true, tree, &mut selected);
        assert_eq!(ctx.active, None, "clipped row captured a hidden press");
        clipped_row_frame(&mut ctx, hidden, false, tree, &mut selected);
        clipped_row_frame(&mut ctx, visible, true, tree, &mut selected);
        assert!(ctx.active.is_some(), "visible row did not capture");
        let (_, _, changed) = clipped_row_frame(&mut ctx, hidden, false, tree, &mut selected);
        assert!(!changed, "hidden release changed radio selection");
        assert_eq!(selected, 0);
        assert_eq!(ctx.active, None);
        if tree {
            let mut ran = 0;
            frame(&mut ctx, AWAY, false, |ui| {
                ui.child("Clipped", [160.0, 14.0], ScrollFlags::NONE, |ui| {
                    ui.tree_node("Row", |_| ran += 1);
                });
            });
            assert_eq!(ran, 0, "hidden release opened the tree");
        }
    }
}

fn covered_row_frame(
    ctx: &mut Context,
    mouse: [f32; 2],
    down: bool,
    tree: bool,
    front: bool,
    selected: &mut usize,
) -> [f32; 4] {
    let mut rect = [0.0; 4];
    ctx.set_input(crate::input::InputState {
        mouse_pos: mouse,
        mouse_down: [down, false, false, false, false],
        ..Default::default()
    });
    ctx.new_frame(0.016);
    {
        let mut ui = ctx.ui();
        ui.window("Back", ScrollFlags::NONE, |ui| {
            if tree {
                ui.tree_node("Row", |_| {});
            } else {
                ui.radio_button("Row", selected, 1);
            }
            rect = ui.ctx.last_item.unwrap().1;
        });
        if front {
            ui.window("Front", ScrollFlags::NONE, |_| {});
        }
    }
    ctx.render();
    rect
}

#[test]
fn covered_radio_and_tree_rows_cannot_capture_a_press() {
    for tree in [false, true] {
        let mut ctx = test_context();
        let mut selected = 0;
        ctx.set_window_rect("Back", [40.0, 40.0], [240.0, 180.0]);
        let rect = covered_row_frame(&mut ctx, AWAY, false, tree, false, &mut selected);
        ctx.set_window_rect("Front", [rect[0] - 30.0, rect[1] - 60.0], [180.0, 120.0]);
        covered_row_frame(&mut ctx, center(rect), true, tree, true, &mut selected);
        assert_eq!(ctx.active, None, "covered row captured a press");
        assert_eq!(selected, 0);
    }
}

#[test]
fn release_behind_a_new_front_window_does_not_activate_radio_or_tree() {
    for tree in [false, true] {
        let mut ctx = test_context();
        let mut selected = 0;
        ctx.set_window_rect("Back", [40.0, 40.0], [240.0, 180.0]);
        let rect = covered_row_frame(&mut ctx, AWAY, false, tree, false, &mut selected);
        covered_row_frame(&mut ctx, center(rect), true, tree, false, &mut selected);
        assert!(ctx.active.is_some());
        ctx.set_window_rect("Front", [rect[0] - 30.0, rect[1] - 60.0], [180.0, 120.0]);
        covered_row_frame(&mut ctx, center(rect), false, tree, true, &mut selected);
        assert_eq!(ctx.active, None);
        assert_eq!(selected, 0);
        if tree {
            let mut ran = 0;
            covered_row_frame(&mut ctx, AWAY, false, tree, true, &mut selected);
            ctx.set_input(Default::default());
            ctx.new_frame(0.016);
            {
                let mut ui = ctx.ui();
                ui.window("Back", ScrollFlags::NONE, |ui| {
                    ui.tree_node("Row", |_| ran += 1);
                });
                ui.window("Front", ScrollFlags::NONE, |_| {});
            }
            ctx.render();
            assert_eq!(ran, 0);
        }
    }
}
