# Manual Pengguna (Bahasa Indonesia)

Semua jam yang tampil adalah **WIB (Asia/Jakarta)**.

## 1. Masuk pertama kali

- Buka aplikasi → buat **admin pertama** (hanya sekali, saat belum ada pengguna).
- Masuk dengan username + password. Sesi 8 jam; setelah kedaluwarsa, masuk ulang.

## 2. Dashboard

- Pilih tanggal → angka hadir/terlambat/tidak hadir/cuti, status perangkat,
  grafik 7 hari, ringkasan shift, aktivitas terbaru, peringatan perangkat offline.
- Tombol **aksi cepat** menuju Karyawan, Perangkat, Shift, Laporan.

## 3. Karyawan

- **Tambah/Ubah**: NIK (unik, wajib), nama (wajib), email, telepon, alamat,
  tanggal masuk (wajib), departemen, jabatan, cabang, atasan, pola shift, status.
- **Hapus** = nonaktif (soft delete); riwayat audit tetap ada.
- **Mapping device**: daftarkan pasangan (karyawan ↔ ID mesin) karena ID di mesin
  bisa berbeda dari ID aplikasi. Tanpa mapping, log masuk daftar "tanpa mapping".
- **Foto profil**: pilih karyawan → *Pilih & unggah foto* (PNG/JPG, maks 2 MB);
  tersimpan di direktori data aplikasi.
- **Impor CSV**: tempel CSV → *Preview & validasi* → perbaiki yang merah →
  *Impor baris valid*. Laporan kesalahan tampil per baris.
- **Ekspor CSV**: unduh seluruh data tampil.

## 4. Cabang & Departemen

- Kelola cabang, departemen (terikat cabang), jabatan.

## 5. Shift & Jadwal

- Preset: **Pagi 07:00–15:00**, **Siang 15:00–23:00**, **Malam 23:00–07:00**
  (besok). Semua kolom dapat diubah: toleransi, batas clock-in/out, istirahat,
  lembur sebelum/sesudah, hari berlaku.
- **Penugasan**: satu baris = (karyawan, shift, tanggal efektif, tanggal akhir opsional).
  Untuk rotasi, buat baris baru dengan tanggal efektif berikutnya — jangan ubah
  baris lama (agar histori periode lalu utuh).
- Aturan shift berversi (`version`); hasil hitung mencatat versi yang dipakai.
- **Shift malam**: tanggal kerja = tanggal mulai (mis. lembur 23:00–07:00
  milik tanggal asal meski pulang keesokan harinya). Jam tak valid
  (mis. 25:00) ditolak saat simpan.
- **Tanggal akhir**: kosongkan untuk "berlaku terus"; isi untuk menutup
  penugasan (rotasi rapi tanpa bentrok).

## 5b. Jadwal & Kalender (menu Jadwal & Kalender)

- **Kalender mingguan**: matriks karyawan × 7 hari per kode shift;
  hari tanpa penugasan ditandai, navigasi mundur/maju per minggu.
- **Penugasan massal**: pilih shift + rentang tanggal + banyak karyawan →
  *Pratinjau dampak* dulu (menampilkan yang akan dibuat vs dilewati karena
  bentrok) → *Simpan penugasan*.
- **Salin minggu**: salin penugasan yang MULAI pada minggu tampil ke minggu
  tujuan (maju/mundur; dengan pratinjau; yang bentrok dilewati).
  Penugasan panjang yang sudah mencakup minggu tujuan tak perlu disalin.
- **Deteksi**: daftar shift tumpang tindih per karyawan + daftar hari kosong
  tanpa jadwal pada rentang tampil.
- **Tukar shift**: ajukan (karyawan, dari → ke, tanggal, alasan wajib) →
  setujui/tolak. Persetujuan membuat penugasan satu hari; histori lama utuh.

## 6. Absensi

