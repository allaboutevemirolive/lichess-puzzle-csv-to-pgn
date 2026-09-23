use std::{env, path::PathBuf, process::ExitCode};

use lichess_puzzle_csv_to_pgn::convert_csv_to_pgn_with_progress;

fn usage() -> &'static str {
    "Usage:\n  lichess-puzzle-csv-to-pgn-cli --input PUZZLES.csv --output FILTERED.pgn --min-rating 1500\n\nThe minimum rating is inclusive. The input must be an unpacked Lichess puzzle CSV."
}

fn next_value<I>(arguments: &mut I, option: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    arguments
        .next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("Error: {message}\n\n{}", usage());
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut minimum_rating = None;
    let mut arguments = env::args().skip(1);

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "-h" | "--help" => {
                println!("{}", usage());
                return Ok(());
            }
            "-i" | "--input" => {
                input = Some(PathBuf::from(next_value(&mut arguments, "--input")?));
            }
            "-o" | "--output" => {
                output = Some(PathBuf::from(next_value(&mut arguments, "--output")?));
            }
            "-m" | "--min-rating" => {
                let raw = next_value(&mut arguments, "--min-rating")?;
                minimum_rating = Some(
                    raw.parse::<u32>()
                        .map_err(|_| "--min-rating must be a non-negative whole number")?,
                );
            }
            _ => return Err(format!("unrecognized argument `{argument}`")),
        }
    }

    let input = input.ok_or("--input is required")?;
    let output = output.ok_or("--output is required")?;
    let minimum_rating = minimum_rating.ok_or("--min-rating is required")?;
    let stats = convert_csv_to_pgn_with_progress(&input, &output, minimum_rating, |progress| {
        if progress.records_read > 0 {
            eprintln!(
                "Read {} rows; wrote {} puzzles…",
                progress.records_read, progress.puzzles_written
            );
        }
    })
    .map_err(|error| error.to_string())?;

    println!(
        "Finished: wrote {} puzzles from {} rows ({} below the minimum rating) to {}.",
        stats.puzzles_written,
        stats.records_read,
        stats.skipped_by_rating,
        output.display()
    );
    Ok(())
}
