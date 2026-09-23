//! Streaming conversion of Lichess puzzle CSV exports to PGN.
//!
//! The converter validates the standard Lichess CSV header, filters on an
//! inclusive minimum rating, converts UCI moves to SAN, and only replaces the
//! destination after the entire conversion has completed successfully.

use std::{
    error::Error,
    fmt::{self, Write as _},
    fs,
    io::{self, BufWriter, Write},
    num::NonZeroU32,
    path::Path,
};

use csv::{ReaderBuilder, StringRecord};
use shakmaty::{CastlingMode, Chess, Color, Position, fen::Fen, san::SanPlus, uci::UciMove};
use tempfile::NamedTempFile;

const REQUIRED_COLUMNS: [&str; 9] = [
    "PuzzleId",
    "FEN",
    "Moves",
    "Rating",
    "RatingDeviation",
    "Popularity",
    "NbPlays",
    "Themes",
    "GameUrl",
];

/// A recoverable failure while reading, validating, or writing a conversion.
#[derive(Debug)]
pub struct ConversionError {
    message: String,
}

impl ConversionError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn row(row_number: u64, message: impl fmt::Display) -> Self {
        Self::new(format!("CSV row {row_number}: {message}"))
    }
}

impl fmt::Display for ConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ConversionError {}

impl From<io::Error> for ConversionError {
    fn from(error: io::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<csv::Error> for ConversionError {
    fn from(error: csv::Error) -> Self {
        Self::new(error.to_string())
    }
}

/// Aggregate information about a completed conversion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConversionStats {
    /// Data rows examined, excluding the CSV header.
    pub records_read: u64,
    /// Puzzles written to the PGN.
    pub puzzles_written: u64,
    /// Rows omitted because their rating was below the configured minimum.
    pub skipped_by_rating: u64,
}

/// A snapshot sent periodically while a conversion is running.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConversionProgress {
    pub records_read: u64,
    pub puzzles_written: u64,
}

#[derive(Debug, Clone, Copy)]
struct ColumnIndexes {
    puzzle_id: usize,
    fen: usize,
    moves: usize,
    rating: usize,
    rating_deviation: usize,
    popularity: usize,
    nb_plays: usize,
    themes: usize,
    game_url: usize,
    opening_tags: Option<usize>,
}

impl ColumnIndexes {
    fn from_headers(headers: &StringRecord) -> Result<Self, ConversionError> {
        let column = |name: &str| {
            headers
                .iter()
                .position(|header| header.trim_start_matches('\u{feff}') == name)
                .ok_or_else(|| {
                    ConversionError::new(format!(
                        "The CSV is missing the required `{name}` column. Download the Lichess puzzle CSV export and unpack it before converting."
                    ))
                })
        };

        Ok(Self {
            puzzle_id: column(REQUIRED_COLUMNS[0])?,
            fen: column(REQUIRED_COLUMNS[1])?,
            moves: column(REQUIRED_COLUMNS[2])?,
            rating: column(REQUIRED_COLUMNS[3])?,
            rating_deviation: column(REQUIRED_COLUMNS[4])?,
            popularity: column(REQUIRED_COLUMNS[5])?,
            nb_plays: column(REQUIRED_COLUMNS[6])?,
            themes: column(REQUIRED_COLUMNS[7])?,
            game_url: column(REQUIRED_COLUMNS[8])?,
            opening_tags: headers
                .iter()
                .position(|header| header.trim_start_matches('\u{feff}') == "OpeningTags"),
        })
    }
}

#[derive(Debug)]
struct Puzzle<'a> {
    id: &'a str,
    fen: &'a str,
    moves: &'a str,
    rating: &'a str,
    rating_deviation: &'a str,
    popularity: &'a str,
    nb_plays: &'a str,
    themes: &'a str,
    game_url: &'a str,
    opening_tags: &'a str,
}

impl<'a> Puzzle<'a> {
    fn from_record(
        record: &'a StringRecord,
        columns: ColumnIndexes,
        row_number: u64,
    ) -> Result<Self, ConversionError> {
        let field = |index: usize, name: &str| {
            record.get(index).ok_or_else(|| {
                ConversionError::row(
                    row_number,
                    format!("missing `{name}`. The row has fewer fields than its header."),
                )
            })
        };

        Ok(Self {
            id: field(columns.puzzle_id, "PuzzleId")?,
            fen: field(columns.fen, "FEN")?,
            moves: field(columns.moves, "Moves")?,
            rating: field(columns.rating, "Rating")?,
            rating_deviation: field(columns.rating_deviation, "RatingDeviation")?,
            popularity: field(columns.popularity, "Popularity")?,
            nb_plays: field(columns.nb_plays, "NbPlays")?,
            themes: field(columns.themes, "Themes")?,
            game_url: field(columns.game_url, "GameUrl")?,
            opening_tags: columns
                .opening_tags
                .and_then(|index| record.get(index))
                .unwrap_or_default(),
        })
    }
}

