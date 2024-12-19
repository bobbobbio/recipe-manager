use super::{
    calendar::{this_week, RecipeWeek},
    new_error_toast, query,
    search::SearchWidget,
    unit_conversion,
};
use crate::database;
use crate::database::models::{
    Ingredient, IngredientCaloriesEntry, IngredientMeasurement, IngredientUsageId, Recipe,
    RecipeCategoryId, RecipeDuration, RecipeId,
};
use eframe::egui;

struct IngredientBeingEdited {
    usage_id: IngredientUsageId,
    new_ingredient_name: String,
    ingredient: Option<Ingredient>,
    quantity: String,
    quantity_units: Option<IngredientMeasurement>,
    cached_ingredient_search: Option<query::CachedQuery<Ingredient>>,
}

impl IngredientBeingEdited {
    fn new(usage: &RecipeIngredient) -> Self {
        Self {
            usage_id: usage.id,
            new_ingredient_name: usage.ingredient.name.clone(),
            ingredient: Some(usage.ingredient.clone()),
            quantity: usage.quantity.to_string(),
            quantity_units: usage.quantity_units,
            cached_ingredient_search: None,
        }
    }
}

pub struct RecipeIngredient {
    pub id: IngredientUsageId,
    pub ingredient: Ingredient,
    pub quantity: f32,
    pub quantity_units: Option<IngredientMeasurement>,
    pub calories: Vec<IngredientCaloriesEntry>,
}

impl RecipeIngredient {
    fn calories(&self) -> Option<f32> {
        use unit_conversion::{conversion_factor, MeasurementKind};

        for c in &self.calories {
            if c.quantity_units == self.quantity_units {
                return Some(c.calories * self.quantity);
            }
        }
        for c in &self.calories {
            if let (Some(a), Some(b)) = (self.quantity_units, c.quantity_units) {
                if MeasurementKind::from(a) == MeasurementKind::from(b) {
                    return Some(c.calories * conversion_factor(a, b) * self.quantity / c.quantity);
                }
            }
        }
        None
    }
}

fn right_align_cell(ui: &mut egui::Ui, text: String) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(text);
    });
}

pub enum UpdateEvent {
    Closed,
    Renamed(Recipe),
    Scheduled(chrono::NaiveWeek),
    CategoryChanged,
}

pub struct RecipeWindow {
    recipe: Recipe,

    ingredients: Vec<RecipeIngredient>,
    ingredient_being_edited: Option<IngredientBeingEdited>,

    new_ingredient_name: String,
    new_ingredient: Option<Ingredient>,
    cached_ingredient_search: Option<query::CachedQuery<Ingredient>>,

    week: RecipeWeek,

    new_category_name: String,
    new_category: Option<RecipeCategoryId>,
    cached_category_search: Option<query::CachedQuery<RecipeCategoryId>>,

    edit_mode: bool,
}

impl RecipeWindow {
    pub fn new(conn: &mut database::Connection, recipe_id: RecipeId, edit_mode: bool) -> Self {
        let (recipe, category_name, ingredients) = query::get_recipe(conn, recipe_id);
        Self {
            recipe,

            ingredients,
            ingredient_being_edited: None,

            new_ingredient_name: String::new(),
            new_ingredient: None,
            cached_ingredient_search: None,

            week: RecipeWeek::new(conn, this_week()),

            new_category_name: category_name,
            new_category: None,
            cached_category_search: None,

            edit_mode,
        }
    }

