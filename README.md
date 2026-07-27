# recall-cli

Ein Windows-basiertes Screenshot-Archiv im Terminal-Stil, inspiriert von Microsoft Recall.

## Installation

```bash
cargo build --release
```

Die Binary befindet sich dann unter `target/release/recall-cli.exe`.

## Befehle

| Befehl | Beschreibung |
|--------|-------------|
| `recall-cli snapshot` | Einzelnen Screenshot aufnehmen |
| `recall-cli start --interval <Sekunden>` | Periodische Screenshots starten |
| `recall-cli stop` | Erfassung beenden |
| `recall-cli list [--date <Datum>] [--limit <N>]` | Screenshots auflisten |
| `recall-cli show <ID> [--output <Pfad>]` | Screenshot-Details anzeigen oder exportieren |
| `recall-cli search <Begriff>` | OCR-Volltextsuche |
| `recall-cli stats` | Statistiken anzeigen |
| `recall-cli export [--from <Datum>] [--to <Datum>] [--output <Verzeichnis>]` | Screenshots exportieren |
| `recall-cli cleanup [--older-than <Tage>]` | Alte Screenshots loeschen (Standard: 30 Tage) |

## Beispiele

```bash
# Screenshot aufnehmen
recall-cli snapshot

# Alle 60 Sekunden Screenshots speichern
recall-cli start --interval 60

# Heutige Screenshots anzeigen
recall-cli list --date 2026-07-26

# Screenshot als PNG exportieren
recall-cli show 1 --output desktop.png

# Nach Text suchen
recall-cli search "browser"

# Alle Screenshots in einen Ordner exportieren
recall-cli export --from "2026-07-01" --to "2026-07-31" --output ./exports

# Screenshots aelter als 7 Tage loeschen
recall-cli cleanup --older-than 7

# Automatisches Aufraeumen beim Start (alle 30 Tage)
recall-cli start --interval 60 --cleanup-days 30
```

## Technische Details

| Komponente | Technologie |
|------------|-------------|
| Sprache | Rust (Edition 2021) |
| Screenshot | Win32 GDI (`BitBlt`) |
| Speicherung | SQLite + Zstd-Komprimierung |
| OCR | Windows.Media.Ocr |
| CLI | clap (derive) |
| Async | Tokio |
| DB-Pfad | `~/.recall/recall.db` |

## Projektstruktur

```
recall-cli/
├── Cargo.toml
├── src/
│   ├── main.rs           # CLI-Einstiegspunkt
│   ├── capture.rs        # Screenshot-Erfassung (Win32 GDI)
│   ├── storage.rs        # SQLite + Zstd-Komprimierung
│   ├── scheduler.rs      # Intervall-basierte Erfassung
│   ├── ocr.rs            # Texterkennung (Windows.Media.Ocr)
│   └── search.rs         # Export-Logik
```

## Entwicklungsstatus

- [x] Screenshot-Erfassung
- [x] SQLite-Speicherung mit Zstd-Komprimierung
- [x] Periodische Erfassung (Scheduler)
- [x] CLI mit list, search, show, export, stats
- [x] OCR-Integration (Windows.Media.Ocr)
- [ ] Mehrere Monitore
- [x] Automatisches Aufraeumen alter Eintraege
