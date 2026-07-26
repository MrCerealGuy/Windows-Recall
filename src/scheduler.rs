use crate::{capture, ocr, storage::Database};
use anyhow::Result;
use std::time::Duration;
use tokio::signal;

pub async fn run(db: Database, interval_secs: u64) -> Result<()> {
    let pid = std::process::id();
    let pid_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".recall")
        .join("recall.pid");
    std::fs::write(&pid_path, pid.to_string())?;

    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match capture_screen_once(&db) {
                    Ok(id) => println!("[{}] Screenshot #{} gespeichert.", chrono::Local::now().format("%H:%M:%S"), id),
                    Err(e) => eprintln!("[{}] Fehler: {}", chrono::Local::now().format("%H:%M:%S"), e),
                }
            }
            _ = signal::ctrl_c() => {
                println!("\nRecall wird beendet.");
                let _ = std::fs::remove_file(&pid_path);
                break;
            }
        }
    }

    Ok(())
}

fn capture_screen_once(db: &Database) -> Result<i64> {
    let data = capture::capture_screen()?;
    let ocr_text = ocr::recognize(&data);
    let id = db.save_screenshot(&data, &ocr_text)?;
    Ok(id)
}
