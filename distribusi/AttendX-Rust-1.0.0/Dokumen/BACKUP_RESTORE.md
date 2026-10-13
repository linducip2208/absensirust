# Backup & Restore

Lokasi default: `%APPDATA%\AttendX\` → `attendx.db`, `backups\`.

## Backup

- **Manual**: menu Backup → *Buat backup sekarang* → file
  `backups\attendx-backup-YYYY-MM-DD-HH-MM-SS.db`, rotasi 10 file terakhir.
- File adalah salinan SQLite utuh (WAL ikut konsisten karena copy saat idle IPC).
- **Terjadwal**: jalankan `backup_create` berkala (otomatisasi OS-level,
  mis. Task Scheduler memanggil CLI pendamping bila tersedia) — untuk rilis ini
  gunakan backup manual rutin + rotasi otomatis.

## Restore

1. Isi path file backup → *Restore dari file*.
2. Aplikasi memvalidasi header `SQLite format 3` — file bukan-DB **ditolak**
   sebelum menyentuh DB aktif.
3. Salinan pengaman `attendx.pre-restore-<waktu>.db` dibuat otomatis.
4. File backup disalin menjadi DB aktif → **restart aplikasi**.

## Kesehatan

- Menu *Periksa sekarang*: versi aplikasi, versi DB (`MAX(schema_migrations)`),
  `PRAGMA integrity_check` (harus `ok`), jam WIB saat ini.
- Bila integrity bukan `ok`: restore dari backup terakhir yang valid,
  lalu jalankan ulang health check. Jangan operasikan DB yang corrupt.

## Diagnostik untuk dukungan teknis

- Ekspor: hasil health check + laporan *error_sync* + log koneksi perangkat.
- Dilarang menyertakan: password/hash, token sesi, PIN, template biometrik,
  dan data pribadi yang tidak diperlukan.
