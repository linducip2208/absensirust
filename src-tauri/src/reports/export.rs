//! Ekspor laporan CSV / XLSX / PDF.
//! PDF dibuat dengan writer minimal hand-rolled (tanpa dependensi berat)
//! agar build Windows tetap ringan; tetap PDF valid dengan header jelas.

use crate::errors::{AppError, AppResult};

pub fn to_csv(headers: &[String], rows: &[Vec<String>]) -> AppResult<String> {
    let mut w = csv::Writer::from_writer(vec![]);
    w.write_record(headers)
        .map_err(|e| AppError::Validation(e.to_string()))?;
    for r in rows {
        w.write_record(r)
            .map_err(|e| AppError::Validation(e.to_string()))?;
    }
    let data = w
        .into_inner()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    String::from_utf8(data).map_err(|e| AppError::Validation(e.to_string()))
}

pub fn to_xlsx(headers: &[String], rows: &[Vec<String>], sheet: &str) -> AppResult<Vec<u8>> {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name(sheet)
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let bold = rust_xlsxwriter::Format::new().set_bold();
    for (c, h) in headers.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, h, &bold)
            .map_err(|e| AppError::Validation(e.to_string()))?;
    }
    for (r, row) in rows.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            ws.write_string((r + 1) as u32, c as u16, v)
                .map_err(|e| AppError::Validation(e.to_string()))?;
        }
    }
    wb.save_to_buffer()
        .map_err(|e| AppError::Validation(e.to_string()))
}

/// PDF minimal valid: satu halaman teks (latin-1 safe), cukup untuk laporan operasional.
pub fn to_pdf(title: &str, meta: &str, headers: &[String], rows: &[Vec<String>]) -> Vec<u8> {
    fn esc(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '(' | ')' | '\\' => format!("\\{c}"),
                c if (c as u32) < 256 => c.to_string(),
                _ => "?".to_string(),
            })
            .collect()
    }
    let mut lines: Vec<String> = vec![
        title.to_string(),
        meta.to_string(),
        format!("Jumlah baris: {}", rows.len()),
        headers.join(" | "),
    ];
    for r in rows.iter().take(200) {
        lines.push(r.join(" | "));
    }
    let mut content = String::from("BT /F1 9 Tf 40 780 Td 12 TL ");
    for (i, l) in lines.iter().enumerate() {
        let t = esc(l);
        let short: String = t.chars().take(140).collect();
        if i == 0 {
            content.push_str(&format!("({}) Tj T* ", esc(&short)));
        } else {
            content.push_str(&format!("({}) Tj T* ", short));
        }
    }
    content.push_str("ET");
    let objs = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = vec![];
    for (i, o) in objs.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{}\nendobj\n", i + 1, o));
    }
    let xref = pdf.len();
    pdf.push_str(&format!(
        "xref\n0 {}\n0000000000 65535 f \n",
        objs.len() + 1
    ));
    for off in offsets {
        pdf.push_str(&format!("{off:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF",
        objs.len() + 1
    ));
    pdf.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_export_ok() {
        let s = to_csv(
            &["nama".into(), "nik".into()],
            &[vec!["Budi".into(), "001".into()]],
        )
        .unwrap();
        assert!(s.contains("Budi"));
    }
    #[test]
    fn xlsx_export_ok() {
        let b = to_xlsx(&["nama".into()], &[vec!["Budi Santoso".into()]], "Laporan").unwrap();
        assert!(b.len() > 1000);
        assert_eq!(&b[0..2], b"PK");
    }
    #[test]
    fn pdf_export_valid_header() {
        let b = to_pdf(
            "Laporan",
            "Dibuat: 2026-10-13 10:00:00 WIB",
            &["nama".into()],
            &[vec!["Budi".into()]],
        );
        assert!(b.starts_with(b"%PDF-1.4"));
        assert!(b.windows(12).any(|w| w == b"Jumlah baris"));
    }
}
