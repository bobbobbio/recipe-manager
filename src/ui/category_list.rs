use super::{query, recipe_list::RecipeListWindow};
use crate::database;
use crate::database::models::{RecipeCategory, RecipeCategoryId};
use diesel::ExpressionMethods as _;
use diesel::QueryDsl as _;
use diesel::RunQueryDsl as _;
use diesel::SelectableHelper as _;
use std::collections::HashMap;

struct CategoryBeingEdited {
    id: RecipeCategoryId,
    name: String,
}

pub struct CategoryListWindow {
    categories: Vec<RecipeCategory>,
    new_category_name: String,
    edit_mode: bool,
    category_being_edited: Option<CategoryBeingEdited>,
}

impl CategoryListWindow {
    pub fn new(conn: &mut database::Connection) -> Self {
        use database::schema::recipe_categories::dsl::*;
        Self {
            categories: recipe_categories
                .select(RecipeCategory::as_select())
                .order_by(name.asc())
                .load(conn)
                .unwrap(),
            new_category_name: String::new(),
            edit_mode: false,
            category_being_edited: None,
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        recipe_list_windows: &mut HashMap<RecipeCategoryId, RecipeListWindow>,
    ) {
        let mut refresh_self = false;
        let mut categories_to_delete = vec![];
        let mut add_category = false;
        egui::Window::new("Categories").show(ctx, |ui| {
            let scroll_height = ui.available_height() - 35.0;
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .max_height(scroll_height)
                .show(ui, |ui| {
                    egui::Grid::new("categories grid").show(ui, |ui| {
                        for RecipeCategory { name, id: cat_id } in &self.categories {
                            if let Some(e) = &mut self.category_being_edited {
                                if e.id == *cat_id {
                                    ui.add(egui::TextEdit::singleline(&mut e.name));
                                    if ui.button("Save").clicked() {
                                        query::edit_category(conn, e.id, &e.name);
                                        if let Some(w) = recipe_list_windows.get_mut(&e.id) {
                                            w.category_name_changed(e.name.clone());
                                        }
                                        refresh_self = true;
                                    }
                                    ui.end_row();
                                    continue;
                                }
                            }

                            let mut shown = recipe_list_windows.contains_key(&cat_id);
                            ui.toggle_value(&mut shown, name.clone());
                            if self.edit_mode {
                                if ui.button("Edit").clicked() {
                                    self.category_being_edited = Some(CategoryBeingEdited {
                                        id: *cat_id,
                                        name: name.clone(),
                                    });
                                }
                                if ui.button("Delete").clicked() {
                                    categories_to_delete.push(*cat_id);
                                }
                            }
                            ui.end_row();

                            if shown && !recipe_list_windows.contains_key(&cat_id) {
                                let cat = RecipeCategory {
                                    id: *cat_id,
                                    name: name.clone(),
                                };
                                recipe_list_windows
                                    .insert(*cat_id, RecipeListWindow::new(conn, cat));
                            } else if !shown {
                                recipe_list_windows.remove(cat_id);
                            }
                        }
                    });
                });
            ui.separator();
            ui.horizontal(|ui| {
                ui.toggle_value(&mut self.edit_mode, "Edit");
                if self.edit_mode {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_category_name)
                            .desired_width(ui.available_width() - 100.0),
                    );
                    add_category = ui.button("Add").clicked();
                }
            });
        });

        if add_category {
            query::add_category(conn, &self.new_category_name);
            self.new_category_name = "".into();
            refresh_self = true;
        }
        for cat in categories_to_delete {
            if query::delete_category(conn, cat) {
                refresh_self = true;
                recipe_list_windows.remove(&cat);
            }
        }

        if refresh_self {
            *self = Self::new(conn);
        }
    }
}
