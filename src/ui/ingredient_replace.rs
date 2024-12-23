use crate::database::models::Ingredient;
use crate::{
    database,
    ui::{new_error_toast, query, search::SearchWidget},
};

pub enum UpdateEvent {
    Closed,
    IngredientReplaced,
    IngredientDeleted,
}

#[derive(Default)]
pub struct IngredientReplaceWindow {
    remove_name: String,
    remove: Option<Ingredient>,
    remove_cached_query: Option<query::CachedQuery<Ingredient>>,

    fill_name: String,
    fill: Option<Ingredient>,
    fill_cached_query: Option<query::CachedQuery<Ingredient>>,

    delete: bool,
    result_text: Option<String>,
}

impl IngredientReplaceWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];
        let mut open = true;
        egui::Window::new("Replace Ingredients")
            .open(&mut open)
            .max_height(10.0)
            .default_width(600.0)
            .show(ctx, |ui| {
                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::exact(120.0))
                    .size(egui_extras::Size::remainder())
                    .size(egui_extras::Size::exact(25.0))
                    .size(egui_extras::Size::remainder())
                    .size(egui_extras::Size::exact(80.0))
                    .size(egui_extras::Size::exact(60.0))
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            ui.label("Replace all usages of");
                        });
                        strip.cell(|ui| {
                            ui.add(
                                SearchWidget::new(
                                    "replace ingredient remove",
                                    &mut self.remove_name,
                                    &mut self.remove,
                                    |query| {
                                        query::search_ingredients(
                                            conn,
                                            &mut self.remove_cached_query,
                                            query,
                                        )
                                    },
                                )
                                .desired_width(f32::INFINITY),
                            );
                        });
                        strip.cell(|ui| {
                            ui.label("with");
                        });
                        strip.cell(|ui| {
                            ui.add(
                                SearchWidget::new(
                                    "replace ingredient fill",
                                    &mut self.fill_name,
                                    &mut self.fill,
                                    |query| {
                                        query::search_ingredients(
                                            conn,
                                            &mut self.fill_cached_query,
                                            query,
                                        )
                                    },
                                )
                                .desired_width(f32::INFINITY),
                            );
                        });
                        strip.cell(|ui| {
                            ui.checkbox(&mut self.delete, "and delete");
                        });
                        strip.cell(|ui| {
                            if ui.button("Execute").clicked() {
                                match (&self.remove, &self.fill) {
                                    (Some(remove), Some(fill)) => {
                                        let num_replaced =
                                            query::replace_ingredient(conn, remove.id, fill.id);
                                        events.push(UpdateEvent::IngredientReplaced);
                                        if self.delete {
                                            query::delete_ingredient(conn, remove.id);
                                            events.push(UpdateEvent::IngredientDeleted);
                                        }
                                        *self = Self::new();
                                        self.result_text =
                                            Some(format!("{num_replaced} recipes updated."));
                                    }
                                    _ => {
                                        toasts.add(new_error_toast("Couldn't find ingredient"));
                                    }
                                }
                            }
                        });
                    });
                if let Some(text) = &self.result_text {
                    ui.label(text);
                }
            });
        if !open {
            events.push(UpdateEvent::Closed);
        }
        events
    }
}
