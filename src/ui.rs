// Copyright 2023 Remi Bernotavicius

use crate::import;
use diesel::BelongingToDsl as _;
use diesel::ExpressionMethods as _;
use diesel::QueryDsl as _;
use diesel::RunQueryDsl as _;
use diesel::SelectableHelper as _;
use eframe::egui;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::mem;

use crate::database;
use crate::database::models::{
    Ingredient, IngredientId, IngredientMeasurement, IngredientUsage, IngredientUsageId, Recipe,
    RecipeCategory, RecipeCategoryId, RecipeDuration, RecipeHandle, RecipeId,
};

struct CategoryBeingEdited {
    id: RecipeCategoryId,
    name: String,
}

struct CategoryListWindow {
    categories: HashMap<RecipeCategoryId, RecipeCategory>,
    new_category_name: String,
    edit_mode: bool,
    category_being_edited: Option<CategoryBeingEdited>,
}

impl CategoryListWindow {
    fn new(conn: &mut database::Connection) -> Self {
        use database::schema::recipe_categories::dsl::*;
        Self {
            categories: recipe_categories
                .select(RecipeCategory::as_select())
                .load(conn)
                .unwrap()
                .into_iter()
                .map(|cat| (cat.id, cat))
                .collect(),
            new_category_name: String::new(),
            edit_mode: false,
            category_being_edited: None,
        }
    }

    fn add_category(conn: &mut database::Connection, new_category_name: &str) {
        use database::schema::recipe_categories::dsl::*;
        use diesel::insert_into;

        insert_into(recipe_categories)
            .values(name.eq(new_category_name))
            .execute(conn)
            .unwrap();
    }

    fn delete_category(conn: &mut database::Connection, delete_id: RecipeCategoryId) {
        let count: i64 = {
            use database::schema::recipes::dsl::*;

            recipes
                .filter(category.eq(delete_id))
                .count()
                .get_result(conn)
                .unwrap()
        };

        if count == 0 {
            use database::schema::recipe_categories::dsl::*;
            use diesel::delete;

            delete(recipe_categories.filter(id.eq(delete_id)))
                .execute(conn)
                .unwrap();
        }
    }

    fn edit_category(
        conn: &mut database::Connection,
        id_to_edit: RecipeCategoryId,
        new_name: &str,
    ) {
        use database::schema::recipe_categories::dsl::*;
        use diesel::update;

        update(recipe_categories.filter(id.eq(id_to_edit)))
            .set(name.eq(new_name))
            .execute(conn)
            .unwrap();
    }

