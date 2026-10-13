/// Zona waktu operasional: Asia/Jakarta (WIB, UTC+7, tanpa DST).
/// Semua timestamp disimpan sebagai TEXT "YYYY-MM-DD HH:MM:SS" waktu WIB
/// agar konsisten dan dapat dibaca lintas platform. Kolom UTC opsional
/// hanya untuk audit. Aturan konversi didokumentasikan di docs/SHIFT_ENGINE.md.
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, TimeZone};
use chrono_tz::Asia::Jakarta;

pub const WIB_OFFSET_SECS: i32 = 7 * 3600;
pub const DISPLAY_FMT: &str = "%Y-%m-%d %H:%M:%S";
pub const DATE_FMT: &str = "%Y-%m-%d";

pub fn wib_offset() -> FixedOffset {
    FixedOffset::east_opt(WIB_OFFSET_SECS).expect("offset WIB valid")
}

/// Sekarang dalam WIB sebagai NaiveDateTime.
pub fn now_wib_naive() -> NaiveDateTime {
    chrono::Utc::now().with_timezone(&Jakarta).naive_local()
}

pub fn now_wib_string() -> String {
    now_wib_naive().format(DISPLAY_FMT).to_string()
}

pub fn today_wib() -> NaiveDate {
    chrono::Utc::now().with_timezone(&Jakarta).date_naive()
}

pub fn parse_wib(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, DISPLAY_FMT).ok()
}

pub fn format_wib(dt: &NaiveDateTime) -> String {
    dt.format(DISPLAY_FMT).to_string()
}

pub fn to_utc(dt_wib: &NaiveDateTime) -> DateTime<chrono::Utc> {
    let wib = wib_offset()
        .from_local_datetime(dt_wib)
        .single()
        .unwrap_or_else(|| {
            wib_offset()
                .from_local_datetime(dt_wib)
                .earliest()
                .expect("fallback WIB")
        });
    wib.with_timezone(&chrono::Utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wib_is_utc_plus_7() {
        let naive = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let utc = to_utc(&naive);
        assert_eq!(utc.format("%H:%M").to_string(), "05:00");
    }
}
