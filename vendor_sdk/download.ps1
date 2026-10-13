# Unduh langsung semua SDK vendor ke folder ini (idempoten: yang sudah ada dilewati).
# Cara pakai (Windows PowerShell 5.1+):
#   powershell -ExecutionPolicy Bypass -File vendor_sdk/download.ps1
# Butuh: git, curl.exe (bawaan Windows 10+).
# Catatan: link langsung MediaFire kedaluwarsa, jadi di-parse ulang dari halaman
# support setiap kali dijalankan. Bila halaman berubah, perbarui $SolutionPage.

$ErrorActionPreference = 'Stop'
$SdkRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

function Get-WithCurl($Url, $Out) {
    Write-Host "  -> $Url"
    & curl.exe -L -A 'Mozilla/5.0' -o $Out $Url
    if ($LASTEXITCODE -ne 0) { throw "curl gagal ($LASTEXITCODE): $Url" }
}

# 1. ZKTeco Standalone SDK resmi (x86, docs minim) ---------------------------
if (Test-Path (Join-Path $SdkRoot 'ZKTeco-Standalone-SDK')) {
    Write-Host '[1/4] ZKTeco-Standalone-SDK sudah ada, dilewati.'
} else {
    Write-Host '[1/4] clone ZKTeco/Standalone-SDK ...'
    & git clone --depth 1 https://github.com/ZKTeco/Standalone-SDK.git (Join-Path $SdkRoot 'ZKTeco-Standalone-SDK')
}

# 2. ZKTeco PullSDK x64 via NuGet --------------------------------------------
$nupkg = Join-Path $SdkRoot 'ZKTecoStandAlonePullSDKx64.6.3.1.37.nupkg'
$xa = Join-Path $SdkRoot 'zkteco-x64'
if ((Test-Path $nupkg) -and (Test-Path $xa)) {
    Write-Host '[2/4] ZKTeco x64 sudah ada, dilewati.'
} else {
    Write-Host '[2/4] unduh ZKTecoStandAlonePullSDKx64 6.3.1.37 ...'
    Get-WithCurl 'https://www.nuget.org/api/v2/package/ZKTecoStandAlonePullSDKx64/6.3.1.37' $nupkg
    Copy-Item -LiteralPath $nupkg -Destination (Join-Path $SdkRoot 'zkteco-x64.zip') -Force
    if (Test-Path $xa) { Remove-Item -LiteralPath $xa -Recurse -Force }
    Expand-Archive -LiteralPath (Join-Path $SdkRoot 'zkteco-x64.zip') -DestinationPath $xa -Force
    Write-Host '  ok: zkteco-x64/lib/zkemkeeper.dll'
}

# 3. Solution Standalone SDK 6.3.1.54 (x64+x86+manual+demo) --------------------
$solDir = Join-Path $SdkRoot 'solution-6.3.1.54'
$solZip = Join-Path $SdkRoot 'solution-6.3.1.54.zip'
if ((Test-Path $solDir) -and (Test-Path $solZip)) {
    Write-Host '[3/4] Solution 6.3.1.54 sudah ada, dilewati.'
} else {
    Write-Host '[3/4] unduh Solution Standalone SDK 6.3.1.54 ...'
    $SolutionPage = 'https://sol-download-center.blogspot.com/p/standalone-sdk-63154.html'
    $page = Join-Path $SdkRoot '.solution_page.html'
    Get-WithCurl $SolutionPage $page
    $html = Get-Content -LiteralPath $page -Raw
    $m = [regex]::Match($html, 'https://download\d*\.mediafire\.com[^"''\s]+')
    if (-not $m.Success) { throw 'link langsung MediaFire tidak ketemu; perbarui $SolutionPage' }
    $direct = $m.Value -replace '&amp;', '&'
    Write-Host "  direct: $($direct.Substring(0, 60))..."
    Get-WithCurl $direct $solZip
    $firstByte = [System.IO.File]::ReadAllBytes($solZip)[0]
    if ($firstByte -eq 60) { throw 'yang terunduh halaman HTML, bukan ZIP (link kedaluwarsa?)' }
    if (Test-Path $solDir) { Remove-Item -LiteralPath $solDir -Recurse -Force }
    Expand-Archive -LiteralPath $solZip -DestinationPath $solDir -Force
    Remove-Item -LiteralPath $page -Force -ErrorAction SilentlyContinue
    Write-Host '  ok: solution-6.3.1.54/.../SDK/x64 + doc + Demo'
}

# 4. Hikvision HCNetSDK (cermin dev) ------------------------------------------
if (Test-Path (Join-Path $SdkRoot 'Hikvision-Set')) {
    Write-Host '[4/4] Hikvision-Set sudah ada, dilewati.'
} else {
    Write-Host '[4/4] clone jussivirkkala/Hikvision-Set ...'
    & git clone --depth 1 https://github.com/jussivirkkala/Hikvision-Set.git (Join-Path $SdkRoot 'Hikvision-Set')
}

Write-Host ''
Write-Host 'Selesai. Lanjut staging DLL runtime:'
Write-Host '  powershell -ExecutionPolicy Bypass -File vendor_sdk/stage.ps1'
