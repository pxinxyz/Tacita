fn draw(ui) {
    ui.begin_window("Rhai probe");
    ui.label("Scripted controls");
    this.enabled = ui.checkbox("Enabled", this.enabled);
    this.gain = ui.slider("Gain", this.gain, 0.0, 1.0);
    if ui.button("Reset") {
        this.enabled = false;
        this.gain = 0.5;
    }
    ui.end_window();
    this.frames += 1;
}
