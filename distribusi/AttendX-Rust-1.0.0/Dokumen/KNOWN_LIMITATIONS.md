# Keterbatasan yang Diketahui (jujur, per 2026-10-14)

1. **SDK vendor belum teruji.** TIME TECH AC6, ZKTeco, Hikvision, Solution:
   BUTUH file SDK + perangkat fisik. Aplikasi menyediakan interface, registry,
   dan mock SIMULATED — bukan koneksi nyata.
2. **Adapter DLL 32-bit** (bila ditemui) butuh helper process out-of-process
   yang belum ditulis (menunggu DLL nyata untuk diverifikasi).
3. **Event push-only** belum diimplementasikan (menunggu dokumentasi vendor).
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
