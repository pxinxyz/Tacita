
#ifndef RESTRICTED_GUI_H
#define RESTRICTED_GUI_H

/* Hand-written. Kept honest by a test that compares these layouts against the Rust structs. Edit one side without the other and CI finds you. */

#include <cstddef>
#include <cstdint>

#ifdef __cplusplus
extern "C" {
#endif


#define GUI_ERROR_NONE 0
#define GUI_ERROR_CONTRACT_VIOLATION 1
#define GUI_ERROR_PANIC 2
#define GUI_ERROR_INVALID_INPUT 3


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

#define GUI_KEY_EVENT_PRESS 0
#define GUI_KEY_EVENT_RELEASE 1
#define GUI_KEY_EVENT_CHAR 2

#define GUI_KEY_BINDING_NONE 0
#define GUI_KEY_BINDING_KEY 1
#define GUI_KEY_BINDING_MOUSE 2
#define GUI_KEY_BINDING_CHORD 3


typedef struct GuiContext GuiContext;

typedef struct GuiVec2 {
    float x;
    float y;
} GuiVec2;

typedef struct GuiRect {
    GuiVec2 min;
    GuiVec2 max;
} GuiRect;

typedef struct GuiColor {
    uint8_t r;
    uint8_t g;
    uint8_t b;
    uint8_t a;
} GuiColor;

typedef struct GuiStr {
    const uint8_t *ptr; 
    size_t len;
} GuiStr;

typedef struct GuiKeyBinding {
    uint8_t tag;         
    uint16_t key;        
    uint8_t mouse_button; 
    uint8_t ctrl;
    uint8_t shift;
    uint8_t alt;
} GuiKeyBinding;

typedef struct GuiVert {
    float pos[2];   
    float uv[2];    
    uint8_t col[4]; 
} GuiVert;

typedef struct GuiCmd {
    uint32_t elem_count; 
    float clip_rect[4];  
    uint64_t texture;    
} GuiCmd;

typedef struct GuiDrawData {
    const GuiVert *vertices;   
    uint32_t vertex_count;
    const uint32_t *indices;   
    uint32_t index_count;
    const GuiCmd *commands;    
    uint32_t command_count;
} GuiDrawData;

typedef struct GuiKeyEvent {
    uint8_t kind; 
    uint16_t key;
    uint32_t ch;
} GuiKeyEvent;

typedef struct GuiInput {
    float mouse_pos[2];          
    uint8_t mouse_down[3];       
    float mouse_wheel;           
    float viewport_size[2];      
    const GuiKeyEvent *keys;     
    size_t key_count;
} GuiInput;


#ifdef __cplusplus
static_assert(sizeof(GuiVec2) == 8, "GuiVec2 ABI drift");
static_assert(offsetof(GuiVec2, y) == 4, "GuiVec2 ABI drift");
static_assert(sizeof(GuiRect) == 16, "GuiRect ABI drift");
static_assert(offsetof(GuiRect, max) == 8, "GuiRect ABI drift");
static_assert(sizeof(GuiColor) == 4, "GuiColor ABI drift");
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
static_assert(sizeof(GuiInput) == 40, "GuiInput ABI drift");
static_assert(offsetof(GuiInput, mouse_down) == 8, "GuiInput ABI drift");
static_assert(offsetof(GuiInput, mouse_wheel) == 12, "GuiInput ABI drift");
static_assert(offsetof(GuiInput, viewport_size) == 16, "GuiInput ABI drift");
static_assert(offsetof(GuiInput, keys) == 24, "GuiInput ABI drift");
static_assert(offsetof(GuiInput, key_count) == 32, "GuiInput ABI drift");
#endif


uint8_t gui_last_error_code(void);

GuiContext *gui_context_new(const uint8_t *font_bytes, size_t len);

void gui_context_free(GuiContext *ctx);

void gui_new_frame(GuiContext *ctx, const GuiInput *input, float dt);

GuiDrawData gui_render(GuiContext *ctx);

const uint8_t *gui_atlas_pixels(GuiContext *ctx, uint32_t *w, uint32_t *h);

uint64_t gui_atlas_generation(GuiContext *ctx);


bool gui_begin_window(GuiContext *ctx, const uint8_t *title, size_t title_len);

void gui_end_window(GuiContext *ctx);

bool gui_begin_collapsing_header(GuiContext *ctx, const uint8_t *title, size_t title_len);

void gui_end_collapsing_header(GuiContext *ctx);

void gui_begin_scroll_region(GuiContext *ctx, const uint8_t *label, size_t label_len, GuiVec2 size);

void gui_end_scroll_region(GuiContext *ctx);

void gui_begin_tab_bar(GuiContext *ctx, const uint8_t *label, size_t label_len);

bool gui_tab(GuiContext *ctx, const uint8_t *title, size_t title_len);

void gui_end_tab_bar(GuiContext *ctx);


void gui_label(GuiContext *ctx, const uint8_t *text, size_t text_len);

bool gui_button(GuiContext *ctx, const uint8_t *label, size_t label_len);

bool gui_checkbox(GuiContext *ctx, const uint8_t *label, size_t label_len, bool *value);

bool gui_slider_float(
    GuiContext *ctx, const uint8_t *label, size_t label_len, float *value, float min, float max);

bool gui_slider_int(
    GuiContext *ctx, const uint8_t *label, size_t label_len, int32_t *value, int32_t min, int32_t max);

void gui_same_line(GuiContext *ctx);

bool gui_text_input(GuiContext *ctx, const uint8_t *label, size_t label_len, uint8_t *buf, size_t buf_cap);

bool gui_combo(GuiContext *ctx, const uint8_t *label, size_t label_len,
    const GuiStr *items, size_t item_count, size_t *selected);

bool gui_color_edit_rgba(GuiContext *ctx, const uint8_t *label, size_t label_len, uint8_t *value);

bool gui_keybind(GuiContext *ctx, const uint8_t *label, size_t label_len, GuiKeyBinding *binding);

void gui_tooltip_text(GuiContext *ctx, const uint8_t *text, size_t text_len);


void gui_draw_line(GuiContext *ctx, GuiVec2 p0, GuiVec2 p1, GuiColor col, float thickness);

void gui_draw_rect_filled(GuiContext *ctx, GuiVec2 min, GuiVec2 max, GuiColor col);

void gui_draw_rect_outline(GuiContext *ctx, GuiVec2 min, GuiVec2 max, GuiColor col, float thickness);

void gui_draw_circle_filled(GuiContext *ctx, GuiVec2 center, float radius, GuiColor col);

void gui_draw_circle_outline(GuiContext *ctx, GuiVec2 center, float radius, GuiColor col, float thickness);

void gui_draw_arc(GuiContext *ctx, GuiVec2 center, float radius,
    float angle_min, float angle_max, GuiColor col, float thickness);

void gui_draw_polygon_filled(GuiContext *ctx, const GuiVec2 *points, size_t point_count, GuiColor col);

void gui_draw_image(GuiContext *ctx, uint64_t texture, GuiVec2 min, GuiVec2 max,
    GuiVec2 uv_min, GuiVec2 uv_max, GuiColor tint);

GuiVec2 gui_measure_text(GuiContext *ctx, const uint8_t *text, size_t text_len);

void gui_draw_text_at(GuiContext *ctx, GuiVec2 pos, const uint8_t *text, size_t text_len, GuiColor col);


uint64_t gui_widget_id(GuiContext *ctx, const uint8_t *label, size_t label_len);

GuiRect gui_allocate(GuiContext *ctx, GuiVec2 size);

bool gui_is_hovered(GuiContext *ctx, GuiRect rect);

void gui_capture(GuiContext *ctx, uint64_t id);

void gui_release_capture(GuiContext *ctx, uint64_t id);

bool gui_is_captured(GuiContext *ctx, uint64_t id);

GuiVec2 gui_mouse_pos(GuiContext *ctx);

bool gui_mouse_down(GuiContext *ctx, size_t button);
bool gui_mouse_pressed(GuiContext *ctx, size_t button);
bool gui_mouse_released(GuiContext *ctx, size_t button);

#ifdef __cplusplus
} 
#endif

#endif