- Pilih tanggal kerja → **Hitung ulang absensi tanggal ini**.
- Tabel hasil: masuk, pulang, menit kerja, telat, anomali, status.
- **Perlu review**: Incomplete / NeedsReview / Absent — selidiki sebelum menyimpulkan.
- **Log tanpa mapping**: petakan di Karyawan → Mapping device, lalu hitung ulang
  (data tidak hilang).
- **Koreksi**: hanya `clock_in`, `clock_out`, `status`; **alasan wajib**;
  nilai lama/baru + pelaku + waktu tercatat dan tidak bisa dihapus user biasa.
  Event mentah perangkat tidak tersentuh.

## 7. Perangkat

- Tambah perangkat: nama, merek, model, serial, IP/host, cabang, lokasi,
  tipe koneksi, **nama adapter** (pilih dari daftar: `mock-simulated` untuk
  uji; `zkteco-terminal`, `solution-terminal`, `hikvision-terminal` untuk
  mesin asli; `timetech-ac6` belum didukung).
- **Koneksi & kredensial (kolom config_json, format JSON)**:
  ZKTeco/Solution: `{"port":4370,"machine":1}`;
  Hikvision: `{"port":8000,"username":"admin","password":"..."}`.
- **Tabel "DLL vendor di mesin ini"**: pastikan status **Siap** sebelum test
  koneksi. Kalau Belum: salin DLL ke folder exe/`exe/sdk`/`%APPDATA%\AttendX\sdk`
  (ZK: registrasi `regsvr32 zkemkeeper.dll` sekali sebagai Administrator).
- **Test koneksi**: cek cepat + catat riwayat (berhasil/gagal + durasi).
- **Sync log**: tarik log absensi (incremental via checkpoint, idempotent).
- Registry adapter menampilkan status jujur per vendor + operasi yang
  didukung; mock selalu **SIMULATED** (jangan dipakai untuk produksi).

## 8. Izin, Cuti & Lembur

- Buat pengajuan (pending) → setujui/tolak di kolom status.
- Jenis cuti + kuota dapat dikonfigurasi. **Lampiran**: pilih pengajuan →
  *Pilih & unggah lampiran* (PDF/JPG/PNG, maks 5 MB).
- Laporan hanya menghitung pengajuan **approved**.

## 9. Laporan

- Pilih jenis → filter tanggal/cabang/departemen/shift/status/karyawan →
  **Tampilkan** → **Ekspor CSV/XLSX/PDF** (header jelas, tanggal WIB).
  PDF multi-halaman A4 (header berulang, nomor halaman, ringkasan).
- Jenis *rekon* membandingkan raw log vs hasil untuk audit.
- **Impor jadwal CSV** menerima header (`nik,nama,shift,efektif,akhir`) atau
  tanpa header (urutan `nik,shift,efektif,akhir`) — hasil *Ekspor CSV*
  bisa langsung diimpor kembali.

## 10. Kalender & Libur

- Daftarkan libur nasional/perusahaan; kalender kerja untuk hari khusus.

## 11. Backup & Kesehatan

- **Buat backup sekarang** → file `attendx-backup-<waktu>.db` (rotasi 10 file).
- **Restore**: *Pilih file…* (hanya dari folder backups) → validasi
  integritas dulu → DB aktif diganti (salinan pengaman `pre-restore-*`
  dibuat otomatis) → langsung aktif tanpa restart.
- **Periksa sekarang**: versi app, versi DB, `integrity_check`, jam WIB.

## 12. Pengguna & Audit
- Super Admin menambah pengguna + memilih role (5 role, permission di backend).
- Audit log hanya-baca dari UI.

## 13. Pengaturan (khusus Super Admin)

- **Interval sync otomatis**, **jumlah backup disimpan**, **timeout sesi**
  (berlaku segera, juga sebagai kunci sesi otomatis).
- **Scheduler**: *Aktifkan* agar semua perangkat tersinkron otomatis tiap
  interval; *Matikan* untuk kembali manual.
- **Sync semua sekarang**: sinkronisasi seluruh perangkat aktif sekaligus
  dengan ringkasan per perangkat.
