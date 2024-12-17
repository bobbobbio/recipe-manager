use super::{new_error_toast, query, search::SearchWidget};
use crate::database;
use crate::database::models::{
    Ingredient, IngredientId, IngredientMeasurement, IngredientUsage, RecipeHandle, RecipeId,
};
use eframe::egui;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt;

pub fn this_week() -> chrono::NaiveWeek {
    let today = chrono::Local::now().date_naive();
    today.week(chrono::Weekday::Sun)
}

fn full_day_name(day: chrono::Weekday) -> &'static str {
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

fn rich_text_header() -> String {
    let mut rich_text = String::new();
    rich_text += "{\\rtf1\n";
    rich_text +=
        "{\\fonttbl\\f0\\fnil\\fcharset0 HelveticaNeue-Bold;\\f1\\fswiss\\fcharset0 Helvetica;}\n";

    rich_text += "\\pard";
    for i in 1..13 {
        rich_text += &format!("\\tx{}", i * 560);
    }
    rich_text += "\\pardirnatural\\partightenfactor0\n";
    rich_text
}

fn rich_text_heading(text: &str, week: &chrono::NaiveWeek) -> String {
    let mut rich_text = String::new();
    rich_text += &format!("\\f0\\b\\fs24 \\cf0 {text} for the Week \\\n");
    rich_text += &week
        .first_day()
        .format_with_items(chrono::format::StrftimeItems::new("of the %e, %B %Y\n"))
        .to_string();
    rich_text += "\\f1\\b0 ";
    rich_text
}

fn generate_and_open_menu(week: &RecipeWeek) -> crate::Result<()> {
    let mut rich_text = rich_text_header();
    rich_text += &rich_text_heading("Menu", week.week());
    for (day, recipe) in week.recipes() {
        let day_str = full_day_name(day);
        let recipe = recipe.map(|r| r.name).unwrap_or("No Recipe".into());
        let tabs = if day == chrono::Weekday::Wed {
            "\t"
        } else {
            "\t\t"
        };

        rich_text += &format!("\\\n{day_str}{tabs}{recipe}");
    }
    rich_text += "}";

    let menus_dir = crate::data_path()?.join("menus");
    std::fs::create_dir_all(&menus_dir)?;
    let menu_path = menus_dir.join(format!("menu-{}.rtf", week.week().first_day()));
    std::fs::write(&menu_path, rich_text)?;
    open::that(menu_path)?;
    Ok(())
}

struct ShoppingListItem {
    name: String,
    usages: BTreeMap<Option<IngredientMeasurement>, f32>,
}

impl ShoppingListItem {
    fn new(name: String) -> Self {
        Self {
            name,
            usages: BTreeMap::new(),
        }
    }
}

impl fmt::Display for ShoppingListItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut usages = self.usages.iter().filter_map(|(m, u)| m.map(|m| (m, u)));
        if let Some((m, u)) = usages.next() {
            write!(f, "{u} {}", m.as_str())?;
        }
        for (m, u) in usages {
            write!(f, " and {u} {}", m.as_str())?;
        }
        if let Some(u) = self.usages.get(&None) {
            if self.usages.len() > 1 {
                write!(f, " and {u} {}", self.name)?;
            } else {
                write!(f, "{u} {}", self.name)?;
            }
        } else {
            write!(f, " of {}", self.name)?;
        }
        Ok(())
    }
}

#[test]
fn shopping_list_item() {
    use maplit::btreemap;

    let item = ShoppingListItem {
        name: "tomatoes".into(),
        usages: btreemap! {
            Some(IngredientMeasurement::Cups) => 2.0,
        },
    };
    assert_eq!(item.to_string(), "2 cups of tomatoes");

    let item = ShoppingListItem {
        name: "cans of tomatoes".into(),
        usages: btreemap! {
            Some(IngredientMeasurement::Cups) => 2.0,
            None => 3.0,
        },
    };
    assert_eq!(item.to_string(), "2 cups and 3 cans of tomatoes");

    let item = ShoppingListItem {
        name: "cans of tomatoes".into(),
        usages: btreemap! {
            Some(IngredientMeasurement::Cups) => 2.0,
            Some(IngredientMeasurement::Tablespoons) => 0.5,
            None => 3.0,
        },
    };
    assert_eq!(
        item.to_string(),
        "2 cups and 0.5 tbsp. and 3 cans of tomatoes"
    );

    let item = ShoppingListItem {
        name: "cans of tomatoes".into(),
        usages: btreemap! {
            None => 3.0,
        },
    };
    assert_eq!(item.to_string(), "3 cans of tomatoes");
}

type CategorizedIngredients = BTreeMap<Option<String>, BTreeMap<IngredientId, ShoppingListItem>>;

fn sort_ingredients_by_category(
    ingredients: Vec<(IngredientUsage, Ingredient)>,
) -> CategorizedIngredients {
    let mut map: CategorizedIngredients = BTreeMap::new();
    for (usage, i) in ingredients {
        *map.entry(i.category)
            .or_default()
            .entry(i.id)
            .or_insert(ShoppingListItem::new(i.name))
            .usages
            .entry(usage.quantity_units)
            .or_default() += usage.quantity;
    }
    map
}

fn generate_and_open_shopping_list(
    week: &chrono::NaiveWeek,
    ingredients: Vec<(IngredientUsage, Ingredient)>,
) -> crate::Result<()> {
    let ingredients = sort_ingredients_by_category(ingredients);

    let mut rich_text = rich_text_header();
    rich_text += &rich_text_heading("Shopping List", week);
    rich_text += "\\\n";

    for (cat, ingredients) in &ingredients {
        if let Some(cat) = cat {
            rich_text += &format!("\\\n\\f0\\b ****{cat}****\n\\f1\\b0 ");
            for i in ingredients.values() {
                rich_text += &format!("\\\n{i}");
            }
            rich_text += "\\\n";
        }
    }

    // All the uncategorized ingredients go at the end
    if let Some(ingredients) = ingredients.get(&None) {
        rich_text += &format!("\\\n\\f0\\b ********\n\\f1\\b0 ");
        for i in ingredients.values() {
            rich_text += &format!("\\\n{i}");
        }
        rich_text += "\\\n";
    }

    rich_text += "}";

    let menus_dir = crate::data_path()?.join("shopping-lists");
    std::fs::create_dir_all(&menus_dir)?;
    let menu_path = menus_dir.join(format!("shopping-list-{}.rtf", week.first_day()));
    std::fs::write(&menu_path, rich_text)?;
    open::that(menu_path)?;
    Ok(())
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
        ctx: &egui::Context,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
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
                        ui.label(full_day_name(day));
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
                ui.separator();
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
                    if ui.button("Menu").clicked() {
                        if let Err(error) = generate_and_open_menu(&self.week) {
                            toasts.add(new_error_toast(format!("Error generating menu: {error}")));
                        }
                    }
                    if ui.button("Shopping List").clicked() {
                        let mut ingredients = vec![];
                        for (_, recipe) in self.week.recipes() {
                            if let Some(recipe) = recipe {
                                ingredients
                                    .extend(query::get_ingredients_for_recipe(conn, recipe.id));
                            }
                        }
                        if let Err(error) =
                            generate_and_open_shopping_list(self.week.week(), ingredients)
                        {
                            toasts.add(new_error_toast(format!(
                                "Error generating shopping list: {error}"
                            )));
                        }
                    }
                });
            });

        if !open {
            events.push(UpdateEvent::Closed);
        }
        events
    }
}
