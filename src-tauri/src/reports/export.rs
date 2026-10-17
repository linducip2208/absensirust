//! Ekspor laporan CSV / XLSX / PDF.
//! PDF dibuat dengan writer hand-rolled multi-halaman (tanpa dependensi berat)
//! agar build Windows tetap ringan: judul + meta + header tabel berulang +
//! nomor halaman + ringkasan, font standar PDF, teks ASCII (transliterasi).

use crate::errors::{AppError, AppResult};

/// Lindungi pembuka Excel dari serangan formula CSV (satuan kerja: kolom
/// nama/alamat bisa berbunyi "=COMMAND|..."). Awalan berbahaya diberi
/// apostrof; angka negatif (mis. -12) TIDAK disentuh.
fn csv_safe_cell(v: &str) -> String {
    if v.is_empty() {
        return String::new();
    }
    let first = v.as_bytes()[0];
    let dangerous = matches!(first, b'=' | b'+' | b'@')
        || (first == b'-' && v.len() > 1 && !v.as_bytes()[1].is_ascii_digit());
    if dangerous {
        format!("'{v}")
    } else {
        v.to_string()
    }
}

pub fn to_csv(headers: &[String], rows: &[Vec<String>]) -> AppResult<String> {
    let mut w = csv::Writer::from_writer(vec![]);
    w.write_record(headers.iter().map(|h| csv_safe_cell(h)))
        .map_err(|e| AppError::Validation(e.to_string()))?;
    for r in rows {
        w.write_record(r.iter().map(|c| csv_safe_cell(c)))
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

struct TableRow {
    cells: Vec<Vec<String>>,
    height: f32,
}

/// PDF laporan multi-halaman yang layak dibaca.
pub fn to_pdf(title: &str, meta: &str, headers: &[String], rows: &[Vec<String>]) -> Vec<u8> {
    const PAGE_W: f32 = 595.0; // A4 portrait (pt)
    const PAGE_H: f32 = 842.0;
    const MARGIN: f32 = 40.0;
    const LEADING: f32 = 12.0;
    const FOOTER_Y: f32 = 28.0;
    const MAX_ROWS: usize = 2000; // sama dengan LIMIT query laporan
    const AVG_CHAR_PT: f32 = 5.0; // Helvetica 9pt ≈ 5pt/karakter

    let ncols = headers
        .len()
        .max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
    if ncols == 0 {
        return to_pdf_simple(title, "tidak ada kolom");
    }
    let heads: Vec<String> = (0..ncols)
        .map(|i| headers.get(i).cloned().unwrap_or_default())
        .collect();
    let data: Vec<Vec<String>> = rows
        .iter()
        .take(MAX_ROWS)
        .map(|r| {
            (0..ncols)
                .map(|i| r.get(i).cloned().unwrap_or_default())
                .collect()
        })
        .collect();

    // Lebar kolom proporsional terhadap isi (per karakter, dibatasi).
    let usable = PAGE_W - MARGIN * 2.0;
    let mut weights: Vec<f32> = vec![8.0; ncols];
    for (i, h) in heads.iter().enumerate() {
        weights[i] = weights[i].max(h.chars().count().min(60) as f32);
    }
    for r in data.iter().take(200) {
        for (i, c) in r.iter().enumerate() {
            weights[i] = weights[i].max(c.chars().count().min(60) as f32);
        }
    }
    let total_w: f32 = weights.iter().sum();
    let widths: Vec<f32> = weights
        .iter()
        .map(|w| (w / total_w * usable).max(36.0))
        .collect();
    let mut xs = Vec::with_capacity(ncols);
    let mut acc = MARGIN;
    for w in &widths {
        xs.push(acc);
        acc += w;
    }
    let max_chars: Vec<usize> = widths
        .iter()
        .map(|w| ((w / AVG_CHAR_PT) as usize).max(6))
        .collect();

    let header_row = TableRow {
        cells: heads
            .iter()
            .enumerate()
            .map(|(i, h)| wrap_text(h, max_chars[i]))
            .collect(),
        height: 0.0,
    };
    let mut header_row = header_row;
    header_row.height =
        header_row.cells.iter().map(|c| c.len()).max().unwrap_or(1) as f32 * LEADING;
    let body: Vec<TableRow> = data
        .iter()
        .map(|r| {
            let cells: Vec<Vec<String>> = r
                .iter()
                .enumerate()
                .map(|(i, c)| wrap_text(c, max_chars[i]))
                .collect();
            let height = cells.iter().map(|c| c.len()).max().unwrap_or(1) as f32 * LEADING;
            TableRow { cells, height }
        })
        .collect();

    // Paginasi: halaman pertama memuat blok judul; header tabel diulang.
    let title_block_h = LEADING * 5.0;
    let mut page_ranges: Vec<(usize, usize)> = Vec::new();
    let mut idx = 0usize;
    let mut first = true;
    loop {
        let mut y = PAGE_H - MARGIN - if first { title_block_h } else { 0.0 } - header_row.height;
        let start = idx;
        while idx < body.len() && y - body[idx].height >= MARGIN + FOOTER_Y + 6.0 {
            y -= body[idx].height;
            idx += 1;
        }
        if start == idx && idx < body.len() {
            idx += 1; // baris raksasa: paksa satu baris per halaman
        }
        page_ranges.push((start, idx));
        first = false;
        if idx >= body.len() {
            break;
        }
    }
    let npages = page_ranges.len();
    let title_short: String = title.chars().take(60).collect();

    let mut contents: Vec<String> = Vec::new();
    for (pi, (s, e)) in page_ranges.iter().enumerate() {
        let mut c = String::new();
        if pi == 0 {
            c.push_str(&format!(
                "BT /F2 14 Tf {x} {y} Td ({t}) Tj ET\nBT /F1 9 Tf {x} {y2} Td ({m}) Tj ET\nBT /F1 9 Tf {x} {y3} Td (Jumlah baris: {n}) Tj ET\n",
                x = MARGIN,
                y = PAGE_H - MARGIN - 6.0,
                t = esc(&title.chars().take(90).collect::<String>()),
                y2 = PAGE_H - MARGIN - 24.0,
                m = esc(&meta.chars().take(100).collect::<String>()),
                y3 = PAGE_H - MARGIN - 38.0,
                n = data.len(),
            ));
        }
        let mut y = PAGE_H - MARGIN - if pi == 0 { title_block_h } else { 0.0 };
        y -= header_row.height;
        c.push_str(&emit_row(&header_row, &xs, &widths, y, true));
        for row in &body[*s..*e] {
            y -= row.height;
            c.push_str(&emit_row(row, &xs, &widths, y, false));
        }
        c.push_str(&format!(
            "BT /F1 8 Tf {x} {y} Td (Halaman {p} dari {n} - {t}) Tj ET\n",
            x = MARGIN,
            y = FOOTER_Y,
            p = pi + 1,
            n = npages,
            t = esc(&title_short),
        ));
        if pi + 1 == npages {
            let truncated = rows.len() > MAX_ROWS;
            c.push_str(&format!(
                "BT /F1 9 Tf {x} {y} Td (Total baris data: {n}{tr}) Tj ET\n",
                x = MARGIN,
                y = FOOTER_Y + 12.0,
                n = data.len(),
                tr = if truncated {
                    format!(" - ditampilkan {MAX_ROWS} pertama")
                } else {
                    String::new()
                },
            ));
        }
        contents.push(c);
    }

    // Susun objek PDF (penomoran dinamis).
    let mut objs: Vec<String> = Vec::new();
    objs.push("<< /Type /Catalog /Pages 2 0 R >>".to_string());
    let kids: Vec<String> = (0..npages).map(|i| format!("{} 0 R", 3 + i * 2)).collect();
    objs.push(format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids.join(" "),
        npages
    ));
    for ci in contents {
        let content_no = objs.len() + 2;
        objs.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 {} 0 R /F2 {} 0 R >> >> /Contents {} 0 R >>",
            3 + npages * 2,
            4 + npages * 2,
            content_no
        ));
        objs.push(format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            ci.len(),
            ci
        ));
    }
    objs.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string());
    objs.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>".to_string());

    assemble(&objs)
}