    fn update(
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
                        let mut sorted_categories: Vec<_> = self.categories.values().collect();
                        sorted_categories.sort_by_key(|cat| &cat.name);

                        for RecipeCategory { name, id: cat_id } in sorted_categories {
                            if let Some(e) = &mut self.category_being_edited {
                                if e.id == *cat_id {
                                    ui.add(egui::TextEdit::singleline(&mut e.name));
                                    if ui.button("Save").clicked() {
                                        Self::edit_category(conn, e.id, &e.name);
                                        if let Some(w) = recipe_list_windows.get_mut(&e.id) {
                                            w.recipe_category.name = e.name.clone();
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
            Self::add_category(conn, &self.new_category_name);
            self.new_category_name = "".into();
            refresh_self = true;
        }
        for cat in categories_to_delete {
            Self::delete_category(conn, cat);
            refresh_self = true;
            recipe_list_windows.remove(&cat);
        }

        if refresh_self {
            *self = Self::new(conn);
        }
    }
}

struct RecipeListWindow {
    recipe_category: RecipeCategory,
    recipes: HashMap<RecipeId, RecipeHandle>,
    edit_mode: bool,
    new_recipe_name: String,
}

impl RecipeListWindow {
    fn new(conn: &mut database::Connection, recipe_category: RecipeCategory) -> Self {
        use database::schema::recipes::dsl::*;
        Self {
            recipes: recipes
                .select(RecipeHandle::as_select())
                .filter(category.eq(recipe_category.id))
                .load(conn)
                .unwrap()
                .into_iter()
                .map(|h| (h.id, h))
                .collect(),
            recipe_category,
            edit_mode: false,
            new_recipe_name: String::new(),
        }
    }

    fn delete_recipe(conn: &mut database::Connection, delete_id: RecipeId) {
        use database::schema::recipes::dsl::*;
        use diesel::delete;

        delete(recipes.filter(id.eq(delete_id)))
            .execute(conn)
            .unwrap();
    }

    fn add_recipe(conn: &mut database::Connection, new_name: &str, new_category: RecipeCategoryId) {
        use database::schema::recipes::dsl::*;
        use diesel::insert_into;

        insert_into(recipes)
            .values((
                name.eq(new_name),
                description.eq(""),
                duration.eq(RecipeDuration::Short),
                category.eq(new_category),
            ))
            .execute(conn)
            .unwrap();
    }

    fn update(
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
                            let mut sorted_recipes: Vec<_> = self.recipes.values().collect();
                            sorted_recipes.sort_by_key(|r| &r.name);

                            for RecipeHandle { name, id } in sorted_recipes {
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
            Self::delete_recipe(conn, recipe);
            refresh_self = true;
            recipe_windows.remove(&recipe);
        }

        if add_recipe {
            Self::add_recipe(conn, &self.new_recipe_name, self.recipe_category.id);
            self.new_recipe_name = "".into();
            refresh_self = true;
        }

        if refresh_self {
            *self = Self::new(conn, self.recipe_category.clone());
        }

        !open
    }
}

struct SearchWidget<'a, SearchFn, ValueT> {
    buf: &'a mut String,
    value: &'a mut Option<ValueT>,
    search_fn: SearchFn,
    pop_up_id: egui::Id,
}

impl<'a, SearchFn, ValueT> SearchWidget<'a, SearchFn, ValueT>
where
    SearchFn: FnOnce(&str) -> Vec<(ValueT, String)>,
{
    fn new(
        id_source: impl Hash,
        buf: &'a mut String,
        value: &'a mut Option<ValueT>,
        search_fn: SearchFn,
    ) -> Self {
        Self {
            buf,
            value,
            search_fn,
            pop_up_id: egui::Id::new(id_source),
        }
    }
}

impl<'a, SearchFn, ValueT> egui::Widget for SearchWidget<'a, SearchFn, ValueT>
where
    SearchFn: FnOnce(&str) -> Vec<(ValueT, String)>,
    ValueT: Clone,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Self {
            pop_up_id,
            buf,
            value,
            search_fn,
        } = self;

        let edit = egui::TextEdit::singleline(buf);
        let edit_output = edit.show(ui);
        let mut r = edit_output.response;
        if r.gained_focus() {
            ui.memory_mut(|m| m.open_popup(pop_up_id));
        }

        let mut changed = false;
        egui::popup_below_widget(
            ui,
            pop_up_id,
            &r,
            egui::PopupCloseBehavior::CloseOnClick,
            |ui| {
                egui::ScrollArea::vertical()
                    .max_height(f32::INFINITY)
                    .show(ui, |ui| {
                        let mut matches_valid = false;
                        for (text_id, text) in search_fn(buf) {
                            if buf == &text {
                                matches_valid = true;
                                if value.is_none() {
                                    *value = Some(text_id.clone());
                                }
                            }

                            if ui.selectable_label(false, &text).clicked() {
                                *value = Some(text_id);
                                *buf = text;
                                changed = true;
                                ui.memory_mut(|m| m.close_popup());
                                matches_valid = true;
                            }
                        }
                        if !matches_valid {
                            *value = None;
                        }
                    });
            },
        );

        if changed {
            r.mark_changed();
        }

        r
    }
}

struct CachedQuery<IdT> {
    query: String,
    results: Vec<(IdT, String)>,
}

struct IngredientBeingEdited {
    usage_id: IngredientUsageId,
    new_ingredient_name: String,
    ingredient: Option<Ingredient>,
    quantity: String,
    quantity_units: Option<IngredientMeasurement>,
    cached_ingredient_search: Option<CachedQuery<Ingredient>>,
}

impl IngredientBeingEdited {
    fn new(i: &Ingredient, u: &IngredientUsage) -> Self {
        Self {
            usage_id: u.id,
            new_ingredient_name: i.name.clone(),
            ingredient: Some(i.clone()),
            quantity: u.quantity.to_string(),
            quantity_units: u.quantity_units,
            cached_ingredient_search: None,
        }
    }
}

struct RecipeWindow {
    recipe: Recipe,
    ingredients: Vec<(IngredientUsage, Ingredient)>,
    ingredient_being_edited: Option<IngredientBeingEdited>,
    new_ingredient_name: String,
    new_ingredient: Option<Ingredient>,
    edit_mode: bool,
    cached_ingredient_search: Option<CachedQuery<Ingredient>>,
    week: RecipeWeek,
}

impl RecipeWindow {
    fn new(conn: &mut database::Connection, recipe_id: RecipeId, edit_mode: bool) -> Self {
        use database::schema::recipes::dsl::*;
        let recipe = recipes
            .select(Recipe::as_select())
            .filter(id.eq(recipe_id))
            .get_result(conn)
            .unwrap();
        let ingredients = IngredientUsage::belonging_to(&recipe)
            .inner_join(database::schema::ingredients::table)
            .select((IngredientUsage::as_select(), Ingredient::as_select()))
            .load(conn)
            .unwrap();
        Self {
            recipe,
            ingredients,
            ingredient_being_edited: None,
            new_ingredient_name: String::new(),
            new_ingredient: None,
            edit_mode,
            cached_ingredient_search: None,
            week: RecipeWeek::new(conn, this_week()),
        }
    }

