pub struct AboutWindow {}

impl AboutWindow {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;

        egui::Window::new("About").open(&mut open).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Recipe Manager");
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
