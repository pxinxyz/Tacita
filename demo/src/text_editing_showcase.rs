use restricted_gui::{
    editing::{TextEditOptions, TextEditor},
    font::{FontId, ShapedOptions, ShapedText, TextDirection},
    ui::Ui,
};
pub struct State {
    single: TextEditor,
    multiline: TextEditor,
    samples: ShapedText,
    fallback: ShapedText,
    color_emoji: ShapedText,
    faded_emoji: ShapedText,
    pub fallback_face: FontId,
    wrap: bool,
    read_only: bool,
    direction: usize,
}
impl Default for State {
    fn default() -> Self {
        Self{single:TextEditor::new("office | e\u{301} | 👩‍💻 | 🇬🇧 | 👍🏽").unwrap(),
            multiline:TextEditor::new("Mixed scripts: abc אבג xyz\nArabic joining: سلام عليكم\nCombining marks: e\u{301} a\u{308}\nEmoji clusters: 👩‍💻 🇬🇧 👍🏽\nSelect across lines, replace, copy or paste.\n\nThis paragraph wraps as the field narrows. Scroll to the final line, then use Ctrl+Home to return to the start.\n\nLast line.").unwrap(),
            samples:ShapedText::default(),fallback:ShapedText::default(),color_emoji:ShapedText::default(),faded_emoji:ShapedText::default(),fallback_face:FontId::DEFAULT,wrap:true,read_only:false,direction:0}
    }
}
impl State {
    pub fn draw(&mut self, ui: &mut Ui) {
        ui.label("Unicode editing");
        ui.label("Drag to select; Shift+arrows, Ctrl+A/C/X/V. Enter adds a line below.");
        let width = ui.available_size()[0].clamp(80., 560.);
        ui.checkbox("Wrap multiline text", &mut self.wrap);
        ui.same_line();
        ui.checkbox("Read only", &mut self.read_only);
        ui.combo(
            "Paragraph direction",
            &["Auto", "Left to right", "Right to left"],
            &mut self.direction,
        );
        let direction = [
            TextDirection::Auto,
            TextDirection::LeftToRight,
            TextDirection::RightToLeft,
        ][self.direction];
        if ui
            .text_edit(
                "##unicode_single",
                &mut self.single,
                TextEditOptions {
                    size: [width, 0.],
                    read_only: self.read_only,
                    direction,
                    ..Default::default()
                },
            )
            .unwrap()
            .changed
        {
            eprintln!("[demo] Unicode single-line text changed");
        }
        let response = ui
            .text_edit(
                "##unicode_multiline",
                &mut self.multiline,
                TextEditOptions {
                    size: [width, 164.],
                    multiline: true,
                    wrap: self.wrap,
                    read_only: self.read_only,
                    direction,
                },
            )
            .unwrap();
        if response.changed {
            eprintln!(
                "[demo] Unicode multiline text changed ({} bytes)",
                self.multiline.text().len()
            );
        }
        ui.label(&format!(
            "selection {}..{} | caret byte {} | preedit {} bytes",
            self.multiline.selection().start,
            self.multiline.selection().end,
            self.multiline.cursor(),
            self.multiline.preedit().len()
        ));
        ui.label("Use your OS input method here: preedit is underlined until commit.");
        ui.label("Noto color emoji: ZWJ sequences, flags, skin tones (full / disabled)");
        ui.shaped_text(
            &mut self.color_emoji,
            "😀  ☕  👩‍💻  🇬🇧  👍🏽  🚀  ❤️  ☺︎",
            ShapedOptions::default(),
        )
        .unwrap();
        let disabled = ui.begin_disabled("color emoji opacity", true);
        ui.shaped_text(
            &mut self.faded_emoji,
            "😀  ☕  👩‍💻  🇬🇧  👍🏽  🚀",
            ShapedOptions::default(),
        )
        .unwrap();
        ui.end_disabled(disabled);
        ui.label("Shaped display: ligatures, Arabic joining, mixed Hebrew / Latin");
        ui.shaped_text(
            &mut self.samples,
            "office — سلام عليكم — abc אבג xyz",
            ShapedOptions {
                width,
                wrap: true,
                direction,
            },
        )
        .unwrap();
        ui.label("Fallback: Font Awesome star + DejaVu combining/Hebrew clusters");
        ui.with_font(self.fallback_face, |ui| {
            ui.shaped_text(
                &mut self.fallback,
                "\u{f005}   e\u{301}   שלום",
                ShapedOptions::default(),
            )
            .unwrap();
        });
        ui.separator();
    }
}