    fn delete_recipe_ingredient(conn: &mut database::Connection, usage_id: IngredientUsageId) {
        use database::schema::ingredient_usages::dsl::*;
        use diesel::delete;

        delete(ingredient_usages)
            .filter(id.eq(usage_id))
            .execute(conn)
            .unwrap();
    }

    fn add_recipe_ingredient(
        conn: &mut database::Connection,
        new_recipe_id: RecipeId,
        new_ingredient_id: IngredientId,
        new_quantity: f32,
    ) {
        use database::schema::ingredient_usages::dsl::*;
        use diesel::insert_into;

        insert_into(ingredient_usages)
            .values((
                recipe_id.eq(new_recipe_id),
                ingredient_id.eq(new_ingredient_id),
                quantity.eq(new_quantity),
            ))
            .execute(conn)
            .unwrap();
    }

    fn edit_recipe_ingredient(
        conn: &mut database::Connection,
        usage_id: IngredientUsageId,
        new_ingredient: &Ingredient,
        new_quantity: f32,
        new_quantity_units: Option<IngredientMeasurement>,
    ) {
        use database::schema::ingredient_usages::dsl::*;
        use diesel::update;

        update(ingredient_usages)
            .filter(id.eq(usage_id))
            .set((
                ingredient_id.eq(new_ingredient.id),
                quantity.eq(new_quantity),
                quantity_units.eq(new_quantity_units),
            ))
            .execute(conn)
            .unwrap();
    }

    fn edit_recipe_duration(
        conn: &mut database::Connection,
        recipe_id: RecipeId,
        new_duration: RecipeDuration,
    ) {
        use database::schema::recipes::dsl::*;
        use diesel::update;

        update(recipes)
            .filter(id.eq(recipe_id))
            .set(duration.eq(new_duration))
            .execute(conn)
            .unwrap();
    }

    fn edit_recipe_description(
        conn: &mut database::Connection,
        recipe_id: RecipeId,
        new_description: &str,
    ) {
        use database::schema::recipes::dsl::*;
        use diesel::update;

        update(recipes)
            .filter(id.eq(recipe_id))
            .set(description.eq(new_description))
            .execute(conn)
            .unwrap();
    }

