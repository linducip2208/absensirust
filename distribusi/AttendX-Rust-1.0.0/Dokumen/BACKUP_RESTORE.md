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

1. *Pilih file…* (pemilih file, hanya `.db`) — path bebas di luar folder
   backups **ditolak** (anti path traversal).
2. Aplikasi memvalidasi header `SQLite format 3` + `integrity_check`
   read-only — file rusak **ditolak** sebelum menyentuh DB aktif.
3. Checkpoint WAL + salinan pengaman `attendx.pre-restore-<waktu>.db` dibuat
   otomatis; WAL/SHM basi dibuang.
4. File backup disalin menjadi DB aktif → koneksi dibuka ulang + verifikasi
   integritas → **langsung aktif tanpa restart**.

## Kesehatan

- Menu *Periksa sekarang*: versi aplikasi, versi DB (`MAX(schema_migrations)`),
  `PRAGMA integrity_check` (harus `ok`), jam WIB saat ini.
- Bila integrity bukan `ok`: restore dari backup terakhir yang valid,
  lalu jalankan ulang health check. Jangan operasikan DB yang corrupt.

## Diagnostik untuk dukungan teknis

- Ekspor: hasil health check + laporan *error_sync* + log koneksi perangkat.
- Dilarang menyertakan: password/hash, token sesi, PIN, template biometrik,
  dan data pribadi yang tidak diperlukan.
