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

    fn update_table_contents(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        recipe_list_windows: &mut HashMap<RecipeCategoryId, RecipeListWindow>,
        body: &mut egui_extras::TableBody<'_>,
        refresh_self: &mut bool,
    ) {
        for RecipeCategory { name, id: cat_id } in &self.categories {
            if let Some(e) = &mut self.category_being_edited {
                if e.id == *cat_id {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut e.name));
                        });
                        row.col(|ui| {
                            if ui.button("Save").clicked() {
                                query::edit_category(conn, e.id, &e.name);
                                if let Some(w) = recipe_list_windows.get_mut(&e.id) {
                                    w.category_name_changed(e.name.clone());
                                }
                                *refresh_self = true;
                            }
                        });
                    });
                    continue;
                }
            }

            body.row(20.0, |mut row| {
                let mut shown = recipe_list_windows.contains_key(&cat_id);
                row.col(|ui| {
                    ui.toggle_value(&mut shown, name.clone());
                });
                if self.edit_mode {
                    row.col(|ui| {
                        if ui.button("Edit").clicked() {
                            self.category_being_edited = Some(CategoryBeingEdited {
                                id: *cat_id,
                                name: name.clone(),
                            });
                        }
                        if ui.button("Delete").clicked() {
                            if query::delete_category(conn, *cat_id) {
                                *refresh_self = true;
                                shown = false;
                            } else {
                                toasts.add(egui_toast::Toast {
                                    text: "Couldn't delete category, it still contains recipes"
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
                }

                if shown && !recipe_list_windows.contains_key(&cat_id) {
                    let cat = RecipeCategory {
                        id: *cat_id,
                        name: name.clone(),
                    };
                    recipe_list_windows.insert(*cat_id, RecipeListWindow::new(conn, cat, false));
                } else if !shown {
                    recipe_list_windows.remove(cat_id);
                }
            });
        }
    }

    fn update_table(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        recipe_list_windows: &mut HashMap<RecipeCategoryId, RecipeListWindow>,
        ui: &mut egui::Ui,
        refresh_self: &mut bool,
    ) {
        let available_height = ui.available_height();
        egui_extras::TableBuilder::new(ui)
            .id_salt("category table")
            .striped(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::remainder())
            .column(egui_extras::Column::exact(90.0))
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height)
            .body(|mut body| {
                self.update_table_contents(
                    conn,
                    toasts,
                    recipe_list_windows,
                    &mut body,
                    refresh_self,
                );
            });
    }

    fn update_add_category(
        &mut self,
        conn: &mut database::Connection,
        ui: &mut egui::Ui,
        refresh_self: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.toggle_value(&mut self.edit_mode, "Edit");
            if self.edit_mode {
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_category_name)
                        .hint_text("category name")
                        .desired_width(ui.available_width() - 110.0),
                );
                let e = !self.new_category_name.is_empty();
                if ui
                    .add_enabled(e, egui::Button::new("New Category"))
                    .clicked()
                {
                    query::add_category(conn, &self.new_category_name);
                    self.new_category_name = "".into();
                    *refresh_self = true;
                }
            }
        });
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        recipe_list_windows: &mut HashMap<RecipeCategoryId, RecipeListWindow>,
    ) {
        let style = ctx.style();
        let button_height = (egui::TextStyle::Button.resolve(&style).size
            + style.spacing.button_padding.y as f32 * 2.0)
            .max(style.spacing.interact_size.y);
        let spacing = style.spacing.item_spacing.y;

        let separator_height = 6.0;
        let add_category_height = button_height + spacing + separator_height + 2.0;

        let mut refresh_self = false;
        egui::Window::new("Categories").show(ctx, |ui| {
            egui_extras::StripBuilder::new(ui)
                .size(egui_extras::Size::remainder())
                .size(egui_extras::Size::exact(add_category_height))
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        self.update_table(conn, toasts, recipe_list_windows, ui, &mut refresh_self);
                    });
                    strip.cell(|ui| {
                        ui.separator();
                        self.update_add_category(conn, ui, &mut refresh_self);
                    });
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