    fn edit_recipe_name(conn: &mut database::Connection, recipe_id: RecipeId, new_name: &str) {
        use database::schema::recipes::dsl::*;
        use diesel::update;

        update(recipes)
            .filter(id.eq(recipe_id))
            .set(name.eq(new_name))
            .execute(conn)
            .unwrap();
    }

    fn search_ingredients(
        conn: &mut database::Connection,
        cached_ingredient_search: &mut Option<CachedQuery<Ingredient>>,
        query: &str,
    ) -> Vec<(Ingredient, String)> {
        if let Some(cached) = cached_ingredient_search.as_ref() {
            if cached.query == query {
                return cached.results.clone();
            }
        }

        use database::schema::ingredients::dsl::*;
        use diesel::expression_methods::TextExpressionMethods as _;

        let result: Vec<_> = ingredients
            .select(Ingredient::as_select())
            .filter(name.like(format!("%{query}%")))
            .load(conn)
            .unwrap()
            .into_iter()
            .map(|i| (i.clone(), i.name))
            .collect();

        *cached_ingredient_search = Some(CachedQuery {
            query: query.into(),
            results: result.clone(),
        });
        result
    }

    fn schedule(
        conn: &mut database::Connection,
        edit_date: chrono::NaiveDate,
        edit_recipe_id: RecipeId,
    ) {
        use database::schema::calendar::dsl::*;
        use diesel::insert_into;

        insert_into(calendar)
            .values((day.eq(edit_date), recipe_id.eq(edit_recipe_id)))
            .on_conflict(day)
            .do_update()
            .set(recipe_id.eq(edit_recipe_id))
            .execute(conn)
            .unwrap();
    }

    fn update_ingredients(&mut self, conn: &mut database::Connection, ui: &mut egui::Ui) {
        let mut refresh_self = false;
        let name = &self.recipe.name;
        egui::Grid::new(format!("{name} ingredients")).show(ui, |ui| {
            ui.label("Name");
            ui.label("Category");
            ui.label("Quantity");
            ui.label("Measurement");
            ui.end_row();

            for (usage, ingredient) in &self.ingredients {
                if let Some(e) = &mut self.ingredient_being_edited {
                    if e.usage_id == usage.id {
                        ui.add(SearchWidget::new(
                            "ingredient",
                            &mut e.new_ingredient_name,
                            &mut e.ingredient,
                            |query| {
                                Self::search_ingredients(
                                    conn,
                                    &mut e.cached_ingredient_search,
                                    query,
                                )
                            },
                        ));

                        if let Some(Ingredient {
                            category: Some(category),
                            ..
                        }) = &e.ingredient
                        {
                            ui.label(category.as_str());
                        } else {
                            ui.label("");
                        }
                        ui.add(egui::TextEdit::singleline(&mut e.quantity));
                        egui::ComboBox::from_id_salt("recipe ingredient quantity units")
                            .selected_text(
                                e.quantity_units.as_ref().map(|q| q.as_str()).unwrap_or(""),
                            )
                            .show_ui(ui, |ui| {
                                for m in IngredientMeasurement::iter() {
                                    ui.selectable_value(&mut e.quantity_units, Some(m), m.as_str());
                                }
                                ui.selectable_value(&mut e.quantity_units, None, "");
                            });
                        if ui.button("Save").clicked() && e.ingredient.is_some() {
                            Self::edit_recipe_ingredient(
                                conn,
                                e.usage_id,
                                e.ingredient.as_ref().unwrap(),
                                e.quantity.parse().unwrap_or(0.0),
                                e.quantity_units,
                            );
                            refresh_self = true;
                        }
                        ui.end_row();
                        continue;
                    }
                }

                ui.label(&ingredient.name);
                ui.label(ingredient.category.as_deref().unwrap_or(""));
                ui.label(usage.quantity.to_string());
                ui.label(
                    usage
                        .quantity_units
                        .as_ref()
                        .map(|c| c.as_str())
                        .unwrap_or(""),
                );
                if self.edit_mode && self.ingredient_being_edited.is_none() {
                    if ui.button("Edit").clicked() {
                        self.ingredient_being_edited =
                            Some(IngredientBeingEdited::new(ingredient, usage));
                    }
                    if ui.button("Delete").clicked() {
                        Self::delete_recipe_ingredient(conn, usage.id);
                        refresh_self = true;
                    }
                }
                ui.end_row();
            }
        });

        if self.edit_mode {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ui.label("Add Ingredient:");
                ui.add(SearchWidget::new(
                    "ingredient",
                    &mut self.new_ingredient_name,
                    &mut self.new_ingredient,
                    |query| {
                        Self::search_ingredients(conn, &mut self.cached_ingredient_search, query)
                    },
                ));

                if ui.button("Add").clicked() {
                    if let Some(ingredient) = &self.new_ingredient {
                        Self::add_recipe_ingredient(conn, self.recipe.id, ingredient.id, 1.0);
                        self.new_ingredient_name = "".into();
                        self.new_ingredient = None;
                        refresh_self = true;
                    }
                }
            });
        }

