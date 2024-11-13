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
        'pounds',
        'ounces',
        'tablespoons',
        'teaspoons'
    ) ),
    FOREIGN KEY(recipe_id) REFERENCES recipes(id),
    FOREIGN KEY(ingredient_id) REFERENCES ingredients(id)
);
