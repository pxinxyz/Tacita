// smoke.cpp — real C++ consumer of tacita.h: creates a context
// from a TTF font, declares one frame with a scrollable window holding a
// child container, a checkbox, a slider, and a button, renders it, and
// asserts the draw data is non-empty. Compiled
// and linked against the staticlib by build_smoke.sh; compiling it also
// compiles the header's layout static_asserts.
//
// Usage: smoke.exe [path-to-font.ttf]   (default: DejaVuSans in the repo)

// Portable file reading with plain fopen (MSVC deprecates it otherwise).
#define _CRT_SECURE_NO_WARNINGS

#include "../tacita.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

#define CHECK(cond, msg)                                                    \
    do {                                                                    \
        if (!(cond)) {                                                      \
            std::fprintf(stderr, "SMOKE FAIL: %s (line %d)\n", msg, __LINE__); \
            return 1;                                                       \
        }                                                                   \
    } while (0)

static std::vector<uint8_t> read_file(const char *path) {
    std::FILE *f = std::fopen(path, "rb");
    if (!f) {
        return {};
    }
    std::fseek(f, 0, SEEK_END);
    const long size = std::ftell(f);
    std::fseek(f, 0, SEEK_SET);
    std::vector<uint8_t> bytes(size > 0 ? static_cast<size_t>(size) : 0);
    if (!bytes.empty()) {
        const size_t got = std::fread(bytes.data(), 1, bytes.size(), f);
        bytes.resize(got);
    }
    std::fclose(f);
    return bytes;
}

