//! This module contains GUI code related to the sequence visulization.

use std::cmp::{max, min};

use eframe::{
    egui::{
        pos2, vec2, Align2, Color32, FontFamily, FontId, Frame, Pos2, Rect, ScrollArea, Sense,
        Shape, Stroke, Ui,
    },
    emath::RectTransform,
    epaint::PathShape,
};

use crate::{
    gui::ROW_SPACING,
    primer::{
        PrimerDirection,
        PrimerDirection::{Forward, Reverse},
    },
    util::{get_row_ranges, make_seq_str, seq_i_to_pixel},
    PagePrimerCreation, State,
};

// Pub for use in `util` functions.
pub const FONT_SIZE_SEQ: f32 = 14.;
pub const COLOR_SEQ: Color32 = Color32::LIGHT_BLUE;

pub const NT_WIDTH_PX: f32 = 8.; // todo: Automatic way? This is valid for monospace font, size 14.
pub const VIEW_AREA_PAD: f32 = 40.;
pub const SEQ_ROW_SPACING_PX: f32 = 34.;

pub const TEXT_X_START: f32 = VIEW_AREA_PAD / 2.;
pub const TEXT_Y_START: f32 = TEXT_X_START;
const MAX_SEQ_AREA_HEIGHT: u16 = 300;

/// Make a visual arrow for a primer. For  use inside a Frame::canvas.
/// Note: This function is fragile, and was constructed partly by trial and error.
fn primer_arrow(
    mut bounds_r0: (Pos2, Pos2),
    mut bounds_r1: Option<(Pos2, Pos2)>, // Assumes no more than two rows.
    direction: PrimerDirection,
    label: &str,
    ui: &mut Ui,
) -> Vec<Shape> {
    let color_arrow = match direction {
        Forward => Color32::from_rgb(255, 0, 255),
        Reverse => Color32::LIGHT_YELLOW,
    };

    let color_label = Color32::LIGHT_GREEN;
    let arrow_width = 2.;

    const VERTICAL_OFFSET: f32 = 20.; // Number of pixels above the sequence text.
    const LABEL_OFFSET: f32 = 7.;
    const HEIGHT: f32 = 16.;
    const SLANT: f32 = 20.; // slant different, in pixels, for the arrow.

    const V_OFF_SET_REV: f32 = 2. * VERTICAL_OFFSET - 12.;

    bounds_r0.0.y -= VERTICAL_OFFSET;
    bounds_r0.1.y -= VERTICAL_OFFSET;
    if let Some(b) = bounds_r1.as_mut() {
        b.0.y -= VERTICAL_OFFSET;
        b.1.y -= VERTICAL_OFFSET;
    }

    let mut result = Vec::new();

    let ctx = ui.ctx();

    let mut top_left = bounds_r0.0;

    // Slant only if single-line.
    let mut top_right = if bounds_r1.is_none() {
        pos2(bounds_r0.1.x - SLANT, bounds_r0.1.y)
    } else {
        pos2(bounds_r0.1.x, bounds_r0.1.y)
    };
    let mut bottom_left = pos2(bounds_r0.0.x, bounds_r0.0.y + HEIGHT);
    let mut bottom_right = pos2(bounds_r0.1.x, bounds_r0.1.y + HEIGHT);

    if direction == Reverse {
        let temp = top_right;
        top_right = top_left;
        top_left = temp;

        let temp = bottom_left;
        bottom_left = bottom_right;
        bottom_right = temp;

        top_left.y += V_OFF_SET_REV;
        top_right.y += V_OFF_SET_REV;
        bottom_left.y += V_OFF_SET_REV;
        bottom_right.y += V_OFF_SET_REV;
    }

    let points = vec![top_left, bottom_left, bottom_right, top_right];

    result.push(Shape::Path(PathShape::closed_line(
        points,
        Stroke::new(arrow_width, color_arrow),
    )));

    if let Some(b) = bounds_r1 {
        let mut top_left = b.0;
        let mut bottom_left = pos2(b.0.x, b.0.y + HEIGHT);
        let mut bottom_right = pos2(b.1.x, b.1.y + HEIGHT);
        let mut top_right = pos2(b.1.x - SLANT, b.1.y);

        // todo: DRY.
        if direction == Reverse {
            top_right.x += SLANT;
            bottom_left.x += SLANT;

            let temp = top_right;
            top_right = top_left;
            top_left = temp;

            let temp = bottom_left;
            bottom_left = bottom_right;
            bottom_right = temp;

            top_left.y += V_OFF_SET_REV;
            top_right.y += V_OFF_SET_REV;
            bottom_left.y += V_OFF_SET_REV;
            bottom_right.y += V_OFF_SET_REV;
        }

        let points = vec![top_left, bottom_left, bottom_right, top_right];

        result.push(Shape::Path(PathShape::closed_line(
            points,
            Stroke::new(arrow_width, color_arrow),
        )));
    }
    // }
    // Reverse => {}
    // };

    let label_start_x = match direction {
        Forward => bounds_r0.0.x,
        Reverse => bounds_r0.1.x,
    } + LABEL_OFFSET;

    let label_pos = match direction {
        Forward => pos2(label_start_x, bounds_r0.0.y + LABEL_OFFSET),
        Reverse => pos2(label_start_x, bounds_r0.0.y + LABEL_OFFSET + V_OFF_SET_REV),
    };

    let label = ctx.fonts(|fonts| {
        Shape::text(
            fonts,
            label_pos,
            Align2::LEFT_CENTER,
            label,
            FontId::new(16., FontFamily::Proportional),
            color_label,
        )
    });

    result.push(label);
    result
}

