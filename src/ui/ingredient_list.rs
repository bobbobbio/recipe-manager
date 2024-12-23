use super::{ingredient_calories::IngredientCaloriesWindow, query, search::SearchWidget};
use crate::database;
use crate::database::models::{Ingredient, IngredientHandle, IngredientId};
use eframe::egui;
use std::collections::HashMap;

struct IngredientBeingEdited {
    id: IngredientId,
    name: String,
    category: String,
    cached_category_search: Option<query::CachedQuery<()>>,
}

impl IngredientBeingEdited {
    fn new(ingredient: Ingredient) -> Self {
        Self {
            id: ingredient.id,
            name: ingredient.name,
            category: ingredient.category.unwrap_or_default(),
            cached_category_search: None,
        }
    }
}

pub enum UpdateEvent {
    Closed,
    IngredientEdited,
}

pub struct IngredientListWindow {
    all_ingredients: Option<query::CachedQuery<Ingredient>>,
    edit_mode: bool,
    new_ingredient_name: String,
    ingredient_being_edited: Option<IngredientBeingEdited>,
    name_search: String,
}

impl IngredientListWindow {
    pub fn new(_conn: &mut database::Connection, edit_mode: bool) -> Self {
        Self {
            all_ingredients: None,
            edit_mode,
            new_ingredient_name: String::new(),
            ingredient_being_edited: None,
            name_search: "".into(),
        }
    }

    fn update_ingredient_editing(
        &mut self,
        ingredient: &Ingredient,
        conn: &mut database::Connection,
        row: &mut egui_extras::TableRow<'_, '_>,
        refresh_self: &mut bool,
        events: &mut Vec<UpdateEvent>,
    ) -> bool {
        let Some(i) = &mut self.ingredient_being_edited else {
            return false;
        };
        if i.id != ingredient.id {
            return false;
        }

        row.col(|ui| {
            ui.add(egui::TextEdit::singleline(&mut i.name));
        });
        row.col(|ui| {
            let mut unused = None;
            ui.add(
                SearchWidget::new(i.id, &mut i.category, &mut unused, |query| {
                    query::search_ingredient_categories(conn, &mut i.cached_category_search, query)
                })
                .hint_text("search for category"),
            );
        });
        row.col(|ui| {
            if ui.button("Save").clicked() {
                query::update_ingredient(conn, i.id, &i.name, &i.category);
                *refresh_self = true;
                events.push(UpdateEvent::IngredientEdited);
            }
        });
        true
    }

    fn update_ingredient_row(
        &mut self,
        ingredient: &Ingredient,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        ingredient_calories_windows: &mut HashMap<IngredientId, IngredientCaloriesWindow>,
        mut search_for_ingredient: impl FnMut(&mut database::Connection, Vec<IngredientHandle>),
        row: &mut egui_extras::TableRow<'_, '_>,
        refresh_self: &mut bool,
    ) {
        row.col(|ui| {
            ui.label(&ingredient.name);
        });
        row.col(|ui| {
            ui.label(ingredient.category.as_deref().unwrap_or(""));
        });

        let mut calories_shown = ingredient_calories_windows.contains_key(&ingredient.id);

        if self.edit_mode {
            row.col(|ui| {
                if ui.button("Edit").clicked() {
                    self.ingredient_being_edited =
                        Some(IngredientBeingEdited::new(ingredient.clone()))
                }
                if ui.button("Delete").clicked() {
                    if query::delete_ingredient(conn, ingredient.id) {
                        *refresh_self = true;
                        calories_shown = false;
                    } else {
                        toasts.add(egui_toast::Toast {
                            text: "Couldn't delete ingredient, \
                                    it is still being used by recipes"
                                .into(),
                            kind: egui_toast::ToastKind::Error,
                            options: egui_toast::ToastOptions::default()
                                .duration_in_seconds(3.0)
                                .show_progress(false)
                                .show_icon(true),
                            ..Default::default()
                        });
                    }
                }
            });
        } else {
            row.col(|ui| {
                if ui.button("Search").clicked() {
                    search_for_ingredient(
                        conn,
                        vec![IngredientHandle {
                            id: ingredient.id,
                            name: ingredient.name.clone(),
                        }],
                    );
                }
                ui.toggle_value(&mut calories_shown, "Calories");
            });
        }
        if calories_shown && !ingredient_calories_windows.contains_key(&ingredient.id) {
            ingredient_calories_windows.insert(
                ingredient.id,
                IngredientCaloriesWindow::new(conn, ingredient.to_handle()),
            );
        } else if !calories_shown {
            ingredient_calories_windows.remove(&ingredient.id);
        }
    }

    fn update_listing(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        ingredient_calories_windows: &mut HashMap<IngredientId, IngredientCaloriesWindow>,
        mut search_for_ingredient: impl FnMut(&mut database::Connection, Vec<IngredientHandle>),
        refresh_self: &mut bool,
        body: &mut egui_extras::TableBody<'_>,
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];

