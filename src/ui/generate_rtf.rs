use super::calendar::{full_day_name, RecipeWeek};
use crate::database::models::{Ingredient, IngredientId, IngredientMeasurement, IngredientUsage};
use std::collections::BTreeMap;
use std::fmt;

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

fn rich_text_heading(text: &str, week: chrono::NaiveWeek) -> String {
    let mut rich_text = String::new();
    rich_text += &format!("\\f0\\b\\fs24 \\cf0 {text} for the Week \\\n");
    rich_text += &week
        .first_day()
        .format_with_items(chrono::format::StrftimeItems::new("of the %e, %B %Y\n"))
        .to_string();
    rich_text += "\\f1\\b0 ";
    rich_text
}

pub fn generate_and_open_menu(week: &RecipeWeek) -> crate::Result<()> {
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

pub fn generate_and_open_shopping_list(
    week: chrono::NaiveWeek,
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
