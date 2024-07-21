use std::cmp::{max, min};
use eframe::{
    egui::{
        pos2, vec2, Align, Align2, Color32, FontFamily, FontId, Frame, Layout, Pos2, Rect,
        RichText, Sense, Shape, Stroke, TextEdit, Ui,
    },
    emath::RectTransform,
    epaint::PathShape,
};
use egui_extras::{Column, TableBuilder};

// todo: monospace font for all seqs.
use crate::{
    gui::{page_primers_selector, PagePrimerCreation, COL_SPACING, ROW_SPACING},
    primer::{design_amplification_primers, design_slic_fc_primers},
    util,
    util::{make_seq_str, save, seq_from_str},
    State,
};
use crate::{
    primer::{PrimerData, PrimerDirection, TuneSetting},
    util::{get_row_ranges, seq_complement, seq_i_to_pixel},
};

const COLOR_GOOD: Color32 = Color32::GREEN;
const COLOR_MARGINAL: Color32 = Color32::GOLD;
const COLOR_BAD: Color32 = Color32::LIGHT_RED;

const DEFAULT_TRIM_AMT: usize = 32 - 20;

// Constants related to the sequence canvas. Pub for use in `util` functions.
pub const FONT_SIZE_SEQ: f32 = 14.;
pub const COLOR_SEQ: Color32 = Color32::LIGHT_BLUE;

pub const NT_WIDTH_PX: f32 = 8.; // todo: Automatic way? This is valid for monospace font, size 14.
pub const VIEW_AREA_PAD: f32 = 40.;
pub const SEQ_ROW_SPACING_PX: f32 = 40.;

pub const TEXT_X_START: f32 = VIEW_AREA_PAD / 2.;
pub const TEXT_Y_START: f32 = TEXT_X_START;

// const TM_IDEAL: f32 = 59.; // todo: Fill thi sin
//
// const THRESHOLDS_TM: (f32, f32) = (59., 60.);
// const THRESHOLDS_GC: (f32, f32) = (59., 60.);

/// Make a visual arrow for a primer. For  use inside a Frame::canvas.
fn primer_arrow(
    mut bounds_r0: (Pos2, Pos2),
    mut bounds_r1: Option<(Pos2, Pos2)>, // Assumes no more than two rows.
    direction: PrimerDirection,
    label: &str,
    ui: &mut Ui,
) -> Vec<Shape> {
    let color_arrow = match direction {
        PrimerDirection::Forward => Color32::from_rgb(255, 0, 255),
        PrimerDirection::Reverse => Color32::LIGHT_YELLOW,
    };

    let color_label = Color32::LIGHT_GREEN;
    let arrow_width = 2.;

    const VERTICAL_OFFSET: f32 = 20.; // Number of pixels above the sequence text.
    const LABEL_OFFSET: f32 = 7.;
    const HEIGHT: f32 = 16.;
    const SLANT: f32 = 20.; // slant different, in pixels, for the arrow.

    bounds_r0.0.y -= VERTICAL_OFFSET;
    bounds_r0.1.y -= VERTICAL_OFFSET;
    if let Some(b) = bounds_r1.as_mut() {
        b.0.y -= VERTICAL_OFFSET;
        b.1.y -= VERTICAL_OFFSET;
    }

    let mut result = Vec::new();

    let ctx = ui.ctx();

    // todo: Handle rev too.

    // match direction {
        // todo: Consolidate this A/R.
        // PrimerDirection::Forward => {
            // Note: "Left" and "Right" are reversed for reverse primers.

            let mut top_left = bounds_r0.0;

            // Slant only if single-line.
            let mut top_right = if bounds_r1.is_none() {
                pos2(bounds_r0.1.x - SLANT, bounds_r0.1.y)
            } else {
                pos2(bounds_r0.1.x, bounds_r0.1.y)
            };
            let mut bottom_left = pos2(bounds_r0.0.x, bounds_r0.0.y + HEIGHT);
            let mut bottom_right = pos2(bounds_r0.1.x, bounds_r0.1.y + HEIGHT);

            if direction == PrimerDirection::Reverse {
                let temp = top_right;
                top_right = top_left;
                top_left = temp;

                let temp = bottom_left;
                bottom_left = bottom_right;
                bottom_right = temp;

                top_left.x += SLANT;
                bottom_left.x -= SLANT;
            }

            let points = vec![
                top_left,
                bottom_left,
                bottom_right,
                top_right,
            ];

            result.push(Shape::Path(PathShape::closed_line(
                points,
                Stroke::new(arrow_width, color_arrow),
            )));

            if let Some(b) = bounds_r1 {
                let points = vec![
                    b.0,                         // top left,
                    pos2(b.0.x, b.0.y + HEIGHT), // bottom left
                    pos2(b.1.x, b.1.y + HEIGHT), // bottom right,
                    pos2(b.1.x - SLANT, b.1.y),  // top-right (slant)
                ];

                result.push(Shape::Path(PathShape::closed_line(
                    points,
                    Stroke::new(arrow_width, color_arrow),
                )));
            }
        // }
        // PrimerDirection::Reverse => {}
    // };

    let label_start_x = match direction {
        PrimerDirection::Forward => bounds_r0.0.x,
        PrimerDirection::Reverse => bounds_r0.1.x,
    }  + LABEL_OFFSET;

    let label = ctx.fonts(|fonts| {
        Shape::text(
            fonts,
            pos2(label_start_x, bounds_r0.0.y + LABEL_OFFSET),
            Align2::LEFT_CENTER,
            label,
            FontId::new(16., FontFamily::Proportional),
            color_label,
            // ctx.style().visuals.text_color(),
        )
    });

    result.push(label);
    result
}