    fn update_ingredient_editing(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        usage: &RecipeIngredient,
        row: &mut egui_extras::TableRow<'_, '_>,
        refresh_self: &mut bool,
    ) -> bool {
        let Some(e) = &mut self.ingredient_being_edited else {
            return false;
        };
        if e.usage_id != usage.id {
            return false;
        }
        row.col(|ui| {
            ui.add(SearchWidget::new(
                e.usage_id,
                &mut e.new_ingredient_name,
                &mut e.ingredient,
                |query| query::search_ingredients(conn, &mut e.cached_ingredient_search, query),
            ));
        });

        row.col(|ui| {
            if let Some(Ingredient {
                category: Some(category),
                ..
            }) = &e.ingredient
            {
                ui.label(category.as_str());
            } else {
                ui.label("");
            }
        });
        row.col(|ui| {
            ui.add(egui::TextEdit::singleline(&mut e.quantity));
        });
        row.col(|ui| {
            egui::ComboBox::from_id_salt(("recipe ingredient quantity units", self.recipe.id))
                .selected_text(e.quantity_units.as_ref().map(|q| q.as_str()).unwrap_or(""))
                .show_ui(ui, |ui| {
                    for m in IngredientMeasurement::iter() {
                        ui.selectable_value(&mut e.quantity_units, Some(m), m.as_str());
                    }
                    ui.selectable_value(&mut e.quantity_units, None, "");
                });
        });
        row.col(|_| {});
        row.col(|ui| {
            if ui.button("Save").clicked() {
                if e.ingredient.is_some() {
                    query::edit_recipe_ingredient(
                        conn,
                        e.usage_id,
                        e.ingredient.as_ref().unwrap(),
                        e.quantity.parse().unwrap_or(0.0),
                        e.quantity_units,
                    );
                    *refresh_self = true;
                } else {
                    toasts.add(new_error_toast("Couldn't find ingredient"));
                }
            }
        });
        true
    }

