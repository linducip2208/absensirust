# Inventaris SDK (status jujur per 2026-10-14 — build 3)

> SDK nyata sudah **diunduh & diverifikasi file-nya** (`vendor_sdk/`, lihat
> `vendor_sdk/README.md`). Adapter nyata sudah **diimplementasikan**
> (`src-tauri/src/devices/`), tetapi status uji hardware tetap:
> **BELUM DIUJI pada perangkat fisik**. Jangan mengklaim kompatibilitas
> hardware kepada pelanggan sebelum uji `#[ignore]` lolos di perangkat nyata.

| Vendor | Model | Versi SDK | File tersedia | Arsitektur DLL | Lisensi | Operasi didukung (kode) | Status uji | Kekurangan |
|---|---|---|---|---|---|---|---|---|
| ZKTeco | terminal absensi (ZKEM) | 6.3.1.54 + PullSDK x64 6.3.1.37 | ✅ `vendor_sdk/` (SDK/x64+x86, manual PDF 267hlm, demo C#/VB) | x64 + x86 ✅ terverifikasi header PE | gratis utk customer; redistribusi installer **belum terverifikasi** | fetch_logs, fetch_users, provision_user, device_info (COM CZKEM) | BELUM DIUJI (hardware) | perangkat fisik untuk uji |
| Solution | X100-C, X105, X107, X304, X401, X601, X606-S, X609, X900 (ZKEM) | 6.3.1.54 (dari support Solution) | ✅ sama dengan ZKTeco (satu platform; `solution.co.id/sdk.php` = "ZKEM SDK") | x64 + x86 ✅ terverifikasi header PE | sama dengan ZKTeco | fetch_logs, fetch_users, provision_user, device_info | BELUM DIUJI (hardware) | perangkat fisik untuk uji |
| Hikvision | terminal akses/absensi | HCNetSDK 6.1.6.3 | ✅ `HCNetSDK.dll` x64 (842 ekspor) + `CHCNetSDK.cs` wrapper | x64 ✅ terverifikasi header PE | paket resmi butuh akun hikvision.com (DLL dev dari cermin) | device_info SAJA (login) | BELUM DIUJI (hardware) | dokumen ISAPI/ACS resmi utk event absensi + perangkat fisik |
| TIME TECH | AC6 | tidak ada | — | n/a | — | — (stub `Unsupported`) | N/A | **tidak ada SDK PC**: AC6 access-controller Wiegand-only menurut distributor resmi |
| SIMULATED | SIMULATED | mock-1.0.0 (SIMULATED) | n/a | n/a | n/a | fetch_logs, fetch_users, provision_user | LULUS (test otomatis) | **Bukan hardware nyata; hanya test/demo** |

## Fakta verifikasi (bukan klaim)

- `zkemkeeper.dll` = COM ActiveX (hanya ekspor `DllGetClassObject` dkk).
  CLSID `CZKEM` `{00853A19-BD51-419B-9269-2DABE57EB61F}` + 339 method `IZKEM`
  dibaca dari typelib di DLL x64 (`vendor_sdk/izkem_methods.txt`).
- Nama method + urutan parameter + arti `InOutMode` (0=Check-In,1=Check-Out,
  2=Break-Out,3=Break-In,4=OT-In,5=OT-Out) dari demo resmi + manual §5.2.1.
- `plcommpro.dll` (Pull C API) **sengaja tidak dipakai**: signature C tidak
  terdokumentasi di paket ini (dilarang mengarang).
- `NET_DVR_*` Hikvision dipakai persis deklarasi `CHCNetSDK.cs` (stdcall,
  ANSI, BOOL 4-byte). Build ini tanpa API event absensi → `fetch_logs` =
  `Unsupported` sampai ada dokumen ISAPI/ACS + hardware.
- TIME TECH AC6: halaman distributor resmi hanya mencantumkan interface
  Wiegand/Lock/Exit (tanpa TCP/IP & SDK) → stub jujur, bukan "TCP generik".

## Implementasi (build 3)

- `devices/zkcom.rs` — COM late-binding IDispatch tanpa dependensi baru
  (hand-rolled `ole32`/`oleaut32`, ABI stabil) + helper murni teruji
  (parse host/port/machine, `InOutMode`, record-id, checkpoint, label arsitektur PE).
- `devices/zkteco.rs` / `devices/solution.rs` — sesi COM self-contained per
  operasi (aman thread-pool), baca log NON-DESTRUKTIF, checkpoint = event_time
  maksimum, `vendor_record_id` deterministik untuk ingest idempotent.
- `devices/hikvision.rs` — `libloading` HCNetSDK (Init/Login_V30/Logout/
  Cleanup); kredensial via `config_json` (`port/username/password`, tak pernah di-log).
- `devices/timetech.rs` — stub `Unsupported` berpesan jelas.
- `sync::service::build_adapter(nama, config_json)` + 5 call-site di `lib.rs`
  kini meneruskan `config_json` (port/machine/kredensial per perangkat).
- Test hardware bergerbang `#[ignore]` (`zkcom_hw`, `zteco_hw`, `solution_hw`,
  `hik_hw`) — cara jalan: `vendor_sdk/README.md`.

## Sebelum klaim "supported" ke pelanggan (checklist, tetap berlaku)

1. Daftarkan DLL (`Register_SDK x64.bat` Administrator) di mesin uji.
2. Uji pada perangkat fisik: connect, health, fetch log (bandingkan dengan
   tampilan mesin), provision, cabut-kabel/timeout, cancellation.
3. Untuk Hikvision: lengkapi dokumen ISAPI/ACS + implementasikan fetch log.
4. Perbarui tabel di atas + `KNOWN_LIMITATIONS.md` dengan hasil uji nyata.

Catatan: dilarang memakai TCP generik / USB polling / impor file sebagai
pengganti SDK nyata tanpa dokumentasi vendor.