        query::search_ingredients(conn, &mut self.all_ingredients, &self.name_search);
        let all_ingredients = std::mem::take(&mut self.all_ingredients);
        let all_ingredients_iter = all_ingredients
            .as_ref()
            .map(|c| c.results.iter())
            .into_iter()
            .flatten();
        for (ingredient, _) in all_ingredients_iter {
            body.row(20.0, |mut row| {
                if self.update_ingredient_editing(
                    ingredient,
                    conn,
                    &mut row,
                    refresh_self,
                    &mut events,
                ) {
                    return;
                }
                self.update_ingredient_row(
                    ingredient,
                    conn,
                    toasts,
                    ingredient_calories_windows,
                    &mut search_for_ingredient,
                    &mut row,
                    refresh_self,
                );
            });
        }
        self.all_ingredients = all_ingredients;
        events
    }

    fn update_table(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        ingredient_calories_windows: &mut HashMap<IngredientId, IngredientCaloriesWindow>,
        search_for_ingredient: impl FnMut(&mut database::Connection, Vec<IngredientHandle>),
        refresh_self: &mut bool,
        ui: &mut egui::Ui,
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];

        let available_height = ui.available_height();
        egui_extras::TableBuilder::new(ui)
            .id_salt("global ingredients table")
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::remainder())
            .column(egui_extras::Column::remainder())
            .column(egui_extras::Column::exact(110.0))
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading("Name");
                });
                header.col(|ui| {
                    ui.heading("Category");
                });
                header.col(|ui| {
                    ui.heading("");
                });
            })
            .body(|mut body| {
                events = self.update_listing(
                    conn,
                    toasts,
                    ingredient_calories_windows,
                    search_for_ingredient,
                    refresh_self,
                    &mut body,
                );
            });
        events
    }

    fn update_add_ingredient(
        &mut self,
        conn: &mut database::Connection,
        refresh_self: &mut bool,
        ui: &mut egui::Ui,
    ) {
        if self.edit_mode {
            egui_extras::StripBuilder::new(ui)
                .size(egui_extras::Size::exact(30.0))
                .size(egui_extras::Size::remainder())
                .size(egui_extras::Size::exact(35.0))
                .horizontal(|mut strip| {
                    strip.cell(|ui| {
                        ui.toggle_value(&mut self.edit_mode, "Edit");
                    });
                    strip.cell(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_ingredient_name)
                                .desired_width(f32::INFINITY),
                        );
                    });
                    strip.cell(|ui| {
                        let e = !self.new_ingredient_name.is_empty();
                        if ui.add_enabled(e, egui::Button::new("Add")).clicked() {
                            query::add_ingredient(conn, &self.new_ingredient_name);
                            self.new_ingredient_name = "".into();
                            *refresh_self = true;
                        }
                    });
                });
        } else {
            ui.toggle_value(&mut self.edit_mode, "Edit");
        }
    }

    pub fn update(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        ingredient_calories_windows: &mut HashMap<IngredientId, IngredientCaloriesWindow>,
        search_for_ingredient: impl FnMut(&mut database::Connection, Vec<IngredientHandle>),
        ctx: &egui::Context,
    ) -> Vec<UpdateEvent> {
        let style = ctx.style();
        let button_height = (egui::TextStyle::Button.resolve(&style).size
            + style.spacing.button_padding.y as f32 * 2.0)
            .max(style.spacing.interact_size.y);
        let spacing = style.spacing.item_spacing.y;
        let separator_height = 6.0;

        let add_height = button_height + spacing + separator_height + 2.0;
        let search_height = button_height + spacing + separator_height + 2.0;

        let mut open = true;
        let mut events = vec![];
        let mut refresh_self = false;
        egui::Window::new("Ingredients")
            .open(&mut open)
            .show(ctx, |ui| {
                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::exact(search_height))
                    .size(egui_extras::Size::remainder())
                    .size(egui_extras::Size::exact(add_height))
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.name_search)
                                    .hint_text("search by name")
                                    .desired_width(f32::INFINITY),
                            );
                            ui.separator();
                        });
                        strip.cell(|ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                events.extend(self.update_table(
                                    conn,
                                    toasts,
                                    ingredient_calories_windows,
                                    search_for_ingredient,
                                    &mut refresh_self,
                                    ui,
                                ));
                            });
                        });
                        strip.cell(|ui| {
                            ui.separator();
                            self.update_add_ingredient(conn, &mut refresh_self, ui);
                        })
                    });
            });

        if !self.edit_mode {
            self.ingredient_being_edited = None;
        }

        if refresh_self {
            *self = Self::new(conn, self.edit_mode);
        }
        if !open {
            events.push(UpdateEvent::Closed);
        }
        events
    }

    pub fn ingredient_deleted(&mut self, conn: &mut database::Connection) {
        *self = Self::new(conn, self.edit_mode);
    }
}
