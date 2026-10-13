# Taruh DLL runtime SDK ke src-tauri/sdk/ (+ salin ke target/debug|release bila ada).
# Cara pakai:
#   powershell -ExecutionPolicy Bypass -File vendor_sdk/stage.ps1
# Hasil: src-tauri/sdk/{*.dll, HCNetSDKCom/, MANIFEST.json, README.txt}
# Aplikasi mencari DLL di: folder exe, folder exe/sdk, %APPDATA%\AttendX\sdk.
# PENTING LISENSI: folder ini untuk development/placement manual di mesin Anda.
# Membundel DLL ke installer untuk didistribusikan butuh izin vendor
# (belum terverifikasi) — lihat vendor_sdk/README.md sebelum mengaktifkan
# "resources" di tauri.conf.json.

$ErrorActionPreference = 'Stop'
$SdkRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $SdkRoot
$OutDir = Join-Path $ProjectRoot 'src-tauri/sdk'

function Get-PeArch($Path) {
    $fs = [System.IO.File]::OpenRead($Path)
    try {
        $br = New-Object System.IO.BinaryReader($fs)
        if ($br.ReadUInt16() -ne 0x5A4D) { return 'unknown' }          # MZ
        $fs.Seek(0x3C, [System.IO.SeekOrigin]::Begin) | Out-Null
        $e = $br.ReadUInt32()
        $fs.Seek($e, [System.IO.SeekOrigin]::Begin) | Out-Null
        if ($br.ReadUInt32() -ne 0x4550) { return 'unknown' }            # PE
        $m = $br.ReadUInt16()
        if ($m -eq 0x8664) { return 'x64' }
        if ($m -eq 0x14C) { return 'x86' }
        return 'unknown'
    } finally {
        $br.Close(); $fs.Close()
    }
}

$zkX64 = Join-Path $SdkRoot 'solution-6.3.1.54/Standalone SDK-6.3.1.54/SDK/x64'
if (!(Test-Path $zkX64)) { $zkX64 = Join-Path $SdkRoot 'zkteco-x64/lib' }
$hikBin = Join-Path $SdkRoot 'Hikvision-Set/bin'
if (!(Test-Path $zkX64)) { throw "SDK ZK x64 tidak ada. Jalankan vendor_sdk/download.ps1 dulu." }
if (!(Test-Path $hikBin)) { throw "Hikvision-Set tidak ada. Jalankan vendor_sdk/download.ps1 dulu." }

if (Test-Path $OutDir) { Remove-Item -LiteralPath $OutDir -Recurse -Force }
New-Item -ItemType Directory -Path $OutDir | Out-Null
New-Item -ItemType Directory -Path (Join-Path $OutDir 'HCNetSDKCom') | Out-Null

$manifest = @()
# ZK x64: seluruh DLL runtime (zkemkeeper + dependensi transport/usb/kripto)
Get-ChildItem -LiteralPath $zkX64 -Filter '*.dll' | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $OutDir -Force
    $arch = Get-PeArch (Join-Path $OutDir $_.Name)
    if ($arch -ne 'x64') { throw "$($_.Name): arsitektur $arch, butuh x64" }
    $manifest += @{ file = $_.Name; bytes = $_.Length; arch = $arch; sdk = 'ZKEM 6.3.1.54 x64' }
}
# Hikvision x64: inti + OpenSSL + plugin (HCNetSDKCom wajib di sebelah DLL)
@('HCNetSDK.dll', 'HCCore.dll', 'libeay32.dll', 'ssleay32.dll') | ForEach-Object {
    $src = Join-Path $hikBin $_
    if (!(Test-Path $src)) { throw "$_ tidak ada di $hikBin" }
    Copy-Item -LiteralPath $src -Destination $OutDir -Force
    $dst = Join-Path $OutDir $_
    $arch = Get-PeArch $dst
    if ($arch -ne 'x64') { throw "${_}: arsitektur $arch, butuh x64" }
    $len = (Get-Item -LiteralPath $dst).Length
    $manifest += @{ file = $_; bytes = $len; arch = $arch; sdk = 'HCNetSDK 6.1.6.3 x64' }
}
Get-ChildItem -LiteralPath (Join-Path $hikBin 'HCNetSDKCom') -Filter '*.dll' | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $OutDir 'HCNetSDKCom') -Force
    $manifest += @{ file = "HCNetSDKCom/$($_.Name)"; bytes = $_.Length; arch = (Get-PeArch $_.FullName); sdk = 'HCNetSDK 6.1.6.3 x64' }
}

@{ generated = (Get-Date).ToString('yyyy-MM-dd HH:mm:ss'); files = $manifest } |
    ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $OutDir 'MANIFEST.json') -Encoding UTF8

Set-Content -LiteralPath (Join-Path $OutDir 'README.txt') -Encoding UTF8 -Value @'
DLL runtime vendor (hasil vendor_sdk/stage.ps1). JANGAN commit ke git.
- zkemkeeper.dll dkk (x64): butuh registrasi sekali sebagai Administrator:
    regsvr32 zkemkeeper.dll   (dari folder ini / folder exe)
  atau jalankan Register_SDK x64.bat dari paket SDK.
- HCNetSDK.dll dkk (x64): cukup ditaruh di sebelah exe (tanpa registrasi).
Penempatan yang dikenali aplikasi: folder exe, folder exe/sdk, %APPDATA%\AttendX\sdk.
'@

# Salin juga ke target dev bila sudah pernah build (agar cargo run langsung pakai)
@('target/debug/sdk', 'target/release/sdk') | ForEach-Object {
    $t = Join-Path $ProjectRoot ("src-tauri/$_")
    $parent = Split-Path -Parent $t
    if (Test-Path $parent) {
        if (Test-Path $t) { Remove-Item -LiteralPath $t -Recurse -Force }
        Copy-Item -LiteralPath $OutDir -Destination $t -Recurse -Force
        Write-Host "  disalin ke $_"
    }
}

Write-Host "OK: $OutDir ($($manifest.Count) file, semua x64)"
Write-Host 'Berikutnya di mesin kasir: salin isi folder ini ke folder exe (atau exe/sdk),'
Write-Host 'lalu: regsvr32 zkemkeeper.dll  (Administrator, sekali saja)'
