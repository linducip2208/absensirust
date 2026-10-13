# Mesin Shift & Aturan Waktu

## 1. Zona waktu

- Operasional: **Asia/Jakarta (WIB, UTC+7, tanpa DST)**.
- Penyimpanan: TEXT `YYYY-MM-DD HH:MM:SS` **waktu WIB** (kolom `event_time`,
  `clock_in`, `clock_out`, `computed_at`, dsb.).
- Tidak ada konversi UTC untuk tampilan. Fungsi `to_utc()` di `timex.rs` hanya
  untuk kebutuhan audit internal bila diperlukan.
- Test memakai fixture tanggal tetap — tidak bergantung jam sistem saat test.

## 2. Window shift

Untuk `work_date` dan aturan shift:

- `start = work_date + start_time`, `end = work_date + end_time`;
  bila `crosses_midnight` (atau `end <= start`) maka `end += 1 hari`.
- Contoh Malam 23:00–07:00 pada `work_date=2026-05-04`:
  window = `2026-05-04 23:00:00` … `2026-05-05 07:00:00`.
- Clock-out setelah tengah malam **tetap milik work_date asal**.
- Clock-in shift malam **tidak** mengambil event dari tanggal sebelumnya
  (batas awal penerimaan = `start − clockin_early_limit`).

## 3. Penerimaan event

- Clock-in diterima bila dalam `[start − early_limit, start + late_limit]`.
- Clock-out diterima hingga `end + clockout_limit`; selebihnya dicatat anomali
  tetapi tetap disimpan.
- Pengelompokan (`group_events_for_shift`): semua event dalam
  `[start − early_limit, end + out_limit]`; clock-in = paling awal,
  clock-out = paling akhir; event < 5 menit setelah clock-in dihitung duplikat.

## 4. Perhitungan harian (`compute_day`)

1. Prioritas status: Cuti → Izin → (Libur tanpa event = Holiday) →
   (Hari libur tanpa event = DayOff) → hitung event.
2. Tanpa event + tanpa cuti/izin/libur = **Absent** (pemanggil tidak memanggil
   untuk tanggal masa depan / karyawan tanpa jadwal).
3. Satu event saja = **Incomplete** + anomali "log tidak lengkap".
4. `late = (clock_in − start)` bila melebihi toleransi, else 0
   (grace period = toleransi).
5. `early = max(0, end − clock_out)` → anomali "pulang cepat".
6. `work = round_down(clock_out − clock_in, rounding) − break`.
7. Lembur sesudah = `max(0, clock_out − end)` dibatasi cap aturan;
   bila kebijakan mewajibkan approval, hanya yang disetujui yang dihitung
   (`overtime_approved`).
8. Status akhir: `Late` bila telat, else `Present`. Anomali digabung (`;`).

## 5. Aturan historis & hitung ulang

- Penugasan shift bertanggal efektif: `effective_date <= work_date`
  dan (`end_date IS NULL` atau `>= work_date`), ambil yang terbaru.
- Perubahan aturan shift menambah `version` + baris `shift_history`
  (payload JSON + pelaku + waktu).
- Hasil hitung menyimpan `shift_id + shift_version + source`
  (`engine v1 / shift <kode> v<n>`), sehingga audit tahu aturan mana yang dipakai.
- Hitung ulang = upsert per (employee, work_date); event mentah dan audit
  koreksi tidak terhapus.

## 6. Kasus yang diuji otomatis

Pagi tepat waktu, grace 10 menit, telat 11 menit, pulang cepat, malam lintas
tengah malam, malam telat + clock-out besok, kepemilikan window malam,
grouping malam menarik event pasca-tengah-malam, incomplete, lembur butuh
approval, libur, cuti menang atas event, shift berdekatan pagi/siang,
pembulatan terkonfigurasi. Lihat `TEST_REPORT.md`.
