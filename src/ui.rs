// Copyright 2023 Remi Bernotavicius

mod category_list;
mod query;
mod recipe;
mod recipe_list;

use crate::database;
use crate::database::models::{
    Ingredient,
    RecipeCategoryId, RecipeHandle, RecipeId,
};
use crate::import;
use category_list::CategoryListWindow;
use diesel::QueryDsl as _;
use diesel::RunQueryDsl as _;
use diesel::SelectableHelper as _;
use eframe::egui;
use recipe::RecipeWindow;
use recipe_list::RecipeListWindow;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::mem;

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
            week: query::get_calendar_week(conn, week),
            start: week,
        }
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
            self.week = query::get_calendar_week(conn, self.start);
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
        self.week = query::get_calendar_week(conn, self.start);
    }

    fn previous(&mut self, conn: &mut database::Connection) {
        use chrono::Weekday::*;

        self.start = self
            .start
            .first_day()
            .checked_sub_days(chrono::Days::new(7))
            .unwrap()
            .week(Sun);
        self.week = query::get_calendar_week(conn, self.start);
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
        query::delete_calendar_entry(conn, self.date_for_day(day));
        self.week.remove(&day);
    }

    fn schedule(&mut self, conn: &mut database::Connection, day: chrono::Weekday, id: RecipeId) {
        query::insert_or_update_calendar_entry(conn, self.date_for_day(day), id);
        *self = Self::new(conn, self.start);
    }
}

#[derive(Default)]
struct RecipeBeingSelected {
    name: String,
    recipe_id: Option<RecipeId>,
    cached_recipe_search: Option<query::CachedQuery<RecipeId>>,
}

struct CalendarWindow {
    week: RecipeWeek,
    edit_mode: bool,
    recipes_being_selected: HashMap<chrono::Weekday, RecipeBeingSelected>,
}

impl CalendarWindow {
    fn new(conn: &mut database::Connection) -> Self {
        Self {
            week: RecipeWeek::new(conn, this_week()),
            edit_mode: false,
            recipes_being_selected: HashMap::new(),
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
                            if self.edit_mode {
                                let e = self.recipes_being_selected.entry(day).or_default();
                                ui.add_sized(
                                    egui::vec2(200.0, 15.0),
                                    SearchWidget::new(
                                        format!("recipe for {day}"),
                                        &mut e.name,
                                        &mut e.recipe_id,
                                        |query| {
                                            query::search_recipes(
                                                conn,
                                                &mut e.cached_recipe_search,
                                                query,
                                            )
                                        },
                                    ),
                                );
                                if ui.button("Select").clicked() && e.recipe_id.is_some() {
                                    self.week.schedule(conn, day, e.recipe_id.unwrap());
                                    self.edit_mode = false;
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut self.edit_mode, "Edit");
                    if ui.button("Next").clicked() {
                        self.week.advance(conn);
                        self.recipes_being_selected.clear();
                    }
                    if ui.button("Previous").clicked() {
                        self.week.previous(conn);
                        self.recipes_being_selected.clear();
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
            let old_name = recipe.recipe().name.to_owned();
            let closed = recipe.update(ctx, &mut self.conn);

            if old_name != recipe.recipe().name {
                if let Some(list) = self.recipe_lists.get_mut(&recipe.recipe().category) {
                    list.recipe_name_changed(recipe.recipe().id, recipe.recipe().name.clone());
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
