use super::query;
use crate::database;
use crate::database::models::{IngredientCaloriesEntry, IngredientHandle, IngredientMeasurement};
use eframe::egui;

#[derive(Default)]
struct NewEntry {
    calories: String,
    quantity: String,
    quantity_units: Option<IngredientMeasurement>,
}

pub struct IngredientCaloriesWindow {
    ingredient: IngredientHandle,
    ingredient_calories: Vec<IngredientCaloriesEntry>,
    new_entry: NewEntry,
}

impl IngredientCaloriesWindow {
    pub fn new(conn: &mut database::Connection, ingredient: IngredientHandle) -> Self {
        let ingredient_calories = query::get_ingredient_calories(conn, ingredient.id);

        Self {
            ingredient,
            ingredient_calories,
            new_entry: NewEntry::default(),
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, conn: &mut database::Connection) -> bool {
        let mut open = true;
        let mut refresh_self = false;
        egui::Window::new(self.ingredient.name.clone())
            .id(egui::Id::new(("ingredient calories", self.ingredient.id)))
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new(("ingredient calories grid", self.ingredient.id)).show(ui, |ui| {
                    ui.label("Calories");
                    ui.label("Quantity");
                    ui.label("Measurement");
                    ui.end_row();

                    for c in &self.ingredient_calories {
                        ui.label(c.calories.to_string());
                        ui.label(c.quantity.to_string());
                        ui.label(c.quantity_units.as_ref().map(|c| c.as_str()).unwrap_or(""));
                        if ui.button("Delete").clicked() {
                            query::delete_ingredient_calories_entry(conn, c.id);
                            refresh_self = true;
                        }
                        ui.end_row();
                    }

                    ui.add(egui::TextEdit::singleline(&mut self.new_entry.calories));
                    ui.add(egui::TextEdit::singleline(&mut self.new_entry.quantity));
                    egui::ComboBox::from_id_salt((
                        "new quantity measurement calories",
                        self.ingredient.id,
                    ))
                    .selected_text(
                        self.new_entry
                            .quantity_units
                            .as_ref()
                            .map(|q| q.as_str())
                            .unwrap_or(""),
                    )
                    .show_ui(ui, |ui| {
                        for m in IngredientMeasurement::iter() {
                            ui.selectable_value(
                                &mut self.new_entry.quantity_units,
                                Some(m),
                                m.as_str(),
                            );
                        }
                        ui.selectable_value(&mut self.new_entry.quantity_units, None, "");
                    });

                    if ui.button("Add").clicked() {
                        query::add_ingredient_calories_entry(
                            conn,
                            self.ingredient.id,
                            self.new_entry.calories.parse().unwrap_or(0.0),
                            self.new_entry.quantity.parse().unwrap_or(0.0),
                            self.new_entry.quantity_units,
                        );
                        refresh_self = true;
                    }
                    ui.end_row()
                });
            });
        if refresh_self {
            *self = Self::new(conn, self.ingredient.clone());
        }

        !open
    }
}
