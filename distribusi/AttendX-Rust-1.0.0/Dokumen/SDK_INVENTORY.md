# Inventaris SDK (status jujur per 2026-10-13)

> **Tidak ada SDK vendor yang tersedia/diuji.** Semua baris di bawah berstatus
> BUTUH SDK/HARDWARE. Jangan mengklaim kompatibilitas hardware kepada pelanggan.

| Vendor | Model | Versi SDK | File tersedia | Arsitektur DLL | Lisensi | Operasi didukung | Status uji | Kekurangan |
|---|---|---|---|---|---|---|---|---|
| TIME TECH | AC6 | belum tersedia | — | belum diverifikasi | belum diketahui | — | BELUM DIUJI | file SDK, dokumen, contoh program, DLL, perangkat fisik |
| ZKTeco | terminal absensi | belum tersedia | — | belum diverifikasi | belum diketahui | — | BELUM DIUJI | file SDK, dokumen, contoh program, DLL, perangkat fisik |
| Hikvision | terminal akses/absensi | belum tersedia | — | belum diverifikasi | belum diketahui | — | BELUM DIUJI | file SDK, dokumen, contoh program, DLL, perangkat fisik |
| Solution | terminal absensi | belum tersedia | — | belum diverifikasi | belum diketahui | — | BELUM DIUJI | file SDK, dokumen, contoh program, DLL, perangkat fisik |
| SIMULATED | SIMULATED | mock-1.0.0 (SIMULATED) | n/a | n/a | n/a | fetch_logs, fetch_users, provision_user | LULUS (test otomatis) | **Bukan hardware nyata; hanya test/demo** |

## Yang sudah disiapkan (tanpa mengarang SDK)

- Kontrak trait `DeviceAdapter` (`src-tauri/src/devices/adapter.rs`):
  initialize, capabilities, connect, disconnect, health_check,
  fetch_attendance_logs, fetch_device_users, provision_user,
  fetch_device_info, get_sdk_version — plus timeout, error mapping,
  logging (connection_log), cancellation (`AtomicBool`).
- Registry (`devices/registry.rs`) + layar Devices → tabel registry.
- Mock adapter (`devices/mock.rs`): checkpoint/pagination/cancel/gagal-simulasi,
  selalu `SIMULATED`.
- Layanan sync: checkpoint per perangkat, retry + backoff, idempotent ingest,
  deduplikasi `(device, vendor_record_id, event_time, device_user)`.

## Sebelum adapter nyata ditulis (checklist)

1. Kumpulkan: installer SDK, `.dll`/`.lib`, header/dokumentasi, contoh program,
   daftar model yang didukung, lisensi redistribusi.
2. Periksa arsitektur DLL (`dumpbin /headers` → `machine x86` vs `x64`).
   Bila 32-bit sementara aplikasi x64 → rancang **helper process 32-bit + IPC**,
   jangan force-load.
3. Petakan operasi SDK → capability flags; yang tidak didukung = `Unsupported`.
4. Uji pada perangkat fisik: connect, health, fetch log (bandingkan dengan
   tampilan mesin), provision (bila didukung), cabut-kabel/timeout, cancellation.
5. Perbarui tabel di atas + `KNOWN_LIMITATIONS.md` dengan hasil uji nyata.

Catatan: dilarang memakai TCP generik / Wiegand / USB polling / impor file
sebagai pengganti SDK nyata tanpa dokumentasi vendor.