        if refresh_self {
            *self = Self::new(conn, self.recipe.id, self.edit_mode);
        }
    }

    fn update(&mut self, ctx: &egui::Context, conn: &mut database::Connection) -> bool {
        let mut open = true;
        egui::Window::new(self.recipe.name.clone())
            .id(egui::Id::new(("recipe", self.recipe.id)))
            .open(&mut open)
            .show(ctx, |ui| {
                self.update_ingredients(conn, ui);
                egui::Grid::new("Recipe Information")
                    .num_columns(2)
                    .show(ui, |ui| {
                        if self.edit_mode {
                            ui.label("Name:");
                            let mut name = self.recipe.name.clone();
                            ui.add(egui::TextEdit::singleline(&mut name));
                            if name != self.recipe.name {
                                Self::edit_recipe_name(conn, self.recipe.id, &name);
                                self.recipe.name = name.clone();
                            }
                            ui.end_row();
                        }
                        ui.label("Duration:");
                        if self.edit_mode {
                            let mut selected = self.recipe.duration.clone();
                            egui::ComboBox::from_id_salt("recipe duration")
                                .selected_text(&selected.to_string())
                                .show_ui(ui, |ui| {
                                    for d in RecipeDuration::iter() {
                                        ui.selectable_value(&mut selected, d, d.to_string());
                                    }
                                });
                            if selected != self.recipe.duration {
                                Self::edit_recipe_duration(conn, self.recipe.id, selected);
                                self.recipe.duration = selected;
                            }
                        } else {
                            ui.label(self.recipe.duration.to_string());
                        }
                        ui.end_row();

                        ui.label("Description:");
                        if self.edit_mode {
                            let mut description = self.recipe.description.clone();
                            ui.add(egui::TextEdit::multiline(&mut description));
                            if description != self.recipe.description {
                                Self::edit_recipe_description(conn, self.recipe.id, &description);
                                self.recipe.description = description;
                            }
                        } else {
                            ui.label(&self.recipe.description);
                        }
                        ui.end_row();
                    });
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut self.edit_mode, "Edit");
                    self.week.pick_date(conn, |date| {
                        ui.add(egui_extras::DatePickerButton::new(date));
                    });

                    ui.menu_button("Schedule", |ui| {
                        let mut scheduled = false;
                        for (day, recipe) in self.week.recipes() {
                            let recipe =
                                recipe.map(|r| r.name.clone()).unwrap_or("No Recipe".into());
                            if ui.button(format!("{day}: {recipe}")).clicked() {
                                Self::schedule(conn, self.week.date_for_day(day), self.recipe.id);
                                ui.close_menu();
                                scheduled = true;
                            }
                        }
                        if scheduled {
                            self.week = RecipeWeek::new(conn, self.week.start);
                        }
                    });
                });
            });
        !open
    }
}

#[derive(Default)]
enum ImportWindow {
    #[default]
    Ready,
    ImportingRecipes {
        importer: crate::import::RecipeImporter,
    },
    ImportingCalendar {
        importer: crate::import::CalendarImporter,
    },
    Failed {
        error: crate::Error,
    },
    Success {
        num_imported: usize,
    },
}

