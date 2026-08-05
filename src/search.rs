use crate::storage::Screenshot;
use anyhow::{Context, Result};
use std::path::Path;

struct ExportedEntry {
    id: i64,
    timestamp: String,
    width: u32,
    height: u32,
    ocr_text: String,
    filename: String,
}

pub fn export_screenshots(
    screenshots: &[Screenshot],
    output_dir: &Path,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<usize> {
    std::fs::create_dir_all(output_dir)?;

    let max_id = screenshots
        .iter()
        .filter(|s| s.png_bytes.is_some())
        .map(|s| s.id)
        .max()
        .unwrap_or(0);
    let pad_width = max_id.to_string().len().max(4);

    let mut entries: Vec<ExportedEntry> = Vec::new();

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
            let filename = format!("recall_{:0width$}_{}.png", s.id, safe_ts, width = pad_width);
            let filepath = output_dir.join(&filename);
            std::fs::write(&filepath, png)
                .with_context(|| format!("Fehler beim Schreiben von {:?}", filepath))?;

            entries.push(ExportedEntry {
                id: s.id,
                timestamp: s.timestamp.clone(),
                width: s.width,
                height: s.height,
                ocr_text: s.ocr_text.clone().unwrap_or_default(),
                filename,
            });
        }
    }

    entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    if !entries.is_empty() {
        let html = build_timeline_html(&entries);
        let index_path = output_dir.join("index.html");
        std::fs::write(&index_path, html)
            .with_context(|| format!("Fehler beim Schreiben von {:?}", index_path))?;
    }

    Ok(entries.len())
}

