use std::{
    fs::File,
    io,
    io::{ErrorKind, Read, Write},
    ops::Range,
};

use bincode::{self, config, Decode, Encode};
use eframe::egui::{pos2, Pos2};

use crate::{
    gui::primer::{NT_WIDTH_PX, SEQ_ROW_SPACING_PX, TEXT_X_START, TEXT_Y_START},
    Nucleotide,
    Nucleotide::{A, C, G, T},
    Seq,
};

/// Utility function to linearly map an input value to an output
pub fn map_linear(val: f32, range_in: (f32, f32), range_out: (f32, f32)) -> f32 {
    // todo: You may be able to optimize calls to this by having the ranges pre-store
    // todo the total range vals.
    let portion = (val - range_in.0) / (range_in.1 - range_in.0);

    portion * (range_out.1 - range_out.0) + range_out.0
}

pub fn make_seq_str(seq: &[Nucleotide]) -> String {
    let mut result = String::new();

    for nt in seq {
        result.push_str(nt.as_str());
    }

    result
}

pub fn seq_from_str(str: &str) -> Seq {
    let mut result = Vec::new();

    for char in str.to_lowercase().chars() {
        match char {
            'a' => result.push(A),
            't' => result.push(T),
            'c' => result.push(C),
            'g' => result.push(G),
            _ => (),
        };
    }

    result
}

/// Reverse direction, and swap C for G, A for T.
pub fn seq_complement(seq: &[Nucleotide]) -> Seq {
    let mut result = seq.to_vec();
    result.reverse();

    for nt in &mut result {
        *nt = match *nt {
            A => T,
            T => A,
            C => G,
            G => C,
        };
    }

    result
}

pub fn save<T: Encode>(filename: &str, data: &T) -> io::Result<()> {
    let config = config::standard();

    let encoded: Vec<u8> = bincode::encode_to_vec(data, config).unwrap();
    let mut file = File::create(filename)?;
    file.write_all(&encoded)?;
    Ok(())
}

pub fn load<T: Decode>(filename: &str) -> io::Result<T> {
    let config = config::standard();

    let mut file = File::open(filename)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let (decoded, len) = match bincode::decode_from_slice(&buffer, config) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Error loading from file. Did the format change?");
            return Err(io::Error::new(ErrorKind::Other, "error loading"));
        }
    };
    Ok(decoded)
}

/// We use this for dividing a nucleotied sequence into rows, for display in a canvas UI.
pub fn get_row_ranges(len: usize, chars_per_row: usize) -> Vec<Range<usize>> {
    let mut result = Vec::new();

    // todo: Round etc instead of adding 1?
    let num_rows = len / chars_per_row + 1; // todo: +/-1 etc?

    for row_i in 0..num_rows {
        result.push(row_i * chars_per_row..row_i * chars_per_row + chars_per_row);
    }

    result
}

// todo: We currently don't use this as a standalone fn; wrap back into `seq_i_to_pixel` a/r.
/// Maps sequence index, as displayed on a manually-wrapped UI display, to row and column indices.
fn seq_i_to_col_row(seq_i: usize, row_ranges: &[Range<usize>]) -> (usize, usize) {
    let mut row = 0;
    let mut row_range = 0..10;

    for (row_, range) in row_ranges.iter().enumerate() {
        if range.contains(&seq_i) {
            row = row_;
            row_range = range.clone();
            break;
        }
    }

    let col = seq_i - row_range.start;

    (col, row)
}

/// Maps sequence index, as displayed on a manually-wrapped UI display, to the relative pixel.
pub fn seq_i_to_pixel(seq_i: usize, row_ranges: &[Range<usize>]) -> Pos2 {
    let (col, row) = seq_i_to_col_row(seq_i, row_ranges);

    pos2(
        TEXT_X_START + col as f32 * NT_WIDTH_PX,
        TEXT_Y_START + row as f32 * SEQ_ROW_SPACING_PX,
    )
}
