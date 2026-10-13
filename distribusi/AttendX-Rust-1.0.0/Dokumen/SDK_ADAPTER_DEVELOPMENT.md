# Panduan Pengembangan Adapter SDK

## 1. Kontrak

Implementasikan trait `DeviceAdapter` di file baru `src-tauri/src/devices/<vendor>.rs`:

```rust
impl DeviceAdapter for ZktecoAdapter { /* ... */ }
```

- `capabilities()` harus jujur: operasi tanpa dukungan SDK → `Capability::Unsupported`.
- `fetch_attendance_logs(checkpoint, limit, cancel)` mengembalikan
  `(Vec<DeviceLog>, checkpoint_baru)`; hormati `cancel` (cek berkala).
- `event_time` **wajib** string WIB `YYYY-MM-DD HH:MM:SS`.
- Simpan `vendor_record_id` asli bila SDK menyediakannya (untuk deduplikasi).
- Jangan log kredensial / PIN / template biometrik.

## 2. FFI DLL

- Gunakan `libloading` (tambahkan dependensi) + signature persis dokumentasi vendor
  (calling convention `stdcall` vs `cdecl` — salah = crash).
- Verifikasi arsitektur: `dumpbin /headers vendor.dll | findstr machine`.
- **DLL 32-bit + proses x64 = tidak bisa di-load langsung.** Solusi:
  buat helper EXE 32-bit terpisah yang memanggil DLL, berkomunikasi dengan
  aplikasi utama via named pipe / file JSON + checkpoint (pola out-of-process).
- Event push (bila SDK hanya push): implementasikan listener sesuai dokumen
  (jangan polling buta); tetap tulis ke `raw_events` lewat ingest yang sama.

## 3. Registrasi

- Daftarkan di `DeviceRegistry::entries()` + `sync::service::build_adapter()`
  (ganti fallback mock untuk `adapter_name` tersebut).
- Perbarui `docs/SDK_INVENTORY.md` (versi SDK, file, arsitektur, lisensi,
  operasi, status uji, kekurangan).
- Tulis test integrasi dengan feature-gate bila butuh hardware
  (`#[ignore]` + dokumen cara menjalankan).

## 4. Larangan

- Mengarang nama fungsi/DLL/protokol.
- Menampilkan adapter belum-teruji sebagai "Online hardware nyata".
- Memakai data biometrik palsu sebagai data perangkat.