int main(int argc, char **argv) {
    const char *font_path =
        argc > 1 ? argv[1] : "../../core/tests/assets/DejaVuSans.ttf";
    const std::vector<uint8_t> font = read_file(font_path);
    CHECK(!font.empty(), "could not read the font file");

    GuiContext *ctx = gui_context_new(font.data(), font.size());
    CHECK(ctx != nullptr, "gui_context_new returned null for a valid font");
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "context creation reported an error");
    const std::vector<uint8_t> color_font = read_file(argc > 2 ? argv[2] : "../../core/tests/assets/Noto-COLRv1.ttf");
    CHECK(!color_font.empty(), "color font file missing");
    const uint32_t emoji_face = gui_add_font(ctx, color_font.data(), color_font.size(), 16.0f);
    CHECK(emoji_face != GUI_FONT_ID_INVALID, "color font registration failed");
    CHECK(gui_set_font_fallback(ctx, 0, &emoji_face, 1), "color fallback configuration failed");
    CHECK(gui_set_color_font_texture(ctx, 90), "color texture configuration failed");
    GuiMotionSettings motion = gui_motion_settings(ctx);
    CHECK(motion.enabled == 0, "library motion default changed");
    CHECK(gui_ease(GUI_EASING_SMOOTH_STEP, 0.5f) == 0.5f, "easing helper failed");
    CHECK(gui_pulse(0.5f, 1.0f) == 1.0f, "pulse helper failed");
    motion.enabled = 1;
    gui_set_motion_settings(ctx, motion);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "motion settings rejected");
    GuiThemeTokens theme = gui_theme_tokens(ctx);
    theme.colors[GUI_THEME_DANGER] = GuiColor{255, 51, 88, 255};
    gui_set_theme_tokens(ctx, theme);
    CHECK(gui_theme_color(ctx, GUI_THEME_DANGER).r == 255, "semantic theme role failed");
    const GuiThemeTokens theme_mid = gui_interpolate_theme(theme, theme, 0.5f);
    CHECK(theme_mid.border_size == theme.border_size, "theme interpolation failed");

    gui_set_window_rect(ctx, reinterpret_cast<const uint8_t *>("smoke"), 5,
        GuiVec2{90.0f, 80.0f}, GuiVec2{360.0f, 420.0f});
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "gui_set_window_rect reported an error");

    const GuiStr config_name{reinterpret_cast<const uint8_t*>("enabled"), 7};
    const uint8_t enabled_payload[1] = {1};
    CHECK(gui_config_register(ctx, config_name, GUI_VALUE_BOOL, enabled_payload, 1), "config registration failed");
    std::vector<uint8_t> saved_config(gui_save_config(ctx, nullptr, 0));
    CHECK(gui_save_config(ctx, saved_config.data(), saved_config.size()) == saved_config.size(), "config save failed");
    CHECK(gui_load_config(ctx, saved_config.data(), saved_config.size()), "config load failed");
    std::vector<uint8_t> saved_theme(gui_save_theme(ctx, nullptr, 0));
    CHECK(gui_save_theme(ctx, saved_theme.data(), saved_theme.size()) == saved_theme.size(), "theme save failed");
    CHECK(gui_load_theme(ctx, saved_theme.data(), saved_theme.size()), "theme load failed");
    const GuiStr split_name{reinterpret_cast<const uint8_t*>("split"), 5};
    CHECK(gui_set_layout_panel(ctx, split_name, GuiPanelState{GUI_PANEL_SPLIT, GUI_SPLIT_HORIZONTAL, {0.4f,0,0,0}}), "panel registration failed");
    std::vector<uint8_t> saved_layout(gui_save_layout(ctx, nullptr, 0));
    gui_save_layout(ctx, saved_layout.data(), saved_layout.size());
    CHECK(gui_load_layout(ctx, saved_layout.data(), saved_layout.size()), "layout load failed");
    CHECK(gui_history_reset(ctx, enabled_payload, 1, 8), "history initialization failed");
    const uint8_t other_payload[1] = {0};
    CHECK(gui_history_record(ctx, other_payload, 1), "history checkpoint failed");
    CHECK(gui_history_step(ctx, -1, nullptr, 0) == 1 && gui_history_can_step(ctx, -1), "history query moved cursor");
    uint8_t restored_payload = 0;
    CHECK(gui_history_step(ctx, -1, &restored_payload, 1) == 1 && restored_payload == 1, "undo failed");
    CHECK(gui_set_double_click_settings(ctx, 0.3f, 4.0f), "click settings failed");
    CHECK(gui_navigate(ctx, GUI_NAV_NEXT), "semantic navigation failed");

    // A second face from the same TTF at a different pixel size (a
    // between-frames call, before the first frame).
    const uint32_t big_face = gui_add_font(ctx, font.data(), font.size(), 28.0f);
    CHECK(big_face != GUI_FONT_ID_INVALID, "gui_add_font failed for valid font bytes");
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "gui_add_font reported an error");
    const char* svg_source = "<svg viewBox='0 0 24 24'><path d='M12 2Q12 12 22 12Q12 12 12 22Q12 12 2 12Q12 12 12 2Z'/></svg>";
    const uint32_t svg_icon = gui_add_svg_icon(ctx, reinterpret_cast<const uint8_t*>(svg_source),
        std::strlen(svg_source), GuiVec2{24,24});
    CHECK(svg_icon != GUI_SVG_ICON_INVALID && gui_last_error_code() == GUI_ERROR_NONE, "SVG registration failed");
    CHECK(gui_text_raster_mode(ctx) == GUI_TEXT_RASTER_LEGACY, "text raster default changed");
    gui_set_text_raster_mode(ctx, GUI_TEXT_RASTER_SUBPIXEL);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE && gui_text_raster_mode(ctx) == GUI_TEXT_RASTER_SUBPIXEL,
        "subpixel coverage selection failed");

    // Garbage bytes are rejected with the invalid-id sentinel and set the
    // per-thread error code.
    const uint8_t garbage[16] = {0xAB};
    CHECK(gui_add_font(ctx, garbage, sizeof(garbage), 16.0f) == GUI_FONT_ID_INVALID,
        "gui_add_font accepted garbage font bytes");
    CHECK(gui_last_error_code() == GUI_ERROR_INVALID_INPUT,
        "gui_add_font with garbage bytes did not report invalid input");

    // Atlas is available before any frame.
    uint32_t atlas_w = 0;
    uint32_t atlas_h = 0;
    const uint8_t *atlas = gui_atlas_pixels(ctx, &atlas_w, &atlas_h);
    CHECK(atlas != nullptr && atlas_w > 0 && atlas_h > 0, "atlas pixels unavailable");

    GuiInput input;
    std::memset(&input, 0, sizeof(input));
    input.viewport_size[0] = 800.0f;
    input.viewport_size[1] = 600.0f;

    gui_new_frame(ctx, &input, 0.016f);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "gui_new_frame reported an error");
    const float tween = gui_animate_float(ctx, 123, 0.0f, 1.0f, 0.4f, GUI_EASING_LINEAR);
    CHECK(tween > 0.0f && tween < 1.0f, "custom tween did not interpolate");
    CHECK(gui_animate_float(ctx, 123, 0.0f, 1.0f, 0.4f, GUI_EASING_LINEAR) == tween,
        "custom tween advanced twice in one frame");
    const float spring = gui_animate_spring(ctx, 124, 0.0f, 1.0f, 0.2f);
    CHECK(spring > 0.0f && spring < 1.0f, "custom spring did not interpolate");
    const GuiColor animated_color = gui_animate_color(ctx, 125, GuiColor{0,0,0,0},
        GuiColor{255,255,255,255}, 0.4f, GUI_EASING_LINEAR);
    CHECK(animated_color.a > 0 && animated_color.a < 255, "custom RGBA tween did not interpolate");

    bool checked = false;
    float volume = 0.5f;
    // A wheel-scrollable window with a drawn scrollbar track.
    const bool opened = gui_begin_window(ctx, reinterpret_cast<const uint8_t *>("smoke"), 5,
        GUI_SCROLL_SCROLLABLE | GUI_SCROLL_SCROLLBAR);
    CHECK(opened, "the smoke window did not open");
    const GuiRoundedRect rounded{GuiRect{GuiVec2{110,120},GuiVec2{280,210}},{24,0,12,8}};
    CHECK(gui_rounded_distance(rounded,GuiVec2{170,160}) < 0, "rounded distance failed");
    gui_draw_shadow(ctx, rounded, GuiShadow{GuiVec2{2,3},0,10,GuiColor{0,0,0,80},0,{0,0,0}});
    gui_draw_glow(ctx, rounded, GuiColor{99,91,255,40}, 8);
    gui_draw_rounded_rect(ctx, rounded, GuiColor{21,24,32,255});
    gui_draw_rounded_border(ctx, rounded, GuiColor{99,91,255,255}, 1, GUI_BORDER_INNER);
    CHECK(gui_begin_rounded_clip(ctx, reinterpret_cast<const uint8_t*>("rounded"), 7, rounded), "rounded clip failed");
    gui_draw_text_at(ctx, GuiVec2{114,126}, reinterpret_cast<const uint8_t*>("Clipped text"), 12, GuiColor{255,255,255,255});
    gui_end_rounded_clip(ctx);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "rounded scope end failed");
    const GuiKeyframe keys[] = {{0.0f, 0.0f, GUI_EASING_LINEAR}, {1.0f, 1.0f, GUI_EASING_CUBIC_OUT}};
    GuiTimelineOptions playback{};
    playback.mode = GUI_PLAYBACK_LOOP; playback.speed = 1.0f; playback.playing = 1;
    CHECK(gui_sample_keyframes(keys, 2, 0.5f) > 0.5f, "keyframe easing failed");
    CHECK(gui_animate_timeline(ctx, 600, keys, 2, playback) > 0.0f, "timeline clock failed");
    gui_seek_timeline(ctx, 600, 0.5f);
    GuiRect animated_region{};
    const GuiRect target_region{GuiVec2{110.0f, 120.0f}, GuiVec2{320.0f, 200.0f}};
    const GuiRect tweened_region = gui_animate_rect(ctx, 601, target_region, target_region, 0.3f, GUI_EASING_LINEAR);
    CHECK(tweened_region.min.x == target_region.min.x, "rectangle tween failed");
    CHECK(gui_begin_animated_layout(ctx, reinterpret_cast<const uint8_t*>("Animated"), 8,
        target_region, 0.3f, GUI_EASING_CUBIC_OUT, &animated_region), "animated layout failed");
    const GuiGradientStop stops[] = {{0.0f, GuiColor{0,0,0,255}}, {1.0f, GuiColor{99,91,255,255}}};
    const GuiGradientPaint paint{GUI_PAINT_RADIAL,GuiVec2{0.5f,0.5f},GuiVec2{0.5f,0.5f},GUI_GRADIENT_CLAMP,0.25f};
    CHECK(gui_begin_alpha_mask(ctx,GuiAlphaMask{1,rounded.rect,{0,0,0,0}}), "alpha mask begin failed");
    CHECK(gui_begin_gradient_mask(ctx,GuiGradientMask{GuiVec2{110,120},GuiVec2{320,120},{1,0}}), "gradient mask begin failed");
    CHECK(gui_begin_reveal_mask(ctx,GuiRevealMask{rounded.rect,GUI_REVEAL_RADIAL,0.5f,0.12f,0,{0,0,0}}), "reveal begin failed");
    const GuiSdfStyle sdf{GUI_SDF_BORDER,1,GUI_BORDER_INNER,8,GuiVec2{-1,-1},2};
    gui_draw_sdf_shape(ctx,rounded,GuiColor{226,229,237,255},sdf);
    gui_draw_sdf_shape_gradient(ctx,rounded,stops,2,paint,sdf);
    gui_end_mask(ctx);
    gui_draw_shape_gradient(ctx,rounded,stops,2,paint);
    gui_draw_gradient_border(ctx,rounded,stops,2,paint,1,GUI_BORDER_OUTER);
    gui_end_mask(ctx);gui_end_mask(ctx);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "paint/mask scopes failed");
    const GuiGradientOptions gradient{GuiVec2{1.0f, 0.0f}, GUI_GRADIENT_MIRROR, 24};
    gui_draw_gradient(ctx, animated_region, stops, 2, gradient);
    gui_draw_animated_gradient(ctx, animated_region, stops, 2, gradient, 0.25f, 2.0f);
    gui_button(ctx, reinterpret_cast<const uint8_t*>("Live"), 4);
    gui_end_animated_layout(ctx);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "animated scope did not close");
    GuiTextOptions text_options{};
    text_options.wrap = 1; text_options.overflow = GUI_TEXT_ELLIPSIS; text_options.max_lines = 2;
    const char* paragraph = "A multiline paragraph with wrapping and clipped text.";
    const GuiTextSpan text_spans[] = {
        {GuiStr{reinterpret_cast<const uint8_t*>("Rich "), 5}, 0, GuiColor{255,255,255,255}},
        {GuiStr{reinterpret_cast<const uint8_t*>("text across spans"), 17}, 0, GuiColor{99,91,255,255}}
    };
    CHECK(gui_measure_text_block(ctx, reinterpret_cast<const uint8_t*>(paragraph), std::strlen(paragraph), 160.0f, text_options).lines > 0, "text measurement failed");
    CHECK(gui_measure_rich_text(ctx, text_spans, 2, 160.0f, text_options).lines > 0, "rich measurement failed");
    gui_text_block(ctx, reinterpret_cast<const uint8_t*>(paragraph), std::strlen(paragraph), 160.0f, text_options);
    gui_rich_text(ctx, text_spans, 2, 160.0f, text_options);
    gui_draw_text_box(ctx, target_region, reinterpret_cast<const uint8_t*>(paragraph), std::strlen(paragraph), GuiColor{255,255,255,255}, text_options);
    gui_draw_rich_text_box(ctx, target_region, text_spans, 2, text_options);
    gui_svg_icon(ctx, svg_icon, GuiVec2{24,24}, GuiColor{120,160,255,255});
    gui_draw_svg_icon(ctx, svg_icon, target_region, GuiColor{120,160,255,128});
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "SVG emission failed");
    GuiLayoutSpec layout_spec{};
    layout_spec.kind = GUI_LAYOUT_ROW; layout_spec.size = GuiVec2{220,40};
    layout_spec.max = GuiVec2{1000000,1000000}; layout_spec.columns = 1;
    layout_spec.gap = GuiVec2{8,8}; layout_spec.align = GUI_LAYOUT_CENTER;
    GuiLayoutItem layout_items[2]{};
    for (auto& item : layout_items) {
        item.preferred = GuiVec2{90,30}; item.max = GuiVec2{1000000,1000000};
        item.grow = 1; item.shrink = 1; item.hit_test = 1;
    }
    layout_items[0].layer = 10;
    CHECK(gui_measure_layout(layout_spec, layout_items, 2).x == 220, "layout measurement failed");
    CHECK(gui_available_size(ctx).x > 0, "available size failed");
    GuiRect layout_rect{};
    CHECK(gui_begin_layout(ctx, reinterpret_cast<const uint8_t*>("layout"), 6,
        layout_spec, layout_items, 2, &layout_rect), "layout begin failed");
    CHECK(gui_layout_item_rect(ctx, 1).max.x <= layout_rect.max.x, "layout item geometry escaped");
    for (size_t i = 0; i < 2; ++i) {
        GuiRect item_rect{};
        CHECK(gui_begin_layout_item(ctx, i, &item_rect), "layout item begin failed");
        gui_button(ctx, reinterpret_cast<const uint8_t*>("item"), 4);
        gui_end_layout_item(ctx);
    }
    gui_end_layout(ctx);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "layout end failed");
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "text exports failed");

    // A child container scope inside it, with its own scrollbar, holding a
    // label and a button. Its end is `gui_end_child` — closing it with
    // `gui_end_scroll_region` would be a contract violation.
    gui_begin_child(ctx, reinterpret_cast<const uint8_t *>("child"), 5, 220.0f, 80.0f,
        GUI_SCROLL_SCROLLABLE | GUI_SCROLL_SCROLLBAR);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "gui_begin_child reported an error");
    gui_label(ctx, reinterpret_cast<const uint8_t *>("in child"), 8);
    const bool child_clicked = gui_button(ctx, reinterpret_cast<const uint8_t *>("Child"), 5);
    (void)child_clicked; // no interaction scripted
    gui_end_child(ctx);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "gui_end_child reported an error");

    // An undefined flag bit is rejected BY THE WRAPPER: contract
    // violation, no scope opened, so no end call is owed. (The next gui_*
    // call resets the per-thread error, so check immediately.)
    gui_begin_child(ctx, reinterpret_cast<const uint8_t *>("bad"), 3, 100.0f, 40.0f,
        GUI_SCROLL_SCROLLABLE | (1u << 4));
    CHECK(gui_last_error_code() == GUI_ERROR_CONTRACT_VIOLATION,
        "an undefined scroll flag bit was not reported as a contract violation");

    gui_checkbox(ctx, reinterpret_cast<const uint8_t *>("Enabled"), 7, &checked);
    CHECK(!gui_pill_toggle(ctx, reinterpret_cast<const uint8_t *>("Pill"), 4, &checked),
        "an idle pill toggle changed state");
    const GuiStr segments[] = {
        {reinterpret_cast<const uint8_t *>("Quiet"), 5},
        {reinterpret_cast<const uint8_t *>("Bright"), 6},
    };
    size_t segment_choice = 0;
    CHECK(!gui_segmented_control(ctx, reinterpret_cast<const uint8_t *>("Choice"), 6,
        segments, 2, &segment_choice), "an idle segmented control changed selection");
    size_t mode = 0;
    CHECK(!gui_radio_button(ctx, reinterpret_cast<const uint8_t *>("Mode A"), 6, &mode, 0),
        "an idle radio button changed selection");
    gui_progress_bar(ctx, reinterpret_cast<const uint8_t *>("Loading"), 7,
        0.4f, GuiVec2{180.0f, 18.0f});
    gui_separator(ctx);
    CHECK(!gui_icon_button(ctx, reinterpret_cast<const uint8_t *>("*##icon"), 7, 26.0f),
        "an idle icon button clicked");
    gui_color_swatch(ctx, reinterpret_cast<const uint8_t *>("Tint"), 4,
        GuiColor{99, 91, 255, 160}, GuiVec2{22.0f, 22.0f});
    gui_status_badge(ctx, reinterpret_cast<const uint8_t *>("live"), 4,
        GuiColor{0, 229, 153, 255});
    gui_hotkey_hint(ctx, reinterpret_cast<const uint8_t *>("M4"), 2);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "compact controls reported an error");
    const bool tree_open = gui_begin_tree_node(ctx,
        reinterpret_cast<const uint8_t *>("Branch"), 6);
    if (tree_open) {
        gui_label(ctx, reinterpret_cast<const uint8_t *>("Nested"), 6);
    }
    gui_end_tree_node(ctx);
    gui_slider_float(ctx, reinterpret_cast<const uint8_t *>("Volume"), 6, &volume, 0.0f, 1.0f);
    CHECK(!gui_filled_slider_float(ctx, reinterpret_cast<const uint8_t *>("Fill"), 4,
        &volume, 0.0f, 1.0f), "an idle filled float slider changed value");
    int32_t filled_count = 3;
    CHECK(!gui_filled_slider_int(ctx, reinterpret_cast<const uint8_t *>("Count"), 5,
        &filled_count, 0, 10), "an idle filled integer slider changed value");
    float float_lower = 0.2f, float_upper = 0.8f;
    int32_t int_lower = 2, int_upper = 8;
    CHECK(!gui_range_slider_float(ctx, reinterpret_cast<const uint8_t *>("Range F"), 7,
        &float_lower, &float_upper, 0.0f, 1.0f), "an idle float range changed values");
    CHECK(!gui_range_slider_int(ctx, reinterpret_cast<const uint8_t *>("Range I"), 7,
        &int_lower, &int_upper, 0, 10), "an idle integer range changed values");
    CHECK(!gui_drag_float(ctx, reinterpret_cast<const uint8_t *>("Drag F"), 6,
        &volume, 0.1f, 0.0f, 1.0f), "an idle float drag changed value");
    CHECK(!gui_drag_int(ctx, reinterpret_cast<const uint8_t *>("Drag I"), 6,
        &filled_count, 0.1f, 0, 10), "an idle integer drag changed value");
    gui_spinner(ctx, reinterpret_cast<const uint8_t *>("Loading"), 7, 22.0f, 1.0f);
    uint8_t tint[4] = {99, 91, 255, 255};
    CHECK(!gui_color_picker_rgba(ctx, reinterpret_cast<const uint8_t *>("Tint"), 4,
        tint), "an idle rich color picker changed value");
    const bool clicked = gui_button(ctx, reinterpret_cast<const uint8_t *>("Apply"), 5);
    (void)clicked; // no interaction scripted; the declaration is what matters
    const GuiStr list_items[] = {
        {reinterpret_cast<const uint8_t *>("one"), 3},
        {reinterpret_cast<const uint8_t *>("two"), 3},
    };
    size_t list_selected = 0;
    CHECK(!gui_list_box(ctx, reinterpret_cast<const uint8_t *>("List"), 4,
        list_items, 2, &list_selected, GuiVec2{180.0f, 90.0f}),
        "an idle list box unexpectedly changed selection");
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "gui_list_box reported an error");
    uint8_t search_query[32] = {};
    CHECK(!gui_searchable_combo(ctx, reinterpret_cast<const uint8_t *>("Search"), 6,
        list_items, 2, &list_selected, search_query, sizeof search_query),
        "an idle searchable combo unexpectedly changed selection");
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "searchable combo reported an error");
    // Text declared inside a font scope rasterizes with the second face.
    // An unknown face id is a contract violation, not a crash, and opens
    // no scope. (The next gui_* call resets the per-thread error, so check
    // immediately.)
    gui_push_font(ctx, 999);
    CHECK(gui_last_error_code() == GUI_ERROR_CONTRACT_VIOLATION,
        "gui_push_font with an unknown id was not reported as a contract violation");
    gui_push_font(ctx, big_face);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "gui_push_font reported an error");
    gui_label(ctx, reinterpret_cast<const uint8_t *>("Big face"), 8);
    gui_pop_font(ctx);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "gui_pop_font reported an error");
    GuiSurfaceStyle surface = gui_surface_default();
    surface.flags = GUI_SURFACE_GRID | GUI_SURFACE_DITHER;
    surface.dither_kind = GUI_DITHER_BLUE_NOISE;
    const GuiRoundedRect stencil_rect{GuiRect{100.0f,100.0f,240.0f,220.0f},{20.0f,20.0f,20.0f,20.0f}};
    CHECK(gui_begin_stencil_clip(ctx,GuiStr{reinterpret_cast<const uint8_t*>("Stencil"),7},stencil_rect),"stencil begin failed");
    gui_draw_surface(ctx,stencil_rect,1,0,GuiRect{0,0,1,1},GuiColor{255,255,255,255},surface);
    gui_end_stencil_clip(ctx);
    GuiFilterStep filter_steps[GUI_MAX_FILTER_PASSES]{};
    const size_t filter_count=gui_filter_plan(65,33,GuiBlur{GUI_BLUR_DUAL_KAWASE,4.0f,3},1,0.7f,1.0f,filter_steps,GUI_MAX_FILTER_PASSES);
    CHECK(filter_count==8 && filter_steps[filter_count-1].filter.kind==GUI_FILTER_BLOOM_COMPOSE,"filter plan failed");
    gui_end_window(ctx);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "widget declaration reported an error");

    const GuiDrawData draw = gui_render(ctx);
    GuiRenderPlan* render_plan=gui_render_plan_new();
    CHECK(render_plan!=nullptr && gui_render_plan_prepare(render_plan,ctx,GUI_RENDER_SORT_DISJOINT|GUI_RENDER_INSTANCE_QUADS,1.f),"render planning failed");
    size_t planned_count=0,instance_count=0;
    const GuiPlannedDraw* planned=gui_render_plan_draws(render_plan,&planned_count);
    const GuiQuadInstance* instances=gui_render_plan_instances(render_plan,&instance_count);
    CHECK(planned!=nullptr && planned_count>0 && instances!=nullptr && instance_count>0,"render plan output missing");
    for(size_t i=0;i<planned_count;++i) {
        const GuiPlannedDraw& run=planned[i];
        CHECK(run.kind<=GUI_GEOMETRY_QUADS,"unknown planned geometry");
        CHECK(run.first+run.count <= (run.kind==GUI_GEOMETRY_QUADS?instance_count:draw.index_count),"planned range outside buffer");
    }
    CHECK(!gui_render_plan_prepare(render_plan,ctx,8,1.f),"invalid planning flags accepted");
    size_t retained_count=0;
    CHECK(gui_render_plan_draws(render_plan,&retained_count)==planned && retained_count==planned_count,"rejected plan replaced valid output");
    gui_render_plan_free(render_plan);
    size_t effect_count=0;
    const GuiDrawEffects* effects=gui_frame_effects(ctx,&effect_count);
    CHECK(effects != nullptr && effect_count > 0, "effects table unavailable");
    bool saw_effect=false;
    for (uint32_t i=0;i<draw.command_count;++i) {
        CHECK(draw.commands[i].effects <= effect_count, "effect reference outside table");
        saw_effect |= draw.commands[i].effects != 0;
    }
    CHECK(saw_effect, "paint did not produce an effect reference");
    bool saw_sdf=false,saw_reveal=false;
    for (size_t i=0;i<effect_count;++i) {
        saw_sdf |= effects[i].sdf.parameters[0] == GUI_SDF_BORDER;
        for (uint32_t j=0;j<effects[i].masks.count;++j)
            saw_reveal |= effects[i].masks.ops[j].kind == GUI_REVEAL_RADIAL;
    }
    CHECK(saw_sdf && saw_reveal, "shader shape/reveal records missing");
    uint8_t effect_uniform[GUI_EFFECT_UNIFORM_SIZE]{};
    CHECK(gui_effect_uniform(effects,effect_uniform,sizeof(effect_uniform)), "shader uniform packing failed");
    CHECK(gui_effect_uniform_alpha(effects,GUI_ALPHA_INPUT_PREMULTIPLIED|GUI_ALPHA_OUTPUT_PREMULTIPLIED,effect_uniform,sizeof(effect_uniform)),"alpha uniform packing failed");
    float alpha_flags=0.f;std::memcpy(&alpha_flags,effect_uniform+52,4);
    CHECK(alpha_flags==3.f,"alpha uniform offset/value incorrect");
    GuiDrawEffects image_pass{};
    image_pass.filter = filter_steps[0].filter;
    CHECK(gui_effect_uniform(&image_pass,effect_uniform,sizeof(effect_uniform)), "host filter uniform packing failed");
    image_pass.filter.kind = 999;
    CHECK(!gui_effect_uniform(&image_pass,effect_uniform,sizeof(effect_uniform)) &&
        gui_last_error_code() == GUI_ERROR_INVALID_INPUT, "foreign filter enum was accepted");
    CHECK(gui_effect_uniform(effects,effect_uniform,sizeof(effect_uniform)), "frame uniform restoration failed");
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "gui_render reported an error");
    CHECK(draw.vertex_count > 0, "render produced no vertices");
    CHECK(draw.index_count > 0, "render produced no indices");
    CHECK(draw.command_count > 0, "render produced no commands");
    CHECK(draw.vertices != nullptr && draw.indices != nullptr && draw.commands != nullptr,
        "render produced null slices");

    // A second render without a new frame returns the identical slices
    // (documented lifetime).
    const GuiDrawData again = gui_render(ctx);
    CHECK(again.vertices == draw.vertices && again.vertex_count == draw.vertex_count,
        "repeated render did not return the cached slices");

    // A frame with mouse4 (back, index 3) held: the widened 5-button
    // GuiInput reaches the query exports; index 5 stays out of range.
    input.mouse_down[3] = 1;
    gui_new_frame(ctx, &input, 0.016f);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "mouse4 frame reported an error");
    CHECK(gui_begin_window(ctx, reinterpret_cast<const uint8_t *>("smoke"), 5,
              GUI_SCROLL_SCROLLABLE | GUI_SCROLL_SCROLLBAR),
        "the smoke window did not open");
    CHECK(gui_mouse_down(ctx, 3), "mouse4 down was not observed");
    CHECK(gui_mouse_pressed(ctx, 3), "mouse4 press edge was not observed");
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "mouse4 queries reported an error");
    CHECK(!gui_mouse_down(ctx, 5), "out-of-range button index was accepted");
    CHECK(gui_last_error_code() == GUI_ERROR_INVALID_INPUT,
        "out-of-range button index did not report invalid input");
    gui_end_window(ctx);
    const GuiDrawData draw2 = gui_render(ctx);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "mouse4 frame render reported an error");
    CHECK(draw2.vertex_count > 0, "mouse4 frame produced no vertices");

    // Widget-surface call with no frame active is a contract violation,
    // not a crash; the error is per-thread and recoverable.
    gui_label(ctx, reinterpret_cast<const uint8_t *>("x"), 1);
    CHECK(gui_last_error_code() == GUI_ERROR_CONTRACT_VIOLATION,
        "out-of-frame widget call was not reported as a contract violation");


    CHECK(gui_update_display_metrics(ctx,900,700,1.25f), "portable DPI rebuild failed");
    CHECK(gui_set_inspection_enabled(ctx,true), "inspection enable failed");
    GuiSpriteAtlas* sprites=gui_sprite_atlas_new(50,8,64,1);
    CHECK(sprites!=nullptr,"sprite atlas creation failed");
    uint8_t texels[16]{};for(auto& byte:texels)byte=255;
    const uint32_t sprite=gui_sprite_atlas_insert(sprites,texels,sizeof(texels),2,2);
    CHECK(sprite==0,"sprite insert failed");
    const GuiImageRegion image=gui_sprite_atlas_region(sprites,sprite);
    CHECK(image.texture==50&&image.size[0]==2,"sprite descriptor ABI failed");
    gui_new_frame(ctx,&input,0.016f);
    CHECK(gui_begin_window(ctx,reinterpret_cast<const uint8_t*>("images"),6,GUI_SCROLL_NONE),"image window failed");
    gui_image(ctx,image,{48,32},1,{255,255,255,255});
    gui_image_button(ctx,reinterpret_cast<const uint8_t*>("sprite"),6,image,{48,32},2,{255,255,255,255});
    GuiNineSlice slices{{1,1,1,1},{4,4,4,4}};
    gui_draw_nine_slice(ctx,image,{120,160,200,200},slices,{255,255,255,255});
    const GuiVec2 path_points[]{{100,100},{160,100},{185,140},{135,180}};
    const GuiStrokeStyle path_style{6,GUI_JOIN_ROUND,GUI_CAP_ROUND,4,0.25f};
    const GuiColor path_color{99,91,255,255};
    CHECK(gui_draw_polyline(ctx,path_points,4,0,path_style,path_color),"connected stroke failed");
    const float dash_lengths[]{15,7,3};
    CHECK(gui_draw_dashed_polyline(ctx,path_points,4,1,path_style,dash_lengths,3,-5,path_color),"closed odd dash failed");
    CHECK(gui_draw_quadratic_bezier(ctx,path_points,path_style,path_color),"quadratic failed");
    CHECK(gui_draw_cubic_bezier(ctx,path_points,path_style,path_color),"cubic failed");
    const GuiVec2 concave[]{{100,200},{180,200},{180,230},{130,230},{130,280},{100,280}};
    CHECK(gui_draw_concave_polygon(ctx,concave,6,path_color),"concave fill failed");
    auto bad_stroke=path_style;bad_stroke.join=999;
    CHECK(!gui_draw_polyline(ctx,path_points,4,0,bad_stroke,path_color)&&
        gui_last_error_code()==GUI_ERROR_INVALID_INPUT,"bad join accepted");
    GuiLayoutSpec extended_spec{};
    extended_spec.kind=GUI_LAYOUT_ROW;extended_spec.size={180,100};
    extended_spec.max={1000000,1000000};extended_spec.gap={8,8};
    extended_spec.align=GUI_LAYOUT_STRETCH;extended_spec.columns=3;
    GuiLayoutItem extended_items[3]{};
    for(auto& item:extended_items){item.preferred={80,30};item.min={24,20};
        item.max={1000000,1000000};item.grow=1;item.shrink=1;item.hit_test=1;}
    GuiLayoutOptions wrap_options{1,GUI_CONTENT_STRETCH};
    CHECK(gui_measure_layout_extended(extended_spec,extended_items,3,wrap_options,nullptr).y==100,
        "extended measure failed");
    GuiRect extended_rect{};
    CHECK(gui_begin_layout_extended(ctx,reinterpret_cast<const uint8_t*>("wrap"),4,
        extended_spec,extended_items,3,wrap_options,nullptr,&extended_rect),"wrap begin failed");
    for(size_t i=0;i<3;++i){CHECK(gui_begin_layout_item(ctx,i,&extended_rect),"wrapped item failed");
        gui_button(ctx,reinterpret_cast<const uint8_t*>("cell"),4);gui_end_layout_item(ctx);}
    gui_end_layout(ctx);
    extended_spec.kind=GUI_LAYOUT_GRID;
    const GuiGridCell grid_cells[]{{0,0,1,2},{0,2,2,1},{GUI_GRID_AUTO,GUI_GRID_AUTO,1,1}};
    const GuiLayoutOptions grid_options{0,GUI_CONTENT_START};
    CHECK(gui_begin_layout_extended(ctx,reinterpret_cast<const uint8_t*>("spans"),5,
        extended_spec,extended_items,3,grid_options,grid_cells,&extended_rect),"span begin failed");
    for(size_t i=0;i<3;++i){CHECK(gui_begin_layout_item(ctx,i,&extended_rect),"spanning item failed");
        gui_button(ctx,reinterpret_cast<const uint8_t*>("cell"),4);gui_end_layout_item(ctx);}
    gui_end_layout(ctx);
    // Data APIs are a real borrowed C consumer, including owned edit models.
    GuiVisibleRows clipped{};
    CHECK(gui_clip_rows(100000,20,4,-240,{0,48},&clipped) && clipped.start==10 && clipped.end==12,
        "row clipper range failed");
    GuiSelection* selection=gui_selection_new(10000);
    CHECK(selection && gui_selection_select(selection,1,0,0),"selection allocation failed");
    CHECK(gui_selection_select(selection,5,0,1) && gui_selection_count(selection)==5,
        "selection range failed");
    const GuiStr data_label{reinterpret_cast<const uint8_t*>("data"),4};
    const GuiStr empty_label{nullptr,0};
    std::vector<GuiStr> data_labels(10000,data_label);
    gui_multi_select_list(ctx,data_label,data_labels.data(),data_labels.size(),selection,{240,100});
    GuiDataTableState data_state{};
    CHECK(gui_data_table_state_init(&data_state),"table defaults failed");
    const GuiStr data_headers[]{data_label,data_label};
    CHECK(gui_begin_data_table(ctx,data_label,data_headers,2,100,&data_state,{{240,120},28,1,1}),
        "data table begin failed");
    CHECK(gui_visible_rows(ctx,&clipped),"table range query failed");
    for(size_t row=clipped.start;row<clipped.end;++row){
        for(size_t col=0;col<2;++col){if(gui_begin_data_table_cell(ctx,row,col)){
            gui_label(ctx,data_label.ptr,data_label.len);gui_end_data_cell(ctx);}}}
    data_state.scroll_x=gui_end_data_table(ctx);
    GuiNumericEdit* float_edit=gui_numeric_edit_new();
    GuiNumericEdit* int_edit=gui_numeric_edit_new();
    CHECK(float_edit && int_edit,"numeric edit allocation failed");
    float direct_float=1.25f;int32_t direct_int=2147483646;
    gui_input_float(ctx,data_label,&direct_float,float_edit,0.25f,0,10);
    gui_input_int(ctx,data_label,&direct_int,int_edit,1,0,2147483647);
    CHECK(direct_int==2147483646 && !gui_numeric_edit_invalid(int_edit),"numeric idle precision failed");
    CHECK(gui_begin_menu_bar(ctx,data_label),"menu bar begin failed");
    if(gui_begin_menu(ctx,data_label,{200,120})){gui_menu_item(ctx,data_label,empty_label,1);gui_end_menu(ctx);}
    gui_end_menu_bar(ctx);
    CHECK(gui_begin_property_grid(ctx,data_label,80,32),"property grid failed");
    CHECK(gui_begin_property_row(ctx,data_label),"property row failed");
    gui_label(ctx,data_label.ptr,data_label.len);gui_end_property_row(ctx);gui_end_property_grid(ctx);
    const GuiCurveKey curve_keys[]{{0,0,0,2,GUI_CURVE_CUBIC},{1,1,0,0,GUI_CURVE_LINEAR}};
    GuiCurveEditor* curve=gui_curve_editor_new(curve_keys,2);
    const GuiGradientStop editor_stops[]{{0,{0,0,0,255}},{1,{200,100,50,255}}};
    GuiGradientEditor* gradient_editor=gui_gradient_editor_new(editor_stops,2);
    CHECK(curve && gradient_editor,"editor construction failed");
    float curve_value=0;CHECK(gui_curve_evaluate(curve,0.5f,&curve_value)&&curve_value==0.75f,"Hermite curve failed");
    GuiColor gradient_value{};CHECK(gui_gradient_sample(gradient_editor,0.5f,&gradient_value)&&gradient_value.r==100,"gradient sampling failed");
    CHECK(gui_curve_editor_select(curve,0)&&gui_curve_editor_selected(curve)==0,"curve selection failed");
    GuiCurveKey curve_out{};CHECK(gui_curve_editor_get(curve,1,&curve_out)&&curve_out.time==1,"curve read failed");
    GuiGradientStop gradient_out{};CHECK(gui_gradient_editor_get(gradient_editor,1,&gradient_out)&&gradient_out.position==1,"stop read failed");
    const GuiStr curve_label{reinterpret_cast<const uint8_t*>("##curve"),7};
    const GuiStr gradient_label{reinterpret_cast<const uint8_t*>("##gradient"),10};
    const GuiStr plot_label{reinterpret_cast<const uint8_t*>("##plot"),6};
    const GuiStr sparkline_label{reinterpret_cast<const uint8_t*>("##sparkline"),11};
    gui_curve_editor(ctx,curve_label,curve,{240,140});
    gui_gradient_editor(ctx,gradient_label,gradient_editor,{240,70});
    const float plot_samples[]{0,1,0.5f,2};GuiPlotHover plot_hover{};
    GuiPlotOptions plot_options{{240,100},{0,1},{99,91,255,255},GUI_PLOT_AUTO_RANGE|GUI_PLOT_AXES};
    CHECK(gui_plot(ctx,plot_label,plot_samples,4,plot_options,&plot_hover),"plot declaration failed");
    CHECK(gui_sparkline(ctx,sparkline_label,plot_samples,4,plot_options,&plot_hover),"sparkline declaration failed");
    gui_curve_editor_free(curve);gui_gradient_editor_free(gradient_editor);
    CHECK(gui_begin_rows(ctx,data_label,100000,24),"row scope failed");
    CHECK(gui_visible_rows(ctx,&clipped),"row range failed");
    for(size_t row=clipped.start;row<clipped.end;++row){if(gui_begin_row(ctx,row)){
        gui_label(ctx,data_label.ptr,data_label.len);gui_end_data_cell(ctx);}}
    gui_end_rows(ctx);

    const uint8_t combining[]{'e',0xcc,0x81};
    GuiTextEditor* text_editor=gui_text_editor_new({combining,sizeof(combining)});
    CHECK(text_editor,"text editor construction failed");
    CHECK(!gui_text_editor_select(text_editor,1,1),"grapheme boundary validation failed");
    CHECK(gui_text_editor_select(text_editor,0,3),"text selection failed");
    GuiTextSelection text_selection{};CHECK(gui_text_editor_selection(text_editor,&text_selection)&&text_selection.cursor==3,"selection ABI failed");
    uint8_t selected_text[3]{};CHECK(gui_text_editor_copy(text_editor,GUI_EDIT_SELECTION,selected_text,3)==3&&!std::memcmp(combining,selected_text,3),"text copy failed");
    GuiTextEditResponse text_response{};
    const GuiStr text_label{reinterpret_cast<const uint8_t*>("##unicode"),9};
    CHECK(gui_text_edit(ctx,text_label,text_editor,{{240,80},GUI_EDIT_MULTILINE|GUI_EDIT_WRAP,GUI_DIRECTION_AUTO},&text_response),"editor declaration failed");
    GuiShapedText* shaped=gui_shaped_text_new();GuiVec2 shaped_size{};
    CHECK(gui_shaped_text(ctx,shaped,{reinterpret_cast<const uint8_t*>("office"),6},{220,1,{0,0,0},GUI_DIRECTION_AUTO},&shaped_size)&&shaped_size.x>0,"shaped display failed");
    GuiShapedCaret shaped_caret{};CHECK(gui_shaped_caret(shaped,2,&shaped_caret),"ligature caret missing");
    CHECK(gui_shaped_hit_test(shaped,shaped_caret.position,&shaped_caret),"shaped hit test failed");
    const uint8_t emoji_text[]{0xf0,0x9f,0x98,0x80};
    CHECK(gui_shaped_text(ctx,shaped,{emoji_text,sizeof(emoji_text)},{0,0,{0,0,0},GUI_DIRECTION_AUTO},&shaped_size),"color emoji declaration failed");
    uint32_t color_w{},color_h{};size_t color_len{};uint64_t color_generation{};
    const uint8_t* color_pixels=gui_color_atlas_pixels(ctx,&color_w,&color_h,&color_len,&color_generation);
    CHECK(color_pixels&&color_generation&&color_len==static_cast<size_t>(color_w)*color_h*4,"color atlas borrow failed");
    gui_shaped_text_free(shaped);gui_text_editor_free(text_editor);
    gui_numeric_edit_free(float_edit);gui_numeric_edit_free(int_edit);gui_selection_free(selection);
    gui_end_window(ctx);gui_render(ctx);
    CHECK(gui_inspection_count(ctx)>=3,"widget trace missing");GuiInspectionNode node{};
    CHECK(gui_inspection_node(ctx,0,&node)&&node.kind==0,"inspection node ABI failed");
    CHECK(gui_inspection_name(ctx,node.id,nullptr,0)==6,"inspection label missing");
    GuiRecording* recording=gui_recording_new();
    CHECK(gui_recording_push(recording,&input,0.016f,nullptr,0),"recording push failed");
    CHECK(gui_recording_len(recording)==1,"recording count failed");
    uint8_t tape[128]{};const size_t tape_size=gui_recording_save(recording,tape,sizeof(tape));
    CHECK(tape_size<=sizeof(tape)&&gui_recording_load(recording,tape,tape_size),"recording codec failed");
    CHECK(gui_replay_next(ctx,recording),"replay frame entry failed");gui_render(ctx);
    CHECK(!gui_replay_next(ctx,recording),"replay must stop at end");
    gui_recording_free(recording);gui_sprite_atlas_free(sprites);
    CHECK(gui_set_accessibility(ctx,true),"accessibility enable failed");
    CHECK(gui_set_duplicate_id_diagnostics(ctx,true),"duplicate diagnostics enable failed");
    CHECK(gui_set_hit_target(ctx,GuiVec2{32.f,32.f}),"minimum target policy failed");
    CHECK(gui_apply_high_contrast(ctx),"high contrast failed");
    bool accessible_checked=false;
    gui_new_frame(ctx,nullptr,0.016f);
    CHECK(gui_begin_window(ctx,reinterpret_cast<const uint8_t*>("host services"),13,GUI_SCROLL_NONE),"host service window failed");
    const uint64_t accessible_id=gui_widget_id(ctx,reinterpret_cast<const uint8_t*>("toggle"),6);
    gui_checkbox(ctx,reinterpret_cast<const uint8_t*>("toggle"),6,&accessible_checked);
    gui_button(ctx,reinterpret_cast<const uint8_t*>("duplicate"),9);gui_button(ctx,reinterpret_cast<const uint8_t*>("duplicate"),9);
    gui_end_window(ctx);gui_render(ctx);
    CHECK(gui_accessibility_count(ctx)>=3,"semantic nodes missing");
    CHECK(gui_duplicate_id_count(ctx)==1,"duplicate declarations not detected");
    GuiAccessibilityNode semantic{};bool found_semantic=false;
    for(size_t i=0;i<gui_accessibility_count(ctx);++i) {
        CHECK(gui_accessibility_node(ctx,i,&semantic),"semantic read failed");
        if(semantic.id==accessible_id) {found_semantic=true;break;}
    }
    CHECK(found_semantic && semantic.role==4,"checkbox role missing");
    CHECK(gui_accessibility_text(ctx,accessible_id,false,nullptr,0)==6,"semantic name length failed");
    CHECK(gui_accessibility_action(ctx,accessible_id,2,0.0),"host activation rejected");
    gui_new_frame(ctx,nullptr,0.016f);
    CHECK(gui_begin_window(ctx,reinterpret_cast<const uint8_t*>("host services"),13,GUI_SCROLL_NONE),"second host window failed");
    CHECK(gui_checkbox(ctx,reinterpret_cast<const uint8_t*>("toggle"),6,&accessible_checked),"deferred activation did not fire");
    gui_end_window(ctx);gui_render(ctx);
    CHECK(accessible_checked,"deferred toggle state incorrect");
    gui_invalidate(ctx);CHECK(gui_next_redraw_in(ctx)==0.f,"invalidation did not wake");
    CHECK(gui_host_services_overflow(ctx)==0,"host services overflowed");
    gui_context_free(ctx);
    std::printf("SMOKE OK: %u vertices, %u indices, %u commands, atlas %ux%u\n",
        draw.vertex_count, draw.index_count, draw.command_count, atlas_w, atlas_h);
    return 0;
}