/// Bungkus teks per kata pada batas karakter (murni, teruji).
fn wrap_text(s: &str, max: usize) -> Vec<String> {
    let max = max.max(1);
    let mut lines = Vec::new();
    for part in s.split('\n') {
        let start_len = lines.len();
        let mut cur = String::new();
        for word in part.split_whitespace() {
            if word.chars().count() > max {
                if !cur.is_empty() {
                    lines.push(std::mem::take(&mut cur));
                }
                let mut chunk = String::new();
                for (n, ch) in word.chars().enumerate() {
                    chunk.push(ch);
                    if (n + 1) % max == 0 {
                        lines.push(std::mem::take(&mut chunk));
                    }
                }
                if !chunk.is_empty() {
                    cur = chunk;
                }
                continue;
            }
            if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > max {
                lines.push(std::mem::take(&mut cur));
            }
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(word);
        }
        if !cur.is_empty() {
            lines.push(cur);
        } else if lines.len() == start_len {
            lines.push(" ".to_string()); // baris kosong eksplisit
        }
    }
    lines
}

/// Satu baris tabel → operator teks PDF pada posisi kolomnya.
fn emit_row(row: &TableRow, xs: &[f32], widths: &[f32], y_top: f32, bold: bool) -> String {
    const LEADING: f32 = 12.0;
    let font = if bold { "F2" } else { "F1" };
    let mut out = String::new();
    for (i, cell) in row.cells.iter().enumerate() {
        let max_c = ((widths[i] / 5.0) as usize).max(1);
        for (j, line) in cell.iter().enumerate() {
            let short: String = line.chars().take(max_c).collect();
            let y = y_top - j as f32 * LEADING - 9.0;
            out.push_str(&format!(
                "BT /{font} 9 Tf {x:.1} {y:.1} Td ({t}) Tj ET\n",
                x = xs[i] + 2.0,
                t = esc(&short),
            ));
        }
    }
    out
}

