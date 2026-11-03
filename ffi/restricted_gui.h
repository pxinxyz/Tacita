/*
 * restricted_gui.h — C++ API contract for the `restricted-gui-ffi` crate.
 *
 * Hand-written; matches `ffi/src/*.rs` exactly. Consistency is enforced
 * from both sides:
 *   - a Rust test (`ffi/src/header_consistency.rs`) parses THIS file and
 *     asserts every declared `gui_*` function exists as an export, every
 *     export is declared here, and every `GUI_*` constant has the value
 *     the implementation uses;
 *   - the `static_assert`s below pin the struct layouts the Rust side
 *     guarantees (asserted against the same numbers in Rust), so any C++
 *     compiler compiling this header (e.g. `ffi/tests/smoke.cpp`) verifies
 *     the C++ view of the ABI.
 *
 * General contract:
 *   - The host owns an opaque `GuiContext`, created from TTF/OTF font
 *     bytes. Feed input as plain C structs once per frame, declare the UI,
 *     then consume draw data as slices into context-owned buffers.
 *     Calls using the same context must be serialized, including reads of
 *     borrowed draw or atlas data; distinct contexts may use distinct
 *     threads. The thread-local error slot does not synchronize a context.
 *   - Every function is wrapped in `catch_unwind` on the Rust side: a
 *     Rust panic NEVER crosses the boundary. A caught panic is recorded
 *     as GUI_ERROR_PANIC and the function returns a safe value (null /
 *     zero-initialized / false).
 *   - `gui_last_error_code()` reports the last error OF THE CALLING
 *     THREAD (thread-local). Every export resets it to GUI_ERROR_NONE on
 *     entry, so it always describes the most recent `gui_*` call on this
 *     thread.
 *   - Strings are `(ptr, len)` byte pairs, UTF-8, NO NUL required. Null
 *     with a zero length is the empty string; null with a nonzero length
 *     or invalid UTF-8 is GUI_ERROR_INVALID_INPUT.
 *   - Null pointers that would be dereferenced are rejected
 *     (GUI_ERROR_INVALID_INPUT); null is legal where a function's own
 *     docs say so (e.g. a null item list with a zero count).
 *     Nonempty pointer/count arrays must be live, correctly aligned, and
 *     readable for the entire call. The wrapper rejects invalid counts and
 *     misalignment it can detect, but cannot verify memory ownership.
 *   - All widget/window calls require an active frame (after
 *     `gui_new_frame`, before `gui_render`); calling them outside a frame
 *     is GUI_ERROR_CONTRACT_VIOLATION.
 *
 * Target: 64-bit Windows (the crate builds `x86_64-pc-windows-msvc`).
 * The layout asserts below are written for that ABI.
 */

#ifndef RESTRICTED_GUI_H
#define RESTRICTED_GUI_H

#include <cstddef>
#include <cstdint>

