//! Attendance & shift engine (zona waktu Asia/Jakarta eksplisit).
//!
//! Aturan konversi waktu:
//! - Semua timestamp operasional disimpan sebagai TEXT WIB "YYYY-MM-DD HH:MM:SS".
//! - Tidak ada konversi UTC untuk tampilan; UTC hanya dipakai internal bila perlu.
//! - Shift malam: window kerja dimulai pada work_date jam mulai, berakhir work_date+1 jam selesai.
//!   Clock-in pada tanggal sebelumnya TIDAK dikaitkan ke malam ini; clock-out setelah tengah
//!   malam tetap milik work_date asal (bukan tanggal kalender clock-out).
//! - Penentuan shift: assignment efektif (effective_date <= work_date, end_date NULL/>= work_date)
//!   + applicable_days + window yang memuat clock-in. Bukan sekadar tanggal kalender.

use chrono::{NaiveDate, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttendanceStatus {
    Present,
    Late,
    Absent,
    Leave,
    Permission,
    Holiday,
    DayOff,
    Incomplete,
    NeedsReview,
}

impl AttendanceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Present => "Present",
            Self::Late => "Late",
            Self::Absent => "Absent",
            Self::Leave => "Leave",
            Self::Permission => "Permission",
            Self::Holiday => "Holiday",
            Self::DayOff => "DayOff",
            Self::Incomplete => "Incomplete",
            Self::NeedsReview => "NeedsReview",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftRule {
    pub id: String,
    pub code: String,
    pub name: String,
    pub start: (u32, u32),
    pub end: (u32, u32),
    pub crosses_midnight: bool,
    pub break_minutes: i64,
    pub late_tolerance_minutes: i64,
    pub clockin_early_limit_minutes: i64,
    pub clockin_late_limit_minutes: i64,
    pub clockout_limit_minutes: i64,
    pub overtime_before_minutes: i64,
    pub overtime_after_minutes: i64,
    pub applicable_days: Vec<u32>,
    pub version: i64,
}

impl ShiftRule {
    // Mapper baris DB -> aturan (14 kolom); bentuk tabular memang butuh banyak parameter.
    #[allow(clippy::too_many_arguments)]
    pub fn from_row(
        code: &str,
        name: &str,
        start: &str,
        end: &str,
        crosses: i64,
        brk: i64,
        tol: i64,
        early: i64,
        late_lim: i64,
        out_lim: i64,
        ot_b: i64,
        ot_a: i64,
        days: &str,
        ver: i64,
    ) -> Self {
        Self {
            id: code.to_string(),
            code: code.to_string(),
            name: name.to_string(),
            start: parse_hm(start),
            end: parse_hm(end),
            crosses_midnight: crosses != 0,
            break_minutes: brk,
            late_tolerance_minutes: tol,
            clockin_early_limit_minutes: early,
            clockin_late_limit_minutes: late_lim,
            clockout_limit_minutes: out_lim,
            overtime_before_minutes: ot_b,
            overtime_after_minutes: ot_a,
            applicable_days: days
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect(),
            version: ver,
        }
    }

    /// Window kerja untuk work_date (NaiveDate WIB).
    pub fn window(&self, work_date: &NaiveDate) -> (NaiveDateTime, NaiveDateTime) {
        let start = work_date
            .and_hms_opt(self.start.0, self.start.1, 0)
            .unwrap();
        let end_base = work_date.and_hms_opt(self.end.0, self.end.1, 0).unwrap();
        let end = if self.crosses_midnight || end_base <= start {
            end_base + chrono::Duration::days(1)
        } else {
            end_base
        };
        (start, end)
    }

    /// Apakah clock-in diterima untuk work_date ini (batas awal/akhir penerimaan).
    pub fn accepts_clockin(&self, work_date: &NaiveDate, t: &NaiveDateTime) -> bool {
        let (start, _) = self.window(work_date);
        let earliest = start - chrono::Duration::minutes(self.clockin_early_limit_minutes);
        let latest = start + chrono::Duration::minutes(self.clockin_late_limit_minutes);
        *t >= earliest && *t <= latest
    }

    /// Batas penerimaan event untuk work_date ini (inklusif):
    /// [shift_start - early_limit, window_end + clockout_limit].
    pub fn accept_bounds(&self, work_date: &NaiveDate) -> (NaiveDateTime, NaiveDateTime) {
        let (start, end) = self.window(work_date);
        (
            start - chrono::Duration::minutes(self.clockin_early_limit_minutes),
            end + chrono::Duration::minutes(self.clockout_limit_minutes),
        )
    }

    pub fn applies_to_weekday(&self, work_date: &NaiveDate) -> bool {
        let wd = work_date.weekday_num_monday();
        self.applicable_days.contains(&wd)
    }
}

/// Format HH:MM valid (00:00–23:59)? Untuk validasi tulis, bukan panic baca.
pub fn valid_hm(s: &str) -> bool {
    let mut it = s.split(':');
    match (it.next(), it.next(), it.next()) {
        (Some(h), Some(m), None) => {
            matches!((h.parse::<u32>(), m.parse::<u32>()), (Ok(h), Ok(m)) if h < 24 && m < 60)
        }
        _ => false,
    }
}

fn parse_hm(s: &str) -> (u32, u32) {
    let mut it = s.split(':');
    let h = it.next().unwrap_or("0").parse().unwrap_or(0);
    let m = it.next().unwrap_or("0").parse().unwrap_or(0);
    (h, m)
}

trait WeekdayNum {
    fn weekday_num_monday(&self) -> u32;
}
impl WeekdayNum for NaiveDate {
    fn weekday_num_monday(&self) -> u32 {
        use chrono::Datelike;
        self.weekday().number_from_monday() - 1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayEvents {
    pub clock_in: Option<NaiveDateTime>,
    pub clock_out: Option<NaiveDateTime>,
    pub duplicate_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeInput {
    pub work_date: NaiveDate,
    pub shift: ShiftRule,
    pub events: DayEvents,
    pub is_holiday: bool,
    pub is_day_off: bool,
    pub on_leave: bool,
    pub on_permission: bool,
    pub rounding_minutes: i64,
    pub overtime_approved_minutes: i64,
    pub require_overtime_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeOutput {
    pub clock_in: Option<String>,
    pub clock_out: Option<String>,
    pub work_minutes: i64,
    pub break_minutes: i64,
    pub late_minutes: i64,
    pub early_leave_minutes: i64,
    pub overtime_minutes: i64,
    pub overtime_approved: bool,
    pub status: AttendanceStatus,
    pub anomaly: String,
    pub source: String,
}

pub fn round_down(mins: i64, step: i64) -> i64 {
    if step <= 1 {
        mins
    } else {
        (mins / step) * step
    }
}

/// Hitung satu hari kerja. Murni & deterministik (tanpa jam sistem).
pub fn compute_day(inp: &ComputeInput) -> ComputeOutput {
    let (win_start, win_end) = inp.shift.window(&inp.work_date);
    let fmt = |d: &NaiveDateTime| d.format("%Y-%m-%d %H:%M:%S").to_string();

    if inp.on_leave {
        return out(
            None,
            None,
            0,
            0,
            0,
            0,
            0,
            false,
            AttendanceStatus::Leave,
            "",
            &inp.shift,
        );
    }
    if inp.on_permission {
        return out(
            None,
            None,
            0,
            0,
            0,
            0,
            0,
            false,
            AttendanceStatus::Permission,
            "",
            &inp.shift,
        );
    }
    if inp.is_holiday {
        // Kerja saat libur tetap dihitung; tanpa event => Holiday.
        if inp.events.clock_in.is_none() {
            return out(
                None,
                None,
                0,
                0,
                0,
                0,
                0,
                false,
                AttendanceStatus::Holiday,
                "",
                &inp.shift,
            );
        }
    }
    if inp.is_day_off && inp.events.clock_in.is_none() && inp.events.clock_out.is_none() {
        return out(
            None,
            None,
            0,
            0,
            0,
            0,
            0,
            false,
            AttendanceStatus::DayOff,
            "",
            &inp.shift,
        );
    }

    let ci = inp.events.clock_in;
    let co = inp.events.clock_out;
    let mut anomalies: Vec<String> = vec![];
    if inp.events.duplicate_count > 0 {
        anomalies.push(format!(
            "clock-in ganda x{}",
            inp.events.duplicate_count + 1
        ));
    }

    match (ci, co) {
        (None, None) => {
            // Jangan simpulkan Absent bila tanggal belum selesai — pemanggil yang memutuskan.
            out(
                None,
                None,
                0,
                0,
                0,
                0,
                0,
                false,
                AttendanceStatus::Absent,
                &anomalies.join(";"),
                &inp.shift,
            )
        }
        (Some(_), None) | (None, Some(_)) => out(
            ci.map(|d| fmt(&d)),
            co.map(|d| fmt(&d)),
            0,
            0,
            0,
            0,
            0,
            false,
            AttendanceStatus::Incomplete,
            &push(anomalies, "log tidak lengkap"),
            &inp.shift,
        ),
        (Some(ci), Some(mut co)) => {
            // Clock-out sebelum clock-in namun masih dalam batas window malam -> anggap lintas tengah malam (+1 hari).
            if co < ci && inp.shift.crosses_midnight {
                let cand = co + chrono::Duration::days(1);
                if cand <= win_end + chrono::Duration::minutes(inp.shift.clockout_limit_minutes) {
                    co = cand;
                }
            }
            if co < ci {
                return out(
                    Some(fmt(&ci)),
                    Some(fmt(&co)),
                    0,
                    0,
                    0,
                    0,
                    0,
                    false,
                    AttendanceStatus::NeedsReview,
                    &push(anomalies, "clock-out sebelum clock-in"),
                    &inp.shift,
                );
            }
            // Clock-out melewati batas penerimaan.
            if co > win_end + chrono::Duration::minutes(inp.shift.clockout_limit_minutes) {
                anomalies.push("clock-out melewati batas penerimaan".into());
            }
            let late_raw = (ci - win_start).num_minutes();
            let late = if late_raw <= inp.shift.late_tolerance_minutes {
                0
            } else {
                late_raw
            };
            let early_raw = (win_end - co).num_minutes();
            let early = early_raw.max(0);
            let mut gross = (co - ci).num_minutes().max(0);
            gross = round_down(gross, inp.rounding_minutes.max(1));
            let work = (gross - inp.shift.break_minutes).max(0);
            // Lembur sesudah shift.
            let after = (co - win_end).num_minutes().max(0);
            let mut ot = after.min(inp.shift.overtime_after_minutes + 480); // batasi wajar 8 jam di atas cap
            ot = ot.min(
                inp.shift
                    .overtime_after_minutes
                    .max(after.min(inp.shift.overtime_after_minutes)),
            );
            // Jika kebijakan mewajibkan approval, hanya hitung yang disetujui.
            let (ot_final, ot_approved) = if inp.require_overtime_approval {
                (
                    ot.min(inp.overtime_approved_minutes),
                    inp.overtime_approved_minutes > 0,
                )
            } else {
                (ot, true)
            };
            // Lembur sebelum shift (datang sangat awal) bila kebijakan mengizinkan.
            let _before = if inp.shift.overtime_before_minutes > 0 {
                (win_start - ci)
                    .num_minutes()
                    .max(0)
                    .min(inp.shift.overtime_before_minutes)
            } else {
                0
            };
            let status = if late > 0 {
                AttendanceStatus::Late
            } else {
                AttendanceStatus::Present
            };
            let mut o = out(
                Some(fmt(&ci)),
                Some(fmt(&co)),
                work,
                inp.shift.break_minutes,
                late,
                early,
                ot_final,
                ot_approved,
                status,
                &anomalies.join(";"),
                &inp.shift,
            );
            if early > 0 {
                o.anomaly = push(
                    vec![o.anomaly.clone()]
                        .into_iter()
                        .filter(|s| !s.is_empty())
                        .collect(),
                    "pulang cepat",
                );
            }
            if ci.hour() == 23 || co.hour() < 12 && inp.shift.crosses_midnight {
                // penanda audit lintas tanggal, bukan anomali.
            }
            o
        }
    }
}

fn push(mut v: Vec<String>, s: &str) -> String {
    v.push(s.to_string());
    v.into_iter()
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join(";")
}

// Konstruktor hasil hitung harian; satu baris hasil punya banyak kolom.
#[allow(clippy::too_many_arguments)]
fn out(
    ci: Option<String>,
    co: Option<String>,
    work: i64,
    brk: i64,
    late: i64,
    early: i64,
    ot: i64,
    ot_appr: bool,
    status: AttendanceStatus,
    anomaly: &str,
    shift: &ShiftRule,
) -> ComputeOutput {
    ComputeOutput {
        clock_in: ci,
        clock_out: co,
        work_minutes: work,
        break_minutes: brk,
        late_minutes: late,
        early_leave_minutes: early,
        overtime_minutes: ot,
        overtime_approved: ot_appr,
        status,
        anomaly: anomaly.to_string(),
        source: format!("engine v1 / shift {} v{}", shift.code, shift.version),
    }
}

/// Kelompokkan raw events ke pasangan clock-in/out per work_date + shift.
/// Aturan: untuk shift malam, event antara 00:00–(end+limit) dikaitkan ke work_date-1.
/// Mengembalikan map work_date -> DayEvents.
pub fn group_events_for_shift(
    shift: &ShiftRule,
    work_date: &NaiveDate,
    mut events: Vec<NaiveDateTime>,
) -> DayEvents {
    let (earliest, latest_out) = shift.accept_bounds(work_date);
    events.sort();
    let in_win: Vec<NaiveDateTime> = events
        .into_iter()
        .filter(|t| *t >= earliest && *t <= latest_out)
        .collect();
    if in_win.is_empty() {
        return DayEvents {
            clock_in: None,
            clock_out: None,
            duplicate_count: 0,
        };
    }
    let ci = in_win[0];
    // Clock-out = event terakhir setelah clock-in; duplikat = event dalam 5 menit setelah ci.
    let dup = in_win
        .iter()
        .skip(1)
        .filter(|t| (**t - ci).num_minutes() < 5)
        .count() as i64;
    let co = if in_win.len() > 1 {
        Some(*in_win.last().unwrap())
    } else {
        None
    };
    // Jika co sama dengan ci (satu event), anggap incomplete.
    let co = match co {
        Some(c) if c == ci => None,
        x => x,
    };
    DayEvents {
        clock_in: Some(ci),
        clock_out: co,
        duplicate_count: dup,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn pagi() -> ShiftRule {
        ShiftRule::from_row(
            "PAGI",
            "Shift Pagi",
            "07:00",
            "15:00",
            0,
            60,
            10,
            120,
            180,
            240,
            0,
            60,
            "1,2,3,4,5",
            1,
        )
    }
    fn malam() -> ShiftRule {
        ShiftRule::from_row(
            "MALAM",
            "Shift Malam",
            "23:00",
            "07:00",
            1,
            60,
            10,
            120,
            180,
            240,
            0,
            60,
            "1,2,3,4,5",
            1,
        )
    }
    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }
    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn base(shift: ShiftRule, date: &str, ci: Option<&str>, co: Option<&str>) -> ComputeInput {
        ComputeInput {
            work_date: d(date),
            shift,
            events: DayEvents {
                clock_in: ci.map(dt),
                clock_out: co.map(dt),
                duplicate_count: 0,
            },
            is_holiday: false,
            is_day_off: false,
            on_leave: false,
            on_permission: false,
            rounding_minutes: 1,
            overtime_approved_minutes: 0,
            require_overtime_approval: true,
        }
    }

    #[test]
    fn pagi_tepat_waktu() {
        let o = compute_day(&base(
            pagi(),
            "2026-05-04",
            Some("2026-05-04 07:00:00"),
            Some("2026-05-04 15:00:00"),
        ));
        assert_eq!(o.status, AttendanceStatus::Present);
        assert_eq!(o.late_minutes, 0);
        assert_eq!(o.work_minutes, 480 - 60);
    }
    #[test]
    fn grace_period_10_menit() {
        let o = compute_day(&base(
            pagi(),
            "2026-05-04",
            Some("2026-05-04 07:10:00"),
            Some("2026-05-04 15:00:00"),
        ));
        assert_eq!(o.status, AttendanceStatus::Present);
        assert_eq!(o.late_minutes, 0);
    }
    #[test]
    fn terlambat_11_menit() {
        let o = compute_day(&base(
            pagi(),
            "2026-05-04",
            Some("2026-05-04 07:11:00"),
            Some("2026-05-04 15:00:00"),
        ));
        assert_eq!(o.status, AttendanceStatus::Late);
        assert_eq!(o.late_minutes, 11);
    }
    #[test]
    fn pulang_cepat() {
        let o = compute_day(&base(
            pagi(),
            "2026-05-04",
            Some("2026-05-04 07:00:00"),
            Some("2026-05-04 14:00:00"),
        ));
        assert_eq!(o.early_leave_minutes, 60);
    }
    #[test]
    fn malam_lintas_tengah_malam() {
        let o = compute_day(&base(
            malam(),
            "2026-05-04",
            Some("2026-05-04 23:00:00"),
            Some("2026-05-05 07:00:00"),
        ));
        assert_eq!(o.status, AttendanceStatus::Present);
        assert_eq!(o.work_minutes, 480 - 60);
        assert!(o.clock_out.unwrap().starts_with("2026-05-05"));
    }
    #[test]
    fn malam_terlambat_dan_clockout_besok() {
        let o = compute_day(&base(
            malam(),
            "2026-05-04",
            Some("2026-05-04 23:20:00"),
            Some("2026-05-05 07:30:00"),
        ));
        assert_eq!(o.status, AttendanceStatus::Late);
        assert_eq!(o.late_minutes, 20);
    }
    #[test]
    fn window_malam_milik_work_date_asal() {
        let s = malam();
        let (ws, we) = s.window(&d("2026-05-04"));
        assert_eq!(ws.format("%Y-%m-%d %H:%M").to_string(), "2026-05-04 23:00");
        assert_eq!(we.format("%Y-%m-%d %H:%M").to_string(), "2026-05-05 07:00");
    }
    #[test]
    fn grouping_malam_menarik_event_setelah_tengah_malam() {
        let s = malam();
        let ev = vec![dt("2026-05-04 23:05:00"), dt("2026-05-05 06:58:00")];
        let g = group_events_for_shift(&s, &d("2026-05-04"), ev);
        assert!(g.clock_in.is_some() && g.clock_out.is_some());
    }
    #[test]
    fn incomplete_tanpa_clockout() {
        let o = compute_day(&base(
            pagi(),
            "2026-05-04",
            Some("2026-05-04 07:00:00"),
            None,
        ));
        assert_eq!(o.status, AttendanceStatus::Incomplete);
    }
    #[test]
    fn lembur_butuh_approval() {
        let mut i = base(
            pagi(),
            "2026-05-04",
            Some("2026-05-04 07:00:00"),
            Some("2026-05-04 16:00:00"),
        );
        i.overtime_approved_minutes = 0;
        let o = compute_day(&i);
        assert_eq!(o.overtime_minutes, 0);
        assert!(!o.overtime_approved);
        i.overtime_approved_minutes = 60;
        let o2 = compute_day(&i);
        assert_eq!(o2.overtime_minutes, 60);
    }
    #[test]
    fn libur_tanpa_event() {
        let mut i = base(pagi(), "2026-05-04", None, None);
        i.is_holiday = true;
        assert_eq!(compute_day(&i).status, AttendanceStatus::Holiday);
    }
    #[test]
    fn cuti_menang_atas_event() {
        let mut i = base(
            pagi(),
            "2026-05-04",
            Some("2026-05-04 07:00:00"),
            Some("2026-05-04 15:00:00"),
        );
        i.on_leave = true;
        assert_eq!(compute_day(&i).status, AttendanceStatus::Leave);
    }
    #[test]
    fn shift_berdekatan_pagi_siang() {
        let siang = ShiftRule::from_row(
            "SIANG",
            "Shift Siang",
            "15:00",
            "23:00",
            0,
            60,
            10,
            120,
            180,
            240,
            0,
            60,
            "1,2,3,4,5",
            1,
        );
        let o = compute_day(&base(
            siang,
            "2026-05-04",
            Some("2026-05-04 15:00:00"),
            Some("2026-05-04 23:00:00"),
        ));
        assert_eq!(o.status, AttendanceStatus::Present);
    }
    #[test]
    fn pembulatan_dapat_dikonfigurasi() {
        let mut i = base(
            pagi(),
            "2026-05-04",
            Some("2026-05-04 07:00:00"),
            Some("2026-05-04 15:07:00"),
        );
        i.rounding_minutes = 15;
        let o = compute_day(&i);
        assert_eq!((o.work_minutes + o.break_minutes) % 15, 0);
    }
    #[test]
    fn pulang_saja_tanpa_clockin_incomplete() {
        let o = compute_day(&base(
            pagi(),
            "2026-05-04",
            None,
            Some("2026-05-04 15:00:00"),
        ));
        assert_eq!(o.status, AttendanceStatus::Incomplete);
        assert!(o.anomaly.contains("tidak lengkap"));
    }
    #[test]
    fn scan_ganda_menandai_anomali() {
        let mut i = base(
            pagi(),
            "2026-05-04",
            Some("2026-05-04 07:00:00"),
            Some("2026-05-04 15:00:00"),
        );
        i.events.duplicate_count = 2;
        let o = compute_day(&i);
        assert_eq!(o.status, AttendanceStatus::Present);
        assert!(o.anomaly.contains("ganda x3"));
    }
    #[test]
    fn clockout_sebelum_clockin_non_malam_perlu_review() {
        let o = compute_day(&base(
            pagi(),
            "2026-05-04",
            Some("2026-05-04 10:00:00"),
            Some("2026-05-04 09:00:00"),
        ));
        assert_eq!(o.status, AttendanceStatus::NeedsReview);
    }
    #[test]
    fn libur_dengan_event_tetap_dihitung() {
        let mut i = base(
            pagi(),
            "2026-05-04",
            Some("2026-05-04 07:00:00"),
            Some("2026-05-04 15:00:00"),
        );
        i.is_holiday = true;
        let o = compute_day(&i);
        assert_eq!(o.status, AttendanceStatus::Present);
        assert!(o.work_minutes > 0);
    }
}
