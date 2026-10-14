# Keterbatasan yang Diketahui (jujur, per 2026-10-14 — build 5)

1. **Adapter vendor terimplementasi, BELUM teruji hardware.** ZKTeco &
   Solution (COM CZKEM: fetch log/user/provision/info) dan Hikvision
   (HCNetSDK: login/info) memakai SDK asli yang sudah diverifikasi file-nya
   (`vendor_sdk/`), tetapi butuh perangkat fisik untuk uji sebelum diklaim
   "supported". Tanpa DLL terdaftar + hardware, operasi mengembalikan error
   yang jelas (bukan data karangan). TIME TECH AC6 tetap stub `Unsupported`
   (access controller Wiegand-only, tanpa SDK PC dari vendor). Status DLL
   dapat dicek dari UI via `device_sdk_status`.
2. **Adapter Hikvision belum bisa tarik log absensi.** Build HCNetSDK yang
   tersedia tidak mengekspos API event absensi terminal; butuh dokumen
   ISAPI/ACS resmi + hardware. Status capability `Unsupported` (jujur, tampil di UI).
3. **Lisensi redistribusi DLL belum terverifikasi** (ZKTeco/Solution gratis
   untuk customer; Hikvision resmi butuh akun). Aplikasi TIDAK membundel DLL;
   pengguna menyalin DLL ke folder exe/`%APPDATA%\AttendX\sdk` + registrasi
   (lihat `vendor_sdk/README.md` + `INSTALL_WINDOWS.md`).
4. **Backup belum terenkripsi** di rest; kunci manajemen aman belum tersedia,
   maka klaim "terenkripsi" tidak dibuat.
5. ~~Idle lock per-menit~~ → timeout sesi dapat dikonfigurasi (default 8 jam),
   berlaku segera (menu Pengaturan).
6. ~~Lampiran & foto via path manual~~ → pemilih file + unggah bervalidasi
   (foto ≤2 MB, lampiran ≤5 MB) ke direktori data.
7. **Ekspor PDF** = writer minimal satu halaman teks (kini dengan judul,
   waktu pembuatan, dan jumlah baris); tata letak kompleks belum ada.
8. ~~Kalender shift visual belum ada~~ → kalender mingguan + salin minggu +
   bulk dengan pratinjau + deteksi bentrok/kosong + tukar shift berpersetujuan
   (menu Jadwal & Kalender). Drag-and-drop dan rotasi otomatis lanjutan belum ada.
9. ~~Scheduler belum ada UI~~ → scheduler latar + sync semua perangkat
   (menu Pengaturan). Interval minimal 60 detik.
10. Wizard installer NSIS terbentuk tetapi instalasi di mesin bersih belum diuji.
11. i18n: label Indonesia saja (sesuai spesifikasi).