impl ImportWindow {
    fn update(&mut self, conn: &mut database::Connection, ctx: &egui::Context) -> bool {
        let mut open = true;
        egui::Window::new("Import data")
            .open(&mut open)
            .show(ctx, |ui| {
                let next = match self {
                    Self::Ready => Self::update_ready(ui),
                    Self::ImportingRecipes { importer } => {
                        ctx.request_repaint_after(std::time::Duration::from_millis(0));
                        Self::update_importing(conn, importer, ui)
                    }
                    Self::ImportingCalendar { importer } => {
                        ctx.request_repaint_after(std::time::Duration::from_millis(0));
                        Self::update_importing(conn, importer, ui)
                    }
                    Self::Failed { error } => Self::update_failed(error, ui),
                    Self::Success { num_imported } => Self::update_success(*num_imported, ui),
                };
                if let Some(next) = next {
                    *self = next;
                }
            });
        !open
    }

    fn update_ready(ui: &mut egui::Ui) -> Option<Self> {
        if ui.button("import recipes").clicked() {
            if let Some(file) = rfd::FileDialog::new()
                .add_filter("recipebook", &["recipebook"])
                .set_directory("/")
                .pick_file()
            {
                return Some(match import::RecipeImporter::new(file) {
                    Ok(importer) => Self::ImportingRecipes { importer },
                    Err(error) => Self::Failed { error },
                });
            }
        }
        if ui.button("import calendar").clicked() {
            if let Some(file) = rfd::FileDialog::new()
                .add_filter("recipecalendar", &["recipecalendar"])
                .set_directory("/")
                .pick_file()
            {
                return Some(match import::CalendarImporter::new(file) {
                    Ok(importer) => Self::ImportingCalendar { importer },
                    Err(error) => Self::Failed { error },
                });
            }
        }
        None
    }

    fn update_importing(
        conn: &mut database::Connection,
        importer: &mut impl import::Importer,
        ui: &mut egui::Ui,
    ) -> Option<Self> {
        ui.label("importing data..");
        ui.add(egui::widgets::ProgressBar::new(importer.percent_done()));

        if !importer.done() {
            if let Err(error) = importer.import_one(conn) {
                return Some(Self::Failed { error });
            }
        } else {
            return Some(Self::Success {
                num_imported: importer.num_imported(),
            });
        }

        None
    }

    fn update_failed(error: &crate::Error, ui: &mut egui::Ui) -> Option<Self> {
        ui.label(format!("import failed with error: {error}"));
        ui.button("okay").clicked().then_some(Self::Ready)
    }

    fn update_success(num_imported: usize, ui: &mut egui::Ui) -> Option<Self> {
        ui.label(format!(
            "import succeeded. {num_imported} recipes imported."
        ));
        ui.button("okay").clicked().then_some(Self::Ready)
    }
}

struct IngredientListWindow {
    all_ingredients: BTreeMap<String, Ingredient>,
}

impl IngredientListWindow {
    fn new(conn: &mut database::Connection) -> Self {
        use database::schema::ingredients::dsl::*;
        let all_ingredients = ingredients
            .select(Ingredient::as_select())
            .load(conn)
            .unwrap();
        Self {
            all_ingredients: all_ingredients
                .into_iter()
                .map(|i| (i.name.clone(), i))
                .collect(),
        }
    }

    fn update(&mut self, _conn: &mut database::Connection, ctx: &egui::Context) -> bool {
        let mut open = true;
        egui::Window::new("Ingredients")
            .open(&mut open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("All Ingredients").show(ui, |ui| {
                        ui.label("Name");
                        ui.label("Category");
                        ui.end_row();

                        for ingredient in self.all_ingredients.values() {
                            ui.label(&ingredient.name);
                            ui.label(ingredient.category.as_deref().unwrap_or(""));
                            ui.end_row();
                        }
                    });
                })
            });
        !open
    }
}

fn this_week() -> chrono::NaiveWeek {
    let today = chrono::Local::now().date_naive();
    today.week(chrono::Weekday::Sun)
}

