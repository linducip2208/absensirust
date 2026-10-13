# Keamanan

## Autentikasi

- Password di-hash **Argon2id** (salt acak per user); tidak ada plaintext di DB/log.
- **Tanpa kredensial default.** Admin pertama dibuat lewat `setup_admin` yang
  hanya berhasil bila tabel `users` kosong.
- Sesi = token UUID acak, timeout **8 jam** (kedaluwarsa → masuk ulang =
  lock sesi otomatis). Tidak ada "ingat saya" permanen.
- Rate-limit login: 5x gagal per username → kunci 5 menit.

## Otorisasi (permission-based)

| Role | Hak |
|---|---|
| super_admin | semua (`*.*`) + kelola pengguna + backup/restore |
| hr_admin | karyawan, shift, absensi, cuti, laporan, koreksi |
| supervisor | lihat absensi, setujui cuti, lihat laporan/shift |
| operator_device | perangkat + sync + lihat absensi |
| viewer/auditor | lihat absensi + laporan |

Setiap command backend memanggil `check_permission`; UI hanya cerminan.
Percobaan tanpa hak → `Forbidden` + tercatat bila sensitif.

## Proteksi data

- Semua query memakai **parameter binding** (tidak ada interpolasi string
  dari input user ke SQL; nama tabel/kolom divalidasi lewat allow-list).
- Validasi input: NIK/nama/tanggal wajib, email format, waktu WIB ketat,
  alasan koreksi wajib, lampiran dibatasi tipe + ukuran.
- File: DB di direktori profil user (ACL OS); foto profil opsional di dalam
  direktori data (akses path dibatasi); tidak ada secret di frontend
  (bundle `dist/` tidak berisi kredensial).
- Tidak ada server jaringan yang dibuka; IPC Tauri lokal saja.
- Log tidak berisi kredensial SDK/DB; bundle diagnostik bebas rahasia.

## Audit

- `audit_log`: semua create/update/delete + setup admin + backup/restore.
- `corrections_audit`: old/new + pelaku + alasan + waktu WIB (append-only).
- `shift_history`: versi aturan + payload + pelaku.
- `device_connection_log`: test/sync ok/gagal (tanpa kredensial).
- Raw device event tidak dapat diubah/dihapus via UI mana pun.

## Batasan yang disadari

- Backup belum dienkripsi di rest (lihat KNOWN_LIMITATIONS — butuh key management
  yang aman sebelum klaim "terenkripsi").
- Lock sesi otomatis = kedaluwarsa token 8 jam (belum ada idle-timeout per-menit
  yang dapat dikonfigurasi — direncanakan).
