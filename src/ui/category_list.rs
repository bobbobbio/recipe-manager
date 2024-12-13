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
    pub fn new(conn: &mut database::Connection, edit_mode: bool) -> Self {
        use database::schema::recipe_categories::dsl::*;
        Self {
            categories: recipe_categories
                .select(RecipeCategory::as_select())
                .order_by(name.asc())
                .load(conn)
                .unwrap(),
            new_category_name: String::new(),
            edit_mode,
            category_being_edited: None,
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        recipe_list_windows: &mut HashMap<RecipeCategoryId, RecipeListWindow>,
    ) {
        let mut refresh_self = false;
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
                                    if query::delete_category(conn, *cat_id) {
                                        refresh_self = true;
                                        shown = false;
                                    } else {
                                        toasts.add(egui_toast::Toast {
                                            text: "Couldn't delete category, it still contains recipes".into(),
                                            kind: egui_toast::ToastKind::Error,
                                            options: egui_toast::ToastOptions::default()
                                                .duration_in_seconds(3.0)
                                                .show_progress(false)
                                                .show_icon(true),
                                            ..Default::default()
                                        });
                                    }
                                }
                            }
                            ui.end_row();

                            if shown && !recipe_list_windows.contains_key(&cat_id) {
                                let cat = RecipeCategory {
                                    id: *cat_id,
                                    name: name.clone(),
                                };
                                recipe_list_windows
                                    .insert(*cat_id, RecipeListWindow::new(conn, cat, false));
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
                    if ui.button("Add").clicked() {
                        query::add_category(conn, &self.new_category_name);
                        self.new_category_name = "".into();
                        refresh_self = true;
                    }
                }
            });
        });

        if refresh_self {
            *self = Self::new(conn, self.edit_mode);
        }
    }

    pub fn recipes_imported(&mut self, conn: &mut database::Connection) {
        *self = Self::new(conn, self.edit_mode);
    }
}
