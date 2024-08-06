//! Contains references, comments, etc about the plasmid.

use eframe::{
    egui,
    egui::{TextEdit, Ui},
};
use eframe::egui::{Label, Vec2};
use crate::{gui::ROW_SPACING, State};

const LABEL_WIDTH: f32 = 140.; // Helps align the text edits, by forcing a fixed label width.
const WIDTH_RATIO: f32 = 0.6;
const ROW_HEIGHT: usize = 1;

/// A convenience function to create a text edit for Option<String>
fn option_edit(val: &mut Option<String>, label: &str, ui: &mut Ui) {
    ui.horizontal(|ui| {
        // ui.allocate_exact_size(Vec2::new(LABEL_WIDTH, 0.0), egui::Sense::hover()); // Reserve space
        ui.label(label);

        // todo: Way without cloning?
        let mut v = val.clone().unwrap_or_default();
        // Don't use these margins if there is a narrow window.
        let response = ui.add(
            TextEdit::multiline(&mut v)
                .desired_width(ui.available_width() * WIDTH_RATIO)
                .desired_rows(ROW_HEIGHT),
        );
        if response.changed() {
            *val = if !v.is_empty() {
                Some(v.to_owned())
            } else {
                None
            };
        }
    });

    ui.add_space(ROW_SPACING / 2.);
}

pub fn metadata_page(state: &mut State, ui: &mut Ui) {
    // todo: YOu need neat grid alignment. How can we make the labels take up constant space?

    ui.heading("References");
    ui.add_space(ROW_SPACING / 2.);

    for ref_ in &mut state.references {
        ui.horizontal(|ui| {
            ui.label("Title:");
            let response = ui.add(
                TextEdit::multiline(&mut ref_.title)
                    .desired_width(ui.available_width() * WIDTH_RATIO)
                    .desired_rows(ROW_HEIGHT),
            );
        });
        ui.add_space(ROW_SPACING / 2.);

        ui.horizontal(|ui| {
            ui.label("Description:");
            let response = ui.add(
                TextEdit::multiline(&mut ref_.description)
                    .desired_width(ui.available_width() * WIDTH_RATIO)
                    .desired_rows(ROW_HEIGHT),
            );
        });
        ui.add_space(ROW_SPACING / 2.);

        option_edit(&mut ref_.authors, "Authors:", ui);
        option_edit(&mut ref_.consortium, "Consortium:", ui);

        option_edit(&mut ref_.journal, "Journal:", ui);
        option_edit(&mut ref_.pubmed, "Pubmed:", ui);
        option_edit(&mut ref_.remark, "Remarks:", ui);

        ui.add_space(ROW_SPACING * 2.);
    }

    ui.heading("Comments");
    ui.add_space(ROW_SPACING);
    if ui.button("➕ Add").clicked() {
        state.comments.push(String::new());
    }

    for comment in &mut state.comments {
        let response = ui.add(
            TextEdit::multiline(comment)
                .desired_width(ui.available_width() * WIDTH_RATIO)
                .desired_rows(ROW_HEIGHT),
        );
        // if response.changed() {
        // }

        ui.add_space(ROW_SPACING / 2.);
    }
}
