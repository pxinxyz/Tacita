
#define _CRT_SECURE_NO_WARNINGS

#include "../restricted_gui.h"

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

// A real MSVC-compiled C++ consumer, built and run by CI. If this prints SMOKE OK, the FFI is not lying to your face.
int main(int argc, char **argv) {
    const char *font_path =
        argc > 1 ? argv[1] : "../../core/tests/assets/DejaVuSans.ttf";
    const std::vector<uint8_t> font = read_file(font_path);
    CHECK(!font.empty(), "could not read the font file");

    GuiContext *ctx = gui_context_new(font.data(), font.size());
    CHECK(ctx != nullptr, "gui_context_new returned null for a valid font");
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "context creation reported an error");

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

    bool checked = false;
    float volume = 0.5f;
    const bool opened = gui_begin_window(ctx, reinterpret_cast<const uint8_t *>("smoke"), 5);
    CHECK(opened, "the smoke window did not open");
    gui_checkbox(ctx, reinterpret_cast<const uint8_t *>("Enabled"), 7, &checked);
    gui_slider_float(ctx, reinterpret_cast<const uint8_t *>("Volume"), 6, &volume, 0.0f, 1.0f);
    const bool clicked = gui_button(ctx, reinterpret_cast<const uint8_t *>("Apply"), 5);
    (void)clicked; 
    gui_end_window(ctx);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "widget declaration reported an error");

    const GuiDrawData draw = gui_render(ctx);
    CHECK(gui_last_error_code() == GUI_ERROR_NONE, "gui_render reported an error");
    CHECK(draw.vertex_count > 0, "render produced no vertices");
    CHECK(draw.index_count > 0, "render produced no indices");
    CHECK(draw.command_count > 0, "render produced no commands");
    CHECK(draw.vertices != nullptr && draw.indices != nullptr && draw.commands != nullptr,
        "render produced null slices");

    const GuiDrawData again = gui_render(ctx);
    CHECK(again.vertices == draw.vertices && again.vertex_count == draw.vertex_count,
        "repeated render did not return the cached slices");

    gui_label(ctx, reinterpret_cast<const uint8_t *>("x"), 1);
    CHECK(gui_last_error_code() == GUI_ERROR_CONTRACT_VIOLATION,
        "out-of-frame widget call was not reported as a contract violation");

    gui_context_free(ctx);
    std::printf("SMOKE OK: %u vertices, %u indices, %u commands, atlas %ux%u\n",
        draw.vertex_count, draw.index_count, draw.command_count, atlas_w, atlas_h);
    return 0;
}
