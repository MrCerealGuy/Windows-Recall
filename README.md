# recall-cli

A Windows-based screenshot archive in terminal style, inspired by Microsoft Recall.

## Disclaimer

Windows Recall must not be used for espionage purposes or in any way that violates applicable laws. The tool is intended exclusively for personal use to build up your own personal archive. The user is solely responsible for compliance with all applicable laws and regulations.

## Installation

```bash
cargo build --release
```

The binary is located at `target/release/recall-cli.exe`.

## Commands

| Command | Description |
|---------|-------------|
| `recall-cli snapshot` | Take a single screenshot |
| `recall-cli start --interval <seconds>` | Start periodic screenshot capture |
| `recall-cli stop` | Stop capture |
| `recall-cli list [--date <date>] [--limit <N>]` | List screenshots |
| `recall-cli show <ID> [--output <path>]` | Show screenshot details or export |
| `recall-cli search <term>` | OCR full-text search |
| `recall-cli stats` | Show statistics |
| `recall-cli export [--from <date>] [--to <date>] [--output <dir>]` | Export screenshots + OCR timeline |
| `recall-cli cleanup [--older-than <days>]` | Delete old screenshots (default: 30 days) |
| `recall-cli schedule --day <day> --start <HH:MM> --end <HH:MM> [--interval <sec>]` | Create a task in the Windows Task Scheduler |
| `recall-cli unschedule --name <name>` | Remove a task from the Task Scheduler |
| `recall-cli schedule-list` | Show all Recall tasks in the Task Scheduler |

## Examples

```bash
# Take a screenshot
recall-cli snapshot

# Save screenshots every 60 seconds
recall-cli start --interval 60

# Show today's screenshots
recall-cli list --date 2026-07-26

# Export a screenshot as PNG
recall-cli show 1 --output desktop.png

# Search for text
recall-cli search "browser"

# Export all screenshots to a folder (incl. timeline, subfolder per export)
recall-cli export --from "2026-07-01" --to "2026-07-31" --output ./exports

# Delete screenshots older than 7 days
recall-cli cleanup --older-than 7

# Automatic cleanup on start (every 30 days)
recall-cli start --interval 60 --cleanup-days 30

# Take screenshots every 30s every Saturday from 09:00-17:00
recall-cli schedule --day SAT --start 09:00 --end 17:00 --interval 30 --name "Saturday"

# Show all created tasks
recall-cli schedule-list

# Remove a task again
recall-cli unschedule --name "Saturday"
```

## Export Timeline

`recall-cli export` creates a static website `index.html` in the output folder in addition to the PNG files. The timeline shows all screenshots chronologically with OCR text, preview image and timestamp.

- Each export writes to a new timestamp subfolder (e.g. `exports/2026-08-05-0945/`) so existing files are never overwritten and exports form an archive history.
- Filenames are zero-padded (`recall_0001_...`) so sorting by filename is chronological.
- Long OCR texts are truncated to 3 lines by default and can be expanded via the "More ..." button.
- An integrated search filter searches all OCR texts directly in the browser (no external dependencies).

## Technical Details

| Component | Technology |
|-----------|------------|
| Language | Rust (Edition 2021) |
| Screenshot | Win32 GDI (`BitBlt`) |
| Storage | SQLite + Zstd compression |
| OCR | Windows.Media.Ocr |
| CLI | clap (derive) |
| Async | Tokio |
| DB path | `~/.recall/recall.db` |

## Project Structure

```
recall-cli/
├── Cargo.toml
├── examples/            # Example scripts (schedule, export, list, cleanup ...)
├── src/
│   ├── main.rs           # CLI entry point
│   ├── capture.rs        # Screenshot capture (Win32 GDI)
│   ├── storage.rs        # SQLite + Zstd compression
│   ├── scheduler.rs      # Interval-based capture
│   ├── ocr.rs            # Text recognition (Windows.Media.Ocr)
│   └── search.rs         # Export logic + HTML timeline
```

## Development Status

- [x] Screenshot capture
- [x] SQLite storage with Zstd compression
- [x] Periodic capture (scheduler)
- [x] CLI with list, search, show, export, stats
- [x] OCR integration (Windows.Media.Ocr)
- [x] Automatic cleanup of old entries (with counter reset)
- [x] Windows Task Scheduler integration
- [x] Export as static HTML timeline with OCR texts
- [x] Reliable service shutdown (terminates the process via PID)
- [ ] Multiple monitors
