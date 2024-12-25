use super::{generate_rtf, new_error_toast, query, search::SearchWidget, PressedEnterExt as _};
use crate::database;
use crate::database::models::{RecipeHandle, RecipeId};
use std::collections::HashMap;

pub fn this_week() -> chrono::NaiveWeek {
    let today = chrono::Local::now().date_naive();
    today.week(chrono::Weekday::Sun)
}

pub fn full_day_name(day: chrono::Weekday) -> &'static str {
    use chrono::Weekday::*;

    match day {
        Sun => "Sunday",
        Mon => "Monday",
        Tue => "Tuesday",
        Wed => "Wednesday",
        Thu => "Thursday",
        Fri => "Friday",
        Sat => "Saturday",
    }
}

pub struct RecipeWeek {
    start: chrono::NaiveWeek,
    week: HashMap<chrono::Weekday, RecipeHandle>,
}

impl RecipeWeek {
    pub fn new(conn: &mut database::Connection, week: chrono::NaiveWeek) -> Self {
        Self {
            week: query::get_calendar_week(conn, week),
            start: week,
        }
    }

    pub fn pick_date(
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

    pub fn recipes(&self) -> Vec<(chrono::Weekday, Option<RecipeHandle>)> {
        use chrono::Weekday::*;

        [Sun, Mon, Tue, Wed, Thu, Fri, Sat]
            .into_iter()
            .map(|day| (day, self.week.get(&day).cloned()))
            .collect()
    }

    pub fn advance(&mut self, conn: &mut database::Connection) {
        use chrono::Weekday::*;

        self.start = self
            .start
            .first_day()
            .checked_add_days(chrono::Days::new(7))
            .unwrap()
            .week(Sun);
        self.week = query::get_calendar_week(conn, self.start);
    }

    pub fn previous(&mut self, conn: &mut database::Connection) {
        use chrono::Weekday::*;

        self.start = self
            .start
            .first_day()
            .checked_sub_days(chrono::Days::new(7))
            .unwrap()
            .week(Sun);
        self.week = query::get_calendar_week(conn, self.start);
    }

    pub fn date_for_day(&self, day: chrono::Weekday) -> chrono::NaiveDate {
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

    pub fn clear_day(&mut self, conn: &mut database::Connection, day: chrono::Weekday) {
        query::delete_calendar_entry(conn, self.date_for_day(day));
        self.week.remove(&day);
    }

    pub fn schedule(
        &mut self,
        conn: &mut database::Connection,
        day: chrono::Weekday,
        id: RecipeId,
    ) {
        query::insert_or_update_calendar_entry(conn, self.date_for_day(day), id);
        *self = Self::new(conn, self.start);
    }

    pub fn week(&self) -> chrono::NaiveWeek {
        self.start
    }

    pub fn refresh(&mut self, conn: &mut database::Connection) {
        self.week = query::get_calendar_week(conn, self.start);
    }
}

#[derive(Default)]
struct RecipeBeingSelected {
    name: String,
    recipe_id: Option<RecipeId>,
    cached_recipe_search: Option<query::CachedQuery<RecipeId>>,
}

pub enum UpdateEvent {
    Closed,
    RecipeScheduled { week: chrono::NaiveWeek },
}

pub struct CalendarWindow {
    week: RecipeWeek,
    edit_mode: bool,
    recipes_being_selected: HashMap<chrono::Weekday, RecipeBeingSelected>,
}

impl CalendarWindow {
    pub fn new(conn: &mut database::Connection) -> Self {
        Self::new_with_args(conn, false)
    }

    fn new_with_args(conn: &mut database::Connection, edit_mode: bool) -> Self {
        Self {
            week: RecipeWeek::new(conn, this_week()),
            edit_mode,
            recipes_being_selected: HashMap::new(),
        }
    }

    fn update_table(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        body: &mut egui_extras::TableBody<'_>,
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];
        for (day, recipe) in self.week.recipes() {
            body.row(20.0, |mut row| {
                row.col(|ui| {
                    ui.label(full_day_name(day));
                });
                if let Some(recipe) = recipe {
                    row.col(|ui| {
                        ui.label(recipe.name.clone());
                    });
                    row.col(|ui| {
                        if self.edit_mode && ui.button("Clear").clicked() {
                            self.week.clear_day(conn, day);
                        }
                    });
                    row.col(|_| {});
                } else {
                    row.col(|ui| {
                        ui.label("No Recipe");
                    });
                    if self.edit_mode {
                        let entry = self.recipes_being_selected.entry(day).or_default();
                        let mut selected = false;
                        row.col(|ui| {
                            selected |= ui
                                .add(
                                    SearchWidget::new(
                                        ("calendar select recipe", day),
                                        &mut entry.name,
                                        &mut entry.recipe_id,
                                        |query| {
                                            query::search_recipes(
                                                conn,
                                                &mut entry.cached_recipe_search,
                                                query,
                                            )
                                        },
                                    )
                                    .desired_width(ui.available_width() - 20.0)
                                    .hint_text("search for recipe"),
                                )
                                .pressed_enter();
                        });

                        let e = !entry.name.is_empty();
                        row.col(|ui| {
                            selected |= ui.add_enabled(e, egui::Button::new("Select")).clicked();
                        });

                        if selected && e {
                            if let Some(recipe_id) = entry.recipe_id {
                                self.week.schedule(conn, day, recipe_id);
                                *entry = Default::default();

                                events.push(UpdateEvent::RecipeScheduled {
                                    week: self.week.week().clone(),
                                });
                            } else {
                                toasts.add(new_error_toast("Couldn't find recipe"));
                            }
                        }
                    }
                }
            });
        }
        events
    }

    fn update_controls(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        ui: &mut egui::Ui,
    ) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.toggle_value(&mut self.edit_mode, "Edit");
            if ui.button("Previous").clicked() {
                self.week.previous(conn);
                self.recipes_being_selected.clear();
            }
            if ui.button("Next").clicked() {
                self.week.advance(conn);
                self.recipes_being_selected.clear();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Menu").clicked() {
                    if let Err(error) = generate_rtf::generate_and_open_menu(&self.week) {
                        toasts.add(new_error_toast(format!("Error generating menu: {error}")));
                    }
                }
                if ui.button("Shopping List").clicked() {
                    let mut ingredients = vec![];
                    for (_, recipe) in self.week.recipes() {
                        if let Some(recipe) = recipe {
                            ingredients.extend(query::get_ingredients_for_recipe(conn, recipe.id));
                        }
                    }
                    if let Err(error) =
                        generate_rtf::generate_and_open_shopping_list(self.week.week(), ingredients)
                    {
                        toasts.add(new_error_toast(format!(
                            "Error generating shopping list: {error}"
                        )));
                    }
                }
            });
        });
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

        let title_height = text_height + spacing;
        let controls_height = button_height + spacing + separator_height;

        let mut events = vec![];
        let mut open = true;
        egui::Window::new("Calendar")
            .open(&mut open)
            .default_width(500.0)
            .default_height(100.0)
            .show(ctx, |ui| {
                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::exact(title_height))
                    .size(egui_extras::Size::remainder())
                    .size(egui_extras::Size::exact(controls_height))
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("Week of "));
                                self.week.pick_date(conn, |date| {
                                    ui.add(egui_extras::DatePickerButton::new(date));
                                });
                            });
                        });
                        strip.cell(|ui| {
                            egui_extras::TableBuilder::new(ui)
                                .id_salt("calendar table")
                                .striped(false)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .column(egui_extras::Column::exact(80.0))
                                .column(egui_extras::Column::auto())
                                .column(egui_extras::Column::remainder())
                                .column(egui_extras::Column::exact(50.0))
                                .body(|mut body| {
                                    events.extend(self.update_table(conn, toasts, &mut body));
                                });
                        });
                        strip.cell(|ui| {
                            self.update_controls(conn, toasts, ui);
                        });
                    });
            });

        if !open {
            events.push(UpdateEvent::Closed);
        }
        events
    }

    pub fn recipe_scheduled(&mut self, conn: &mut database::Connection) {
        self.week.refresh(conn);
    }

    pub fn calendar_imported(&mut self, conn: &mut database::Connection) {
        self.week.refresh(conn);
    }

    pub fn recipe_deleted(&mut self, conn: &mut database::Connection) {
        *self = Self::new_with_args(conn, self.edit_mode);
    }

    pub fn week(&self) -> chrono::NaiveWeek {
        self.week.week()
    }
}