/// Convert an uncompressed Lichess puzzle CSV into PGN without progress callbacks.
pub fn convert_csv_to_pgn(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    minimum_rating: u32,
) -> Result<ConversionStats, ConversionError> {
    convert_csv_to_pgn_with_progress(input, output, minimum_rating, |_| {})
}

/// Convert an uncompressed Lichess puzzle CSV into PGN.
///
/// `minimum_rating` is inclusive: a puzzle rated exactly that value is
/// exported. The `on_progress` callback is invoked every 10,000 rows and at
/// the end of a successful conversion.
pub fn convert_csv_to_pgn_with_progress<F>(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    minimum_rating: u32,
    mut on_progress: F,
) -> Result<ConversionStats, ConversionError>
where
    F: FnMut(ConversionProgress),
{
    let input = input.as_ref();
    let output = output.as_ref();
    ensure_distinct_paths(input, output)?;

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_path(input)?;
    let columns = ColumnIndexes::from_headers(reader.headers()?)?;

    let output_directory = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !output_directory.is_dir() {
        return Err(ConversionError::new(format!(
            "Output folder does not exist: {}",
            output_directory.display()
        )));
    }

    let temporary_file = NamedTempFile::new_in(output_directory)?;
    let mut writer = BufWriter::new(temporary_file);
    let mut stats = ConversionStats::default();

    for record in reader.records() {
        let record = record?;
        stats.records_read += 1;
        let row_number = stats.records_read + 1;
        let puzzle = Puzzle::from_record(&record, columns, row_number)?;
        let rating = puzzle.rating.parse::<u32>().map_err(|_| {
            ConversionError::row(
                row_number,
                format!(
                    "`Rating` must be a non-negative integer, found {:?}",
                    puzzle.rating
                ),
            )
        })?;

        if rating < minimum_rating {
            stats.skipped_by_rating += 1;
        } else {
            write_puzzle(&mut writer, &puzzle, row_number)?;
            stats.puzzles_written += 1;
        }

        if stats.records_read.is_multiple_of(10_000) {
            on_progress(ConversionProgress {
                records_read: stats.records_read,
                puzzles_written: stats.puzzles_written,
            });
        }
    }

    writer.flush()?;
    let temporary_file = writer.into_inner().map_err(|error| error.into_error())?;
    temporary_file
        .persist(output)
        .map_err(|error| error.error)?;

    on_progress(ConversionProgress {
        records_read: stats.records_read,
        puzzles_written: stats.puzzles_written,
    });
    Ok(stats)
}

fn ensure_distinct_paths(input: &Path, output: &Path) -> Result<(), ConversionError> {
    if input == output {
        return Err(ConversionError::new(
            "Input and output paths must be different.",
        ));
    }

    if let (Ok(input), Ok(output)) = (fs::canonicalize(input), fs::canonicalize(output))
        && input == output
    {
        return Err(ConversionError::new(
            "Input and output paths must be different.",
        ));
    }
    Ok(())
}

fn write_puzzle(
    writer: &mut impl Write,
    puzzle: &Puzzle<'_>,
    row_number: u64,
) -> Result<(), ConversionError> {
    for (name, value) in [
        ("Event", "Lichess Puzzle"),
        ("Site", puzzle.game_url),
        ("Date", "????.??.??"),
        ("Round", "-"),
        ("White", "?"),
        ("Black", "?"),
        ("Result", "*"),
        ("SetUp", "1"),
        ("FEN", puzzle.fen),
        ("PuzzleId", puzzle.id),
        ("Rating", puzzle.rating),
        ("RatingDeviation", puzzle.rating_deviation),
        ("Popularity", puzzle.popularity),
        ("NbPlays", puzzle.nb_plays),
        ("Themes", puzzle.themes),
        ("OpeningTags", puzzle.opening_tags),
    ] {
        writeln!(writer, "[{name} \"{}\"]", escape_tag_value(value))?;
    }
    writeln!(writer)?;
    writeln!(
        writer,
        "{} *",
        moves_to_san(puzzle.fen, puzzle.moves, row_number)?
    )?;
    writeln!(writer)?;
    Ok(())
}