// todo: Move this graphics drawing code to a new module, A/R.
/// Draw the sequence with primers, insertion points, and other data visible, A/R
fn sequence_vis(state: &State, ui: &mut Ui) {
    let nt_chars_per_row = ((ui.available_width() - VIEW_AREA_PAD) / NT_WIDTH_PX) as usize; // todo: +1 etc?

    // let (id, rect) = ui.allocate_space(desired_size);

    let mut shapes = vec![];

    let (mut response, painter) = {
        // Estimate required height, based on seq len.

        // todo: C+P from below, in amplicon code. This needs to be branched as well, or moved below.

        let row_ranges = get_row_ranges(state.seq_amplicon.len(), nt_chars_per_row);
        // let desired_size = ui.available_width() * vec2(1.0, 0.15);
        let desired_size = vec2(
            ui.available_width(),
            row_ranges.len() as f32 * SEQ_ROW_SPACING_PX + 60.,
        );
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

    match state.ui.page_primer_creation {
        PagePrimerCreation::Amplification => {
            let row_ranges = get_row_ranges(state.seq_amplicon.len(), nt_chars_per_row);

            let mut text_y = TEXT_Y_START;

            // todo: Debug temp
            // for i in 0..200 {
            //     let i2 = i * 1;
            //     println!("I: {}, pos2: {:?}", i2, seq_i_to_pixel(i2, &row_ranges));
            // }

            for range in &row_ranges {
                // This max trigger is likely to occur on the last row.
                let r = range.start..min(range.end, state.seq_amplicon.len());
                let seq_this_row = &state.seq_amplicon[r];

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
                // todo: Sort out the direction. By matches, most likely.

                // todo: Do not run these calcs each time! Cache.
                for (direction, seq_range) in &prim_data.matches_amplification_seq {
                    let (start, end) = match direction {
                        PrimerDirection::Forward => (seq_range.start, seq_range.end),
                        PrimerDirection::Reverse => (state.seq_amplicon.len() - seq_range.start, state.seq_amplicon.len() - seq_range.end),
                    };

                    let start_pos = seq_i_to_pixel_rel(start, &row_ranges);
                    let end_pos = seq_i_to_pixel_rel(end, &row_ranges);

                    // Check if we split across rows.
                    let (bounds_row_0, bounds_row_1) = if start_pos.y == end_pos.y {
                        ((start_pos, end_pos), None)
                    } else {
                        // let (col, row) = seq_i_to_col_row(seq_range.start, &row_ranges);

                        // let row_0_end = seq_i_to_pixel_rel(seq_range.start, &row_ranges);
                        let row_0_end = pos2(
                            TEXT_X_START + NT_WIDTH_PX * (1. + nt_chars_per_row as f32),
                            start_pos.y,
                        );
                        // let row_1_start = seq_i_to_pixel_rel(seq_range.start, &row_ranges);
                        let row_1_start = pos2(TEXT_X_START, end_pos.y); // todo: A/R

                        ((start_pos, row_0_end), Some((row_1_start, end_pos)))
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
        }
        PagePrimerCreation::SlicFc => {}
    }
    // ScrollArea::vertical().id_source(0).show(ui, |ui| {
    Frame::canvas(ui.style())
        .fill(Color32::WHITE) // todo: Not working.
        .show(ui, |ui| {
            ui.painter().extend(shapes);
        });
    // });

    ui.add_space(ROW_SPACING);
}

/// Color scores in each category according to these thresholds. These scores should be on a scale
/// between 0 and 1.
fn color_from_score(score: f32) -> Color32 {
    const SCORE_COLOR_THRESH: (f32, f32) = (0.5, 0.8);

    if score > SCORE_COLOR_THRESH.1 {
        COLOR_GOOD
    } else if score > SCORE_COLOR_THRESH.0 {
        COLOR_MARGINAL
    } else {
        COLOR_BAD
    }
}

/// Shows below each primer sequence. Data and controls on trimming primer size for optimization.
/// Returns wheather a button was clicked.
fn primer_tune_display(data: &mut PrimerData, ui: &mut Ui) -> bool {
    // This avoids a double-mutable error
    let mut tuned = false;

    // Section for tuning primer length.
    ui.horizontal(|ui| {
        // This layout allows even spacing.
        // ui.allocate_ui(egui::Vec2::new(ui.available_width(), 0.0), |ui| {
        //     ui.with_layout(Layout::left_to_right(Align::Center), |ui| {

        if let TuneSetting::Enabled(i) = &mut data.tunable_5p {
            ui.label("5'");
            if ui.button("⏴").clicked() {
                if *i > 0 {
                    *i -= 1;
                }
                tuned = true;
            };
            if ui.button("⏵").clicked() {
                let t3p_len = match data.tunable_3p {
                    TuneSetting::Enabled(t) => t,
                    _ => 0,
                };
                if *i + 1 < data.sequence_input.len() - t3p_len {
                    *i += 1;
                }
                tuned = true;
            };

            ui.label(&format!("({i})"));
        }

        // This section shows the trimmed sequence, with the removed parts visible to the left and right.
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.label(RichText::new(&data.seq_removed_5p).color(Color32::GRAY));
            ui.add_space(COL_SPACING);

            if data.tunable_5p != TuneSetting::Disabled || data.tunable_3p != TuneSetting::Disabled
            {
                ui.label(
                    RichText::new(make_seq_str(&data.primer.sequence)).color(Color32::LIGHT_BLUE),
                );
            }

            ui.add_space(COL_SPACING);
            ui.label(RichText::new(&data.seq_removed_3p).color(Color32::GRAY));
        });

        // Note: We need to reverse the item order for this method of right-justifying to work.
        // This is kind of OK with the intent here though.
        ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
            if let TuneSetting::Enabled(i) = &mut data.tunable_3p {
                ui.label("3'");

                if ui.button("⏵").clicked() {
                    if *i > 0 {
                        *i -= 1;
                    }
                    tuned = true;
                };
                if ui.button("⏴").clicked() {
                    let t5p_len = match data.tunable_5p {
                        TuneSetting::Enabled(t) => t,
                        _ => 0,
                    };

                    // todo: We still have a crash her.e
                    if *i + 1 < data.sequence_input.len() - t5p_len {
                        *i += 1;
                    }
                    tuned = true;
                };
                ui.label(&format!("({i})"));
            }
        });

        if tuned {
            data.run_calcs();
        }
    });
    tuned
}

