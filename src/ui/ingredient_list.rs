use super::query;
use crate::database;
use crate::database::models::Ingredient;
use diesel::ExpressionMethods as _;
use diesel::QueryDsl as _;
use diesel::RunQueryDsl as _;
use diesel::SelectableHelper as _;
use eframe::egui;

pub struct IngredientListWindow {
    all_ingredients: Vec<Ingredient>,
    edit_mode: bool,
    new_ingredient_name: String,
}

impl IngredientListWindow {
    pub fn new(conn: &mut database::Connection) -> Self {
        use database::schema::ingredients::dsl::*;
        let all_ingredients = ingredients
            .select(Ingredient::as_select())
            .order_by(name.asc())
            .load(conn)
            .unwrap();
        Self {
            all_ingredients,
            edit_mode: false,
            new_ingredient_name: String::new(),
        }
    }

    pub fn update(&mut self, conn: &mut database::Connection, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut refresh_self = false;
        egui::Window::new("Ingredients")
            .open(&mut open)
            .show(ctx, |ui| {
                let scroll_height = ui.available_height() - 35.0;
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .max_height(scroll_height)
                    .show(ui, |ui| {
                        egui::Grid::new("All Ingredients").show(ui, |ui| {
                            ui.label("Name");
                            ui.label("Category");
                            ui.end_row();

                            for ingredient in &self.all_ingredients {
                                ui.label(&ingredient.name);
                                ui.label(ingredient.category.as_deref().unwrap_or(""));
                                ui.end_row();
                            }
                        });
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut self.edit_mode, "Edit");
                    if self.edit_mode {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_ingredient_name)
                                .desired_width(ui.available_width() - 100.0),
                        );
                        if ui.button("Add").clicked() {
                            query::add_ingredient(conn, &self.new_ingredient_name);
                            self.new_ingredient_name = "".into();
                            refresh_self = true;
                        }
                    }
                });
            });
        if refresh_self {
            *self = Self::new(conn);
        }
        !open
    }
}
