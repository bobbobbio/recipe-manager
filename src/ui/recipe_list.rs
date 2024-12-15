use super::{query, recipe::RecipeWindow};
use crate::database;
use crate::database::models::{RecipeCategory, RecipeHandle, RecipeId};
use diesel::ExpressionMethods as _;
use diesel::QueryDsl as _;
use diesel::RunQueryDsl as _;
use diesel::SelectableHelper as _;
use eframe::egui;
use std::collections::HashMap;

pub struct RecipeListWindow {
    recipe_category: RecipeCategory,
    recipes: Vec<RecipeHandle>,
    recipe_lookup: HashMap<RecipeId, usize>,
    edit_mode: bool,
    new_recipe_name: String,
}

impl RecipeListWindow {
    pub fn new(conn: &mut database::Connection, recipe_category: RecipeCategory) -> Self {
        use database::schema::recipes::dsl::*;
        let recipe_vec = recipes
            .select(RecipeHandle::as_select())
            .filter(category.eq(recipe_category.id))
            .order_by(name.asc())
            .load(conn)
            .unwrap();
        let recipe_lookup = recipe_vec
            .iter()
            .enumerate()
            .map(|(i, h)| (h.id, i))
            .collect();
        Self {
            recipes: recipe_vec,
            recipe_lookup,
            recipe_category,
            edit_mode: false,
            new_recipe_name: String::new(),
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        recipe_windows: &mut HashMap<RecipeId, RecipeWindow>,
    ) -> bool {
        let mut recipes_to_delete = vec![];
        let mut open = true;
        let mut add_recipe = false;
        egui::Window::new(&self.recipe_category.name)
            .id(egui::Id::new((
                "recipe category list",
                self.recipe_category.id,
            )))
            .open(&mut open)
            .show(ctx, |ui| {
                let scroll_height = ui.available_height() - 35.0;
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .max_height(scroll_height)
                    .show(ui, |ui| {
                        egui::Grid::new("recipe_listing").show(ui, |ui| {
                            for RecipeHandle { name, id } in &self.recipes {
                                let mut shown = recipe_windows.contains_key(&id);
                                ui.toggle_value(&mut shown, name.clone());

                                if self.edit_mode {
                                    if ui.button("Delete").clicked() {
                                        recipes_to_delete.push(*id);
                                    }
                                }
                                ui.end_row();

                                if shown && !recipe_windows.contains_key(&id) {
                                    recipe_windows.insert(*id, RecipeWindow::new(conn, *id, false));
                                } else if !shown {
                                    recipe_windows.remove(id);
                                }
                            }
                        });
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut self.edit_mode, "Edit");
                    if self.edit_mode {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_recipe_name)
                                .desired_width(ui.available_width() - 100.0),
                        );
                        add_recipe = ui.button("Add").clicked();
                    }
                });
            });

        let mut refresh_self = false;
        for recipe in recipes_to_delete {
            query::delete_recipe(conn, recipe);
            refresh_self = true;
            recipe_windows.remove(&recipe);
        }

        if add_recipe {
            query::add_recipe(conn, &self.new_recipe_name, self.recipe_category.id);
            self.new_recipe_name = "".into();
            refresh_self = true;
        }

        if refresh_self {
            *self = Self::new(conn, self.recipe_category.clone());
        }

        !open
    }

    pub fn category_name_changed(&mut self, new_name: String) {
        self.recipe_category.name = new_name;
    }

    pub fn recipe_name_changed(&mut self, recipe_id: RecipeId, new_name: String) {
        if let Some(i) = self.recipe_lookup.get_mut(&recipe_id) {
            self.recipes[*i].name = new_name;
        }
    }
}