fn amplification(state: &mut State, ui: &mut Ui) {
    ui.heading("Amplification");

    ui.add_space(ROW_SPACING);

    ui.label("Amplicon:");
    let response =
        ui.add(TextEdit::multiline(&mut state.ui.seq_amplicon_input).desired_width(800.));
    if response.changed() {
        state.seq_amplicon = seq_from_str(&state.ui.seq_amplicon_input);
        state.ui.seq_amplicon_input = make_seq_str(&state.seq_amplicon);
        state.sync_primer_matches(None);
    }
    ui.label(&format!("len: {}", state.ui.seq_amplicon_input.len()));

    ui.add_space(ROW_SPACING);

    ui.horizontal(|ui| {
        if ui.button("➕ Make primers").clicked() {
            // state.sync_seqs();

            if let Some(primers) = design_amplification_primers(&state.seq_amplicon) {
                let sequence_input = make_seq_str(&primers.fwd.sequence);

                let mut primer_fwd = PrimerData {
                    primer: primers.fwd,
                    sequence_input,
                    description: "Amplification Fwd".to_owned(),
                    tunable_5p: TuneSetting::Disabled,
                    tunable_3p: TuneSetting::Enabled(DEFAULT_TRIM_AMT),
                    ..Default::default()
                };

                let sequence_input = make_seq_str(&primers.rev.sequence);
                let mut primer_rev = PrimerData {
                    primer: primers.rev,
                    sequence_input,
                    description: "Amplification Rev".to_owned(),
                    tunable_5p: TuneSetting::Disabled,
                    tunable_3p: TuneSetting::Enabled(DEFAULT_TRIM_AMT),
                    ..Default::default()
                };

                primer_fwd.run_calcs();
                primer_rev.run_calcs();

                state.primer_data.extend([primer_fwd, primer_rev]);
            }
        }
    });
}

