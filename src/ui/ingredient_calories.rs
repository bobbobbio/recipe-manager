use super::query;
use crate::database;
use crate::database::models::{IngredientCaloriesEntry, IngredientHandle, IngredientMeasurement};

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

pub enum UpdateEvent {
    Closed,
    IngredientEdited,
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

    fn update_table(
        &mut self,
        conn: &mut database::Connection,
        ui: &mut egui::Ui,
        refresh_self: &mut bool,
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];

        let available_height = ui.available_height();
        egui_extras::TableBuilder::new(ui)
            .id_salt("global ingredients table")
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::remainder())
            .column(egui_extras::Column::exact(30.0))
            .column(egui_extras::Column::exact(40.0))
            .column(egui_extras::Column::exact(50.0))
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading("Calories");
                });
                header.col(|ui| {
                    ui.heading("Qty");
                });
                header.col(|ui| {
                    ui.heading("Unit");
                });
                header.col(|ui| {
                    ui.heading("");
                });
            })
            .body(|mut body| {
                for c in &self.ingredient_calories {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label(c.calories.to_string());
                        });
                        row.col(|ui| {
                            ui.label(c.quantity.to_string());
                        });
                        row.col(|ui| {
                            ui.label(c.quantity_units.as_ref().map(|c| c.as_str()).unwrap_or(""));
                        });
                        row.col(|ui| {
                            if ui.button("Delete").clicked() {
                                query::delete_ingredient_calories_entry(conn, c.id);
                                *refresh_self = true;
                                events.push(UpdateEvent::IngredientEdited);
                            }
                        });
                    });
                }
            });
        events
    }

    fn update_add_entry(
        &mut self,
        conn: &mut database::Connection,
        ui: &mut egui::Ui,
        refresh_self: &mut bool,
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];
        egui_extras::StripBuilder::new(ui)
            .size(egui_extras::Size::exact(80.0))
            .size(egui_extras::Size::exact(80.0))
            .size(egui_extras::Size::remainder())
            .size(egui_extras::Size::exact(50.0))
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_entry.calories)
                            .hint_text("calories"),
                    );
                });
                strip.cell(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_entry.quantity)
                            .hint_text("quantity"),
                    );
                });
                strip.cell(|ui| {
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
                });
                strip.cell(|ui| {
                    if ui.button("Add").clicked() {
                        query::add_ingredient_calories_entry(
                            conn,
                            self.ingredient.id,
                            self.new_entry.calories.parse().unwrap_or(0.0),
                            self.new_entry.quantity.parse().unwrap_or(0.0),
                            self.new_entry.quantity_units,
                        );
                        *refresh_self = true;
                        events.push(UpdateEvent::IngredientEdited);
                    }
                });
            });
        events
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
    ) -> Vec<UpdateEvent> {
        let style = ctx.style();
        let button_height = (egui::TextStyle::Button.resolve(&style).size
            + style.spacing.button_padding.y as f32 * 2.0)
            .max(style.spacing.interact_size.y);
        let spacing = style.spacing.item_spacing.y;
        let separator_height = 6.0;

        let table_height = (20.0 + spacing) * self.ingredient_calories.len() as f32;
        let add_height = button_height + spacing + separator_height + 2.0;

        let mut open = true;
        let mut refresh_self = false;
        let mut events = vec![];
        egui::Window::new(format!("{} - Calorie Information", &self.ingredient.name))
            .id(egui::Id::new(("ingredient calories", self.ingredient.id)))
            .default_height(table_height + add_height)
            .open(&mut open)
            .show(ctx, |ui| {
                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::remainder())
                    .size(egui_extras::Size::exact(add_height))
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                events.extend(self.update_table(conn, ui, &mut refresh_self));
                            });
                        });
                        strip.cell(|ui| {
                            ui.separator();
                            events.extend(self.update_add_entry(conn, ui, &mut refresh_self));
                        });
                    });
            });

        if refresh_self {
            *self = Self::new(conn, self.ingredient.clone());
        }

        if !open {
            events.push(UpdateEvent::Closed);
        }

        events
    }
}