    fn update_ingredient_row(
        &mut self,
        conn: &mut database::Connection,
        usage: &RecipeIngredient,
        row: &mut egui_extras::TableRow<'_, '_>,
        refresh_self: &mut bool,
    ) {
        row.col(|ui| {
            ui.label(&usage.ingredient.name);
        });
        row.col(|ui| {
            ui.label(usage.ingredient.category.as_deref().unwrap_or(""));
        });
        row.col(|ui| right_align_cell(ui, usage.quantity.to_string()));
        row.col(|ui| {
            ui.label(
                usage
                    .quantity_units
                    .as_ref()
                    .map(|c| c.as_str())
                    .unwrap_or(""),
            );
        });
        row.col(|ui| {
            right_align_cell(
                ui,
                usage
                    .calories()
                    .map(|c| format!("{c:.2}"))
                    .unwrap_or_default(),
            )
        });

        if self.edit_mode && self.ingredient_being_edited.is_none() {
            row.col(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("Edit").clicked() {
                        self.ingredient_being_edited = Some(IngredientBeingEdited::new(usage));
                    }
                    if ui.button("Delete").clicked() {
                        query::delete_recipe_ingredient(conn, usage.id);
                        *refresh_self = true;
                    }
                });
            });
        }
    }

    fn update_ingredients_table(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        body: &mut egui_extras::TableBody<'_>,
        refresh_self: &mut bool,
    ) {
        let ingredients = std::mem::take(&mut self.ingredients);
        for usage in &ingredients {
            body.row(20.0, |mut row| {
                if self.update_ingredient_editing(conn, toasts, usage, &mut row, refresh_self) {
                    return;
                }
                self.update_ingredient_row(conn, usage, &mut row, refresh_self);
            });
        }
        self.ingredients = ingredients;
    }

    fn update_add_ingredient(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        ui: &mut egui::Ui,
        refresh_self: &mut bool,
    ) {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
            egui_extras::StripBuilder::new(ui)
                .size(egui_extras::Size::exact(100.0))
                .size(egui_extras::Size::remainder())
                .size(egui_extras::Size::exact(40.0))
                .horizontal(|mut strip| {
                    strip.cell(|ui| {
                        ui.label("Add Ingredient:");
                    });
                    strip.cell(|ui| {
                        ui.add(
                            SearchWidget::new(
                                "ingredient",
                                &mut self.new_ingredient_name,
                                &mut self.new_ingredient,
                                |query| {
                                    query::search_ingredients(
                                        conn,
                                        &mut self.cached_ingredient_search,
                                        query,
                                    )
                                },
                            )
                            .hint_text("search for ingredient")
                            .desired_width(f32::INFINITY),
                        );
                    });

                    strip.cell(|ui| {
                        if ui.button("Add").clicked() {
                            if let Some(ingredient) = &self.new_ingredient {
                                query::add_recipe_ingredient(
                                    conn,
                                    self.recipe.id,
                                    ingredient.id,
                                    1.0,
                                );
                                self.new_ingredient_name = "".into();
                                self.new_ingredient = None;
                                *refresh_self = true;
                            } else {
                                toasts.add(new_error_toast("Couldn't find ingredient"));
                            }
                        }
                    });
                });
        });
    }

    fn update_ingredients_edit_mode(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        ui: &mut egui::Ui,
        refresh_self: &mut bool,
    ) {
        let available_height = ui.available_height();
        egui_extras::TableBuilder::new(ui)
            .id_salt(("edit ingredients table", self.recipe.id))
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::remainder())
            .column(egui_extras::Column::remainder())
            .column(egui_extras::Column::exact(30.0))
            .column(egui_extras::Column::exact(20.0))
            .column(egui_extras::Column::exact(40.0))
            .column(egui_extras::Column::exact(85.0))
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
                    ui.heading("Qty");
                });
                header.col(|ui| {
                    ui.heading("");
                });
                header.col(|ui| {
                    ui.heading("Cal.");
                });
                header.col(|ui| {
                    ui.heading("");
                });
            })
            .body(|mut body| {
                self.update_ingredients_table(conn, toasts, &mut body, refresh_self);
            });
    }

    fn update_ingredients(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        ui: &mut egui::Ui,
        refresh_self: &mut bool,
    ) {
        let available_height = ui.available_height();
        egui_extras::TableBuilder::new(ui)
            .id_salt(("ingredients table", self.recipe.id))
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::remainder())
            .column(egui_extras::Column::remainder())
            .column(egui_extras::Column::exact(30.0))
            .column(egui_extras::Column::exact(20.0))
            .column(egui_extras::Column::exact(40.0))
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
                    ui.heading("Qty");
                });
                header.col(|ui| {
                    ui.heading("");
                });
                header.col(|ui| {
                    ui.heading("Cal.");
                });
            })
            .body(|mut body| {
                self.update_ingredients_table(conn, toasts, &mut body, refresh_self);
            });
    }

    fn update_recipe_information_edit_mode(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        ui: &mut egui::Ui,
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];

        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);
        egui_extras::StripBuilder::new(ui)
            .size(egui_extras::Size::exact(text_height))
            .size(egui_extras::Size::exact(text_height))
            .size(egui_extras::Size::exact(text_height))
            .size(egui_extras::Size::exact(text_height * 4.0))
            .size(egui_extras::Size::exact(text_height))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    egui_extras::StripBuilder::new(ui)
                        .size(egui_extras::Size::exact(80.0))
                        .size(egui_extras::Size::remainder())
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.label("Name:");
                            });
                            strip.cell(|ui| {
                                let mut name = self.recipe.name.clone();
                                ui.add(
                                    egui::TextEdit::singleline(&mut name)
                                        .desired_width(f32::INFINITY),
                                );
                                if name != self.recipe.name {
                                    query::edit_recipe_name(conn, self.recipe.id, &name);
                                    self.recipe.name = name.clone();
                                    events.push(UpdateEvent::Renamed(self.recipe.clone()));
                                }
                            });
                        });
                });
                strip.cell(|ui| {
                    egui_extras::StripBuilder::new(ui)
                        .size(egui_extras::Size::exact(80.0))
                        .size(egui_extras::Size::remainder())
                        .size(egui_extras::Size::exact(40.0))
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.label("Category:");
                            });
                            strip.cell(|ui| {
                                ui.add(
                                    SearchWidget::new(
                                        ("recipe category", self.recipe.id),
                                        &mut self.new_category_name,
                                        &mut self.new_category,
                                        |query| {
                                            query::search_recipe_categories(
                                                conn,
                                                &mut self.cached_category_search,
                                                query,
                                            )
                                        },
                                    )
                                    .desired_width(f32::INFINITY)
                                    .hint_text("search for category"),
                                );
                            });
                            strip.cell(|ui| {
                                if ui.button("Save").clicked() {
                                    if let Some(cat) = self.new_category {
                                        query::edit_recipe_category(conn, self.recipe.id, cat);
                                        events.push(UpdateEvent::CategoryChanged);
                                    } else {
                                        toasts
                                            .add(new_error_toast("Couldn't find recipe category"));
                                    }
                                }
                            });
                        });
                });
                strip.cell(|ui| {
                    egui_extras::StripBuilder::new(ui)
                        .size(egui_extras::Size::exact(80.0))
                        .size(egui_extras::Size::remainder())
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.label("Duration:");
                            });

                            strip.cell(|ui| {
                                let mut selected = self.recipe.duration.clone();
                                egui::ComboBox::from_id_salt(("recipe duration", self.recipe.id))
                                    .selected_text(&selected.to_string())
                                    .show_ui(ui, |ui| {
                                        for d in RecipeDuration::iter() {
                                            ui.selectable_value(&mut selected, d, d.to_string());
                                        }
                                    });
                                if selected != self.recipe.duration {
                                    query::edit_recipe_duration(conn, self.recipe.id, selected);
                                    self.recipe.duration = selected;
                                }
                            });
                        });
                });
                strip.cell(|ui| {
                    egui_extras::StripBuilder::new(ui)
                        .size(egui_extras::Size::exact(80.0))
                        .size(egui_extras::Size::remainder())
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.label("Description:");
                            });
                            strip.cell(|ui| {
                                let mut description = self.recipe.description.clone();
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut description)
                                            .desired_width(f32::INFINITY),
                                    );
                                });
                                if description != self.recipe.description {
                                    query::edit_recipe_description(
                                        conn,
                                        self.recipe.id,
                                        &description,
                                    );
                                    self.recipe.description = description;
                                }
                            });
                        });
                });
                strip.cell(|ui| {
                    ui.label(format!("Total Calories:   {:.2}", self.total_calories()));
                });
            });
        events
    }

    fn update_recipe_information(&mut self, ui: &mut egui::Ui) {
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        egui_extras::StripBuilder::new(ui)
            .size(egui_extras::Size::exact(text_height))
            .size(egui_extras::Size::exact(text_height * 4.0))
            .size(egui_extras::Size::exact(text_height))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    egui_extras::StripBuilder::new(ui)
                        .size(egui_extras::Size::exact(80.0))
                        .size(egui_extras::Size::remainder())
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.label("Duration:");
                            });
                            strip.cell(|ui| {
                                ui.label(self.recipe.duration.to_string());
                            });
                        });
                });
                strip.cell(|ui| {
                    egui_extras::StripBuilder::new(ui)
                        .size(egui_extras::Size::exact(80.0))
                        .size(egui_extras::Size::remainder())
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.label("Description:");
                            });
                            strip.cell(|ui| {
                                ui.add(egui::Label::new(&self.recipe.description).wrap());
                            });
                        });
                });
                strip.cell(|ui| {
                    egui_extras::StripBuilder::new(ui)
                        .size(egui_extras::Size::exact(80.0))
                        .size(egui_extras::Size::remainder())
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.label("Total Calories:");
                            });
                            strip.cell(|ui| {
                                ui.label(format!("{:.2}", self.total_calories()));
                            });
                        });
                });
            });
    }

    fn total_calories(&self) -> f32 {
        if self.ingredients.is_empty() {
            0.0
        } else {
            self.ingredients
                .iter()
                .filter_map(|i| i.calories())
                .sum::<f32>()
        }
    }

    fn update_recipe_controls(
        &mut self,
        conn: &mut database::Connection,
        ui: &mut egui::Ui,
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];
        ui.horizontal(|ui| {
            ui.toggle_value(&mut self.edit_mode, "Edit");
            if !self.edit_mode {
                self.ingredient_being_edited = None;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button("Schedule", |ui| {
                    for (day, recipe) in self.week.recipes() {
                        let recipe = recipe.map(|r| r.name.clone()).unwrap_or("No Recipe".into());
                        if ui.button(format!("{day}: {recipe}")).clicked() {
                            self.week.schedule(conn, day, self.recipe.id);
                            ui.close_menu();
                            events.push(UpdateEvent::Scheduled(*self.week.week()));
                        }
                    }
                });
                self.week.pick_date(conn, |date| {
                    ui.add(egui_extras::DatePickerButton::new(date));
                });
            });
        });
        events
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
    ) -> Vec<UpdateEvent> {
        let style = ctx.style();
        let text_height = egui::TextStyle::Body
            .resolve(&style)
            .size
            .max(style.spacing.interact_size.y);
        let button_height = (egui::TextStyle::Button.resolve(&style).size
            + style.spacing.button_padding.y as f32 * 2.0)
            .max(style.spacing.interact_size.y);
        let spacing = style.spacing.item_spacing.y;

        let separator_height = 6.0;
        let table_height = 20.0 + (20.0 + spacing) * self.ingredients.len() as f32 + spacing;
        let info_height = (text_height + spacing) * 6.0 + separator_height;
        let controls_height = button_height + spacing + separator_height;

        let add_ingredient_height = button_height + spacing;
        let edit_info_height = (text_height + spacing) * 8.0 + separator_height;

        let edit_height = table_height + add_ingredient_height + edit_info_height + controls_height;

        let mut events = vec![];
        let mut open = true;
        let mut refresh_self = false;

        egui::Window::new(self.recipe.name.clone())
            .id(egui::Id::new(("recipe", self.recipe.id)))
            .default_height(edit_height + 20.0)
            .default_width(400.0)
            .open(&mut open)
            .show(ctx, |ui| {
                if self.edit_mode {
                    egui_extras::StripBuilder::new(ui)
                        .size(egui_extras::Size::remainder())
                        .size(egui_extras::Size::exact(add_ingredient_height))
                        .size(egui_extras::Size::exact(edit_info_height))
                        .size(egui_extras::Size::exact(controls_height))
                        .vertical(|mut strip| {
                            strip.cell(|ui| {
                                egui::ScrollArea::horizontal().show(ui, |ui| {
                                    self.update_ingredients_edit_mode(
                                        conn,
                                        toasts,
                                        ui,
                                        &mut refresh_self,
                                    );
                                });
                            });
                            strip.cell(|ui| {
                                self.update_add_ingredient(conn, toasts, ui, &mut refresh_self);
                            });
                            strip.cell(|ui| {
                                ui.separator();
                                events.extend(
                                    self.update_recipe_information_edit_mode(conn, toasts, ui),
                                );
                            });
                            strip.cell(|ui| {
                                ui.separator();
                                events.extend(self.update_recipe_controls(conn, ui));
                            });
                        });
                } else {
                    egui_extras::StripBuilder::new(ui)
                        .size(egui_extras::Size::remainder())
                        .size(egui_extras::Size::exact(info_height))
                        .size(egui_extras::Size::exact(controls_height))
                        .vertical(|mut strip| {
                            strip.cell(|ui| {
                                egui::ScrollArea::horizontal().show(ui, |ui| {
                                    self.update_ingredients(conn, toasts, ui, &mut refresh_self);
                                });
                            });
                            strip.cell(|ui| {
                                ui.separator();
                                self.update_recipe_information(ui);
                            });
                            strip.cell(|ui| {
                                ui.separator();
                                events.extend(self.update_recipe_controls(conn, ui));
                            });
                        });
                }
            });

        if refresh_self {
            *self = Self::new(conn, self.recipe.id, self.edit_mode);
        }

        if !open {
            events.push(UpdateEvent::Closed);
        }
        events
    }

    pub fn recipe_scheduled(&mut self, conn: &mut database::Connection, week: chrono::NaiveWeek) {
        if self.week.week() == &week {
            self.week.refresh(conn);
        }
    }

    pub fn ingredient_edited(&mut self, conn: &mut database::Connection) {
        *self = Self::new(conn, self.recipe.id, self.edit_mode);
    }
}
