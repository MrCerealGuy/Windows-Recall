mod capture;
mod ocr;
mod scheduler;
mod search;
mod storage;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "recall", version, about = "Windows Recall - Screenshot-Archiv")]
struct Cli {
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start {
        #[arg(short, long, default_value_t = 60)]
        interval: u64,
    },
    Stop,
    Snapshot,
    Show {
        id: i64,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    List {
        #[arg(short, long)]
        date: Option<String>,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    Search {
        query: String,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    Export {
        #[arg(short, long)]
        from: Option<String>,
        #[arg(short, long)]
        to: Option<String>,
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },
    Stats,
}

fn default_db_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".recall").join("recall.db")
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(default_db_path);

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = storage::Database::open(&db_path)?;

    match cli.commands {
        Commands::Start { interval } => {
            println!("Starte Recall mit Intervall {}s ...", interval);
            println!("Druecke Strg+C zum Beenden.");
            scheduler::run(db, interval).await?;
        }
        Commands::Stop => {
            println!("Recall-Dienst wird gestoppt.");
            std::fs::remove_file(db_path.with_extension("pid"))?;
        }
        Commands::Snapshot => {
            let data = capture::capture_screen()?;
            let ocr_text = ocr::recognize(&data);
            let id = db.save_screenshot(&data, &ocr_text)?;
            println!("Screenshot #{} gespeichert.", id);
        }
        Commands::Show { id, output } => {
            match db.get_screenshot(id)? {
                Some(s) => {
                    println!("  ID: #{}", s.id);
                    println!("  Zeitstempel: {}", s.timestamp);
                    println!("  Aufloesung: {}x{}", s.width, s.height);
                    if let Some(ref text) = s.ocr_text {
                        if !text.is_empty() {
                            println!("  OCR-Text: {}", text);
                        }
                    }
                    if let Some(ref png) = s.png_bytes {
                        if let Some(out) = output {
                            std::fs::write(&out, png)?;
                            println!("  Gespeichert nach: {:?}", out);
                        } else {
                            let path = format!("recall_{}.png", s.id);
                            std::fs::write(&path, png)?;
                            println!("  Gespeichert nach: {}", path);
                        }
                    }
                }
                None => eprintln!("Screenshot #{} nicht gefunden.", id),
            }
        }
        Commands::List { date, limit } => {
            let screenshots = db.list_screenshots(date.as_deref(), limit)?;
            for s in &screenshots {
                println!(
                    "  #{} | {} | {}x{} | {}",
                    s.id,
                    s.timestamp,
                    s.width,
                    s.height,
                    s.ocr_text.as_deref().unwrap_or("").chars().take(60).collect::<String>()
                );
            }
            println!("{} Screenshots angezeigt.", screenshots.len());
        }
        Commands::Search { query, limit } => {
            let results = db.search(&query, limit)?;
            for s in &results {
                println!(
                    "  #{} | {} | {}",
                    s.id,
                    s.timestamp,
                    s.ocr_text.as_deref().unwrap_or("").chars().take(80).collect::<String>()
                );
            }
            println!("{} Treffer fuer \"{}\".", results.len(), query);
        }
        Commands::Export { from, to, output } => {
            let screenshots = db.all_screenshots_for_export()?;
            let count = search::export_screenshots(&screenshots, &output, from.as_deref(), to.as_deref())?;
            println!("{} Screenshots nach {:?} exportiert.", count, output);
        }
        Commands::Stats => {
            let stats = db.stats()?;
            println!("Screenshots: {}", stats.total_count);
            println!("Gesamtgroesse: {:.2} MB", stats.total_size_bytes as f64 / 1_048_576.0);
            if let Some(first) = &stats.first_screenshot {
                println!("Erster Screenshot: {}", first);
            }
            if let Some(last) = &stats.last_screenshot {
                println!("Letzter Screenshot: {}", last);
            }
        }
    }

    Ok(())
}
