use crate::capture::CapturedScreen;
use anyhow::{Context, Result};
use windows::Graphics::Imaging::{BitmapDecoder, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

pub fn recognize(screenshot: &CapturedScreen) -> Result<String> {
    let stream = InMemoryRandomAccessStream::new()
        .context("Fehler beim Erstellen des InMemory-Streams")?;

    let output = stream.GetOutputStreamAt(0)
        .context("Fehler beim Abrufen des OutputStream")?;

    let writer = DataWriter::CreateDataWriter(&output)
        .context("Fehler beim Erstellen des DataWriter")?;
    writer
        .WriteBytes(&screenshot.png_bytes)
        .context("Fehler beim Schreiben der PNG-Bytes")?;

    writer.StoreAsync().context("Fehler bei StoreAsync")?.get()?;
    writer.FlushAsync().context("Fehler bei FlushAsync")?.get()?;

    stream.Seek(0).context("Fehler beim Zuruecksetzen des Streams")?;

    let decoder = BitmapDecoder::CreateAsync(&stream)
        .context("Fehler beim Erstellen des BitmapDecoder")?
        .get()?;

    let bitmap = decoder
        .GetSoftwareBitmapAsync()
        .context("Fehler bei GetSoftwareBitmapAsync")?
        .get()?;

    let bitmap = if bitmap.BitmapPixelFormat()? != BitmapPixelFormat::Bgra8 {
        SoftwareBitmap::Convert(&bitmap, BitmapPixelFormat::Bgra8)?
    } else {
        bitmap
    };

    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .context("Fehler beim Erstellen der OCR-Engine (Sprache nicht verfuegbar?)")?;

    let result = engine
        .RecognizeAsync(&bitmap)
        .context("Fehler bei RecognizeAsync")?
        .get()?;

    Ok(result
        .Text()
        .context("Fehler beim Lesen des OCR-Textes")?
        .to_string())
}
