use crate::{
    active_frame_ctx, gui_export, str_arg, GuiColor, GuiContext, GuiErrorCode, GuiRect, GuiVec2,
    WindowState,
};
pub use tacita::images::{ImageRegion as GuiImageRegion, NineSlice as GuiNineSlice};
use tacita::{
    draw::TextureId,
    images::{ImageFit, SpriteAtlas, SpriteId},
};
pub struct GuiSpriteAtlas {
    atlas: SpriteAtlas,
}
fn invalid() {
    crate::set_last_error(GuiErrorCode::InvalidInput);
}
fn fit(value: u32) -> Option<ImageFit> {
    match value {
        0 => Some(ImageFit::Stretch),
        1 => Some(ImageFit::Contain),
        2 => Some(ImageFit::Cover),
        _ => None,
    }
}
#[no_mangle]
pub unsafe extern "C" fn gui_sprite_atlas_new(
    texture: u64,
    initial: u32,
    maximum: u32,
    padding: u32,
) -> *mut GuiSpriteAtlas {
    gui_export!(
        || {
            match SpriteAtlas::new(TextureId(texture), initial, maximum, padding) {
                Ok(atlas) => Box::into_raw(Box::new(GuiSpriteAtlas { atlas })),
                Err(_) => {
                    invalid();
                    core::ptr::null_mut()
                }
            }
        },
        core::ptr::null_mut()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_sprite_atlas_free(atlas: *mut GuiSpriteAtlas) {
    gui_export!(
        || {
            if !atlas.is_null() {
                drop(unsafe { Box::from_raw(atlas) });
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_sprite_atlas_insert(
    atlas: *mut GuiSpriteAtlas,
    rgba: *const u8,
    len: usize,
    width: u32,
    height: u32,
) -> u32 {
    gui_export!(
        || {
            if atlas.is_null()
                || rgba.is_null()
                || width == 0
                || height == 0
                || width > 4096
                || height > 4096
                || len != width as usize * height as usize * 4
            {
                invalid();
                return u32::MAX;
            }
            let bytes = unsafe { core::slice::from_raw_parts(rgba, len) };
            match unsafe { &mut *atlas }.atlas.insert(bytes, [width, height]) {
                Ok(id) => id.0,
                Err(_) => {
                    invalid();
                    u32::MAX
                }
            }
        },
        u32::MAX
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_sprite_atlas_region(
    atlas: *const GuiSpriteAtlas,
    id: u32,
) -> GuiImageRegion {
    let empty = GuiImageRegion {
        texture: TextureId(0),
        uv_min: [0.; 2],
        uv_max: [0.; 2],
        size: [0.; 2],
    };
    gui_export!(
        || {
            if atlas.is_null() {
                invalid();
                return empty;
            }
            match unsafe { &*atlas }.atlas.region(SpriteId(id)) {
                Some(region) => region,
                None => {
                    invalid();
                    empty
                }
            }
        },
        empty
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_sprite_atlas_pixels(
    atlas: *const GuiSpriteAtlas,
    width: *mut u32,
    height: *mut u32,
    len: *mut usize,
    generation: *mut u64,
) -> *const u8 {
    gui_export!(
        || {
            if atlas.is_null() {
                invalid();
                return core::ptr::null();
            }
            let atlas = &unsafe { &*atlas }.atlas;
            let size = atlas.size();
            unsafe {
                if !width.is_null() {
                    *width = size[0];
                }
                if !height.is_null() {
                    *height = size[1];
                }
                if !len.is_null() {
                    *len = atlas.pixels().len();
                }
                if !generation.is_null() {
                    *generation = atlas.generation();
                }
            }
            atlas.pixels().as_ptr()
        },
        core::ptr::null()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_image(
    ctx: *mut GuiContext,
    region: GuiImageRegion,
    size: GuiVec2,
    mode: u32,
    tint: GuiColor,
) -> GuiRect {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return GuiRect::ZERO;
            };
            if !matches!(gui.window, WindowState::Open) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return GuiRect::ZERO;
            }
            let Some(mode) = fit(mode) else {
                invalid();
                return GuiRect::ZERO;
            };
            match gui.ctx.ui().image(region, size.into(), mode, tint.into()) {
                Ok(rect) => rect.into(),
                Err(_) => {
                    invalid();
                    GuiRect::ZERO
                }
            }
        },
        GuiRect::ZERO
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_image_button(
    ctx: *mut GuiContext,
    label: *const u8,
    len: usize,
    region: GuiImageRegion,
    size: GuiVec2,
    mode: u32,
    tint: GuiColor,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if !matches!(gui.window, WindowState::Open) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            }
            let (Some(label), Some(mode)) = (unsafe { str_arg(label, len) }, fit(mode)) else {
                invalid();
                return false;
            };
            match gui
                .ctx
                .ui()
                .image_button(label, region, size.into(), mode, tint.into())
            {
                Ok(clicked) => clicked,
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
pub unsafe extern "C" fn gui_draw_nine_slice(
    ctx: *mut GuiContext,
    region: GuiImageRegion,
    rect: GuiRect,
    slices: GuiNineSlice,
    tint: GuiColor,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if !matches!(gui.window, WindowState::Open) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            if gui
                .ctx
                .ui()
                .draw_nine_slice(region, rect.into(), slices, tint.into())
                .is_err()
            {
                invalid();
            }
        },
        ()
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atlas_ownership_growth_and_invalid_inputs() {
        unsafe {
            let atlas = gui_sprite_atlas_new(50, 4, 16, 1);
            assert!(!atlas.is_null());
            let first =
                gui_sprite_atlas_insert(atlas, [12, 34, 56, 128].repeat(4).as_ptr(), 16, 2, 2);
            assert_eq!(first, 0);
            assert_eq!(
                gui_sprite_atlas_insert(atlas, core::ptr::null(), 16, 2, 2),
                u32::MAX
            );
            let region = gui_sprite_atlas_region(atlas, first);
            assert_eq!(region.size, [2.; 2]);
            assert_eq!(core::mem::size_of::<GuiImageRegion>(), 32);
            assert_eq!(core::mem::offset_of!(GuiImageRegion, texture), 0);
            assert_eq!(core::mem::offset_of!(GuiImageRegion, size), 24);
            assert_eq!(core::mem::size_of::<GuiNineSlice>(), 32);
            assert_eq!(core::mem::offset_of!(GuiNineSlice, destination), 16);
            gui_sprite_atlas_free(atlas);
        }
    }
}