fn primer_creation_slic_fc(state: &mut State, ui: &mut Ui) {
    ui.heading("SLIC and FastCloning");

    ui.add_space(ROW_SPACING);

    ui.label("Insert:");
    let response = ui.add(TextEdit::multiline(&mut state.ui.seq_insert_input).desired_width(800.));
    if response.changed() {
        state.seq_insert = seq_from_str(&state.ui.seq_insert_input);
        state.ui.seq_insert_input = make_seq_str(&state.seq_insert);
        state.sync_primer_matches(None);
    }
    ui.label(&format!("len: {}", state.ui.seq_insert_input.len()));

    ui.add_space(ROW_SPACING);

    ui.label("Vector:");
    let response = ui.add(TextEdit::multiline(&mut state.ui.seq_vector_input).desired_width(800.));
    if response.changed() {
        state.seq_vector = seq_from_str(&state.ui.seq_vector_input);
        state.ui.seq_vector_input = make_seq_str(&state.seq_vector);
        state.sync_primer_matches(None);
    }
    ui.label(&format!("len: {}", state.ui.seq_vector_input.len()));

    ui.horizontal(|ui| {
        let mut entry = state.insert_loc.to_string();
        let response = ui.add(TextEdit::singleline(&mut entry).desired_width(40.));
        if response.changed() {
            state.insert_loc = entry.parse().unwrap_or(0);
        }

        ui.add_space(COL_SPACING);

        if ui.button("➕ Make cloning primers").clicked() {
            // state.sync_seqs();

            if let Some(primers) =
                design_slic_fc_primers(&state.seq_vector, &state.seq_insert, state.insert_loc)
            {
                let sequence_input = make_seq_str(&primers.insert_fwd.sequence);

                let mut insert_fwd = PrimerData {
                    primer: primers.insert_fwd,
                    sequence_input,
                    description: "SLIC Insert Fwd".to_owned(),
                    // Both ends are  tunable, since this glues the insert to the vector
                    tunable_5p: TuneSetting::Enabled(DEFAULT_TRIM_AMT),
                    tunable_3p: TuneSetting::Enabled(DEFAULT_TRIM_AMT),
                    ..Default::default()
                };

                let sequence_input = make_seq_str(&primers.insert_rev.sequence);
                let mut insert_rev = PrimerData {
                    primer: primers.insert_rev,
                    sequence_input,
                    description: "SLIC Insert Rev".to_owned(),
                    // Both ends are tunable, since this glues the insert to the vector
                    tunable_5p: TuneSetting::Enabled(DEFAULT_TRIM_AMT),
                    tunable_3p: TuneSetting::Enabled(DEFAULT_TRIM_AMT),
                    ..Default::default()
                };

                let sequence_input = make_seq_str(&primers.vector_fwd.sequence);
                let mut vector_fwd = PrimerData {
                    primer: primers.vector_fwd,
                    sequence_input,
                    description: "SLIC Vector Fwd".to_owned(),
                    // 5' is non-tunable: This is the insert location.
                    tunable_5p: TuneSetting::Disabled,
                    tunable_3p: TuneSetting::Enabled(DEFAULT_TRIM_AMT),
                    ..Default::default()
                };

                let sequence_input = make_seq_str(&primers.vector_rev.sequence);
                let mut vector_rev = PrimerData {
                    primer: primers.vector_rev,
                    sequence_input,
                    description: "SLIC Vector Rev".to_owned(),
                    tunable_5p: TuneSetting::Enabled(DEFAULT_TRIM_AMT),
                    // 3' is non-tunable: This is the insert location.
                    tunable_3p: TuneSetting::Disabled,
                    ..Default::default()
                };
                insert_fwd.run_calcs();
                insert_rev.run_calcs();
                vector_fwd.run_calcs();
                vector_rev.run_calcs();

                state
                    .primer_data
                    .extend([insert_fwd, insert_rev, vector_fwd, vector_rev]);
            }
        }
    });
}