/// Draw the sequence with primers, insertion points, and other data visible, A/R
pub fn sequence_vis(state: &State, ui: &mut Ui) {
    let nt_chars_per_row = ((ui.available_width() - VIEW_AREA_PAD) / NT_WIDTH_PX) as usize; // todo: +1 etc?

    // let (id, rect) = ui.allocate_space(desired_size);

    let mut shapes = vec![];

    let seq_to_disp = match state.ui.page_primer_creation {
        PagePrimerCreation::Amplification => &state.seq_amplicon,
        PagePrimerCreation::SlicFc => &state.seq_vector_with_insert,
    };
    let seq_len = seq_to_disp.len();

    let row_ranges = get_row_ranges(seq_len, nt_chars_per_row);

    ScrollArea::vertical().id_source(0).show(ui, |ui| {
        Frame::canvas(ui.style())
            .fill(Color32::from_rgb(10, 20, 10))
            .show(ui, |ui| {
                let (mut response, painter) = {
                    // Estimate required height, based on seq len.
                    let total_seq_height = row_ranges.len() as f32 * SEQ_ROW_SPACING_PX + 60.;
                    // let height = min(total_seq_height as u16, MAX_SEQ_AREA_HEIGHT);

                    let height = total_seq_height;

                    let desired_size = vec2(ui.available_width(), height as f32);
                    ui.allocate_painter(desired_size, Sense::click())
                };

                // let to_screen =
                //     RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0), rect);

                let to_screen = RectTransform::from_to(
                    Rect::from_min_size(Pos2::ZERO, response.rect.size()),
                    response.rect,
                );

                // let to_screen = RectTransform::from_to(
                // Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
                // Rect::from_min_size(Pos2::ZERO, response.rect.size()),
                // response.rect,
                // );

                let from_screen = to_screen.inverse();

                let ctx = ui.ctx();

                let mut text_y = TEXT_Y_START;

                for range in &row_ranges {
                    // This max trigger is likely to occur on the last row.
                    let r = range.start..min(range.end, seq_len);
                    let seq_this_row = &seq_to_disp[r];

                    // todo: This line is causing crashes when the view is stretched.
                    let pos = to_screen * pos2(TEXT_X_START, text_y);

                    // todo: Find a way to get the pixel coordinates of each nt char.

                    shapes.push(ctx.fonts(|fonts| {
                        Shape::text(
                            fonts,
                            pos,
                            Align2::LEFT_CENTER,
                            make_seq_str(seq_this_row),
                            // Note: Monospace is important for sequences.
                            FontId::new(FONT_SIZE_SEQ, FontFamily::Monospace),
                            COLOR_SEQ,
                        )
                    }));
                    text_y += SEQ_ROW_SPACING_PX;
                }

                let seq_i_to_pixel_rel = |a, b| to_screen * seq_i_to_pixel(a, b);

                // Add primer arrows.
                for prim_data in &state.primer_data {
                    let primer_matches = match state.ui.page_primer_creation {
                        PagePrimerCreation::Amplification => &prim_data.matches_amplification_seq,
                        PagePrimerCreation::SlicFc => &prim_data.matches_vector_with_insert,
                    };
                    // todo: Sort out the direction. By matches, most likely.

                    // todo: Do not run these calcs each time! Cache.
                    for (direction, seq_range) in primer_matches {
                        let (start, end) = match direction {
                            Forward => (seq_range.start, seq_range.end),
                            Reverse => (seq_len - seq_range.start, seq_len - seq_range.end),
                        };

                        let start_pos = seq_i_to_pixel_rel(start, &row_ranges);
                        let end_pos = seq_i_to_pixel_rel(end, &row_ranges);

                        // Check if we split across rows.
                        let (bounds_row_0, bounds_row_1) = if start_pos.y == end_pos.y {
                            ((start_pos, end_pos), None)
                        } else {
                            // let (col, row) = seq_i_to_col_row(seq_range.start, &row_ranges);

                            // let row_0_end = seq_i_to_pixel_rel(seq_range.start, &row_ranges);

                            match direction {
                                Forward => {
                                    let row_0_end = pos2(
                                        TEXT_X_START + NT_WIDTH_PX * (1. + nt_chars_per_row as f32),
                                        start_pos.y,
                                    );
                                    let row_1_start = pos2(TEXT_X_START, end_pos.y);

                                    ((start_pos, row_0_end), Some((row_1_start, end_pos)))
                                }
                                Reverse => {
                                    // todo: DRY
                                    // let row_0_end = pos2(
                                    //     TEXT_X_START + NT_WIDTH_PX * (1. + nt_chars_per_row as f32),
                                    //     start_pos.y,
                                    // );

                                    let row_0_end = pos2(
                                        ui.available_width()
                                            - (TEXT_X_START
                                                + NT_WIDTH_PX * (1. + nt_chars_per_row as f32)),
                                        start_pos.y,
                                    );

                                    let row_1_start = pos2(ui.available_width(), end_pos.y);

                                    ((row_0_end, start_pos), Some((end_pos, row_1_start)))
                                    // ((start_pos, row_0_end), Some((row_1_start, end_pos)))
                                }
                            }
                        };

                        let mut arrow_fwd = primer_arrow(
                            bounds_row_0,
                            bounds_row_1,
                            *direction,
                            &prim_data.description,
                            ui,
                        );
                        shapes.append(&mut arrow_fwd);
                    }
                }
                // }
                // PagePrimerCreation::SlicFc => {}
                // }
                ui.painter().extend(shapes);
            });
    });

    ui.add_space(ROW_SPACING);
}
