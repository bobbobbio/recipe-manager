CREATE TABLE ingredients (
    id INTEGER PRIMARY KEY NOT NULL,
    name VARCHAR NOT NULL,
    category VARCHAR
);

CREATE TABLE recipe_categories (
    id INTEGER PRIMARY KEY NOT NULL,
    name VARCHAR NOT NULL
);

CREATE TABLE recipes (
    id INTEGER PRIMARY KEY NOT NULL,
    name VARCHAR NOT NULL,
    description TEXT NOT NULL,
    duration TEXT CHECK ( duration IN (
        'short',
        'medium',
        'long',
        'really_long'
    ) ) NOT NULL,
    category INTEGER NOT NULL,
    FOREIGN KEY(category) REFERENCES recipe_categories(id)
);

CREATE TABLE ingredient_usages (
    id INTEGER PRIMARY KEY NOT NULL,
    recipe_id INTEGER NOT NULL,
    ingredient_id INTEGER NOT NULL,
    quantity REAL NOT NULL,
    quantity_units TEXT CHECK ( quantity_units IN (
        'cups',
        'fluid_ounces',
        'grams',
        'kilograms',
        'kiloliters',
        'liters',
        'milligrams',
        'milliliters',
        'ounces',
        'pounds',
        'quart',
        'tablespoons',
        'teaspoons'
    ) ),
    FOREIGN KEY(recipe_id) REFERENCES recipes(id),
    FOREIGN KEY(ingredient_id) REFERENCES ingredients(id)
);

CREATE TABLE ingredient_calories (
    id INTEGER PRIMARY KEY NOT NULL,
    ingredient_id INTEGER NOT NULL,
    calories REAL NOT NULL,
    quantity REAL NOT NULL,
    quantity_units TEXT CHECK ( quantity_units IN (
        'cups',
        'fluid_ounces',
        'grams',
        'kilograms',
        'kiloliters',
        'liters',
        'milligrams',
        'milliliters',
        'ounces',
        'pounds',
        'quart',
        'tablespoons',
        'teaspoons'
    ) ),
    FOREIGN KEY(ingredient_id) REFERENCES ingredients(id)
);

CREATE TABLE calendar (
    day DATE PRIMARY KEY NOT NULL,
    recipe_id INTEGER NOT NULL,
    FOREIGN KEY(recipe_id) REFERENCES recipes(id)
);
