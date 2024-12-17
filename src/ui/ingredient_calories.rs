use crate::database;
use crate::database::models::IngredientHandle;
use eframe::egui;

pub struct IngredientCaloriesWindow {
    ingredient: IngredientHandle,
}

impl IngredientCaloriesWindow {
    pub fn new(_conn: &mut database::Connection, ingredient: IngredientHandle) -> Self {
        Self { ingredient }
    }

    pub fn update(&mut self, ctx: &egui::Context, _conn: &mut database::Connection) -> bool {
        let mut open = true;
        egui::Window::new(self.ingredient.name.clone())
            .id(egui::Id::new(("ingredient calories", self.ingredient.id)))
            .open(&mut open)
            .show(ctx, |ui| ui.label("todo"));

        !open
    }
}