#ifdef __cplusplus
extern "C" {
// Stable sprite IDs; query regions again after atlas growth. Free handles once.
struct GuiSpriteAtlas;
struct GuiImageRegion { uint64_t texture; float uv_min[2], uv_max[2], size[2]; };
struct GuiNineSlice { float source[4], destination[4]; };
GuiSpriteAtlas* gui_sprite_atlas_new(uint64_t texture, uint32_t initial, uint32_t maximum, uint32_t padding);
void gui_sprite_atlas_free(GuiSpriteAtlas* atlas);
uint32_t gui_sprite_atlas_insert(GuiSpriteAtlas* atlas, const uint8_t* rgba, size_t len, uint32_t width, uint32_t height);
GuiImageRegion gui_sprite_atlas_region(const GuiSpriteAtlas* atlas, uint32_t id);
const uint8_t* gui_sprite_atlas_pixels(const GuiSpriteAtlas* atlas, uint32_t* width, uint32_t* height, size_t* len, uint64_t* generation);

#endif

/* ---------------------------------------------------------------------------
 * Error codes (returned by `gui_last_error_code`)
 * ------------------------------------------------------------------------- */

/* The most recent `gui_*` call on this thread succeeded. */
#define GUI_ERROR_NONE 0
/* A documented API contract was violated (e.g. mismatched begin/end). */
#define GUI_ERROR_CONTRACT_VIOLATION 1
/* A Rust panic was caught at the boundary. */
#define GUI_ERROR_PANIC 2
/* A null or otherwise invalid argument was passed. */
#define GUI_ERROR_INVALID_INPUT 3

/* ---------------------------------------------------------------------------
 * Key codes: the `Key` enum as `uint16_t`, in declaration order
 * ------------------------------------------------------------------------- */

#define GUI_KEY_BACKSPACE 0
#define GUI_KEY_DELETE 1
#define GUI_KEY_LEFT 2
#define GUI_KEY_RIGHT 3
#define GUI_KEY_HOME 4
#define GUI_KEY_END 5
#define GUI_KEY_ENTER 6
#define GUI_KEY_ESCAPE 7
#define GUI_KEY_INSERT 8
#define GUI_KEY_A 9
#define GUI_KEY_B 10
#define GUI_KEY_C 11
#define GUI_KEY_D 12
#define GUI_KEY_E 13
#define GUI_KEY_F 14
#define GUI_KEY_G 15
#define GUI_KEY_H 16
#define GUI_KEY_I 17
#define GUI_KEY_J 18
#define GUI_KEY_K 19
#define GUI_KEY_L 20
#define GUI_KEY_M 21
#define GUI_KEY_N 22
#define GUI_KEY_O 23
#define GUI_KEY_P 24
#define GUI_KEY_Q 25
#define GUI_KEY_R 26
#define GUI_KEY_S 27
#define GUI_KEY_T 28
#define GUI_KEY_U 29
#define GUI_KEY_V 30
#define GUI_KEY_W 31
#define GUI_KEY_X 32
#define GUI_KEY_Y 33
#define GUI_KEY_Z 34
#define GUI_KEY_NUM0 35
#define GUI_KEY_NUM1 36
#define GUI_KEY_NUM2 37
#define GUI_KEY_NUM3 38
#define GUI_KEY_NUM4 39
#define GUI_KEY_NUM5 40
#define GUI_KEY_NUM6 41
#define GUI_KEY_NUM7 42
#define GUI_KEY_NUM8 43
#define GUI_KEY_NUM9 44
#define GUI_KEY_F1 45
#define GUI_KEY_F2 46
#define GUI_KEY_F3 47
#define GUI_KEY_F4 48
#define GUI_KEY_F5 49
#define GUI_KEY_F6 50
#define GUI_KEY_F7 51
#define GUI_KEY_F8 52
#define GUI_KEY_F9 53
#define GUI_KEY_F10 54
#define GUI_KEY_F11 55
#define GUI_KEY_F12 56
#define GUI_KEY_SPACE 57
#define GUI_KEY_TAB 58
#define GUI_KEY_SHIFT 59
#define GUI_KEY_CTRL 60
#define GUI_KEY_ALT 61
#define GUI_KEY_UP 62
#define GUI_KEY_DOWN 63

/* `GuiKeyEvent::kind` values. */
#define GUI_KEY_EVENT_PRESS 0   /* a named key was pressed */
#define GUI_KEY_EVENT_RELEASE 1 /* a named key was released */
#define GUI_KEY_EVENT_CHAR 2    /* a character of text input was received */

/* `GuiKeyBinding::tag` values. */
#define GUI_KEY_BINDING_NONE 0  /* no binding */
#define GUI_KEY_BINDING_KEY 1   /* a plain key press; `key` carries the code */
#define GUI_KEY_BINDING_MOUSE 2 /* a mouse button; `mouse_button` 0=left,1=right,2=middle,3=back,4=forward */
#define GUI_KEY_BINDING_CHORD 3 /* `key` plus the `ctrl`/`shift`/`alt` modifiers */

/* `gui_add_font`'s failure return: not a valid face id. Real face ids are
 * small dense indices (0 is the default face the context was created
 * with); when this value comes back, `gui_last_error_code` says why. */
#define GUI_FONT_ID_INVALID 4294967295

#define GUI_TEXT_RASTER_LEGACY 0
#define GUI_TEXT_RASTER_SUBPIXEL 1
/* ---------------------------------------------------------------------------
 * Scroll flags: the `scroll_flags` bitmask of `gui_begin_window` and
 * `gui_begin_child`.
 *
 * GUI_SCROLL_SCROLLBAR WITHOUT GUI_SCROLL_SCROLLABLE is accepted and
 * IGNORED — the bar strip is reserved only for a scrollable container.
 * Any bit outside the two defined ones is GUI_ERROR_CONTRACT_VIOLATION,
 * checked in the wrapper before the core is called, and opens no scope
 * (so the matching end call must NOT be made).
 * ------------------------------------------------------------------------- */

/* Clip content to the container's rect; never scroll, no scrollbar. The
 * container still claims its declared size in the surrounding layout. */
#define GUI_SCROLL_NONE 0u
/* Accept mouse-wheel input and keep a clamped scroll offset. */
#define GUI_SCROLL_SCROLLABLE (1u << 0)
/* Reserve and draw a scrollbar track on the content rect's right edge,
 * but only while the content overflows the visible height — content that
 * fits gets no track, no thumb, and the strip back as content width. */
#define GUI_SCROLL_SCROLLBAR (1u << 1)

/* ---------------------------------------------------------------------------
 * Plain C structs
 * ------------------------------------------------------------------------- */

/* Opaque GUI context handle. Create with `gui_context_new`, destroy with
 * `gui_context_free`. */
typedef struct GuiContext GuiContext;

/* A 2D point/size/vector in screen-space pixels. */
typedef struct GuiVec2 {
    float x;
    float y;
} GuiVec2;

/* An axis-aligned rect as corner points (`min` top-left, `max`
 * bottom-right). */
typedef struct GuiRect {
    GuiVec2 min;
    GuiVec2 max;
} GuiRect;

/* Persistent caller-owned placement for a panel inside a dockspace. */
#define GUI_DOCK_LEFT 0u
#define GUI_DOCK_RIGHT 1u
#define GUI_DOCK_FLOATING 2u
#define GUI_SPLIT_HORIZONTAL 0u
#define GUI_SPLIT_VERTICAL 1u
typedef struct GuiDockState {
    uint32_t zone;
    GuiVec2 float_pos;
} GuiDockState;

/* An RGBA color, one byte per channel. */
typedef struct GuiColor {
    uint8_t r;
    uint8_t g;
    uint8_t b;
    uint8_t a;
} GuiColor;

/* Reusable scrollbar skin. track_click: 0 = jump, 1 = page step.
 * thumb_rounding = -1 uses the theme's normal corner rounding. */
typedef struct GuiScrollbarSkin {
    float track_inset;
    float thumb_inset;
    float track_rounding;
    float thumb_rounding;
    float min_thumb;
    uint32_t track_click;
} GuiScrollbarSkin;

#define GUI_EASING_LINEAR 0u
#define GUI_EASING_SMOOTH_STEP 1u
#define GUI_EASING_SMOOTHER_STEP 2u
#define GUI_EASING_QUAD_IN 3u
#define GUI_EASING_QUAD_OUT 4u
#define GUI_EASING_CUBIC_IN_OUT 5u
#define GUI_EASING_CUBIC_OUT 6u

/* Motion timings in seconds; nonzero flag bytes mean true. Reserved bytes
 * are ignored on input and zero on output. Settings change between frames.
 * Library defaults are instant; enable motion explicitly. */
typedef struct GuiMotionSettings {
    uint8_t enabled;
    uint8_t reduced_motion;
    uint8_t reserved[2];
    uint32_t easing;
    float hover;
    float focus;
    float indicator;
    float popup;
    float collapse;
    float scroll;
} GuiMotionSettings;

GuiMotionSettings gui_motion_settings(GuiContext* ctx);
void gui_set_motion_settings(GuiContext* ctx, GuiMotionSettings settings);
/* Pure math helpers; easing clamps finite progress. Pulse requires finite time
 * and a positive finite period. Invalid input reports GUI_ERROR_INVALID_INPUT. */
float gui_ease(uint32_t easing, float progress);
float gui_pulse(float time, float period);
/* Custom channels require an active frame. Use stable, distinct IDs.
 * Scalars must be finite, timings nonnegative; invalid input returns zero
 * (transparent black for color). A channel advances at most once per frame. */
float gui_animate_float(GuiContext* ctx, uint64_t id, float initial, float target, float duration, uint32_t easing);
float gui_animate_spring(GuiContext* ctx, uint64_t id, float initial, float target, float response);
GuiColor gui_animate_color(GuiContext* ctx, uint64_t id, GuiColor initial, GuiColor target, float duration, uint32_t easing);

#define GUI_PLAYBACK_ONCE 0u
#define GUI_PLAYBACK_LOOP 1u
#define GUI_PLAYBACK_PING_PONG 2u
#define GUI_GRADIENT_CLAMP 0u
#define GUI_GRADIENT_REPEAT 1u
#define GUI_GRADIENT_MIRROR 2u
typedef struct GuiKeyframe { float time; float value; uint32_t easing; } GuiKeyframe;
typedef struct GuiTimelineOptions {
    uint32_t mode;
    float speed;
    uint8_t playing;
    uint8_t reserved[7];
    uint64_t revision;
} GuiTimelineOptions;
typedef struct GuiGradientStop { float position; GuiColor color; } GuiGradientStop;
typedef struct GuiGradientOptions { GuiVec2 axis; uint32_t spread; uint32_t resolution; } GuiGradientOptions;
/* Keys: nonempty, first time 0, strictly increasing finite times/values.
 * Easing belongs to the arriving segment. Arrays remain stable during calls.
 * Increment revision to restart. Signed speed reverses; zero or playing=0 pauses. */
float gui_sample_keyframes(const GuiKeyframe* keys, size_t count, float time);
float gui_animate_timeline(GuiContext* ctx, uint64_t id, const GuiKeyframe* keys, size_t count, GuiTimelineOptions options);
void gui_seek_timeline(GuiContext* ctx, uint64_t id, float time);
GuiRect gui_animate_rect(GuiContext* ctx, uint64_t id, GuiRect initial, GuiRect target, float duration, uint32_t easing);
/* Region uses screen coordinates and does not advance the outer cursor.
 * Pair a successful begin with end; errors leave the scope stack unchanged. */
bool gui_begin_animated_layout(GuiContext* ctx, const uint8_t* label, size_t len, GuiRect target, float duration, uint32_t easing, GuiRect* out_rect);
void gui_end_animated_layout(GuiContext* ctx);
/* Stops: at least two, strictly increasing positions from 0 through 1.
 * Axis must be nonzero and finite; resolution 1..64. Caller time in seconds,
 * period finite and positive. Reduced motion freezes animated phase at zero. */
void gui_draw_gradient(GuiContext* ctx, GuiRect rect, const GuiGradientStop* stops, size_t count, GuiGradientOptions options);
void gui_draw_animated_gradient(GuiContext* ctx, GuiRect rect, const GuiGradientStop* stops, size_t count, GuiGradientOptions options, float time, float period);

#define GUI_THEME_BACKGROUND 0u
#define GUI_THEME_PANEL 1u
#define GUI_THEME_INPUT 2u
#define GUI_THEME_HOVER 3u
#define GUI_THEME_ACCENT 4u
#define GUI_THEME_TEXT 5u
#define GUI_THEME_TITLE_TEXT 6u
#define GUI_THEME_SCROLL_TRACK 7u
#define GUI_THEME_SCROLL_THUMB 8u
#define GUI_THEME_SCROLL_HOVER 9u
#define GUI_THEME_SCROLL_ACTIVE 10u
#define GUI_THEME_SECONDARY 11u
#define GUI_THEME_MUTED 12u
#define GUI_THEME_SUCCESS 13u
#define GUI_THEME_WARNING 14u
#define GUI_THEME_DANGER 15u
/* Metrics are finite nonnegative logical pixels. Existing Style roles remain
 * editable; secondary/muted/status roles retain supplied values. Flags accept
 * any nonzero byte; reserved bytes ignored on input and zero on output. */
typedef struct GuiThemeTokens {
    GuiColor colors[16];
    GuiVec2 frame_padding, item_spacing, window_padding;
    float slider_grab_size, scrollbar_size, corner_rounding, border_size;
    uint8_t anti_aliasing;
    uint8_t reserved[3];
} GuiThemeTokens;
GuiThemeTokens gui_theme_tokens(GuiContext* ctx);
void gui_set_theme_tokens(GuiContext* ctx, GuiThemeTokens tokens);
GuiColor gui_theme_color(GuiContext* ctx, uint32_t role);
GuiThemeTokens gui_interpolate_theme(GuiThemeTokens from, GuiThemeTokens to, float amount);

/* Payload bytes are owned by the caller and copied into the GUI state.
 * len must be <= 32; type_id is a host-defined format discriminator. */
typedef struct GuiDragPayload {
    uint64_t type_id;
    uint8_t len;
    uint8_t bytes[32];
} GuiDragPayload;

/* One string of a combo item list: a `(ptr, len)` byte pair, UTF-8, no
 * NUL required — the same string convention as every other export. */
typedef struct GuiStr {
    const uint8_t *ptr; /* may be null when `len` is zero */
    size_t len;
} GuiStr;

/* SVG icon subset: monochrome paths/basic shapes, affine transforms, winding
 * fills, round joins, butt/round caps. Registration is between frames and may
 * grow the shared atlas. Size is logical; output density follows the font.
 * Unsupported CSS, resources, filters, group opacity and other SVG features
 * are rejected. IDs belong to this atlas and must be re-registered on rebuild. */
#define GUI_SVG_ICON_INVALID 4294967295
#define GUI_BORDER_INNER 0
#define GUI_BORDER_CENTER 1
#define GUI_BORDER_OUTER 2

#define GUI_PAINT_LINEAR 1
#define GUI_PAINT_RADIAL 2
#define GUI_PAINT_CONIC 3
#define GUI_MAX_PAINT_STOPS 16
#define GUI_MAX_MASKS 4
#define GUI_EFFECT_UNIFORM_SIZE 1024
#define GUI_SDF_FILL 1
#define GUI_SDF_BORDER 2
#define GUI_SDF_GLOW 3
#define GUI_SDF_EDGE_GLOW 4
#define GUI_SDF_HIGHLIGHT 5
#define GUI_REVEAL_LEFT_TO_RIGHT 3
#define GUI_REVEAL_RIGHT_TO_LEFT 4
#define GUI_REVEAL_TOP_TO_BOTTOM 5
#define GUI_REVEAL_BOTTOM_TO_TOP 6
#define GUI_REVEAL_RADIAL 7
typedef struct GuiSdfStyle {
    uint32_t kind; float width; uint32_t placement; float reach;
    GuiVec2 light_direction; float light_power;
} GuiSdfStyle;
typedef struct GuiRevealMask {
    GuiRect rect; uint32_t direction; float progress; float softness;
    uint8_t inverted; uint8_t reserved[3];
} GuiRevealMask;
typedef struct GuiGradientPaint { uint32_t kind; GuiVec2 origin; GuiVec2 vector; uint32_t spread; float phase; } GuiGradientPaint;
typedef struct GuiAlphaMask { uint64_t texture; GuiRect rect; float uv[4]; } GuiAlphaMask;
typedef struct GuiGradientMask { GuiVec2 start; GuiVec2 end; float opacity[2]; } GuiGradientMask;
/* Read-only effect records. kind: gradient 0=identity, 1=linear, 2=radial,
 * 3=conic; mask 0=unused, 1=texture alpha, 2=linear opacity, 3..7=reveal.
 * SDF parameters: kind/width/placement/reach; light: direction/power/AA flag. */
typedef struct GuiMaskOp { uint64_t texture; uint32_t kind; uint32_t reserved; float rect[4]; float params[4]; } GuiMaskOp;
typedef struct GuiMaskStack { GuiMaskOp ops[4]; uint32_t count; uint32_t reserved; } GuiMaskStack;
typedef struct GuiGradientFill {
    float rect[4]; float origin[2]; float vector[2]; uint32_t kind; uint32_t spread;
    float phase; uint32_t stop_count; GuiGradientStop stops[16];
} GuiGradientFill;
typedef struct GuiSdfShape { float rect[4]; float radii[4]; float parameters[4]; float light[4]; } GuiSdfShape;
typedef struct GuiSurfaceStyle {
    uint32_t flags;
    float phase;
    float pixel_scale[2];
    float noise;
    float grain;
    float dither;
    uint32_t dither_kind;
    float exposure;
    float contrast;
    float saturation;
    float desaturation;
    float tint[4];
    float scanlines;
    float spacing;
    float grid_width;
    float dot_radius;
    float vignette;
    float shimmer;
    float sweep_width;
    float sweep_speed;
    float distortion;
    float frequency;
    float chromatic;
    float refraction;
    float rect[4];
} GuiSurfaceStyle;
typedef struct GuiImageFilter { uint32_t kind, overwrite; float radius, threshold, intensity, texel[2], reserved; uint64_t secondary; float params[6]; } GuiImageFilter;
typedef struct GuiBlur { uint32_t method; float radius; uint32_t levels; } GuiBlur;
typedef struct GuiFilterStep { uint32_t source, size[2], reserved; GuiImageFilter filter; } GuiFilterStep;
typedef struct GuiDrawEffects { GuiGradientFill gradient; GuiMaskStack masks; GuiSdfShape sdf; GuiSurfaceStyle surface; GuiImageFilter filter; uint32_t stencil[2]; } GuiDrawEffects;
#define GUI_SURFACE_NOISE 1u
#define GUI_SURFACE_GRAIN 2u
#define GUI_SURFACE_DITHER 4u
#define GUI_SURFACE_GRADE 8u
#define GUI_SURFACE_DESATURATE 16u
#define GUI_SURFACE_VIGNETTE 32u
#define GUI_SURFACE_CHROMATIC 64u
#define GUI_SURFACE_SCANLINES 128u
#define GUI_SURFACE_GRID 256u
#define GUI_SURFACE_DOTS 512u
#define GUI_SURFACE_SHIMMER 1024u
#define GUI_SURFACE_SWEEP 2048u
#define GUI_SURFACE_DISTORT 4096u
#define GUI_SURFACE_GLASS 8192u
#define GUI_SURFACE_ALL 16383u
#define GUI_DITHER_BAYER 0u
#define GUI_DITHER_BLUE_NOISE 1u
#define GUI_BLUR_SEPARABLE 0u
#define GUI_BLUR_DUAL_KAWASE 1u
#define GUI_MAX_FILTER_PASSES 15u
#define GUI_FILTER_COPY 0u
#define GUI_FILTER_GAUSSIAN_X 1u
#define GUI_FILTER_GAUSSIAN_Y 2u
#define GUI_FILTER_KAWASE_DOWN 3u
#define GUI_FILTER_KAWASE_UP 4u
#define GUI_FILTER_THRESHOLD 5u
#define GUI_FILTER_BLOOM_COMPOSE 6u
typedef struct GuiRoundedRect { GuiRect rect; float radii[4]; } GuiRoundedRect;

/* Per-pixel paints; 2..16 ordered stops covering [0,1]. Rect/radii follow the
 * shape API; origin/vector are normalized within rect. Radial vector is ellipse
 * radii, conic vector a reference direction. Masks multiply alpha only and do
 * not change layout/input. Up to four nested masks; true begins require end. */
/* Opt-in single-quad SDF path. Logical width/reach, per-corner radii.
 * Reveal masks use normalized progress/softness in [0,1], with exact endpoints.
 * Successful reveal begins close with gui_end_mask. Presentation only. */
void gui_draw_sdf_shape(GuiContext* ctx, GuiRoundedRect shape, GuiColor color, GuiSdfStyle style);
void gui_draw_sdf_shape_gradient(GuiContext* ctx, GuiRoundedRect shape, const GuiGradientStop* stops, size_t count, GuiGradientPaint paint, GuiSdfStyle style);
bool gui_begin_reveal_mask(GuiContext* ctx, GuiRevealMask mask);
void gui_draw_shape_gradient(GuiContext* ctx, GuiRoundedRect shape, const GuiGradientStop* stops, size_t count, GuiGradientPaint paint);
void gui_draw_gradient_border(GuiContext* ctx, GuiRoundedRect shape, const GuiGradientStop* stops, size_t count, GuiGradientPaint paint, float width, uint32_t placement);
bool gui_begin_alpha_mask(GuiContext* ctx, GuiAlphaMask mask);
bool gui_begin_gradient_mask(GuiContext* ctx, GuiGradientMask mask);
void gui_end_mask(GuiContext* ctx);
/* After gui_render: command.effects n>0 selects table[n-1]; zero is identity.
 * Records borrow ctx until gui_new_frame. Shader packing is 1024 bytes;
 * output must not overlap borrowed frame data. */

/* New effect records require hosts to update shaders; vertex/command ABI is unchanged.
   flags are GUI_SURFACE_* combinations; distances are logical/texture pixels.
   GPU postprocessors capture the scene before GUI. C hosts execute gui_filter_plan
   steps into sampleable targets using overwrite blending and the shared kernels. */
GuiSurfaceStyle gui_surface_default(void);
void gui_draw_surface(GuiContext* ctx, GuiRoundedRect rect, uint64_t texture, uint8_t textured, GuiRect uv, GuiColor color, GuiSurfaceStyle style);
void gui_draw_filtered_image(GuiContext* ctx, uint64_t texture, GuiRect rect, GuiRect uv, GuiColor color, GuiImageFilter filter);
bool gui_begin_stencil_clip(GuiContext* ctx, GuiStr label, GuiRoundedRect rect);
void gui_end_stencil_clip(GuiContext* ctx);
/* NULL output queries count. Short buffers fail without writing. secondary on a
   BLOOM_COMPOSE step must be the original input TextureId; source 0=input, n=step n-1. */
size_t gui_filter_plan(uint32_t width, uint32_t height, GuiBlur blur, uint8_t bloom, float threshold, float intensity, GuiFilterStep* output, size_t capacity);

const GuiDrawEffects* gui_frame_effects(GuiContext* ctx, size_t* count);
/* Packs a readable frame or host record; rejects unknown enum codes without
   writing output. Output must not overlap any borrowed frame/record memory. */
bool gui_effect_uniform(const GuiDrawEffects* effect, uint8_t* output, size_t capacity);
/* CPU distance contours: TL/TR/BR/BL radii clamp to half the shorter side.
 * Coverage ramps span one physical pixel; no SDF shader or atlas allocation.
 * Rounded clips geometrically clip triangles (no stencil or feathered mask).
 * Draw functions require an active frame and open window. */
typedef struct GuiShadow {
    GuiVec2 offset; float spread; float blur; GuiColor color;
    uint8_t inset; uint8_t reserved[3];
} GuiShadow;
float gui_rounded_distance(GuiRoundedRect shape, GuiVec2 point);
void gui_draw_rounded_rect(GuiContext* ctx, GuiRoundedRect shape, GuiColor color);
void gui_draw_rounded_border(GuiContext* ctx, GuiRoundedRect shape, GuiColor color, float width, uint32_t mode);
void gui_draw_shadow(GuiContext* ctx, GuiRoundedRect shape, GuiShadow shadow);
void gui_draw_glow(GuiContext* ctx, GuiRoundedRect shape, GuiColor color, float reach);
/* Successful begin pairs with end; rejected begins open nothing. */
bool gui_begin_rounded_clip(GuiContext* ctx, const uint8_t* label, size_t len, GuiRoundedRect shape);
void gui_end_rounded_clip(GuiContext* ctx);
uint32_t gui_add_svg_icon(GuiContext* ctx, const uint8_t* source, size_t len, GuiVec2 size);
void gui_draw_svg_icon(GuiContext* ctx, uint32_t id, GuiRect rect, GuiColor tint);
GuiRect gui_svg_icon(GuiContext* ctx, uint32_t id, GuiVec2 size, GuiColor tint);

#define GUI_JOIN_MITER 0
#define GUI_JOIN_BEVEL 1
#define GUI_JOIN_ROUND 2
#define GUI_CAP_BUTT 0
#define GUI_CAP_SQUARE 1
#define GUI_CAP_ROUND 2
#define GUI_PATH_MAX_POINTS 2048
#define GUI_POLYGON_MAX_POINTS 512
#define GUI_DASH_MAX_LENGTHS 16
#define GUI_GRID_AUTO 4294967295u
#define GUI_CONTENT_START 0
#define GUI_CONTENT_CENTER 1
#define GUI_CONTENT_END 2
#define GUI_CONTENT_STRETCH 3
#define GUI_CONTENT_SPACE_BETWEEN 4
/* Width 0..4096 logical px, miter limit 1..64, tolerance 0.05..4 physical px.
 * Joined centerlines retain phase through corners. Odd dash arrays repeat twice.
 * Dash lengths are 0.01..1000000 logical px; signed phase is bounded to +/-1000000.
 * Point coordinates are finite +/-1000000. Entire rejected calls draw nothing.
 * Ordinary joins have shared coverage; self-crossing strokes use painter overlap. */
typedef struct GuiStrokeStyle {
    float width; uint32_t join, cap; float miter_limit, tolerance;
} GuiStrokeStyle;
bool gui_draw_polyline(GuiContext* ctx, const GuiVec2* points, size_t count,
    uint8_t closed, GuiStrokeStyle style, GuiColor color);
bool gui_draw_dashed_polyline(GuiContext* ctx, const GuiVec2* points, size_t count,
    uint8_t closed, GuiStrokeStyle style, const float* lengths, size_t dash_count,
    float phase, GuiColor color);
/* Quadratic/cubic arrays contain exactly 3/4 points. Matching cubic ends close. */
bool gui_draw_quadratic_bezier(GuiContext* ctx, const GuiVec2* points, GuiStrokeStyle style, GuiColor color);
bool gui_draw_cubic_bezier(GuiContext* ctx, const GuiVec2* points, GuiStrokeStyle style, GuiColor color);
/* One simple contour in either winding; intersections, touches and holes rejected. */
bool gui_draw_concave_polygon(GuiContext* ctx, const GuiVec2* points, size_t count, GuiColor color);
typedef struct GuiLayoutOptions { uint32_t wrap, align_content; } GuiLayoutOptions;
typedef struct GuiGridCell { uint32_t row, column, row_span, column_span; } GuiGridCell;

#define GUI_LAYOUT_ROW 0
#define GUI_LAYOUT_COLUMN 1
#define GUI_LAYOUT_GRID 2
#define GUI_LAYOUT_START 0
#define GUI_LAYOUT_CENTER 1
#define GUI_LAYOUT_END 2
#define GUI_LAYOUT_STRETCH 3
#define GUI_LAYOUT_BASELINE 4
#define GUI_LAYOUT_JUSTIFY_START 0
#define GUI_LAYOUT_JUSTIFY_CENTER 1
#define GUI_LAYOUT_JUSTIFY_END 2
#define GUI_LAYOUT_SPACE_BETWEEN 3
#define GUI_LAYOUT_MAX_ITEMS 64

typedef struct GuiLayoutSpec {
    uint32_t kind;
    GuiVec2 size, min, max;
    float padding[4]; /* left, top, right, bottom */
    GuiVec2 gap;
    uint32_t align, justify, columns;
} GuiLayoutSpec;
typedef struct GuiLayoutItem {
    GuiVec2 preferred, min, max;
    float grow, shrink, margin[4], baseline;
    uint8_t absolute, hit_test, reserved[2];
    GuiVec2 anchor, offset;
    int32_t layer;
} GuiLayoutItem;

/* Preferred sizes are resolved before drawing. size=0 selects intrinsic size;
 * max must be supplied (typically 1000000); columns must be 1..64. Dimensions,
 * gaps, padding/margins, grow/shrink and baseline are finite 0..1000000. Anchors
 * are 0..1; signed offsets are bounded to +/-1000000. Nonzero byte flags are true.
 * Absolute items don't contribute to intrinsic size/flow. Baseline alignment
 * applies to rows/grids. Borrowed arrays are stable through each call, never
 * copied to a whole-list temporary. Each item is declared at most once, in
 * ascending index order; higher layer/index wins paint and overlapping input.
 * hit_test=0 permits click-through paint. A successful begin requires its end;
 * false begins open no scope. Item scopes clip ordinary widgets and nest. */
GuiVec2 gui_measure_layout(GuiLayoutSpec spec, const GuiLayoutItem* items, size_t count);
bool gui_begin_layout(GuiContext* ctx, const uint8_t* label, size_t len,
    GuiLayoutSpec spec, const GuiLayoutItem* items, size_t count, GuiRect* out_rect);
/* wrap=0/1; content alignment distributes wrapped lines. Grid can't wrap.
 * Spans 1..64; explicit cells cannot overlap or exceed 64 rows/grid columns.
 * row/column both GUI_GRID_AUTO selects first-fit after explicit reservations.
 * Null cells means all default placements; otherwise readable for count entries.
 * Row/column/absolute items require default cells. Output disjoint from inputs.
 * Uses the existing end_layout/item calls; false begin opens nothing. */
GuiVec2 gui_measure_layout_extended(GuiLayoutSpec spec, const GuiLayoutItem* items,
    size_t count, GuiLayoutOptions options, const GuiGridCell* cells);
bool gui_begin_layout_extended(GuiContext* ctx, const uint8_t* label, size_t len,
    GuiLayoutSpec spec, const GuiLayoutItem* items, size_t count,
    GuiLayoutOptions options, const GuiGridCell* cells, GuiRect* out_rect);
GuiRect gui_layout_item_rect(GuiContext* ctx, size_t index);
bool gui_begin_layout_item(GuiContext* ctx, size_t index, GuiRect* out_rect);
void gui_end_layout_item(GuiContext* ctx);
void gui_end_layout(GuiContext* ctx);
GuiVec2 gui_available_size(GuiContext* ctx);

/* Data widgets: true begins require their corresponding ends; false opens
 * nothing. Cell declarations may return false for clipped/offscreen cells.
 * Models retain capacity; own one numeric model per numeric control. Borrowed
 * descriptors/strings are readable and stable for the call. All handles are
 * exclusive during a call; free exactly once, null frees are harmless. */
#define GUI_DATA_MAX_COLUMNS 32u
#define GUI_DATA_MAX_ROWS 1000000u
#define GUI_SORT_NONE 0u
#define GUI_SORT_ASCENDING 1u
#define GUI_SORT_DESCENDING 2u
typedef struct GuiSelection GuiSelection;
typedef struct GuiNumericEdit GuiNumericEdit;
typedef struct GuiVisibleRows { size_t start, end; } GuiVisibleRows;
typedef struct GuiDataTableState {
    float widths[32]; float scroll_x; int32_t sort_column; uint32_t sort_direction;
} GuiDataTableState;
typedef struct GuiDataTableOptions {
    GuiVec2 size; float row_height; uint32_t frozen_columns, frozen_rows;
} GuiDataTableOptions;
GuiSelection* gui_selection_new(size_t count);
void gui_selection_free(GuiSelection* selection);
bool gui_selection_resize(GuiSelection* selection, size_t count);
void gui_selection_clear(GuiSelection* selection);
size_t gui_selection_count(GuiSelection* selection);
bool gui_selection_contains(GuiSelection* selection, size_t index);
bool gui_selection_select(GuiSelection* selection, size_t index, uint8_t ctrl, uint8_t shift);
GuiNumericEdit* gui_numeric_edit_new(void);
void gui_numeric_edit_free(GuiNumericEdit* edit);
bool gui_numeric_edit_invalid(GuiNumericEdit* edit);
bool gui_clip_rows(size_t count, float height, float gap, float origin, GuiVec2 clip, GuiVisibleRows* out);
bool gui_begin_rows(GuiContext* ctx, GuiStr label, size_t count, float height);
bool gui_visible_rows(GuiContext* ctx, GuiVisibleRows* out);
bool gui_begin_row(GuiContext* ctx, size_t index);
void gui_end_rows(GuiContext* ctx);
bool gui_multi_select_list(GuiContext* ctx, GuiStr label, const GuiStr* items, size_t count, GuiSelection* selection, GuiVec2 size);
/* Call init once, then retain state. Sort requests are host-applied; -1/NONE
 * is unsorted. Widths 32..10000. Headers remain pinned; frozen rows/columns
 * form independently clipped quadrants. Visible range excludes frozen rows:
 * declare frozen rows separately, then visible_rows, each across columns. */
bool gui_data_table_state_init(GuiDataTableState* out);
bool gui_begin_data_table(GuiContext* ctx, GuiStr label, const GuiStr* headers, size_t columns, size_t rows, GuiDataTableState* state, GuiDataTableOptions options);
bool gui_begin_data_table_cell(GuiContext* ctx, size_t row, size_t column);
void gui_end_data_cell(GuiContext* ctx);
/* Store the returned horizontal offset in the caller state after closing. */
float gui_end_data_table(GuiContext* ctx);
/* Enter/blur commits valid text; Escape cancels. Invalid/nonfinite edits
 * preserve value, steppers clamp. False also means unchanged; inspect errors. */
bool gui_input_float(GuiContext* ctx, GuiStr label, float* value, GuiNumericEdit* edit, float step, float min, float max);
bool gui_input_int(GuiContext* ctx, GuiStr label, int32_t* value, GuiNumericEdit* edit, int32_t step, int32_t min, int32_t max);
bool gui_begin_menu_bar(GuiContext* ctx, GuiStr label);
bool gui_begin_menu(GuiContext* ctx, GuiStr label, GuiVec2 size);
bool gui_begin_submenu(GuiContext* ctx, GuiStr label, GuiVec2 size);
bool gui_menu_item(GuiContext* ctx, GuiStr label, GuiStr hint, uint8_t enabled);
void gui_end_menu(GuiContext* ctx);
void gui_end_menu_bar(GuiContext* ctx);
bool gui_begin_property_grid(GuiContext* ctx, GuiStr label, float label_width, float row_height);
bool gui_begin_property_row(GuiContext* ctx, GuiStr label);
void gui_end_property_row(GuiContext* ctx);
void gui_end_property_grid(GuiContext* ctx);

/* Bounded data plots/editors. Uniform plot samples are borrowed; NaN/Inf are
* gaps. Display geometry is capped independently of sample count. flags choose
* automatic range/axes; manual range must be finite/increasing. SIZE_MAX in
* hover.index means no hovered sample; false preserves output and reports error.
* Editors own retained buffers; keep one handle per stable widget ID, free once.
* Replacements validate before mutation. Curve keys use normalized time/value,
* strict time order and slopes in [-10000,10000]. Gradient endpoints stay 0/1.
* GUI editor false also means unchanged; inspect gui_last_error_code. */
#define GUI_CURVE_MAX_KEYS 64u
#define GUI_GRADIENT_EDITOR_MAX_STOPS 16u
#define GUI_CURVE_STEP 0u
#define GUI_CURVE_LINEAR 1u
#define GUI_CURVE_CUBIC 2u
#define GUI_PLOT_AUTO_RANGE 1u
#define GUI_PLOT_AXES 2u
typedef struct GuiCurveKey { float time, value, in_slope, out_slope; uint32_t interpolation; } GuiCurveKey;
typedef struct GuiPlotOptions { GuiVec2 size, range; GuiColor color; uint32_t flags; } GuiPlotOptions;
typedef struct GuiPlotHover { size_t index; float value; } GuiPlotHover;
typedef struct GuiCurveEditor GuiCurveEditor;
typedef struct GuiGradientEditor GuiGradientEditor;
bool gui_plot(GuiContext* ctx, GuiStr label, const float* samples, size_t count, GuiPlotOptions options, GuiPlotHover* out);
bool gui_sparkline(GuiContext* ctx, GuiStr label, const float* samples, size_t count, GuiPlotOptions options, GuiPlotHover* out);
GuiCurveEditor* gui_curve_editor_new(const GuiCurveKey* data, size_t count);
void gui_curve_editor_free(GuiCurveEditor* editor);
size_t gui_curve_editor_count(GuiCurveEditor* editor);
bool gui_curve_editor_get(GuiCurveEditor* editor, size_t index, GuiCurveKey* out);
bool gui_curve_editor_replace(GuiCurveEditor* editor, const GuiCurveKey* data, size_t count);
bool gui_curve_editor_select(GuiCurveEditor* editor, int32_t index);
int32_t gui_curve_editor_selected(GuiCurveEditor* editor);
bool gui_curve_editor(GuiContext* ctx, GuiStr label, GuiCurveEditor* editor, GuiVec2 size);
GuiGradientEditor* gui_gradient_editor_new(const GuiGradientStop* data, size_t count);
void gui_gradient_editor_free(GuiGradientEditor* editor);
size_t gui_gradient_editor_count(GuiGradientEditor* editor);
bool gui_gradient_editor_get(GuiGradientEditor* editor, size_t index, GuiGradientStop* out);
bool gui_gradient_editor_replace(GuiGradientEditor* editor, const GuiGradientStop* data, size_t count);
bool gui_gradient_editor_select(GuiGradientEditor* editor, int32_t index);
int32_t gui_gradient_editor_selected(GuiGradientEditor* editor);
bool gui_gradient_editor(GuiContext* ctx, GuiStr label, GuiGradientEditor* editor, GuiVec2 size);
bool gui_curve_evaluate(GuiCurveEditor* editor, float time, float* out);
bool gui_gradient_sample(GuiGradientEditor* editor, float position, GuiColor* out);

#define GUI_TEXT_ALIGN_START 0u
#define GUI_TEXT_ALIGN_CENTER 1u
#define GUI_TEXT_ALIGN_END 2u
#define GUI_TEXT_CLIP 0u
#define GUI_TEXT_TRUNCATE 1u
#define GUI_TEXT_ELLIPSIS 2u

/* Nonzero wrap means true; reserved bytes ignored. Spacing is finite and
 * nonnegative. max_lines zero means unlimited. Zero initialization gives
 * unwrapped, start-aligned clipped text. */
typedef struct GuiTextOptions {
    uint8_t wrap;
    uint8_t reserved[3];
    uint32_t overflow, horizontal, vertical;
    float line_spacing;
    uint32_t max_lines;
} GuiTextOptions;
typedef struct GuiTextMetrics {
    GuiVec2 size;
    size_t lines;
    uint8_t truncated;
    uint8_t reserved[7];
} GuiTextMetrics;
/* Display text, registered face ID and literal straight RGBA. All strings
 * and the array must remain readable/stable through the call. */
typedef struct GuiTextSpan {
    GuiStr text;
    uint32_t font;
    GuiColor color;
} GuiTextSpan;

/* Multiline/rich layout. All require an active frame; drawing also requires
 * an open window. Validate UTF-8, faces, enums and geometry before mutation.
 * Invalid input returns zero metrics and INVALID_INPUT. No array conversion.
 * LF/CRLF/CR break lines; tabs advance four spaces. Wrapping crosses spans;
 * long words break at UTF-8 scalar boundaries. No shaping/grapheme clustering.
 * Ellipsis uses the first span on that line; boxes clip and fit complete lines.
 * Measures do not advance layout; blocks reserve width and measured height;
 * box draws use ordered finite rectangles and do not advance layout. */
GuiTextMetrics gui_measure_text_block(GuiContext* ctx, const uint8_t* text, size_t len,
    float width, GuiTextOptions options);
GuiTextMetrics gui_measure_rich_text(GuiContext* ctx, const GuiTextSpan* spans, size_t count,
    float width, GuiTextOptions options);
GuiTextMetrics gui_text_block(GuiContext* ctx, const uint8_t* text, size_t len,
    float width, GuiTextOptions options);
GuiTextMetrics gui_rich_text(GuiContext* ctx, const GuiTextSpan* spans, size_t count,
    float width, GuiTextOptions options);
GuiTextMetrics gui_draw_text_box(GuiContext* ctx, GuiRect rect, const uint8_t* text,
    size_t len, GuiColor color, GuiTextOptions options);
GuiTextMetrics gui_draw_rich_text_box(GuiContext* ctx, GuiRect rect, const GuiTextSpan* spans,
    size_t count, GuiTextOptions options);

/* The C form of the core's key binding, edited by `gui_keybind`. A flat
 * struct with an explicit `tag` rather than a tagged union: a host-held
 * garbage discriminant would be instant UB when read as a Rust enum, so
 * the tag is validated instead (GUI_ERROR_INVALID_INPUT on unknown tags,
 * key codes, or mouse button indices >= 5). */
typedef struct GuiKeyBinding {
    uint8_t tag;         /* one of the GUI_KEY_BINDING_* constants */
    uint16_t key;        /* for GUI_KEY_BINDING_KEY / _CHORD; ignored otherwise */
    uint8_t mouse_button; /* for GUI_KEY_BINDING_MOUSE; ignored otherwise */
    /* Chord modifiers; ignored unless tag is _CHORD. uint8_t, not bool:
     * any nonzero value means "held", so hosts may pass any byte pattern
     * safely (uninitialized memory included). */
    uint8_t ctrl;
    uint8_t shift;
    uint8_t alt;
} GuiKeyBinding;

/* A single vertex of emitted geometry; layout-identical to the core's
 * `DrawVert` (the draw-data vertex slice is a zero-copy cast). The core's
 * `DrawVert` is `#[repr(C)]` with this exact layout pinned as public ABI
 * (pos @0, uv @8, col @16, size 20, align 4, no padding); the Rust-side
 * `ffi_struct_layouts_match_core_layouts` test compares every field offset
 * against it, and the static_asserts below pin the C++ side. */
typedef struct GuiVert {
    float pos[2];   /* screen-space position, pixels, origin top-left, +y down */
    float uv[2];    /* texture coordinate in the owning command's texture */
    uint8_t col[4]; /* vertex color, RGBA, multiplied with the sampled texel */
} GuiVert;

/* A run of consecutive indices sharing a texture, clip rect and effects record.
 * `texture` is the core texture handle: 0 = font atlas, 1 = white texel,
 * >= 2 host-defined (see `gui_draw_image`). */
typedef struct GuiCmd {
    uint32_t elem_count; /* number of indices this command consumes */
    float clip_rect[4];  /* [x_min, y_min, x_max, y_max], screen-space pixels */
    uint32_t effects;    /* one-based effect table index; former offset-20 padding */
    uint64_t texture;    /* texture sampled by this command's vertices */
} GuiCmd;

/* The complete output of one frame. The slices BORROW the context: they
 * are valid until the next `gui_new_frame` call, and repeated
 * `gui_render` calls without an intervening `gui_new_frame` return
 * identical slices. The host must not hold them across a frame boundary.
 * A zero-initialized value (null pointers, zero counts) is always safe to
 * "render" as nothing. */
typedef struct GuiDrawData {
    const GuiVert *vertices;   /* `vertex_count` elements */
    uint32_t vertex_count;
    const uint32_t *indices;   /* `index_count` elements */
    uint32_t index_count;
    const GuiCmd *commands;    /* consume `indices` in order, `command_count` elements */
    uint32_t command_count;
} GuiDrawData;

/* A single keyboard or text event during a frame. `key` is the key code
 * (the GUI_KEY_* mapping) for press/release events; `ch` is the Unicode
 * scalar for GUI_KEY_EVENT_CHAR events and ignored otherwise. Events with
 * an unknown `kind`, an out-of-range `key`, or an invalid Unicode scalar
 * are IGNORED, never errors. */
typedef struct GuiKeyEvent {
    uint8_t kind; /* 0 = press, 1 = release, 2 = char */
    uint16_t key;
    uint32_t ch;
} GuiKeyEvent;

/* Everything the host observed from input devices during one frame. */
typedef struct GuiInput {
    float mouse_pos[2];          /* cursor position, pixels, origin top-left */
    uint8_t mouse_down[5];       /* button state at frame end: [left, right, middle,
                                    back, forward] (back/forward = thumb buttons, Win32
                                    X1/X2); any nonzero value means down (any byte
                                    pattern is safe) */
    float mouse_wheel;           /* vertical wheel delta this frame, in lines */
    float viewport_size[2];      /* surface size in pixels; [0,0] = unknown */
    const GuiKeyEvent *keys;     /* `key_count` events in order, or null for none */
    size_t key_count;
} GuiInput;

/* ---------------------------------------------------------------------------
 * Layout asserts — the Rust side guarantees exactly these numbers (pinned
 * by `header_consistency.rs`); compiling this header verifies the C++ ABI
 * view matches them.
 * ------------------------------------------------------------------------- */

#ifdef __cplusplus
static_assert(sizeof(GuiCurveKey)==20 && alignof(GuiCurveKey)==4 && offsetof(GuiCurveKey,time)==0 && offsetof(GuiCurveKey,value)==4 && offsetof(GuiCurveKey,in_slope)==8 && offsetof(GuiCurveKey,out_slope)==12 && offsetof(GuiCurveKey,interpolation)==16, "GuiCurveKey ABI drift");
static_assert(sizeof(GuiPlotOptions)==24 && alignof(GuiPlotOptions)==4 && offsetof(GuiPlotOptions,size)==0 && offsetof(GuiPlotOptions,range)==8 && offsetof(GuiPlotOptions,color)==16 && offsetof(GuiPlotOptions,flags)==20, "GuiPlotOptions ABI drift");
static_assert(sizeof(GuiPlotHover)==2*sizeof(size_t) && alignof(GuiPlotHover)==alignof(size_t) && offsetof(GuiPlotHover,index)==0 && offsetof(GuiPlotHover,value)==sizeof(size_t), "GuiPlotHover ABI drift");
static_assert(sizeof(GuiVisibleRows)==2*sizeof(size_t) && alignof(GuiVisibleRows)==alignof(size_t) && offsetof(GuiVisibleRows,start)==0 && offsetof(GuiVisibleRows,end)==sizeof(size_t), "GuiVisibleRows ABI drift");
static_assert(sizeof(GuiDataTableState)==140 && alignof(GuiDataTableState)==4 && offsetof(GuiDataTableState,widths)==0 && offsetof(GuiDataTableState,scroll_x)==128 && offsetof(GuiDataTableState,sort_column)==132 && offsetof(GuiDataTableState,sort_direction)==136, "GuiDataTableState ABI drift");
static_assert(sizeof(GuiDataTableOptions)==20 && alignof(GuiDataTableOptions)==4 && offsetof(GuiDataTableOptions,size)==0 && offsetof(GuiDataTableOptions,row_height)==8 && offsetof(GuiDataTableOptions,frozen_columns)==12 && offsetof(GuiDataTableOptions,frozen_rows)==16, "GuiDataTableOptions ABI drift");
static_assert(sizeof(GuiVec2) == 8, "GuiVec2 ABI drift");
static_assert(sizeof(GuiRoundedRect) == 32 && alignof(GuiRoundedRect) == 4, "GuiRoundedRect ABI drift");
static_assert(sizeof(GuiGradientPaint)==28 && alignof(GuiGradientPaint)==4 && offsetof(GuiGradientPaint,kind)==0 && offsetof(GuiGradientPaint,origin)==4 && offsetof(GuiGradientPaint,vector)==12 && offsetof(GuiGradientPaint,spread)==20 && offsetof(GuiGradientPaint,phase)==24, "GuiGradientPaint ABI drift");
static_assert(sizeof(GuiAlphaMask)==40 && alignof(GuiAlphaMask)==8 && offsetof(GuiAlphaMask,texture)==0 && offsetof(GuiAlphaMask,rect)==8 && offsetof(GuiAlphaMask,uv)==24, "GuiAlphaMask ABI drift");
static_assert(sizeof(GuiGradientMask)==24 && alignof(GuiGradientMask)==4 && offsetof(GuiGradientMask,start)==0 && offsetof(GuiGradientMask,end)==8 && offsetof(GuiGradientMask,opacity)==16, "GuiGradientMask ABI drift");
static_assert(sizeof(GuiMaskOp)==48 && alignof(GuiMaskOp)==8 && offsetof(GuiMaskOp,texture)==0 && offsetof(GuiMaskOp,kind)==8 && offsetof(GuiMaskOp,reserved)==12 && offsetof(GuiMaskOp,rect)==16 && offsetof(GuiMaskOp,params)==32, "GuiMaskOp ABI drift");
static_assert(sizeof(GuiMaskStack)==200 && alignof(GuiMaskStack)==8 && offsetof(GuiMaskStack,ops)==0 && offsetof(GuiMaskStack,count)==192 && offsetof(GuiMaskStack,reserved)==196, "GuiMaskStack ABI drift");
static_assert(sizeof(GuiGradientFill)==176 && alignof(GuiGradientFill)==4 && offsetof(GuiGradientFill,rect)==0 && offsetof(GuiGradientFill,origin)==16 && offsetof(GuiGradientFill,vector)==24 && offsetof(GuiGradientFill,kind)==32 && offsetof(GuiGradientFill,spread)==36 && offsetof(GuiGradientFill,phase)==40 && offsetof(GuiGradientFill,stop_count)==44 && offsetof(GuiGradientFill,stops)==48, "GuiGradientFill ABI drift");
static_assert(sizeof(GuiSdfStyle)==28 && alignof(GuiSdfStyle)==4 && offsetof(GuiSdfStyle,kind)==0 && offsetof(GuiSdfStyle,width)==4 && offsetof(GuiSdfStyle,placement)==8 && offsetof(GuiSdfStyle,reach)==12 && offsetof(GuiSdfStyle,light_direction)==16 && offsetof(GuiSdfStyle,light_power)==24, "GuiSdfStyle ABI drift");
static_assert(sizeof(GuiRevealMask)==32 && alignof(GuiRevealMask)==4 && offsetof(GuiRevealMask,rect)==0 && offsetof(GuiRevealMask,direction)==16 && offsetof(GuiRevealMask,progress)==20 && offsetof(GuiRevealMask,softness)==24 && offsetof(GuiRevealMask,inverted)==28 && offsetof(GuiRevealMask,reserved)==29, "GuiRevealMask ABI drift");
static_assert(sizeof(GuiSdfShape)==64 && alignof(GuiSdfShape)==4 && offsetof(GuiSdfShape,rect)==0 && offsetof(GuiSdfShape,radii)==16 && offsetof(GuiSdfShape,parameters)==32 && offsetof(GuiSdfShape,light)==48, "GuiSdfShape ABI drift");
static_assert(sizeof(GuiSurfaceStyle)==128 && alignof(GuiSurfaceStyle)==4 && offsetof(GuiSurfaceStyle,flags)==0 && offsetof(GuiSurfaceStyle,phase)==4 && offsetof(GuiSurfaceStyle,pixel_scale)==8 && offsetof(GuiSurfaceStyle,noise)==16 && offsetof(GuiSurfaceStyle,grain)==20 && offsetof(GuiSurfaceStyle,dither)==24 && offsetof(GuiSurfaceStyle,dither_kind)==28 && offsetof(GuiSurfaceStyle,exposure)==32 && offsetof(GuiSurfaceStyle,contrast)==36 && offsetof(GuiSurfaceStyle,saturation)==40 && offsetof(GuiSurfaceStyle,desaturation)==44 && offsetof(GuiSurfaceStyle,tint)==48 && offsetof(GuiSurfaceStyle,scanlines)==64 && offsetof(GuiSurfaceStyle,spacing)==68 && offsetof(GuiSurfaceStyle,grid_width)==72 && offsetof(GuiSurfaceStyle,dot_radius)==76 && offsetof(GuiSurfaceStyle,vignette)==80 && offsetof(GuiSurfaceStyle,shimmer)==84 && offsetof(GuiSurfaceStyle,sweep_width)==88 && offsetof(GuiSurfaceStyle,sweep_speed)==92 && offsetof(GuiSurfaceStyle,distortion)==96 && offsetof(GuiSurfaceStyle,frequency)==100 && offsetof(GuiSurfaceStyle,chromatic)==104 && offsetof(GuiSurfaceStyle,refraction)==108 && offsetof(GuiSurfaceStyle,rect)==112,"GuiSurfaceStyle ABI drift");
static_assert(sizeof(GuiImageFilter)==64 && alignof(GuiImageFilter)==8 && offsetof(GuiImageFilter,kind)==0 && offsetof(GuiImageFilter,overwrite)==4 && offsetof(GuiImageFilter,radius)==8 && offsetof(GuiImageFilter,threshold)==12 && offsetof(GuiImageFilter,intensity)==16 && offsetof(GuiImageFilter,texel)==20 && offsetof(GuiImageFilter,reserved)==28 && offsetof(GuiImageFilter,secondary)==32 && offsetof(GuiImageFilter,params)==40,"GuiImageFilter ABI drift");
static_assert(sizeof(GuiBlur)==12 && alignof(GuiBlur)==4 && offsetof(GuiBlur,method)==0 && offsetof(GuiBlur,radius)==4 && offsetof(GuiBlur,levels)==8,"GuiBlur ABI drift");
static_assert(sizeof(GuiFilterStep)==80 && alignof(GuiFilterStep)==8 && offsetof(GuiFilterStep,source)==0 && offsetof(GuiFilterStep,size)==4 && offsetof(GuiFilterStep,reserved)==12 && offsetof(GuiFilterStep,filter)==16,"GuiFilterStep ABI drift");
static_assert(sizeof(GuiDrawEffects)==640 && alignof(GuiDrawEffects)==8 && offsetof(GuiDrawEffects,gradient)==0 && offsetof(GuiDrawEffects,masks)==176 && offsetof(GuiDrawEffects,sdf)==376 && offsetof(GuiDrawEffects,surface)==440 && offsetof(GuiDrawEffects,filter)==568 && offsetof(GuiDrawEffects,stencil)==632, "GuiDrawEffects ABI drift");
static_assert(offsetof(GuiCmd,effects)==20, "GuiCmd effect padding drift");
static_assert(offsetof(GuiRoundedRect, rect) == 0 && offsetof(GuiRoundedRect, radii) == 16, "GuiRoundedRect field drift");
static_assert(sizeof(GuiShadow) == 24 && alignof(GuiShadow) == 4, "GuiShadow ABI drift");
static_assert(offsetof(GuiShadow, offset) == 0 && offsetof(GuiShadow, spread) == 8 &&
    offsetof(GuiShadow, blur) == 12 && offsetof(GuiShadow, color) == 16 &&
    offsetof(GuiShadow, inset) == 20 && offsetof(GuiShadow, reserved) == 21, "GuiShadow field drift");
static_assert(sizeof(GuiStrokeStyle) == 20 && alignof(GuiStrokeStyle) == 4, "GuiStrokeStyle ABI drift");
static_assert(offsetof(GuiStrokeStyle, width) == 0 && offsetof(GuiStrokeStyle, join) == 4 &&
    offsetof(GuiStrokeStyle, cap) == 8 && offsetof(GuiStrokeStyle, miter_limit) == 12 &&
    offsetof(GuiStrokeStyle, tolerance) == 16, "GuiStrokeStyle field drift");
static_assert(sizeof(GuiLayoutOptions) == 8 && alignof(GuiLayoutOptions) == 4 &&
    offsetof(GuiLayoutOptions, wrap) == 0 && offsetof(GuiLayoutOptions, align_content) == 4, "GuiLayoutOptions ABI drift");
static_assert(sizeof(GuiGridCell) == 16 && alignof(GuiGridCell) == 4 &&
    offsetof(GuiGridCell, row) == 0 && offsetof(GuiGridCell, column) == 4 &&
    offsetof(GuiGridCell, row_span) == 8 && offsetof(GuiGridCell, column_span) == 12, "GuiGridCell ABI drift");
static_assert(sizeof(GuiLayoutSpec) == 64 && alignof(GuiLayoutSpec) == 4, "GuiLayoutSpec ABI drift");
static_assert(offsetof(GuiLayoutSpec, kind) == 0 && offsetof(GuiLayoutSpec, size) == 4 &&
    offsetof(GuiLayoutSpec, min) == 12 && offsetof(GuiLayoutSpec, max) == 20 &&
    offsetof(GuiLayoutSpec, padding) == 28 && offsetof(GuiLayoutSpec, gap) == 44 &&
    offsetof(GuiLayoutSpec, align) == 52 && offsetof(GuiLayoutSpec, justify) == 56 &&
    offsetof(GuiLayoutSpec, columns) == 60, "GuiLayoutSpec field drift");
static_assert(sizeof(GuiLayoutItem) == 76 && alignof(GuiLayoutItem) == 4, "GuiLayoutItem ABI drift");
static_assert(offsetof(GuiLayoutItem, preferred) == 0 && offsetof(GuiLayoutItem, min) == 8 &&
    offsetof(GuiLayoutItem, max) == 16 && offsetof(GuiLayoutItem, grow) == 24 &&
    offsetof(GuiLayoutItem, shrink) == 28 && offsetof(GuiLayoutItem, margin) == 32 &&
    offsetof(GuiLayoutItem, baseline) == 48 && offsetof(GuiLayoutItem, absolute) == 52 &&
    offsetof(GuiLayoutItem, hit_test) == 53 && offsetof(GuiLayoutItem, reserved) == 54 &&
    offsetof(GuiLayoutItem, anchor) == 56 && offsetof(GuiLayoutItem, offset) == 64 &&
    offsetof(GuiLayoutItem, layer) == 72, "GuiLayoutItem field drift");
static_assert(sizeof(GuiTextOptions) == 24 && alignof(GuiTextOptions) == 4, "GuiTextOptions ABI drift");
static_assert(offsetof(GuiTextOptions, wrap) == 0 && offsetof(GuiTextOptions, reserved) == 1 &&
    offsetof(GuiTextOptions, overflow) == 4 && offsetof(GuiTextOptions, horizontal) == 8 &&
    offsetof(GuiTextOptions, vertical) == 12 && offsetof(GuiTextOptions, line_spacing) == 16 &&
    offsetof(GuiTextOptions, max_lines) == 20, "GuiTextOptions field drift");
static_assert(sizeof(GuiTextMetrics) == 24 && alignof(GuiTextMetrics) == 8, "GuiTextMetrics ABI drift");
static_assert(offsetof(GuiTextMetrics, size) == 0 && offsetof(GuiTextMetrics, lines) == 8 &&
    offsetof(GuiTextMetrics, truncated) == 16 && offsetof(GuiTextMetrics, reserved) == 17, "GuiTextMetrics field drift");
static_assert(sizeof(GuiTextSpan) == 24 && alignof(GuiTextSpan) == 8, "GuiTextSpan ABI drift");
static_assert(offsetof(GuiTextSpan, text) == 0 && offsetof(GuiTextSpan, font) == 16 &&
    offsetof(GuiTextSpan, color) == 20, "GuiTextSpan field drift");
static_assert(offsetof(GuiVec2, y) == 4, "GuiVec2 ABI drift");
static_assert(sizeof(GuiRect) == 16, "GuiRect ABI drift");
static_assert(offsetof(GuiRect, max) == 8, "GuiRect ABI drift");
static_assert(sizeof(GuiDockState) == 12, "GuiDockState ABI drift");
static_assert(offsetof(GuiDockState, float_pos) == 4, "GuiDockState ABI drift");
static_assert(sizeof(GuiColor) == 4, "GuiColor ABI drift");
static_assert(sizeof(GuiScrollbarSkin) == 24, "GuiScrollbarSkin ABI drift");
static_assert(sizeof(GuiMotionSettings) == 32, "GuiMotionSettings ABI drift");
static_assert(sizeof(GuiKeyframe) == 12 && alignof(GuiKeyframe) == 4, "GuiKeyframe ABI drift");
static_assert(offsetof(GuiKeyframe, time) == 0 && offsetof(GuiKeyframe, value) == 4 && offsetof(GuiKeyframe, easing) == 8, "GuiKeyframe ABI drift");
static_assert(sizeof(GuiTimelineOptions) == 24 && alignof(GuiTimelineOptions) == 8, "GuiTimelineOptions ABI drift");
static_assert(offsetof(GuiTimelineOptions, mode) == 0 && offsetof(GuiTimelineOptions, speed) == 4 && offsetof(GuiTimelineOptions, playing) == 8 && offsetof(GuiTimelineOptions, reserved) == 9 && offsetof(GuiTimelineOptions, revision) == 16, "GuiTimelineOptions ABI drift");
static_assert(sizeof(GuiGradientStop) == 8 && alignof(GuiGradientStop) == 4 && offsetof(GuiGradientStop, position) == 0 && offsetof(GuiGradientStop, color) == 4, "GuiGradientStop ABI drift");
static_assert(sizeof(GuiGradientOptions) == 16 && alignof(GuiGradientOptions) == 4 && offsetof(GuiGradientOptions, axis) == 0 && offsetof(GuiGradientOptions, spread) == 8 && offsetof(GuiGradientOptions, resolution) == 12, "GuiGradientOptions ABI drift");
static_assert(sizeof(GuiThemeTokens) == 108 && alignof(GuiThemeTokens) == 4, "GuiThemeTokens ABI drift");
static_assert(offsetof(GuiThemeTokens, colors) == 0 && offsetof(GuiThemeTokens, frame_padding) == 64 && offsetof(GuiThemeTokens, item_spacing) == 72 && offsetof(GuiThemeTokens, window_padding) == 80, "GuiThemeTokens ABI drift");
static_assert(offsetof(GuiThemeTokens, slider_grab_size) == 88 && offsetof(GuiThemeTokens, scrollbar_size) == 92 && offsetof(GuiThemeTokens, corner_rounding) == 96 && offsetof(GuiThemeTokens, border_size) == 100, "GuiThemeTokens ABI drift");
static_assert(offsetof(GuiThemeTokens, anti_aliasing) == 104 && offsetof(GuiThemeTokens, reserved) == 105, "GuiThemeTokens ABI drift");
static_assert(alignof(GuiMotionSettings) == 4, "GuiMotionSettings ABI drift");
static_assert(offsetof(GuiMotionSettings, enabled) == 0, "GuiMotionSettings ABI drift");
static_assert(offsetof(GuiMotionSettings, reduced_motion) == 1, "GuiMotionSettings ABI drift");
static_assert(offsetof(GuiMotionSettings, reserved) == 2, "GuiMotionSettings ABI drift");
static_assert(offsetof(GuiMotionSettings, easing) == 4, "GuiMotionSettings ABI drift");
static_assert(offsetof(GuiMotionSettings, hover) == 8, "GuiMotionSettings ABI drift");
static_assert(offsetof(GuiMotionSettings, focus) == 12, "GuiMotionSettings ABI drift");
static_assert(offsetof(GuiMotionSettings, indicator) == 16, "GuiMotionSettings ABI drift");
static_assert(offsetof(GuiMotionSettings, popup) == 20, "GuiMotionSettings ABI drift");
static_assert(offsetof(GuiMotionSettings, collapse) == 24, "GuiMotionSettings ABI drift");
static_assert(offsetof(GuiMotionSettings, scroll) == 28, "GuiMotionSettings ABI drift");
static_assert(offsetof(GuiScrollbarSkin, track_click) == 20, "GuiScrollbarSkin ABI drift");
static_assert(sizeof(GuiDragPayload) == 48, "GuiDragPayload ABI drift");
static_assert(offsetof(GuiDragPayload, len) == 8, "GuiDragPayload ABI drift");
static_assert(offsetof(GuiDragPayload, bytes) == 9, "GuiDragPayload ABI drift");
static_assert(sizeof(GuiStr) == 16, "GuiStr ABI drift");
static_assert(offsetof(GuiStr, len) == 8, "GuiStr ABI drift");
static_assert(sizeof(GuiKeyBinding) == 8, "GuiKeyBinding ABI drift");
static_assert(offsetof(GuiKeyBinding, key) == 2, "GuiKeyBinding ABI drift");
static_assert(offsetof(GuiKeyBinding, mouse_button) == 4, "GuiKeyBinding ABI drift");
static_assert(offsetof(GuiKeyBinding, ctrl) == 5, "GuiKeyBinding ABI drift");
static_assert(offsetof(GuiKeyBinding, alt) == 7, "GuiKeyBinding ABI drift");
static_assert(sizeof(GuiVert) == 20, "GuiVert ABI drift");
static_assert(offsetof(GuiVert, uv) == 8, "GuiVert ABI drift");
static_assert(offsetof(GuiVert, col) == 16, "GuiVert ABI drift");
static_assert(sizeof(GuiCmd) == 32, "GuiCmd ABI drift");
static_assert(offsetof(GuiCmd, clip_rect) == 4, "GuiCmd ABI drift");
static_assert(offsetof(GuiCmd, texture) == 24, "GuiCmd ABI drift");
static_assert(sizeof(GuiDrawData) == 48, "GuiDrawData ABI drift");
static_assert(offsetof(GuiDrawData, vertex_count) == 8, "GuiDrawData ABI drift");
static_assert(offsetof(GuiDrawData, indices) == 16, "GuiDrawData ABI drift");
static_assert(offsetof(GuiDrawData, index_count) == 24, "GuiDrawData ABI drift");
static_assert(offsetof(GuiDrawData, commands) == 32, "GuiDrawData ABI drift");
static_assert(offsetof(GuiDrawData, command_count) == 40, "GuiDrawData ABI drift");
static_assert(sizeof(GuiKeyEvent) == 8, "GuiKeyEvent ABI drift");
static_assert(offsetof(GuiKeyEvent, key) == 2, "GuiKeyEvent ABI drift");
static_assert(offsetof(GuiKeyEvent, ch) == 4, "GuiKeyEvent ABI drift");
static_assert(sizeof(GuiInput) == 48, "GuiInput ABI drift");
static_assert(offsetof(GuiInput, mouse_down) == 8, "GuiInput ABI drift");
static_assert(offsetof(GuiInput, mouse_wheel) == 16, "GuiInput ABI drift");
static_assert(offsetof(GuiInput, viewport_size) == 20, "GuiInput ABI drift");
static_assert(offsetof(GuiInput, keys) == 32, "GuiInput ABI drift");
static_assert(offsetof(GuiInput, key_count) == 40, "GuiInput ABI drift");
#endif /* __cplusplus */

/* ---------------------------------------------------------------------------
 * Errors, lifecycle, frames, atlas
 * ------------------------------------------------------------------------- */

/* Returns the error code (GUI_ERROR_*) of the most recent `gui_*` call on
 * the calling thread. Does not itself reset or set the code. Never
 * unwinds. */
uint8_t gui_last_error_code(void);

/* Creates a context from TTF/OTF font bytes. Returns null — and sets
 * GUI_ERROR_INVALID_INPUT — when `font_bytes` is null, `len` is zero, or
 * the bytes are not a parseable font.
 * `font_bytes` must point to `len` readable bytes (or be null). */
GuiContext *gui_context_new(const uint8_t *font_bytes, size_t len);

/* Destroys a context created by `gui_context_new`. Null is a no-op.
 * Freeing the same context twice is UB (standard C ownership contract;
 * there is no double-free guard). */
void gui_context_free(GuiContext *ctx);

/* Begins a frame: feeds `input` to the GUI and starts UI declaration.
 * A null `input` means neutral input (mouse at origin, nothing held, no
 * keys, unknown viewport). A null `ctx` sets GUI_ERROR_INVALID_INPUT.
 * A second `gui_new_frame` before `gui_render` violates the frame
 * contract and is reported as GUI_ERROR_PANIC (the core asserts on it).
 * `input`'s `keys` pointer must be readable for `key_count` elements
 * when non-null. */
void gui_new_frame(GuiContext *ctx, const GuiInput *input, float dt);

/* Ends the frame and returns the assembled draw data (borrowed slices;
 * see the `GuiDrawData` docs). Calling it again without an intervening
 * `gui_new_frame` returns the same slices; before the first
 * `gui_new_frame` it returns zero-initialized data. A null `ctx` sets
 * GUI_ERROR_INVALID_INPUT and returns zeroed data.
 *
 * A begin without its matching end (window, collapsing header, tree node,
 * child, scroll region, tab bar) — or a `gui_push_font` without its `gui_pop_font` —
 * is reported here as GUI_ERROR_CONTRACT_VIOLATION: the
 * frame still renders, and the wrapper closes what can still be closed so
 * the host can continue. */
GuiDrawData gui_render(GuiContext *ctx);

/* Returns the font atlas as single-channel alpha pixels (row-major,
 * `*w` x `*h` bytes) for upload as the texture behind `GuiCmd::texture
 * == 0`. Borrows the context: re-fetch whenever `gui_atlas_generation`
 * changes, never hold across context-mutating calls. Null arguments set
 * GUI_ERROR_INVALID_INPUT and yield null. */
const uint8_t *gui_atlas_pixels(GuiContext *ctx, uint32_t *w, uint32_t *h);

/* Returns the atlas generation counter: when it changes, the texture
 * must be re-uploaded from `gui_atlas_pixels` before drawing. A null
 * `ctx` sets GUI_ERROR_INVALID_INPUT and yields 0. */
uint64_t gui_atlas_generation(GuiContext *ctx);

/* Opt-in grayscale quarter-physical-pixel coverage on both axes. Measurements
 * and advances stay unchanged; oversized/full-atlas phases retain the base
 * glyph. Set between frames, then re-upload if atlas generation changed.
 * Enabling primes cached glyphs. Ordinary unseen-text warmup still applies. */
void gui_set_text_raster_mode(GuiContext* ctx, uint32_t mode);
uint32_t gui_text_raster_mode(GuiContext* ctx);

/* ---------------------------------------------------------------------------
 * Fonts
 *
 * The context starts with one face (id 0, from `gui_context_new`'s bytes).
 * `gui_add_font` registers more faces BETWEEN FRAMES; `gui_push_font` /
 * `gui_pop_font` select the face subsequently declared text rasterizes
 * with. Every push must be paired with exactly one pop before
 * `gui_render`; mismatches the wrapper can check — an unknown id, a pop
 * with nothing open, a push left open at `gui_render` — are
 * GUI_ERROR_CONTRACT_VIOLATION (the core would panic; the wrapper checks
 * first, so it never does).
 * ------------------------------------------------------------------------- */

/* Registers a new font face from TTF/OTF `font_bytes` rasterized at
 * `px_size` pixels, returning its face id for `gui_push_font`. A
 * between-frames host operation: the face's ASCII glyphs are baked
 * synchronously, so the atlas may grow and `gui_atlas_generation` may
 * change — re-fetch `gui_atlas_pixels` then. Returns GUI_FONT_ID_INVALID
 * and sets GUI_ERROR_INVALID_INPUT (the same mapping `gui_context_new`
 * uses for font-load failures) when `ctx` or `font_bytes` is null, `len`
 * is zero, or the bytes do not parse / do not fit the atlas.
 * `font_bytes` must point to `len` readable bytes (or be null). */
uint32_t gui_add_font(GuiContext *ctx, const uint8_t *font_bytes, size_t len, float px_size);

/* Pushes the face `id` (from `gui_add_font`, or 0 for the default face)
 * onto the font scope stack: subsequently declared text rasterizes with
 * it until the matching `gui_pop_font`. Requires an active frame; an
 * unknown `id` is GUI_ERROR_CONTRACT_VIOLATION. */
void gui_push_font(GuiContext *ctx, uint32_t id);

/* Closes the font scope `gui_push_font` opened. With no font scope open
 * (or no frame active) this is GUI_ERROR_CONTRACT_VIOLATION. */
void gui_pop_font(GuiContext *ctx);

/* ---------------------------------------------------------------------------
 * Windows and containers (begin/end pairs)
 *
 * Every successful begin must be paired with its end exactly once, in reverse begin
 * order across ALL container types. A begin returning `false` (closed
 * window, header, or tree node) STILL pairs with its end call, which is
 * then a graceful no-op. An animated-layout false result means a rejected
 * begin and requires no end. A crossed end leaves the innermost scope open
 * and reports GUI_ERROR_CONTRACT_VIOLATION; close that scope first to
 * recover. `gui_tab` requires a tab bar as the innermost scope, and
 * `gui_end_window` requires every container to have ended. A begin left
 * without its end is reported by `gui_render`, which unwinds live scopes
 * in reverse order. Violations the wrapper cannot see (e.g. a widget
 * declared outside any window) are caught as GUI_ERROR_PANIC.
 * ------------------------------------------------------------------------- */

/* Sets the initial or current rectangle of the title-keyed window. Call
 * between frames. Later calls replace user drag or resize changes. Invalid
 * UTF-8 or a null title with nonzero length is GUI_ERROR_INVALID_INPUT;
 * calling during a frame is GUI_ERROR_CONTRACT_VIOLATION. */
void gui_set_window_rect(GuiContext *ctx, const uint8_t *title, size_t title_len,
    GuiVec2 pos, GuiVec2 size);

/* Applies one scrollbar skin to all scrollable windows and child panels.
 * Call between frames. Invalid geometry or track_click is INVALID_INPUT. */
void gui_set_scrollbar_skin(GuiContext *ctx, GuiScrollbarSkin skin);

/* Opens the window `title`'s scope with the scroll behavior
 * `scroll_flags` (a GUI_SCROLL_* bitmask; GUI_SCROLL_NONE is the
 * non-scrolling window). Returns `true` when content may be
 * declared (call `gui_end_window` exactly once), `false` when the window
 * is closed — the matching `gui_end_window` is then a graceful no-op. A
 * nested begin (a window already open), or an undefined `scroll_flags`
 * bit, is GUI_ERROR_CONTRACT_VIOLATION and opens no window. */
bool gui_begin_window(GuiContext *ctx, const uint8_t *title, size_t title_len,
    uint32_t scroll_flags);

/* Closes the window scope. With no window open or with a container still
 * open this is GUI_ERROR_CONTRACT_VIOLATION and leaves the window open for
 * recovery; after a closed window's `false` begin it is a no-op. */
void gui_end_window(GuiContext *ctx);

/* One active popup at a time. IDs come from gui_widget_id and are stable
 * within the declaring scope. Escape closes either popup kind. An ordinary
 * popup also closes on an outside left press; a modal consumes that press. */
void gui_open_popup(GuiContext *ctx, uint64_t id);
void gui_open_modal(GuiContext *ctx, uint64_t id);
void gui_close_popup(GuiContext *ctx);
bool gui_is_popup_open(GuiContext *ctx, uint64_t id);
/* Pair every begin with gui_end_popup, including false results. Contents
 * may use ordinary widgets and draw into the top overlay. Popup scopes
 * cannot nest. Rectangles are absolute screen coordinates. */
bool gui_begin_popup(GuiContext *ctx, uint64_t id, GuiRect rect);
bool gui_begin_modal(GuiContext *ctx, uint64_t id, GuiRect rect);
bool gui_begin_context_menu(GuiContext *ctx, uint64_t id, GuiRect target, GuiVec2 size);
void gui_end_popup(GuiContext *ctx);

/* Opens the collapsing header `title`, returning `true` when it is open
 * and content may be declared (headers start closed). Same pairing rules
 * as the window scope. */
bool gui_begin_collapsing_header(GuiContext *ctx, const uint8_t *title, size_t title_len);

/* Closes the collapsing header scope. With no header open this is
 * GUI_ERROR_CONTRACT_VIOLATION; after a closed header's `false` begin it
 * is a no-op. */
void gui_end_collapsing_header(GuiContext *ctx);

/* Opens a persistent tree node (initially closed). A false result skips
 * child declarations; its matching end call is a no-op. */
bool gui_begin_tree_node(GuiContext *ctx, const uint8_t *label, size_t label_len);

/* Closes the tree node opened by gui_begin_tree_node. */
void gui_end_tree_node(GuiContext *ctx);

/* Declares the scroll region `label` of `size` (pixels) and opens its
 * content scope; call `gui_end_scroll_region` exactly once afterwards.
 * Sugar for `gui_begin_child` with GUI_SCROLL_SCROLLABLE, but a DISTINCT
 * scope kind — see `gui_end_child`. */
void gui_begin_scroll_region(GuiContext *ctx, const uint8_t *label, size_t label_len, GuiVec2 size);

/* Closes the scroll region scope. With no region open — or with a
 * `gui_begin_child` scope innermost instead — this is
 * GUI_ERROR_CONTRACT_VIOLATION, and the mismatched scope stays open. */
void gui_end_scroll_region(GuiContext *ctx);

/* Declares the child container `label` of `width` x `height` pixels with
 * the scroll behavior `scroll_flags` selects (a GUI_SCROLL_* bitmask) and
 * opens its content scope; call `gui_end_child` exactly once afterwards.
 * A fixed-size, clipped region that claims its declared size in the
 * surrounding layout whether or not it scrolls. An undefined
 * `scroll_flags` bit is GUI_ERROR_CONTRACT_VIOLATION and opens NO scope,
 * so `gui_end_child` must not be called for it. */
void gui_begin_child(GuiContext *ctx, const uint8_t *label, size_t label_len,
    float width, float height, uint32_t scroll_flags);

/* Closes the child container scope `gui_begin_child` opened. With no
 * child open — or with a `gui_begin_scroll_region` scope innermost
 * instead — this is GUI_ERROR_CONTRACT_VIOLATION, and the mismatched
 * scope stays open. The two pairs are deliberately NOT interchangeable
 * even though a scroll region is a child with GUI_SCROLL_SCROLLABLE: the
 * wrapper tags each open scope with the pair that opened it, which is the
 * only thing that can tell a crossed end apart. */
void gui_end_child(GuiContext *ctx);

/* Fixed equal-cell table. Each cell is a clipped scope for ordinary widgets.
 * Pair every begin with its matching end before opening another cell. */
void gui_begin_table(GuiContext *ctx, const uint8_t *label, size_t label_len,
    GuiVec2 size, size_t columns, size_t rows);
void gui_begin_table_cell(GuiContext *ctx, size_t row, size_t column);
void gui_end_panel_cell(GuiContext *ctx);
void gui_end_table(GuiContext *ctx);

/* Two clipped panes with a draggable divider; ratio is caller-owned. */
void gui_begin_split(GuiContext *ctx, const uint8_t *label, size_t label_len,
    GuiVec2 size, uint32_t axis, float *ratio);
void gui_begin_split_pane(GuiContext *ctx, size_t index);
void gui_end_split(GuiContext *ctx);

/* Child panel with a visible bottom-right resize grip. */
void gui_begin_resizable_child(GuiContext *ctx, const uint8_t *label, size_t label_len,
    GuiVec2 *size, GuiVec2 min_size, uint32_t scroll_flags);
void gui_end_resizable_child(GuiContext *ctx);

/* Dock panel headers drag to left/right dock lanes; release elsewhere to
 * float. GuiDockState persists placement across frames in host memory. */
void gui_begin_dockspace(GuiContext *ctx, const uint8_t *label, size_t label_len, GuiVec2 size);
void gui_begin_dock_panel(GuiContext *ctx, const uint8_t *label, size_t label_len,
    GuiVec2 floating_size, GuiDockState *placement);
void gui_end_dock_panel(GuiContext *ctx);
void gui_end_dockspace(GuiContext *ctx);

/* Labeled clipped controls panel. */
void gui_begin_group_box(GuiContext *ctx, const uint8_t *label, size_t label_len, float width, float height);
void gui_end_group_box(GuiContext *ctx);

/* Clipped preview drawing area; returns its content rectangle through out_rect. */
bool gui_begin_preview_canvas(GuiContext *ctx, const uint8_t *label, size_t label_len,
    float width, float height, GuiRect *out_rect);
void gui_end_preview_canvas(GuiContext *ctx);

/* Fades nested controls and suppresses their input while disabled. */
void gui_begin_disabled(GuiContext *ctx, const uint8_t *label, size_t label_len, bool disabled);
void gui_end_disabled(GuiContext *ctx);

/* Opens the tab bar `label`; declare tabs with `gui_tab` and close with
 * `gui_end_tab_bar`. */
void gui_begin_tab_bar(GuiContext *ctx, const uint8_t *label, size_t label_len);

/* Declares the tab `title` inside the innermost open tab bar, returning
 * `true` when it is the active tab (declare its content then). Calling
 * it without a tab bar on top is GUI_ERROR_CONTRACT_VIOLATION. */
bool gui_tab(GuiContext *ctx, const uint8_t *title, size_t title_len);

/* Closes the tab bar. With no bar open this is
 * GUI_ERROR_CONTRACT_VIOLATION. */
void gui_end_tab_bar(GuiContext *ctx);

/* ---------------------------------------------------------------------------
 * Leaf widgets
 *
 * Widgets returning `bool` report the interaction: `true` exactly on the
 * frame(s) the value changed / the click happened.
 * ------------------------------------------------------------------------- */

/* A static text label. */
void gui_label(GuiContext *ctx, const uint8_t *text, size_t text_len);

/* A clickable button; returns `true` exactly once per click — on the
 * frame the left mouse button is released inside it. */
bool gui_button(GuiContext *ctx, const uint8_t *label, size_t label_len);

/* A square icon-font button; side is the requested logical-pixel edge.
 * Returns true on a release inside the button. */
bool gui_icon_button(GuiContext *ctx, const uint8_t *label, size_t label_len, float side);

/* Inert full-width hairline. */
void gui_separator(GuiContext *ctx);

/* Inert RGBA swatch over an alpha checkerboard, with optional label. */
void gui_color_swatch(GuiContext *ctx, const uint8_t *label, size_t label_len,
    GuiColor color, GuiVec2 size);

/* Inert status badge with a tinted marker and optional text. */
void gui_status_badge(GuiContext *ctx, const uint8_t *label, size_t label_len,
    GuiColor color);

/* Inert framed hotkey hint. Adds visible brackets around the label. */
void gui_hotkey_hint(GuiContext *ctx, const uint8_t *label, size_t label_len);

/* A checkbox toggling the host's `value`; returns `true` on the frames
 * the value flips. A null `value` is GUI_ERROR_INVALID_INPUT. */
bool gui_checkbox(GuiContext *ctx, const uint8_t *label, size_t label_len, bool *value);

/* Compact pill indicator with ordinary click-to-toggle behavior. */
bool gui_pill_toggle(GuiContext *ctx, const uint8_t *label, size_t label_len, bool *value);

/* Horizontal choices; the host owns selected. Null items is legal for zero count. */
bool gui_segmented_control(GuiContext *ctx, const uint8_t *label, size_t label_len,
    const GuiStr *items, size_t item_count, size_t *selected);

/* A mutually exclusive choice. On release, selecting a different index
 * writes to selected and returns true. Null selected is invalid input. */
bool gui_radio_button(GuiContext *ctx, const uint8_t *label, size_t label_len,
    size_t *selected, size_t index);

/* Semantic inline feedback: 0 success, 1 warning, 2 error. */
#define GUI_VALIDATION_SUCCESS 0
#define GUI_VALIDATION_WARNING 1
#define GUI_VALIDATION_ERROR 2
void gui_validation_message(GuiContext *ctx, uint8_t state, const uint8_t *message, size_t message_len);

/* Full-width navigation row and clickable path segments. */
bool gui_nav_row(GuiContext *ctx, const uint8_t *label, size_t label_len, size_t *selected, size_t index);
bool gui_breadcrumbs(GuiContext *ctx, const uint8_t *label, size_t label_len,
    const GuiStr *items, size_t item_count, size_t *selected);

/* Inert progress display. Fraction is clamped to the visible 0..1 range. */
void gui_progress_bar(GuiContext *ctx, const uint8_t *label, size_t label_len,
    float fraction, GuiVec2 size);

/* Inert loading spinner. Diameter is logical pixels, clamped to 0..512;
 * phase is a clockwise angle in radians supplied by the host. */
void gui_spinner(GuiContext *ctx, const uint8_t *label, size_t label_len,
    float diameter, float phase);

/* A float slider editing the host's `value` within `min..=max`; returns
 * `true` on the frames the value changes. */
bool gui_slider_float(
    GuiContext *ctx, const uint8_t *label, size_t label_len, float *value, float min, float max);

/* An integer slider editing the host's `value` within `min..=max`. */
bool gui_slider_int(
    GuiContext *ctx, const uint8_t *label, size_t label_len, int32_t *value, int32_t min, int32_t max);

/* Full-width recessed sliders with a left-to-right fill and inline text. */
bool gui_filled_slider_float(
    GuiContext *ctx, const uint8_t *label, size_t label_len, float *value, float min, float max);
bool gui_filled_slider_int(
    GuiContext *ctx, const uint8_t *label, size_t label_len, int32_t *value, int32_t min, int32_t max);

/* Two-handle sliders; lower and upper are distinct writable pointers.
 * The handles cannot cross. */
bool gui_range_slider_float(GuiContext *ctx, const uint8_t *label, size_t label_len,
    float *lower, float *upper, float min, float max);
bool gui_range_slider_int(GuiContext *ctx, const uint8_t *label, size_t label_len,
    int32_t *lower, int32_t *upper, int32_t min, int32_t max);

/* Relative horizontal drags; speed is value units per logical pixel. */
bool gui_drag_float(GuiContext *ctx, const uint8_t *label, size_t label_len,
    float *value, float speed, float min, float max);
bool gui_drag_int(GuiContext *ctx, const uint8_t *label, size_t label_len,
    int32_t *value, float speed, int32_t min, int32_t max);

/* Places the next widget on the same line as the previous one. */
void gui_same_line(GuiContext *ctx);

/* A single-line text field editing the HOST's buffer in place. `buf` is
 * a NUL-terminated C string of capacity `buf_cap` bytes (content plus
 * the terminator); the current content is the bytes up to the first NUL,
 * which must exist within `buf_cap`. The edited result is written back
 * CLAMPED to `buf_cap - 1` bytes at a UTF-8 char boundary, followed by
 * the terminating NUL — the export NEVER writes at or past
 * `buf[buf_cap]`. Returns `true` on the frames the content changes.
 * `buf_cap == 0`, a null `buf`, a missing NUL, or non-UTF-8 content is
 * GUI_ERROR_INVALID_INPUT. `buf` is `uint8_t*`; hosts holding the text in
 * a plain `char buf[N]` need a cast, e.g.
 * `gui_text_input(ctx, label, len, (uint8_t *)buf, sizeof buf)`. */
bool gui_text_input(GuiContext *ctx, const uint8_t *label, size_t label_len, uint8_t *buf, size_t buf_cap);

/* Standalone search field with clear action; buffer rules match gui_text_input. */
bool gui_search_box(GuiContext *ctx, const uint8_t *label, size_t label_len, uint8_t *buf, size_t buf_cap);

/* A combo box selecting one of `items` (a pointer to `item_count`
 * `GuiStr` entries; null with a zero count is the legal, inert empty
 * list). The host's `selected` index is written on a pick; returns `true`
 * on that frame. An out-of-range `*selected` shows an empty current
 * item. */
bool gui_combo(GuiContext *ctx, const uint8_t *label, size_t label_len,
    const GuiStr *items, size_t item_count, size_t *selected);

/* A searchable combo. `selected` and the NUL-terminated UTF-8 `query_buf`
 * belong to the host. The query clears when opened, edits while open, and
 * copies back clamped to `query_cap - 1` bytes at a UTF-8 boundary. Returns
 * true only when a result is chosen. Null items with zero count are legal.
 * Only the selected label is decoded while closed; open searches scan the
 * array in place without a whole-list temporary. Invalid accessed labels,
 * a null selection/query pointer, a zero query capacity, missing NUL, or
 * invalid query UTF-8 report GUI_ERROR_INVALID_INPUT. */
bool gui_searchable_combo(GuiContext *ctx, const uint8_t *label, size_t label_len,
    const GuiStr *items, size_t item_count, size_t *selected,
    uint8_t *query_buf, size_t query_cap);

/* A persistent scrollable list with a fixed `size` in logical pixels.
 * Clicking a different row updates `selected` and returns `true`. The
 * `items` array must hold `item_count` readable GuiStr entries, but only
 * visible labels are decoded each frame. Invalid UTF-8 in a visible label
 * is GUI_ERROR_INVALID_INPUT before selection changes; an offscreen label
 * is validated when scrolled into view. Null items with zero count are
 * legal and show an inert empty panel. */
bool gui_list_box(GuiContext *ctx, const uint8_t *label, size_t label_len,
    const GuiStr *items, size_t item_count, size_t *selected, GuiVec2 size);

/* A color editor (swatch + popup) editing the host's RGBA `value`
 * (exactly 4 readable/writable bytes); returns `true` on the frames the
 * value changes. */
bool gui_color_edit_rgba(GuiContext *ctx, const uint8_t *label, size_t label_len, uint8_t *value);

/* Rich SV/hue picker with sliders, hex input, and current/original previews.
 * `value` points to exactly 4 readable/writable RGBA bytes. */
bool gui_color_picker_rgba(GuiContext *ctx, const uint8_t *label, size_t label_len, uint8_t *value);

/* A keybind selector capturing a key, mouse button, or modifier chord
 * into the host's `binding`; returns `true` on the capture frame. A
 * `binding` whose tag, key code, or mouse button index is unknown is
 * GUI_ERROR_INVALID_INPUT (see the `GuiKeyBinding` docs). */
bool gui_keybind(GuiContext *ctx, const uint8_t *label, size_t label_len, GuiKeyBinding *binding);

/* A tooltip anchored to the previously declared interactive widget,
 * shown while it is hovered. */
void gui_tooltip_text(GuiContext *ctx, const uint8_t *text, size_t text_len);
/* Timed inert notification, drawn above windows. Non-positive durations
 * are ignored; at most six toasts remain visible. */
void gui_toast(GuiContext *ctx, const uint8_t *text, size_t text_len, float seconds);

/* ---------------------------------------------------------------------------
 * Canvas primitives (draw-only, absolute screen coordinates, no layout
 * advance) plus text measuring and free text drawing.
 * ------------------------------------------------------------------------- */

/* A straight line segment of `thickness` pixels in `col`. */
void gui_draw_line(GuiContext *ctx, GuiVec2 p0, GuiVec2 p1, GuiColor col, float thickness);

/* A filled axis-aligned rect (reversed corners are normalized). */
void gui_draw_rect_filled(GuiContext *ctx, GuiVec2 min, GuiVec2 max, GuiColor col);

/* An axis-aligned rect outline of `thickness` pixels. */
void gui_draw_rect_outline(GuiContext *ctx, GuiVec2 min, GuiVec2 max, GuiColor col, float thickness);

/* A filled circle. */
void gui_draw_circle_filled(GuiContext *ctx, GuiVec2 center, float radius, GuiColor col);

/* A circle outline of `thickness` pixels. */
void gui_draw_circle_outline(GuiContext *ctx, GuiVec2 center, float radius, GuiColor col, float thickness);

/* A circular arc sweeping `angle_min..=angle_max` (radians) at `radius`,
 * stroked `thickness` pixels wide. */
void gui_draw_arc(GuiContext *ctx, GuiVec2 center, float radius,
    float angle_min, float angle_max, GuiColor col, float thickness);

/* A filled convex polygon of `point_count` points (null with a zero
 * count is a legal no-op). */
void gui_draw_polygon_filled(GuiContext *ctx, const GuiVec2 *points, size_t point_count, GuiColor col);

/* An image quad sampling the host texture `texture` (the `uint64_t`
 * handle carried through to `GuiCmd::texture`; 0 = font atlas, 1 = white
 * texel, >= 2 host-defined), stretched over `min..max`, tinted by
 * `tint`. UVs are not clamped or normalized. */
void gui_draw_image(GuiContext *ctx, uint64_t texture, GuiVec2 min, GuiVec2 max,
    GuiVec2 uv_min, GuiVec2 uv_max, GuiColor tint);

/* The pixel size `text` would occupy if drawn. Returns zero on error. */
GuiVec2 gui_measure_text(GuiContext *ctx, const uint8_t *text, size_t text_len);

/* Draws `text` in `col` with the pen's top-left at `pos` (absolute
 * screen coordinates), into the current window's draw list; no layout
 * advance. */
void gui_draw_text_at(GuiContext *ctx, GuiVec2 pos, const uint8_t *text, size_t text_len, GuiColor col);

/* ---------------------------------------------------------------------------
 * Custom-widget helpers: identity, layout, interaction capture, and
 * mouse queries — the FFI form of the pattern in `.docs/custom-widgets.md`.
 * ------------------------------------------------------------------------- */

/* The stable ID of a custom widget labeled `label` in the current scope
 * (the full label, `##` suffix included, feeds the hash). Derive once
 * per declaration, before `gui_allocate`. Returns 0 on error. */
uint64_t gui_widget_id(GuiContext *ctx, const uint8_t *label, size_t label_len);

/* Advances the layout cursor by `size` and returns the placed rect in
 * absolute screen coordinates. Returns the zero rect on error. */
GuiRect gui_allocate(GuiContext *ctx, GuiVec2 size);

/* The gated hover query: `true` when the mouse is inside `rect` and no
 * widget holds the interaction capture (plus the core's scroll-clip and
 * z-order gates). */
bool gui_is_hovered(GuiContext *ctx, GuiRect rect);

/* Makes `id` (from `gui_widget_id`) the widget capturing interaction —
 * typically on a press edge inside the widget's own rect. */
void gui_capture(GuiContext *ctx, uint64_t id);

/* Releases the interaction capture ONLY if it is held by `id`. */
void gui_release_capture(GuiContext *ctx, uint64_t id);

/* Whether `id` currently holds the interaction capture. */
bool gui_is_captured(GuiContext *ctx, uint64_t id);
void gui_focusable(GuiContext *ctx, uint64_t id, GuiRect rect);
bool gui_is_focused(GuiContext *ctx, uint64_t id);
void gui_focus(GuiContext *ctx, uint64_t id);
bool gui_keyboard_activated(GuiContext *ctx, uint64_t id);
bool gui_drag_source(GuiContext *ctx, uint64_t id, GuiRect rect, GuiDragPayload payload);
bool gui_drag_target(GuiContext *ctx, GuiRect rect, uint64_t type_id, GuiDragPayload *out);

/* The mouse cursor position this frame, absolute screen coordinates.
 * Returns zero on error. */
GuiVec2 gui_mouse_pos(GuiContext *ctx);

/* Mouse button queries; `button` is 0 = left, 1 = right, 2 = middle,
 * 3 = back, 4 = forward — any other index is GUI_ERROR_INVALID_INPUT.
 * `down` reports the held
 * state this frame; `pressed` / `released` are edge queries, true
 * exactly once per press / release. */
bool gui_mouse_down(GuiContext *ctx, size_t button);
bool gui_mouse_pressed(GuiContext *ctx, size_t button);
bool gui_mouse_released(GuiContext *ctx, size_t button);


/* Input/state: navigation edges are queued before gui_new_frame.
 * Popup/modal nesting retains at most eight levels. Modifiers are a bitmask. */
#define GUI_NAV_NEXT 0u
#define GUI_NAV_PREVIOUS 1u
#define GUI_NAV_LEFT 2u
#define GUI_NAV_RIGHT 3u
#define GUI_NAV_UP 4u
#define GUI_NAV_DOWN 5u
#define GUI_NAV_ACTIVATE 6u
#define GUI_NAV_CANCEL 7u
#define GUI_SHORTCUT_GLOBAL 0u
#define GUI_SHORTCUT_WINDOW 1u
#define GUI_SHORTCUT_POPUP 2u
#define GUI_MOD_CTRL 1u
#define GUI_MOD_SHIFT 2u
#define GUI_MOD_ALT 4u
#define GUI_VALUE_BOOL 0u
#define GUI_VALUE_INTEGER 1u
#define GUI_VALUE_NUMBER 2u
#define GUI_VALUE_COLOR 3u
#define GUI_VALUE_TEXT 4u
#define GUI_PANEL_RECT 0u
#define GUI_PANEL_SIZE 1u
#define GUI_PANEL_SPLIT 2u
#define GUI_PANEL_DOCK 3u
#define GUI_PANEL_SCROLL 4u
#define GUI_RELOAD_WAITING 0u
#define GUI_RELOAD_UNCHANGED 1u
#define GUI_RELOAD_APPLIED 2u
#define GUI_RELOAD_INVALID 3u
#define GUI_RELOAD_IO 4u
#define GUI_HISTORY_BEGIN 0u
#define GUI_HISTORY_END 1u
#define GUI_HISTORY_CANCEL 2u
typedef struct GuiPanelState { uint32_t kind; uint32_t mode; float values[4]; } GuiPanelState;
static_assert(sizeof(GuiPanelState) == 24, "GuiPanelState size");
static_assert(alignof(GuiPanelState) == 4, "GuiPanelState alignment");
static_assert(offsetof(GuiPanelState, kind) == 0, "GuiPanelState.kind");
static_assert(offsetof(GuiPanelState, mode) == 4, "GuiPanelState.mode");
static_assert(offsetof(GuiPanelState, values) == 8, "GuiPanelState.values");
/* Panel values: rect=min/max XY; size=WH; split=ratio (mode=axis);
 * dock=relative XY (mode=zone); scroll=offset. Unused values must be zero. */
bool gui_navigate(GuiContext* ctx, uint32_t event);
size_t gui_stack_depth(GuiContext* ctx, uint8_t modal);
bool gui_set_double_click_settings(GuiContext* ctx, float seconds, float distance);
bool gui_shortcut(GuiContext* ctx, uint64_t id, uint16_t key_code, uint32_t modifiers, uint32_t scope, uint64_t owner);
bool gui_double_clicked(GuiContext* ctx, uint64_t id, GuiRect rect, size_t button);
/* Typed payloads: bool=one byte 0/1; integer=i64 LE; number=f64 LE finite;
 * color=four RGBA bytes; text=UTF-8 <=4096 bytes. Names <=128 UTF-8 bytes.
 * Register between frames. Set/get may run during a frame. Config load
 * requires an exact match to the registered names and value types. */
bool gui_config_register(GuiContext* ctx, GuiStr name, uint32_t kind, const uint8_t* data, size_t len);
bool gui_config_set(GuiContext* ctx, GuiStr name, uint32_t kind, const uint8_t* data, size_t len);
size_t gui_config_get(GuiContext* ctx, GuiStr name, uint8_t* out, size_t cap);
uint32_t gui_config_kind(GuiContext* ctx, GuiStr name);
/* All save/load calls are between frames. Saves return the required size;
 * null/zero out is a query. Short outputs report INVALID_INPUT and remain
 * untouched. Failed loads leave existing state intact. Theme is editable
 * UTF-8 key=value text; config/layout are versioned binary snapshots. */
size_t gui_save_config(GuiContext* ctx, uint8_t* out, size_t cap);
bool gui_load_config(GuiContext* ctx, const uint8_t* data, size_t len);
size_t gui_save_theme(GuiContext* ctx, uint8_t* out, size_t cap);
bool gui_load_theme(GuiContext* ctx, const uint8_t* data, size_t len);
bool gui_set_layout_panel(GuiContext* ctx, GuiStr name, GuiPanelState state);
bool gui_get_layout_panel(GuiContext* ctx, GuiStr name, GuiPanelState* out);
size_t gui_save_layout(GuiContext* ctx, uint8_t* out, size_t cap);
bool gui_load_layout(GuiContext* ctx, const uint8_t* data, size_t len);
/* History stores caller-defined bytes, <=2 MiB per checkpoint, capacity 2..1024.
 * The caller applies returned snapshots. Group begin/record/end joins an edit.
 * Direction=-1 undo,+1 redo. Step null/zero queries size without moving;
 * successful copying advances once. Check can_step for empty snapshots.
 * Reset/step are between frames; record/group/can_step can run during frames. */
bool gui_history_reset(GuiContext* ctx, const uint8_t* data, size_t len, size_t capacity);
bool gui_history_record(GuiContext* ctx, const uint8_t* data, size_t len);
bool gui_history_can_step(GuiContext* ctx, int32_t direction);
size_t gui_history_step(GuiContext* ctx, int32_t direction, uint8_t* out, size_t cap);
bool gui_history_group(GuiContext* ctx, uint32_t action);
/* Explicit between-frame polling; UTF-8 path, interval 50..60000 ms.
 * Invalid/replaced/missing files retain the last good theme. Poll returns
 * WAITING, UNCHANGED, APPLIED, INVALID or IO. No background thread. */
bool gui_watch_style(GuiContext* ctx, GuiStr path, uint32_t interval_ms);
void gui_unwatch_style(GuiContext* ctx);
uint32_t gui_poll_style(GuiContext* ctx);

#ifdef __cplusplus
// Fit: 0 stretch, 1 contain, 2 cover. Zero size selects intrinsic/aspect-preserving size.
GuiRect gui_image(GuiContext*, GuiImageRegion, GuiVec2 size, uint32_t fit, GuiColor tint);
bool gui_image_button(GuiContext*, const uint8_t* label, size_t len, GuiImageRegion, GuiVec2 size, uint32_t fit, GuiColor tint);
void gui_draw_nine_slice(GuiContext*, GuiImageRegion, GuiRect, GuiNineSlice, GuiColor);
static_assert(sizeof(GuiImageRegion)==32 && offsetof(GuiImageRegion,size)==24, "image region ABI");
static_assert(sizeof(GuiNineSlice)==32 && offsetof(GuiNineSlice,destination)==16, "nine-slice ABI");
// Explicit bounded recordings. Replay does not reset the application's model.
struct GuiRecording;
GuiRecording* gui_recording_new();
void gui_recording_free(GuiRecording*);
bool gui_recording_push(GuiRecording*, const GuiInput*, float dt, const uint32_t* navigation, size_t count);
size_t gui_recording_save(GuiRecording*, uint8_t* out, size_t capacity);
bool gui_recording_load(GuiRecording*, const uint8_t* data, size_t len);
size_t gui_recording_len(const GuiRecording*);
bool gui_recording_seek(GuiRecording*, size_t index);
bool gui_replay_next(GuiContext*, GuiRecording*);
bool gui_update_display_metrics(GuiContext*, uint32_t physical_width, uint32_t physical_height, float scale);
// Inspection: kind 0 scope / 1 item, parent UINT32_MAX root; logical bounds.
// Flags: 1 interactive, 2 focusable, 4 disabled, 8 hovered, 16 captured, 32 focused, 64 explicit bounds.
struct GuiInspectionNode { uint64_t id; uint32_t parent,kind; float rect[4]; uint32_t flags,depth; };
static_assert(sizeof(GuiInspectionNode)==40 && offsetof(GuiInspectionNode,rect)==16 && offsetof(GuiInspectionNode,flags)==32, "inspection ABI");
bool gui_set_inspection_enabled(GuiContext*, bool enabled);
size_t gui_inspection_count(const GuiContext*);
bool gui_inspection_node(const GuiContext*, size_t index, GuiInspectionNode* out);
size_t gui_inspection_name(const GuiContext*, uint64_t id, uint8_t* out, size_t capacity);

// Explicit Unicode editing: existing input and draw structs stay unchanged.
#define GUI_EDIT_MAX_BYTES 65536u
#define GUI_EDIT_MULTILINE 1u
#define GUI_EDIT_WRAP 2u
#define GUI_EDIT_READ_ONLY 4u
#define GUI_DIRECTION_AUTO 0u
#define GUI_DIRECTION_LTR 1u
#define GUI_DIRECTION_RTL 2u
#define GUI_IME_PREEDIT 0u
#define GUI_IME_COMMIT 1u
#define GUI_IME_CANCEL 2u
#define GUI_IME_PREEDIT_HIDDEN 3u
#define GUI_CLIPBOARD_COPY 0u
#define GUI_CLIPBOARD_CUT 1u
#define GUI_CLIPBOARD_PASTE 2u
#define GUI_EDIT_DOCUMENT 0u
#define GUI_EDIT_SELECTION 1u
#define GUI_EDIT_PREEDIT 2u
struct GuiTextEditor;
struct GuiShapedText;
struct GuiTextEditOptions { GuiVec2 size; uint32_t flags,direction; };
struct GuiShapedOptions { float width; uint8_t wrap,reserved[3]; uint32_t direction; };
struct GuiTextSelection { size_t anchor,cursor; };
struct GuiTextEditResponse { uint64_t id; float rect[4]; uint8_t changed,focused,reserved[6]; };
struct GuiTextCaret { uint64_t owner; float rect[4]; uint8_t composing,reserved[7]; };
struct GuiClipboardRequest { uint64_t request_id,owner; uint32_t action,reserved; GuiStr text; };
struct GuiShapedCaret { size_t byte; GuiVec2 position; size_t line; };
// Text/selection offsets use UTF-8 bytes. Documents are at most 64 KiB, with no NUL.
// Handles are owned and freed once. All input/output buffers are disjoint from handles/context.
GuiTextEditor* gui_text_editor_new(GuiStr);
void gui_text_editor_free(GuiTextEditor*);
bool gui_text_editor_set(GuiTextEditor*, GuiStr);
// No terminator. Null/zero queries required bytes; short buffers remain untouched, InvalidInput.
size_t gui_text_editor_copy(GuiTextEditor*, uint32_t kind, uint8_t* out, size_t capacity);
bool gui_text_editor_select(GuiTextEditor*, size_t anchor, size_t cursor);
bool gui_text_editor_selection(GuiTextEditor*, GuiTextSelection* out);
bool gui_text_edit(GuiContext*, GuiStr label, GuiTextEditor*, GuiTextEditOptions, GuiTextEditResponse* out);
// Between frames. Preedit is transient; cursor offsets refer to supplied preedit text.
bool gui_text_event(GuiContext*, uint64_t owner, uint32_t kind, GuiStr, size_t anchor, size_t cursor);
// Logical rectangle; the host converts to physical coordinates for its candidate UI.
bool gui_text_caret(GuiContext*, GuiTextCaret* out);
// Borrowed request text expires at completion, next declaration or context free.
bool gui_clipboard_request(GuiContext*, GuiClipboardRequest* out);
// Between frames. Success=0 never cuts; success=1 acknowledges Copy/Cut or supplies Paste text.
bool gui_clipboard_complete(GuiContext*, uint64_t request_id, uint8_t success, GuiStr);
bool gui_clipboard_process_native(GuiContext*); // optional Win32 service; between frames
// At most 16 ordered distinct registered IDs, excluding primary. Configure between frames.
bool gui_set_font_fallback(GuiContext*, uint32_t primary, const uint32_t* ids, size_t count);
// Between frames: a caller-owned texture ID >= 2, or zero to disable.
bool gui_set_color_font_texture(GuiContext*, uint64_t texture);
// Borrow straight sRGB RGBA after declaration and before drawing. Empty returns
// null/zero dimensions. Upload when generation changes; re-query after mutation.
const uint8_t* gui_color_atlas_pixels(GuiContext*, uint32_t* w, uint32_t* h,
    size_t* len, uint64_t* generation);
size_t gui_font_fallback(GuiContext*, uint32_t primary, uint32_t* out, size_t capacity);
GuiShapedText* gui_shaped_text_new();
void gui_shaped_text_free(GuiShapedText*);
bool gui_shaped_text(GuiContext*, GuiShapedText*, GuiStr, GuiShapedOptions, GuiVec2* size);
bool gui_shaped_caret(GuiShapedText*, size_t byte, GuiShapedCaret* out);
bool gui_shaped_hit_test(GuiShapedText*, GuiVec2 layout_point, GuiShapedCaret* out);
static_assert(sizeof(GuiTextEditOptions)==16 && alignof(GuiTextEditOptions)==4 && offsetof(GuiTextEditOptions,size)==0 && offsetof(GuiTextEditOptions,flags)==8 && offsetof(GuiTextEditOptions,direction)==12, "GuiTextEditOptions ABI");
static_assert(sizeof(GuiShapedOptions)==12 && alignof(GuiShapedOptions)==4 && offsetof(GuiShapedOptions,width)==0 && offsetof(GuiShapedOptions,wrap)==4 && offsetof(GuiShapedOptions,reserved)==5 && offsetof(GuiShapedOptions,direction)==8, "GuiShapedOptions ABI");
static_assert(sizeof(GuiTextSelection)==16 && alignof(GuiTextSelection)==8 && offsetof(GuiTextSelection,anchor)==0 && offsetof(GuiTextSelection,cursor)==8, "GuiTextSelection ABI");
static_assert(sizeof(GuiTextEditResponse)==32 && alignof(GuiTextEditResponse)==8 && offsetof(GuiTextEditResponse,id)==0 && offsetof(GuiTextEditResponse,rect)==8 && offsetof(GuiTextEditResponse,changed)==24 && offsetof(GuiTextEditResponse,focused)==25 && offsetof(GuiTextEditResponse,reserved)==26, "GuiTextEditResponse ABI");
static_assert(sizeof(GuiTextCaret)==32 && alignof(GuiTextCaret)==8 && offsetof(GuiTextCaret,owner)==0 && offsetof(GuiTextCaret,rect)==8 && offsetof(GuiTextCaret,composing)==24 && offsetof(GuiTextCaret,reserved)==25, "GuiTextCaret ABI");
static_assert(sizeof(GuiClipboardRequest)==40 && alignof(GuiClipboardRequest)==8 && offsetof(GuiClipboardRequest,request_id)==0 && offsetof(GuiClipboardRequest,owner)==8 && offsetof(GuiClipboardRequest,action)==16 && offsetof(GuiClipboardRequest,reserved)==20 && offsetof(GuiClipboardRequest,text)==24, "GuiClipboardRequest ABI");
static_assert(sizeof(GuiShapedCaret)==24 && alignof(GuiShapedCaret)==8 && offsetof(GuiShapedCaret,byte)==0 && offsetof(GuiShapedCaret,position)==8 && offsetof(GuiShapedCaret,line)==16, "GuiShapedCaret ABI");

/* Optional accessibility/host services. Existing input/draw records are unchanged.
Roles: group=0 window=1 text=2 button=3 checkbox=4 radio=5 slider=6 text-input=7
combo=8 tab=9 tree-item=10 progress=11 image=12 list=13 list-item=14 menu-item=15.
States: disabled=1 focused=2 checked=4 selected=8 expanded=16 read-only=32 hidden=64 blocked=128.
Actions: focus=1 activate=2 set-number=4.
Cursors: default=0 pointer=1 text=2 resize-horizontal=3 resize-vertical=4
resize-diagonal=5 grab=6 grabbing=7 not-allowed=8.
Metadata/cursor declarations require a live emitting window/container.
Native handles attach/publish/drain/free on the HWND thread. Destroying the window
invalidates outstanding provider objects; free the bridge once after detachment. */
struct GuiAccessibilityNode { uint64_t id; uint32_t parent, role; float rect[4]; uint32_t states, actions; };
struct GuiDuplicateId { uint64_t id; float first[4], second[4]; };
struct GuiScreenReaderBridge;
bool gui_set_accessibility(GuiContext* ctx, bool on);
size_t gui_accessibility_count(GuiContext* ctx);
bool gui_accessibility_node(GuiContext* ctx, size_t index, GuiAccessibilityNode* out);
/* UTF-8 byte count, no terminator; null/zero queries size. Short buffers unchanged. */
size_t gui_accessibility_text(GuiContext* ctx, uint64_t id, bool value, uint8_t* out, size_t capacity);
/* Action kinds: focus=1, activate=2, set-number=4. */
bool gui_accessibility_action(GuiContext* ctx, uint64_t id, uint32_t kind, double number);
bool gui_accessibility_info(GuiContext* ctx, uint64_t id, uint32_t role, GuiStr value, uint32_t states, uint32_t actions);
bool gui_accessibility_name(GuiContext* ctx, uint64_t id, GuiStr name);
bool gui_set_hit_target(GuiContext* ctx, GuiVec2 minimum);
uint32_t gui_cursor_shape(GuiContext* ctx);
bool gui_request_cursor(GuiContext* ctx, uint64_t id, GuiRect rect, uint32_t shape);
void gui_invalidate(GuiContext* ctx);
/* Relative seconds; -1 means idle. Host animations register their own deadlines. */
float gui_next_redraw_in(GuiContext* ctx);
bool gui_request_redraw_in(GuiContext* ctx, float seconds);
bool gui_set_duplicate_id_diagnostics(GuiContext* ctx, bool on);
size_t gui_duplicate_id_count(GuiContext* ctx);
bool gui_duplicate_id(GuiContext* ctx, size_t index, GuiDuplicateId* out);
bool gui_apply_high_contrast(GuiContext* ctx);
GuiScreenReaderBridge* gui_screen_reader_attach(GuiContext* ctx, uintptr_t hwnd);
void gui_screen_reader_free(GuiScreenReaderBridge* handle);
bool gui_screen_reader_publish(GuiScreenReaderBridge* handle, GuiContext* ctx, GuiVec2 screen_origin, float scale);
size_t gui_screen_reader_drain(GuiScreenReaderBridge* handle, GuiContext* ctx);
uint32_t gui_host_services_overflow(GuiContext* ctx);
static_assert(sizeof(GuiAccessibilityNode)==40 && alignof(GuiAccessibilityNode)==8 && offsetof(GuiAccessibilityNode,id)==0 && offsetof(GuiAccessibilityNode,parent)==8 && offsetof(GuiAccessibilityNode,role)==12 && offsetof(GuiAccessibilityNode,rect)==16 && offsetof(GuiAccessibilityNode,states)==32 && offsetof(GuiAccessibilityNode,actions)==36, "GuiAccessibilityNode ABI");
static_assert(sizeof(GuiDuplicateId)==40 && alignof(GuiDuplicateId)==8 && offsetof(GuiDuplicateId,id)==0 && offsetof(GuiDuplicateId,first)==8 && offsetof(GuiDuplicateId,second)==24, "GuiDuplicateId ABI");
/* Optional renderer planning. Disabled by default; stencil/filter commands
   never reorder. Triangle ranges borrow this frame's GuiDrawData; quad vertices
   belong to the plan until its next successful prepare/free. Effects still
   reference gui_frame_effects from that same context frame. */
typedef struct GuiRenderPlan GuiRenderPlan;
#define GUI_RENDER_SORT_DISJOINT 1
#define GUI_RENDER_INSTANCE_QUADS 2
#define GUI_GEOMETRY_TRIANGLES 0
#define GUI_GEOMETRY_QUADS 1
#define GUI_ALPHA_INPUT_PREMULTIPLIED 1
#define GUI_ALPHA_OUTPUT_PREMULTIPLIED 2
#define GUI_ALPHA_SECONDARY_PREMULTIPLIED 4
typedef struct GuiPlannedDraw { GuiCmd command; uint32_t kind, first, count; } GuiPlannedDraw;
typedef struct GuiQuadInstance { GuiVert vertices[4]; } GuiQuadInstance;
GuiRenderPlan* gui_render_plan_new(void);
void gui_render_plan_free(GuiRenderPlan* plan);
bool gui_render_plan_prepare(GuiRenderPlan* plan, const GuiContext* ctx, uint32_t flags, float scale);
const GuiPlannedDraw* gui_render_plan_draws(const GuiRenderPlan* plan, size_t* count);
const GuiQuadInstance* gui_render_plan_instances(const GuiRenderPlan* plan, size_t* count);
uint32_t gui_render_plan_reordered(const GuiRenderPlan* plan);
bool gui_effect_uniform_alpha(const GuiDrawEffects* effect, uint32_t flags, uint8_t* output, size_t capacity);
static_assert(sizeof(GuiPlannedDraw)==48 && alignof(GuiPlannedDraw)==8 && offsetof(GuiPlannedDraw,command)==0 && offsetof(GuiPlannedDraw,kind)==32 && offsetof(GuiPlannedDraw,first)==36 && offsetof(GuiPlannedDraw,count)==40,"GuiPlannedDraw ABI");
static_assert(sizeof(GuiQuadInstance)==80 && alignof(GuiQuadInstance)==4 && offsetof(GuiQuadInstance,vertices)==0,"GuiQuadInstance ABI");
} /* extern "C" */
#endif

#endif /* RESTRICTED_GUI_H */
