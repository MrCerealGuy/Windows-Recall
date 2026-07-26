use crate::storage::Screenshot;
use anyhow::{Context, Result};
use std::path::Path;

pub fn export_screenshots(
    screenshots: &[Screenshot],
    output_dir: &Path,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<usize> {
    std::fs::create_dir_all(output_dir)?;

    let mut count = 0;

    for s in screenshots {
        if let Some(ref png) = s.png_bytes {
            let in_range = match (from, to) {
                (Some(f), Some(t)) => s.timestamp.as_str() >= f && s.timestamp.as_str() <= t,
                (Some(f), None) => s.timestamp.as_str() >= f,
                (None, Some(t)) => s.timestamp.as_str() <= t,
                (None, None) => true,
            };

            if !in_range {
                continue;
            }

            let safe_ts = s.timestamp.replace([':', ' ', '+'], "_");
            let filename = format!("recall_{}_{}.png", s.id, safe_ts);
            let filepath = output_dir.join(&filename);
            std::fs::write(&filepath, png)
                .with_context(|| format!("Fehler beim Schreiben von {:?}", filepath))?;

            count += 1;
        }
    }

    Ok(count)
}
