use std::io::Read;

use clap::Parser;

#[derive(Parser)]
#[command(name = "ma", about = "Render Mermaid sequence diagrams as ASCII art")]
struct Cli {
    /// Input file (reads from stdin if not provided)
    file: Option<std::path::PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let input = match cli.file {
        Some(path) => std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("ERROR: failed to read {}: {e}", path.display());
            std::process::exit(1);
        }),
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
                eprintln!("ERROR: failed to read stdin: {e}");
                std::process::exit(1);
            });
            buf
        }
    };

    match ma::render(&input) {
        Ok(output) => print!("{output}"),
        Err(e) => {
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        }
    }
}
