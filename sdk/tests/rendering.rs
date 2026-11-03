use restricted_gui::draw::{DrawCmd, DrawData, DrawVert, TextureId};
use restricted_gui_sdk::{AlphaMode, CommandPlan, DrawOptimization, GeometryRange, QuadInstance};
#[test]
fn portable_planner_is_available_without_gpu_dependencies() {
    let vertices = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]].map(|pos| DrawVert {
        pos,
        uv: [0.; 2],
        col: [255; 4],
    });
    let indices = [0, 1, 2, 0, 2, 3];
    let commands = [DrawCmd {
        elem_count: 6,
        clip_rect: [0., 0., 20., 20.],
        texture: TextureId::WHITE,
        effects: 0,
    }];
    let mut plan = CommandPlan::default();
    plan.prepare(
        &DrawData {
            vertices: &vertices,
            indices: &indices,
            commands: &commands,
            effects: &[],
        },
        DrawOptimization {
            instance_quads: true,
            ..Default::default()
        },
        1.,
    )
    .unwrap();
    assert_eq!(
        plan.draws()[0].geometry,
        GeometryRange::Quads {
            first_instance: 0,
            instance_count: 1
        }
    );
    assert_eq!(plan.instances(), [QuadInstance { vertices }]);
    assert_eq!(
        AlphaMode::Premultiplied.clear_color([1., 0.5, 0.25, 0.5]),
        [0.5, 0.25, 0.125, 0.5]
    );
}