struct RecipeWeek {
    start: chrono::NaiveWeek,
    week: HashMap<chrono::Weekday, RecipeHandle>,
}

impl RecipeWeek {
    fn new(conn: &mut database::Connection, week: chrono::NaiveWeek) -> Self {
        Self {
            week: Self::get_calendar_week(conn, week),
            start: week,
        }
    }

    fn get_calendar_week(
        conn: &mut database::Connection,
        start: chrono::NaiveWeek,
    ) -> HashMap<chrono::Weekday, RecipeHandle> {
        use chrono::Datelike as _;
        use database::schema::calendar::dsl::*;
        use diesel::BoolExpressionMethods as _;

        calendar
            .inner_join(database::schema::recipes::table)
            .select((day, RecipeHandle::as_select()))
            .filter(day.ge(start.first_day()).and(day.le(start.last_day())))
            .load(conn)
            .unwrap()
            .into_iter()
            .map(|(d, r): (chrono::NaiveDate, RecipeHandle)| (d.weekday(), r))
            .collect()
    }

    fn delete_calendar_entry(conn: &mut database::Connection, delete_day: chrono::NaiveDate) {
        use database::schema::calendar::dsl::*;
        use diesel::delete;

        delete(calendar.filter(day.eq(delete_day)))
            .execute(conn)
            .unwrap();
    }

    fn pick_date(
        &mut self,
        conn: &mut database::Connection,
        body: impl FnOnce(&mut chrono::NaiveDate),
    ) {
        use chrono::Weekday::*;

        let mut date = self.start.first_day();
        body(&mut date);
        let new_start = date.week(Sun);
        if self.start != new_start {
            self.start = new_start;
            self.week = Self::get_calendar_week(conn, self.start);
        }
    }

    fn recipes(&self) -> Vec<(chrono::Weekday, Option<RecipeHandle>)> {
        use chrono::Weekday::*;

        [Sun, Mon, Tue, Wed, Thu, Fri, Sat]
            .into_iter()
            .map(|day| (day, self.week.get(&day).cloned()))
            .collect()
    }

    fn advance(&mut self, conn: &mut database::Connection) {
        use chrono::Weekday::*;

        self.start = self
            .start
            .first_day()
            .checked_add_days(chrono::Days::new(7))
            .unwrap()
            .week(Sun);
        self.week = Self::get_calendar_week(conn, self.start);
    }

    fn previous(&mut self, conn: &mut database::Connection) {
        use chrono::Weekday::*;

        self.start = self
            .start
            .first_day()
            .checked_sub_days(chrono::Days::new(7))
            .unwrap()
            .week(Sun);
        self.week = Self::get_calendar_week(conn, self.start);
    }

    fn date_for_day(&self, day: chrono::Weekday) -> chrono::NaiveDate {
        use chrono::Weekday::*;

        let day_number = [Sun, Mon, Tue, Wed, Thu, Fri, Sat]
            .into_iter()
            .position(|i| i == day)
            .unwrap();
        self.start
            .first_day()
            .checked_add_days(chrono::Days::new(day_number as u64))
            .unwrap()
    }

    fn clear_day(&mut self, conn: &mut database::Connection, day: chrono::Weekday) {
        Self::delete_calendar_entry(conn, self.date_for_day(day));
        self.week.remove(&day);
    }
}

struct CalendarWindow {
    week: RecipeWeek,
    edit_mode: bool,
}

impl CalendarWindow {
    fn new(conn: &mut database::Connection) -> Self {
        Self {
            week: RecipeWeek::new(conn, this_week()),
            edit_mode: false,
        }
    }

