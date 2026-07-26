use anyhow::{Context, Result};
use image::{DynamicImage, RgbImage};
use std::io::Cursor;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
    GetDC, GetDIBits, ReleaseDC, SelectObject, SRCCOPY, BI_RGB, BITMAPINFO,
    BITMAPINFOHEADER, DIB_RGB_COLORS,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

pub struct CapturedScreen {
    pub width: u32,
    pub height: u32,
    pub png_bytes: Vec<u8>,
}

pub fn capture_screen() -> Result<CapturedScreen> {
    unsafe {
        let screen_dc = GetDC(None);
        if screen_dc.is_invalid() {
            anyhow::bail!("Fehler beim Zugriff auf den Screen-DC");
        }

        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);

        if width == 0 || height == 0 {
            ReleaseDC(None, screen_dc);
            anyhow::bail!("Ungueltige Bildschirmgroesse: {}x{}", width, height);
        }

        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.is_invalid() {
            ReleaseDC(None, screen_dc);
            anyhow::bail!("Fehler beim Erstellen des Memory-DC");
        }

        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.is_invalid() {
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
            anyhow::bail!("Fehler beim Erstellen des Bitmap");
        }

        let old_bitmap = SelectObject(mem_dc, bitmap);

        let result = BitBlt(mem_dc, 0, 0, width, height, screen_dc, 0, 0, SRCCOPY);

        if result.is_err() {
            SelectObject(mem_dc, old_bitmap);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
            anyhow::bail!("BitBlt fehlgeschlagen");
        }

        let mut bmp_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default(); 1],
        };

        let mut pixel_data = vec![0u8; (width * height * 4) as usize];
        GetDIBits(
            mem_dc,
            bitmap,
            0,
            height as u32,
            Some(pixel_data.as_mut_ptr() as *mut _),
            &mut bmp_info,
            DIB_RGB_COLORS,
        );

        SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, screen_dc);

        let mut img = RgbImage::new(width as u32, height as u32);
        for y in 0..height as u32 {
            for x in 0..width as u32 {
                let idx = ((y * width as u32 + x) * 4) as usize;
                let b = pixel_data[idx];
                let g = pixel_data[idx + 1];
                let r = pixel_data[idx + 2];
                img.put_pixel(x, y, image::Rgb([r, g, b]));
            }
        }

        let dynamic = DynamicImage::ImageRgb8(img);
        let mut png_bytes = Vec::new();
        dynamic
            .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .context("Fehler beim PNG-Encoding")?;

        Ok(CapturedScreen {
            width: width as u32,
            height: height as u32,
            png_bytes,
        })
    }
}