fn build_timeline_html(entries: &[ExportedEntry]) -> String {
    let count = entries.len();
    let mut items = String::new();

    for e in entries {
        let ts_display = e.timestamp.replace('T', " ").replace('+', " +");
        let ocr_display = if e.ocr_text.trim().is_empty() {
            "<em>Kein OCR-Text erkannt.</em>".to_string()
        } else {
            format!(
                "<pre class=\"ocr-text\">{}</pre><button class=\"ocr-more\" type=\"button\">Mehr ...</button>",
                escape_html(&e.ocr_text.trim().to_string())
            )
        };

        items.push_str(&format!(
            r#"<div class="item" data-search="{ocr}">
                <div class="marker"></div>
                <div class="card">
                    <div class="meta">
                        <span class="id">Screenshot #{id}</span>
                        <span class="ts">{ts}</span>
                        <span class="dim">{w}x{h}</span>
                    </div>
                    <a class="thumb" href="{file}" target="_blank" title="In neuem Tab oeffnen">
                        <img src="{file}" alt="Screenshot #{id}" loading="lazy">
                    </a>
                    <div class="ocr">{ocr_display}</div>
                </div>
            </div>"#,
            id = e.id,
            ts = ts_display,
            w = e.width,
            h = e.height,
            file = escape_html(&e.filename),
            ocr = escape_html(&e.ocr_text),
            ocr_display = ocr_display,
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Recall Timeline ({count} Screenshots)</title>
<style>
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{
    font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
    background: #0f1115;
    color: #e6e8eb;
    padding: 24px 16px 64px;
  }}
  header {{
    max-width: 1000px;
    margin: 0 auto 32px;
    text-align: center;
  }}
  header h1 {{ font-size: 1.6rem; font-weight: 600; margin-bottom: 8px; }}
  header p {{ color: #8b919c; font-size: 0.9rem; }}
  #search {{
    display: block;
    width: 100%;
    max-width: 480px;
    margin: 16px auto 0;
    padding: 10px 14px;
    font-size: 1rem;
    color: #e6e8eb;
    background: #171a21;
    border: 1px solid #2a2f3a;
    border-radius: 8px;
  }}
  #search:focus {{ outline: none; border-color: #4a7cf7; }}
  #no-results {{ display: none; text-align: center; color: #8b919c; padding: 40px 0; }}
  .timeline {{ position: relative; max-width: 1000px; margin: 0 auto; }}
  .timeline::before {{
    content: "";
    position: absolute;
    left: 50%;
    top: 0;
    bottom: 0;
    width: 2px;
    background: #2a2f3a;
    transform: translateX(-50%);
  }}
  .item {{
    position: relative;
    width: 50%;
    padding: 0 32px 40px;
  }}
  .item:nth-child(even) {{ left: 50%; padding-left: 48px; padding-right: 0; }}
  .item .marker {{
    position: absolute;
    top: 24px;
    right: -8px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #4a7cf7;
    border: 3px solid #0f1115;
    box-shadow: 0 0 0 2px #4a7cf7;
  }}
  .item:nth-child(even) .marker {{ right: auto; left: -8px; }}
  .card {{
    background: #171a21;
    border: 1px solid #232837;
    border-radius: 12px;
    overflow: hidden;
    transition: border-color 0.2s;
  }}
  .card:hover {{ border-color: #3a4150; }}
  .meta {{
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    font-size: 0.8rem;
    background: #1c2029;
  }}
  .meta .id {{ font-weight: 600; color: #4a7cf7; }}
  .meta .ts {{ color: #8b919c; font-family: ui-monospace, Consolas, monospace; }}
  .meta .dim {{ color: #6d7480; }}
  .thumb img {{
    display: block;
    width: 100%;
    height: auto;
    max-height: 420px;
    object-fit: cover;
    object-position: top;
    background: #000;
  }}
  .ocr {{ padding: 12px 14px; font-size: 0.85rem; }}
  .ocr-text {{
    white-space: pre-wrap;
    word-break: break-word;
    font-family: ui-monospace, Consolas, monospace;
    color: #c6cbd3;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }}
  .ocr.expanded .ocr-text {{
    -webkit-line-clamp: unset;
    display: block;
  }}
  .ocr-more {{
    display: block;
    margin-top: 8px;
    padding: 0;
    border: none;
    background: none;
    color: #4a7cf7;
    font-size: 0.8rem;
    font-family: inherit;
    cursor: pointer;
  }}
  .ocr-more:hover {{ text-decoration: underline; }}
  .ocr-more[hidden] {{ display: none; }}
  .ocr em {{ color: #8b919c; }}
  @media (max-width: 720px) {{
    .timeline::before {{ left: 8px; transform: none; }}
    .item, .item:nth-child(even) {{ width: 100%; left: 0; padding: 0 0 32px 32px; }}
    .item .marker, .item:nth-child(even) .marker {{ left: 0; right: auto; }}
  }}
</style>
</head>
<body>
<header>
  <h1>Recall Timeline</h1>
  <p>{count} Screenshots</p>
  <input type="text" id="search" placeholder="OCR-Text filtern ..." autocomplete="off">
</header>
<div class="timeline" id="timeline">{items}</div>
<div id="no-results">Keine Treffer fuer den Suchbegriff.</div>
<script>
  const input = document.getElementById("search");
  const items = Array.from(document.querySelectorAll(".item"));
  const none = document.getElementById("no-results");
  input.addEventListener("input", () => {{
    const q = input.value.trim().toLowerCase();
    let visible = 0;
    for (const it of items) {{
      const show = q === "" || it.dataset.search.toLowerCase().includes(q);
      it.style.display = show ? "" : "none";
      if (show) visible++;
    }}
    none.style.display = visible === 0 ? "block" : "none";
  }});
  const blocks = Array.from(document.querySelectorAll(".ocr"));
  for (const blk of blocks) {{
    const text = blk.querySelector(".ocr-text");
    const btn = blk.querySelector(".ocr-more");
    if (!text || !btn) continue;
    if (text.scrollHeight <= text.clientHeight) {{ btn.hidden = true; continue; }}
    btn.addEventListener("click", () => {{
      const expanded = blk.classList.toggle("expanded");
      btn.textContent = expanded ? "Weniger ..." : "Mehr ...";
    }});
  }}
</script>
</body>
</html>"#,
        count = count,
        items = items,
    )
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
