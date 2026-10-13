# Folder SDK vendor — BELUM ADA FILE ASLI

Folder ini SENGAJA kosong. File SDK adalah milik vendor resmi dan harus
diperoleh dari distributor/reseller mesin absensi Anda.

## Yang harus diminta per vendor (taruh di subfolder masing-masing)

### time-tech-ac6\ (TIME TECH AC6)
- DLL komunikasi + file pendamping (.lib/deskripsi fungsi)
- Dokumen protokol + contoh program + daftar model yang didukung
- Info lisensi redistribusi

### zkteco\ (ZKTeco)
- Pull SDK (mis. zkemkeeper.dll untuk model lama, atau SDK model baru)
- Dokumen SDK + contoh program + daftar model yang didukung
- Info lisensi redistribusi

### hikvision\ (Hikvision)
- HCNetSDK / SDK ISAPI (DLL + LIB + header) sesuai model terminal
- Dokumen + contoh program
- Info lisensi redistribusi

### solution\ (Solution)
- SDK + dokumen + contoh program sesuai model terminal
- Info lisensi redistribusi

## Setelah file didapat

1. Taruh file di subfolder yang sesuai (jangan ubah nama asli dari vendor).
2. Beri tahu pengembang: adapter nyata akan ditulis + diuji pada perangkat fisik.
3. Status di aplikasi & dokumen akan diperbarui dari BUTUH SDK/HARDWARE
   menjadi TERUJI hanya setelah pengujian hardware nyata.

## Larangan

- Jangan mengunduh DLL dari situs tidak resmi (risiko malware).
- Jangan mengklaim hardware kompatibel sebelum diuji.
