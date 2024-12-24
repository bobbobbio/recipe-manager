pub struct AboutWindow {}

impl AboutWindow {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;

        egui::Window::new("About")
            .resizable([false, false])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(
                        egui::Image::new(egui::include_image!("../../images/appicon.png"))
                            .maintain_aspect_ratio(true)
                            .max_width(100.0),
                    );
                    ui.heading("Recipe Manager");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.label("Copyright Remi Bernotavicius 2024");
                    ui.hyperlink_to(
                        "Code on GitHub",
                        "https://github.com/bobbobbio/recipe-manager",
                    );
                });
            });

        !open
    }
}
