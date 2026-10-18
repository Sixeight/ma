use std::io::Read;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "ma",
    about = "Render Mermaid diagrams as ASCII art (sequence, flowchart, ER, state, class)"
)]
struct Cli {
    /// Input file (reads from stdin if not provided)
    file: Option<std::path::PathBuf>,

    /// Maximum output width in columns
    #[arg(long, short = 'w')]
    width: Option<usize>,
}

fn main() {
    let cli = Cli::parse();

    let input = match cli.file {
        Some(path) => std::fs::File::open(&path)
            .map_err(|e| e.to_string())
            .and_then(read_input)
            .unwrap_or_else(|e| {
                eprintln!("ERROR: failed to read {}: {e}", path.display());
                std::process::exit(1);
            }),
        None => read_input(std::io::stdin().lock()).unwrap_or_else(|e| {
            eprintln!("ERROR: failed to read stdin: {e}");
            std::process::exit(1);
        }),
    };

    match ma::render_with_options(&input, cli.width) {
        Ok(output) => print!("{output}"),
        Err(e) => {
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        }
    }
}

fn read_input(reader: impl Read) -> Result<String, String> {
    let mut bytes = Vec::new();
    reader
        .take((ma::MAX_SOURCE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > ma::MAX_SOURCE_BYTES {
        return Err(format!(
            "source exceeds {}-byte limit",
            ma::MAX_SOURCE_BYTES
        ));
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
