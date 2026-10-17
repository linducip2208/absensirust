# Keterbatasan yang Diketahui (jujur, per 2026-10-17 — build 10)

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
7. **Ekspor PDF** = writer hand-rolled multi-halaman A4 (judul, meta, header
   tabel berulang per halaman, nomor halaman, ringkasan; font standar
   Helvetica; transliterasi ASCII). Tata letak kompleks (logo, grafik,
   multi-kolom) belum ada.
8. ~~Kalender shift visual belum ada~~ → kalender mingguan + salin minggu +
   bulk dengan pratinjau + deteksi bentrok/kosong + tukar shift berpersetujuan
   (menu Jadwal & Kalender). Drag-and-drop dan rotasi otomatis lanjutan belum ada.
9. ~~Scheduler belum ada UI~~ → scheduler latar + sync semua perangkat
   (menu Pengaturan). Interval minimal 60 detik.
10. Wizard installer NSIS terbentuk tetapi instalasi di mesin bersih belum diuji.
11. i18n: label Indonesia saja (sesuai spesifikasi).
12. **Adapter COM (zkcom) bisa membocorkan alokasi BSTR pada jalur error** saat
    SDK gagal berulang-ulang (kebocoran memori kecil hanya di mesin yang
    mengalami error SDK beruntun; jalur sukses bebas bocor). Menitip dibereskan
    setelah uji hardware; tanpa perangkat, perbaikan tidak bisa diverifikasi
    sehingga tidak diklaim tuntas.
13. **Lembur sebelum shift (`overtime_before_minutes`) tersimpan sebagai
    kebijakan tetapi belum menambah lembur.** Mesin hanya menghitung lembur
    setelah jam selesai (dengan approval bila wajib). Kolom disiapkan agar
    UI/DB tak berubah saat fitur ini diaktifkan.
14. **Ekspor PDF transliterasi ASCII**: setiap karakter non-ASCII di luar tabel
    Latin (mis. '·', '•', simbol khusus) diganti '?'. Tes `pdf_indonesian_chars_survive`
    menuncatkannya; tata letak kaya (logo/grafik/multi-kolom) tetap belum.
15. **Backup adalah salinan level-file setelah `wal_checkpoint(TRUNCATE)`.**
    Penulisan di antara checkpoint dan salinan (hampir hanya mungkin dari
    tugas terjadwal internal) tidak ikut dalam salinan itu dan baru masuk
    backup berikutnya; restore menjalankan `integrity_check`.
16. **Mundur jam perangkat tidak terdeteksi oleh checkpoint.** Checkpoint
    memegang `event_time` maksimum per perangkat; bila jam terminal
    dimundurkan (atau salah set), log ber-timestamp lama dianggap baris
    lama dan dilewati senyap — enkripsi tidak ada yang bisa membedakan
    baris mundur vs bersejarah (butuh ID urut vendor yang andal; vendor
    tidak menjanjikan seragam). Baris lama tetap aman (tidak ganda); yang
    hilang hanya deteksi mundurnya. Administrasi: atur jam perangkat NTP/
    manual diperiksa berkala.
17. **Dua punch identik dalam riak <1 detik** (sama `device_user_id` +
    waktu + arah, mis. hardware memantul kirim dua kali) tetap ditulis
    dua baris `raw_events` (ID unik vendor). Tidak ada ganda di hasil:
    heuristik klaster/kouta arah menghitungnya `duplicate_count` dan
    clock-out tetap event ≥5 menit — hanya detail antrean yang memuat
    keduanya; arsitektur pembacaan log retry aman.
18. **Saldo cuti dihitung sepanjang sejarah** (`kuota − terpakai approved`
    semua tahun); belum ada aturan periode (mulai ulang per tahun,
    carryover, jatuh tempo). UI diberi label "sepanjang sejarah" agar
    angka tidak mengklaim saldo tahun-berjalan. Perhitungan berbasis
    periode menunggu kebijakan HR final.
