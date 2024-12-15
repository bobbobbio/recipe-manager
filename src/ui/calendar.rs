use super::{query, SearchWidget};
use crate::database;
use crate::database::models::{RecipeHandle, RecipeId};
use eframe::egui;
use std::collections::HashMap;

pub fn this_week() -> chrono::NaiveWeek {
    let today = chrono::Local::now().date_naive();
    today.week(chrono::Weekday::Sun)
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

    pub fn week(&self) -> &chrono::NaiveWeek {
        &self.start
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
        Self {
            week: RecipeWeek::new(conn, this_week()),
            edit_mode: false,
            recipes_being_selected: HashMap::new(),
        }
    }

    pub fn refresh(&mut self, conn: &mut database::Connection) {
        self.week.refresh(conn);
    }

    pub fn update(
        &mut self,
        conn: &mut database::Connection,
        ctx: &egui::Context,
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];
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
                                    events.push(UpdateEvent::RecipeScheduled {
                                        week: self.week.week().clone(),
                                    });
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

        if !open {
            events.push(UpdateEvent::Closed);
        }
        events
    }
}