fn moves_to_san(fen_text: &str, moves: &str, row_number: u64) -> Result<String, ConversionError> {
    let fen = fen_text
        .parse::<Fen>()
        .map_err(|error| ConversionError::row(row_number, format!("invalid FEN: {error}")))?;
    let mut position = fen
        .into_position::<Chess>(CastlingMode::Standard)
        .map_err(|error| {
            ConversionError::row(
                row_number,
                format!("invalid chess position in FEN: {error}"),
            )
        })?;

    let mut formatted_moves = String::new();
    let mut found_move = false;
    for token in moves.split_whitespace() {
        found_move = true;
        let move_number: NonZeroU32 = position.fullmoves();
        let turn = position.turn();
        let uci_move = token.parse::<UciMove>().map_err(|error| {
            ConversionError::row(row_number, format!("invalid UCI move `{token}`: {error}"))
        })?;
        let chess_move = uci_move.to_move(&position).map_err(|error| {
            ConversionError::row(
                row_number,
                format!("illegal UCI move `{token}` for its FEN position: {error}"),
            )
        })?;
        let san = SanPlus::from_move_and_play_unchecked(&mut position, chess_move);

        if !formatted_moves.is_empty() {
            formatted_moves.push(' ');
        }
        match turn {
            Color::White => {
                let _ = write!(formatted_moves, "{}. {san}", move_number.get());
            }
            Color::Black if formatted_moves.is_empty() => {
                let _ = write!(formatted_moves, "{}... {san}", move_number.get());
            }
            Color::Black => formatted_moves.push_str(&san.to_string()),
        }
    }

    if found_move {
        Ok(formatted_moves)
    } else {
        Err(ConversionError::row(
            row_number,
            "`Moves` must contain at least one UCI move.",
        ))
    }
}

fn escape_tag_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\r', '\n'], " ")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use tempfile::tempdir;

    const HEADER: &str =
        "PuzzleId,FEN,Moves,Rating,RatingDeviation,Popularity,NbPlays,Themes,GameUrl,OpeningTags\n";
    const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[test]
    fn exports_only_ratings_at_or_above_the_minimum_as_valid_san_pgn() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("puzzles.csv");
        let output = directory.path().join("puzzles.pgn");
        let source = format!(
            "{HEADER}low,{STARTING_FEN},e2e4 e7e5,1499,70,42,10,healthy,https://example.test/low,\nselected,{STARTING_FEN},e2e4 e7e5,1500,71,43,11,\"healthy opening\",https://example.test/selected,\"French, Defence\"\n"
        );
        fs::write(&input, source).unwrap();

        let stats = convert_csv_to_pgn(&input, &output, 1500).unwrap();
        let pgn = fs::read_to_string(output).unwrap();

        assert_eq!(
            stats,
            ConversionStats {
                records_read: 2,
                puzzles_written: 1,
                skipped_by_rating: 1,
            }
        );
        assert!(pgn.contains("[PuzzleId \"selected\"]"));
        assert!(pgn.contains("[Themes \"healthy opening\"]"));
        assert!(pgn.contains("[OpeningTags \"French, Defence\"]"));
        assert!(pgn.contains("1. e4 e5 *"));
        assert!(!pgn.contains("low"));
    }

    #[test]
    fn keeps_existing_output_when_a_selected_row_is_invalid() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("puzzles.csv");
        let output = directory.path().join("puzzles.pgn");
        fs::write(
            &input,
            format!(
                "{HEADER}bad,{STARTING_FEN},e2e5,2000,70,42,10,healthy,https://example.test/bad,\n"
            ),
        )
        .unwrap();
        fs::write(&output, "existing PGN").unwrap();

        let error = convert_csv_to_pgn(&input, &output, 0).unwrap_err();

        assert!(error.to_string().contains("illegal UCI move `e2e5`"));
        assert_eq!(fs::read_to_string(output).unwrap(), "existing PGN");
    }

    #[test]
    fn rejects_a_csv_without_the_lichess_header() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("not-puzzles.csv");
        let output = directory.path().join("puzzles.pgn");
        fs::write(&input, "name,rating\nexample,1500\n").unwrap();

        let error = convert_csv_to_pgn(&input, &output, 0).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("missing the required `PuzzleId`")
        );
        assert!(!output.exists());
    }
}
