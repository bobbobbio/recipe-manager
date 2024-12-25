use super::{query, recipe::RecipeWindow, PressedEnterExt as _};
use crate::database;
use crate::database::models::{RecipeCategory, RecipeHandle, RecipeId};
use std::collections::HashMap;

pub enum UpdateEvent {
    Closed,
    RecipeDeleted(RecipeId),
}

pub struct RecipeListWindow {
    recipe_category: RecipeCategory,
    recipes: Vec<RecipeHandle>,
    recipe_lookup: HashMap<RecipeId, usize>,
    edit_mode: bool,
    new_recipe_name: String,
}

impl RecipeListWindow {
    pub fn new(
        conn: &mut database::Connection,
        recipe_category: RecipeCategory,
        edit_mode: bool,
    ) -> Self {
        let recipe_vec = query::get_recipes(conn, recipe_category.id);
        let recipe_lookup = recipe_vec
            .iter()
            .enumerate()
            .map(|(i, h)| (h.id, i))
            .collect();
        Self {
            recipes: recipe_vec,
            recipe_lookup,
            recipe_category,
            edit_mode,
            new_recipe_name: String::new(),
        }
    }

    fn update_table(
        &mut self,
        conn: &mut database::Connection,
        recipe_windows: &mut HashMap<RecipeId, RecipeWindow>,
        ui: &mut egui::Ui,
        selected_week: Option<chrono::NaiveWeek>,
        refresh_self: &mut bool,
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];

        let available_height = ui.available_height();
        egui_extras::TableBuilder::new(ui)
            .id_salt(("recipe category list table", self.recipe_category.id))
            .striped(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::remainder())
            .column(egui_extras::Column::exact(50.0))
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height)
            .body(|mut body| {
                for RecipeHandle { name, id } in &self.recipes {
                    body.row(20.0, |mut row| {
                        let mut shown = recipe_windows.contains_key(&id);
                        row.col(|ui| {
                            ui.toggle_value(&mut shown, name.clone());
                        });

                        row.col(|ui| {
                            if self.edit_mode {
                                if ui.button("Delete").clicked() {
                                    query::delete_recipe(conn, *id);
                                    events.push(UpdateEvent::RecipeDeleted(*id));
                                    *refresh_self = true;
                                    shown = false;
                                }
                            }
                        });

                        if shown && !recipe_windows.contains_key(&id) {
                            recipe_windows
                                .insert(*id, RecipeWindow::new(conn, *id, selected_week, false));
                        } else if !shown {
                            recipe_windows.remove(id);
                        }
                    });
                }
            });
        events
    }

    fn update_add_recipe(
        &mut self,
        conn: &mut database::Connection,
        ui: &mut egui::Ui,
        refresh_self: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.toggle_value(&mut self.edit_mode, "Edit");
            if self.edit_mode {
                let mut new_recipe = false;
                new_recipe |= ui
                    .add(
                        egui::TextEdit::singleline(&mut self.new_recipe_name)
                            .hint_text("recipe name")
                            .desired_width(ui.available_width() - 100.0),
                    )
                    .pressed_enter();
                let e = !self.new_recipe_name.is_empty();
                new_recipe |= ui.add_enabled(e, egui::Button::new("New Recipe")).clicked();

                if new_recipe && e {
                    query::add_recipe(conn, &self.new_recipe_name, self.recipe_category.id);
                    self.new_recipe_name = "".into();
                    *refresh_self = true;
                }
            }
        });
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        selected_week: Option<chrono::NaiveWeek>,
        recipe_windows: &mut HashMap<RecipeId, RecipeWindow>,
    ) -> Vec<UpdateEvent> {
        let style = ctx.style();
        let button_height = (egui::TextStyle::Button.resolve(&style).size
            + style.spacing.button_padding.y as f32 * 2.0)
            .max(style.spacing.interact_size.y);
        let spacing = style.spacing.item_spacing.y;

        let separator_height = 6.0;
        let add_recipe_height = button_height + spacing + separator_height + 2.0;

        let mut events = vec![];
        let mut open = true;
        let mut refresh_self = false;
        egui::Window::new(&self.recipe_category.name)
            .id(egui::Id::new((
                "recipe category list",
                self.recipe_category.id,
            )))
            .open(&mut open)
            .show(ctx, |ui| {
                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::remainder())
                    .size(egui_extras::Size::exact(add_recipe_height))
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            events.extend(self.update_table(
                                conn,
                                recipe_windows,
                                ui,
                                selected_week,
                                &mut refresh_self,
                            ));
                        });
                        strip.cell(|ui| {
                            ui.separator();
                            self.update_add_recipe(conn, ui, &mut refresh_self);
                        });
                    });
            });

        if refresh_self {
            *self = Self::new(conn, self.recipe_category.clone(), self.edit_mode);
        }

        if !open {
            events.push(UpdateEvent::Closed);
        }

        events
    }

    pub fn category_name_changed(&mut self, new_name: String) {
        self.recipe_category.name = new_name;
    }

    pub fn recipe_name_changed(&mut self, recipe_id: RecipeId, new_name: String) {
        if let Some(i) = self.recipe_lookup.get_mut(&recipe_id) {
            self.recipes[*i].name = new_name;
        }
    }

    pub fn recipe_category_changed(&mut self, conn: &mut database::Connection) {
        *self = Self::new(conn, self.recipe_category.clone(), self.edit_mode);
    }
}