    fn update(&mut self, conn: &mut database::Connection, ctx: &egui::Context) -> bool {
        let mut open = true;
        egui::Window::new("Calendar")
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("calendar grid").show(ui, |ui| {
                    ui.label(format!("Week of "));
                    self.week.pick_date(conn, |date| {
                        ui.add(egui_extras::DatePickerButton::new(date));
                    });
                    ui.end_row();

                    for (day, recipe) in self.week.recipes() {
                        ui.label(day.to_string());
                        if let Some(recipe) = recipe {
                            ui.label(recipe.name.clone());
                            if self.edit_mode && ui.button("Clear").clicked() {
                                self.week.clear_day(conn, day);
                                self.edit_mode = false;
                            }
                        } else {
                            ui.label("No Recipe");
                        }
                        ui.end_row();
                    }
                });
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut self.edit_mode, "Edit");
                    if ui.button("Next").clicked() {
                        self.week.advance(conn);
                    }
                    if ui.button("Previous").clicked() {
                        self.week.previous(conn);
                    }
                });
            });

        !open
    }
}

pub struct RecipeManager {
    category_list: CategoryListWindow,
    conn: database::Connection,
    import_window: Option<ImportWindow>,
    recipe_lists: HashMap<RecipeCategoryId, RecipeListWindow>,
    recipes: HashMap<RecipeId, RecipeWindow>,
    ingredient_list_window: Option<IngredientListWindow>,
    calendar_window: Option<CalendarWindow>,
}

impl RecipeManager {
    pub fn new(mut conn: database::Connection) -> Self {
        Self {
            category_list: CategoryListWindow::new(&mut conn),
            conn,
            import_window: None,
            recipe_lists: Default::default(),
            recipes: Default::default(),
            ingredient_list_window: None,
            calendar_window: None,
        }
    }

    fn update_category_list_window(&mut self, ctx: &egui::Context) {
        self.category_list
            .update(ctx, &mut self.conn, &mut self.recipe_lists);
    }

    fn update_recipe_list_windows(&mut self, ctx: &egui::Context) {
        for (id, mut list) in mem::take(&mut self.recipe_lists) {
            let closed = list.update(ctx, &mut self.conn, &mut self.recipes);

            if !closed {
                self.recipe_lists.insert(id, list);
            }
        }
    }

    fn update_recipes(&mut self, ctx: &egui::Context) {
        for (id, mut recipe) in mem::take(&mut self.recipes) {
            let old_name = recipe.recipe.name.clone();
            let closed = recipe.update(ctx, &mut self.conn);

            if old_name != recipe.recipe.name {
                if let Some(list) = self.recipe_lists.get_mut(&recipe.recipe.category) {
                    if let Some(r) = list.recipes.get_mut(&recipe.recipe.id) {
                        r.name = recipe.recipe.name.clone();
                    }
                }
            }

            if !closed {
                self.recipes.insert(id, recipe);
            }
        }
    }

    fn update_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Import").clicked() && self.import_window.is_none() {
                        self.import_window = Some(ImportWindow::default());
                        ui.close_menu();
                    }
                    if ui.button("Ingredients").clicked() && self.ingredient_list_window.is_none() {
                        self.ingredient_list_window =
                            Some(IngredientListWindow::new(&mut self.conn));
                        ui.close_menu();
                    }
                    if ui.button("Calendar").clicked() && self.calendar_window.is_none() {
                        self.calendar_window = Some(CalendarWindow::new(&mut self.conn));
                        ui.close_menu();
                    }
                });
            });
        });
    }

    fn update_import_window(&mut self, ctx: &egui::Context) {
        if let Some(window) = &mut self.import_window {
            if window.update(&mut self.conn, ctx) {
                self.import_window = None;
            }
        }
    }

    fn update_ingredient_window(&mut self, ctx: &egui::Context) {
        if let Some(window) = &mut self.ingredient_list_window {
            if window.update(&mut self.conn, ctx) {
                self.ingredient_list_window = None;
            }
        }
    }

    fn update_calendar_window(&mut self, ctx: &egui::Context) {
        if let Some(window) = &mut self.calendar_window {
            if window.update(&mut self.conn, ctx) {
                self.calendar_window = None;
            }
        }
    }
}

impl eframe::App for RecipeManager {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_menu(ctx);
        self.update_import_window(ctx);
        self.update_ingredient_window(ctx);
        self.update_category_list_window(ctx);
        self.update_recipe_list_windows(ctx);
        self.update_recipes(ctx);
        self.update_calendar_window(ctx);
    }
}