/// Rakit objek menjadi file PDF valid dengan xref tepat (byte offset).
fn assemble(objs: &[String]) -> Vec<u8> {
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

/// PDF satu halaman darurat (tanpa kolom).
fn to_pdf_simple(title: &str, meta: &str) -> Vec<u8> {
    let content = format!(
        "BT /F1 12 Tf 40 780 Td 14 TL ({}) Tj T* ({}) Tj ET",
        esc(title),
        esc(meta)
    );
    assemble(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ])
}

/// Escape teks PDF + transliterasi ke ASCII (font standar tanpa embedding).
fn esc(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '(' | ')' | '\\' => vec!['\\', c],
            c if (c as u32) < 128 => vec![c],
            _ => vec![latin_ascii(c)],
        })
        .collect()
}

fn latin_ascii(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'Ā' | 'Ă' | 'Ą' => 'A',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ę' | 'ě' => 'e',
        'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ę' | 'Ě' => 'E',
        'ì' | 'í' | 'î' | 'ï' | 'ī' => 'i',
        'Ì' | 'Í' | 'Î' | 'Ï' | 'Ī' => 'I',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ō' => 'o',
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ō' => 'O',
        'ù' | 'ú' | 'û' | 'ü' | 'ū' => 'u',
        'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ū' => 'U',
        'ñ' => 'n',
        'Ñ' => 'N',
        'ç' | 'ć' | 'č' => 'c',
        'Ç' | 'Ć' | 'Č' => 'C',
        'š' | 'ś' => 's',
        'Š' | 'Ś' => 'S',
        'ž' | 'ź' | 'ż' => 'z',
        'Ž' | 'Ź' | 'Ż' => 'Z',
        'ý' | 'ÿ' => 'y',
        'Ý' => 'Y',
        '–' | '—' => '-',
        '‘' | '’' => '\'',
        '“' | '”' => '"',
        '…' => '.',
        _ => '?',
    }
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
    fn csv_formula_dilindungi_angka_negatif_utuh() {
        // Formula berbahaya diberi awalan apostrof — Excel tidak mengeksekusinya.
        let s = to_csv(
            &["nama".into(), "ket".into()],
            &[
                vec!["=1+1 | cmd".into(), "x".into()],
                vec!["+SUM(A1)".into(), "x".into()],
                vec!["@meta".into(), "x".into()],
                // Angka negatif sah JANGAN dirusak.
                vec!["-12".into(), "x".into()],
                vec!["-12 hati".into(), "x".into()],
            ],
        )
        .unwrap();
        assert!(s.contains("'=1+1"), "awal formula: {s}");
        assert!(s.contains("'+SUM"), "+ formula: {s}");
        assert!(s.contains("'@meta"), "@ formula: {s}");
        assert!(
            s.lines()
                .any(|l| l.contains(",-12,") || l.starts_with("-12,")),
            "angka: {s}"
        );
        assert!(
            !s.contains("'-12"),
            "angka negatif tak boleh berawalan ' : {s}"
        );
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
        let s = String::from_utf8_lossy(&b);
        assert!(s.contains("Halaman 1 dari 1"));
    }
    #[test]
    fn pdf_multipage_numbers_pages() {
        let rows: Vec<Vec<String>> = (0..120)
            .map(|i| {
                vec![
                    format!("2026-10-{0:02} 07:00:00", (i % 28) + 1),
                    format!("Karyawan {i}"),
                ]
            })
            .collect();
        let b = to_pdf(
            "Rekap",
            "Periode Oktober",
            &["Tanggal".into(), "Nama".into()],
            &rows,
        );
        assert!(b.starts_with(b"%PDF-1.4"));
        let s = String::from_utf8_lossy(&b);
        assert!(s.contains("Halaman 2 dari"), "harus lebih dari 1 halaman");
        assert!(s.contains("Total baris data: 120"));
        // Header tabel diulang di setiap halaman (2 kemunculan untuk 2+ halaman).
        assert!(s.matches("(Tanggal)").count() >= 2);
    }
    #[test]
    fn pdf_indonesian_chars_survive() {
        let b = to_pdf(
            "Laporan Kehadiran André",
            "Periode",
            &["Nama".into()],
            &[vec!["André–Sari “hadir”".into()]],
        );
        assert!(b.is_ascii(), "PDF harus murni ASCII agar xref tepat");
        let s = String::from_utf8_lossy(&b);
        assert!(s.contains("Andre-Sari"));
    }
    #[test]
    fn wrap_text_basic() {
        assert_eq!(
            wrap_text("a b c", 3),
            vec!["a b".to_string(), "c".to_string()]
        );
        assert_eq!(
            wrap_text("abcdef", 3),
            vec!["abc".to_string(), "def".to_string()]
        );
        assert_eq!(wrap_text("", 5), vec![" ".to_string()]);
    }
}