/// EGUI component for the Primer page
pub fn primer_page(state: &mut State, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new("Primer QC").color(Color32::WHITE));
        ui.add_space(COL_SPACING);

        let add_btn = ui
            .button("➕ Add primer")
            .on_hover_text("Adds a primer to the list below. Ctrl + A");
        if add_btn.clicked() {
            state.primer_data.push(Default::default())
        }

        if ui.button("Tune all").clicked() {
            for data in &mut state.primer_data {
                // todo
            }
        }

        ui.add_space(COL_SPACING * 2.);

        if ui
            .button("Save")
            .on_hover_text("Save primer data. Ctrl + S")
            .clicked()
        {
            if let Err(e) = save("plasmid_tools.save", state) {
                println!("Error saving: {e}");
            }
        }

        // if ui.button("Load").clicked() {}

        // // todo: Temp. Find a better way.
        // if ui.button("Sync primer disp").clicked() {
        //     for p_data in &mut state.primer_data {
        //         p_data.matches_amplification_seq = p_data.primer.match_to_seq(&state.seq_amplicon);
        //         p_data.matches_slic_insert = p_data.primer.match_to_seq(&state.seq_insert);
        //         p_data.matches_slic_vector = p_data.primer.match_to_seq(&state.seq_vector);
        //     }
        // }
    });

    ui.label("Tuning instructions: Include more of the target sequence than required on the end[s] that can be tuned. These are the \
     ends that do not define your insert, gene of interest, insertion point etc. Mark that end as tunable using the \"Tune\" button.\
     ");

    ui.add_space(ROW_SPACING);

    if let Some(sel_i) = state.ui.primer_selected {
        ui.horizontal(|ui| {
            if sel_i + 1 > state.primer_data.len() {
                // This currently happens if deleting the final primer.
                eprintln!("Error: Exceeded primer selection len");
                state.ui.primer_selected = None;
                return;
            }

            ui.heading(&format!(
                "Selected: {}",
                &state.primer_data[sel_i].description
            ));

            ui.add_space(COL_SPACING);

            if ui.button(RichText::new("Up")).clicked() {
                // todo: Arrow icons
                if sel_i != 0 {
                    state.primer_data.swap(sel_i, sel_i - 1);
                    state.ui.primer_selected = Some(sel_i - 1);
                }
            }
            if ui.button(RichText::new("Dn")).clicked() && sel_i != state.primer_data.len() - 1 {
                state.primer_data.swap(sel_i, sel_i + 1);
                state.ui.primer_selected = Some(sel_i + 1);
            }

            if ui
                .button(RichText::new("Delete 🗑").color(Color32::RED))
                .clicked()
            {
                state.primer_data.remove(sel_i);
            }

            if ui
                .button(RichText::new("Deselect").color(Color32::GOLD))
                .clicked()
            {
                state.ui.primer_selected = None;
            }
        });

        ui.add_space(ROW_SPACING);
    }

    let mut run_match_sync = None; // Avoids a double-mutation error.

    TableBuilder::new(ui)
        .column(Column::initial(700.).resizable(true))
        .column(Column::initial(160.).resizable(true))
        .column(Column::auto().resizable(true))
        .column(Column::auto().resizable(true))
        .column(Column::initial(40.).resizable(true))
        .column(Column::initial(36.).resizable(true))
        .column(Column::auto().resizable(true))
        .column(Column::auto().resizable(true))
        .column(Column::auto().resizable(true))
        .column(Column::auto().resizable(true))
        .column(Column::remainder())
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.heading("Sequence (5' ⏵ 3')");
            });
            header.col(|ui| {
                ui.heading("Description");
            });
            header.col(|ui| {
                ui.heading("Len").on_hover_text("Number of nucleotides in the (tuned, if applicable) primer");
            });
            header.col(|ui| {
                ui.heading("Qual").on_hover_text("Overall primer quality. This is an abstract estimate, taking all other listed factors into account.");
            });
            header.col(|ui| {
                ui.heading("TM").on_hover_text("Primer melting temperature, in °C. See the readme for calculations and assumptions.");
            });
            header.col(|ui| {
                ui.heading("GC").on_hover_text("The percentage of nucleotides that are C or G.");
            });
            header.col(|ui| {
                ui.heading("3'GC").on_hover_text("3' end stability: The number of G or C nucleotides in the last 5 nucleotides.");
            });
            header.col(|ui| {
                ui.heading("Cplx").on_hover_text("Sequence complexity. See the readme for calculations and assumptions.");
            });
            header.col(|ui| {
                ui.heading("Dmr").on_hover_text("Potential of forming a self-end dimer. See the readme for calculations and assumptions.");
            });
            header.col(|ui| {
                ui.heading("Rep").on_hover_text("Count of repeats of a single or double nt sequence >4 in a row.");
            });

            // For selecting the row.
            header.col(|ui| {});
        })
        .body(|mut body| {
            for (i, data) in state.primer_data.iter_mut().enumerate() {
                body.row(30.0, |mut row| {
                    row.col(|ui| {
                        // gui.label(make_seq_str(&col.sequence));
                        // gui.label(&col.sequence_input);
                        // let mut val = col.sequence_input;

                        ui.horizontal(|ui| {
                            if ui
                                .button(RichText::new("Tun").color(if let TuneSetting::Enabled(_) = data.tunable_5p {
                                    Color32::GREEN
                                } else {
                                    Color32::LIGHT_GRAY
                                }))
                                .clicked()
                            {
                                data.tunable_5p.toggle();
                                if data.tunable_5p == TuneSetting::Disabled {
                                    data.run_calcs(); // To re-sync the sequence without parts removed.
                                }
                            }

                            let response = ui.add(
                                TextEdit::singleline(&mut data.sequence_input).desired_width(400.),
                            );

                            if response.changed() {
                                data.sequence_input =
                                    make_seq_str(&seq_from_str(&data.sequence_input));
                                data.run_calcs();
                                run_match_sync = Some(i);
                            }

                            if ui
                                .button(RichText::new("Tun").color(if let TuneSetting::Enabled(_) = data.tunable_3p {
                                    Color32::GREEN
                                } else {
                                    Color32::LIGHT_GRAY
                                }))
                                .clicked()
                            {
                                data.tunable_3p.toggle();
                                if data.tunable_3p == TuneSetting::Disabled {
                                    data.run_calcs(); // To re-sync the sequence without parts removed.
                                }
                            }
                        });

                        let updated_seq = primer_tune_display(data, ui);
                        if updated_seq {
                            run_match_sync = Some(i);
                        }
                    });

                    row.col(|ui| {
                        ui.add(TextEdit::singleline(&mut data.description));
                    });

                    row.col(|ui| {
                        ui.label(data.primer.sequence.len().to_string());
                    });

                    row.col(|ui| {
                        let text = match &data.metrics {
                            // todo: PRe-compute the * 100?
                            Some(m) => RichText::new(format!("{:.0}", m.quality_score * 100.))
                                .color(color_from_score(m.quality_score)),

                            None => RichText::new("-"),
                        };
                        ui.label(text);
                    });

                    row.col(|ui| {
                        let text = match &data.metrics {
                            Some(m) => RichText::new(format!("{:.1}°C", m.melting_temp))
                                .color(color_from_score(m.tm_score)),

                            None => RichText::new("-"),
                        };
                        ui.label(text);
                    });

                    row.col(|ui| {
                        let text = match &data.metrics {
                            // todo: Cache this calc?
                            Some(m) => RichText::new(format!("{:.0}%", m.gc_portion * 100.))
                                .color(color_from_score(m.gc_score)),
                            None => RichText::new("-"),
                        };
                        ui.label(text);
                    });

                    row.col(|ui| {
                        let text = match &data.metrics {
                            Some(m) => RichText::new(format!("{}", m.gc_3p_count))
                                .color(color_from_score(m.gc_3p_score)),
                            None => RichText::new("-"),
                        };
                        ui.label(text);
                    });

                    row.col(|ui| {
                        let text = match &data.metrics {
                            Some(m) => RichText::new(format!("{}", m.complexity))
                                .color(color_from_score(m.complexity_score)),
                            None => RichText::new("-"),
                        };
                        ui.label(text);
                    });

                    row.col(|ui| {
                        let text = match &data.metrics {
                            Some(m) => RichText::new(format!("{}", m.self_end_dimer))
                                .color(color_from_score(m.dimer_score)),
                            None => RichText::new("-"),
                        };
                        ui.label(text);
                    });

                    row.col(|ui| {
                        let text = match &data.metrics {
                            Some(m) => RichText::new(format!("{}", m.repeats))
                                .color(color_from_score(m.repeats_score)),
                            None => RichText::new("-"),
                        };
                        ui.label(text);
                    });

                    row.col(|ui| {
                        let mut selected = false;

                        if let Some(sel_i) = state.ui.primer_selected {
                            if sel_i == i {
                                selected = true
                            }
                        }

                        if selected {
                            if ui.button(RichText::new("🔘").color(Color32::GREEN)).clicked() {
                                state.ui.primer_selected = None;
                            }
                        } else if ui.button("🔘").clicked() {
                            state.ui.primer_selected = Some(i);
                        }
                    });
                });
            }
        });

    if run_match_sync.is_some() {
        state.sync_primer_matches(run_match_sync);
    }

    ui.add_space(ROW_SPACING * 3.);

    // todo: Only if you have  sequence of some sort
    sequence_vis(&state, ui);

    page_primers_selector(state, ui);

    match state.ui.page_primer_creation {
        PagePrimerCreation::Amplification => {
            amplification(state, ui);
        }
        PagePrimerCreation::SlicFc => {
            primer_creation_slic_fc(state, ui);
        }
    }

    // todo: Visualizer here with the seq, the primers etc
}
