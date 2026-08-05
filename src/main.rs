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
        #[arg(long)]
        cleanup_days: Option<u64>,
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
    Cleanup {
        #[arg(short, long, default_value_t = 30)]
        older_than: u64,
    },
    Schedule {
        #[arg(short, long, default_value = "SAT")]
        day: String,
        #[arg(short, long, default_value = "09:00")]
        start: String,
        #[arg(short, long, default_value = "17:00")]
        end: String,
        #[arg(short, long, default_value_t = 30)]
        interval: u64,
        #[arg(short, long, default_value = "Recall")]
        name: String,
    },
    Unschedule {
        #[arg(short, long, default_value = "Recall")]
        name: String,
    },
    ScheduleList,
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
        Commands::Start { interval, cleanup_days } => {
            println!("Starte Recall mit Intervall {}s ...", interval);
            if let Some(days) = cleanup_days {
                println!("Automatisches Aufraeumen: screenshots aelter als {} Tage.", days);
            }
            println!("Druecke Strg+C zum Beenden.");
            scheduler::run(db, &db_path, interval, cleanup_days).await?;
        }
        Commands::Stop => {
            println!("Recall-Dienst wird gestoppt.");
            let pid_path = db_path.with_extension("pid");
            if let Ok(content) = std::fs::read_to_string(&pid_path) {
                if let Ok(pid) = content.trim().parse::<u32>() {
                    println!("Beende Prozess mit PID {}", pid);
                    let status = std::process::Command::new("taskkill")
                        .args(["/F", "/PID", &pid.to_string()])
                        .status();
                    match status {
                        Ok(s) if s.success() => println!("Prozess PID {} beendet.", pid),
                        Ok(_) => println!("Warnung: taskkill fehlgeschlagen (Prozess evtl. nicht mehr aktiv)."),
                        Err(e) => println!("Warnung: taskkill nicht ausfuehrbar: {}", e),
                    }
                } else {
                    println!("Warnung: Ungueltige PID in {:?}.", pid_path);
                }
            } else {
                println!("Warnung: Keine PID-Datei unter {:?} gefunden.", pid_path);
            }
            let _ = std::fs::remove_file(&pid_path);
        }
        Commands::Snapshot => {
            let data = capture::capture_screen()?;
            let ocr_text = ocr::recognize(&data)?;
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
                            let path = format!("recall_{:04}.png", s.id);
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
        Commands::Cleanup { older_than } => {
            let deleted = db.cleanup(older_than)?;
            println!("{} Screenshots (aelter als {} Tage) geloescht.", deleted, older_than);
            let stats = db.stats()?;
            println!("Verbleibend: {} Screenshots ({:.2} MB)", stats.total_count, stats.total_size_bytes as f64 / 1_048_576.0);
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
        Commands::Schedule { day, start, end, interval, name } => {
            scheduler::schedule_task(&name, &day, &start, &end, interval)?;
        }
        Commands::Unschedule { name } => {
            scheduler::unschedule_task(&name)?;
        }
        Commands::ScheduleList => {
            scheduler::list_tasks()?;
        }
    }

    Ok(())
}
