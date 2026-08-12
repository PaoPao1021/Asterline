param(
    [string]$Version = "",

    [string]$TargetDirectory = "desktop\src-tauri\target\release",

    [string]$OutputDirectory = "dist",

    [string]$WebView2Bootstrapper = ""
)

$ErrorActionPreference = "Stop"

if (-not $Version) {
    $package = Get-Content "desktop\package.json" -Raw | ConvertFrom-Json
    $Version = $package.version
}

if ($Version -notmatch '^\d+\.\d+\.\d+([+-][0-9A-Za-z.-]+)?$') {
    throw "Invalid Desktop version: $Version"
}

$compiler = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue
if (-not $compiler) {
    $candidates = @(
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
    )
    $compilerPath = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $compilerPath) {
        throw "Inno Setup 6 was not found. Install it or add ISCC.exe to PATH."
    }
} else {
    $compilerPath = $compiler.Path
}

$sourceDirectory = (Resolve-Path $TargetDirectory).Path
$binary = Join-Path $sourceDirectory "asterline-desktop.exe"
if (-not (Test-Path $binary)) {
    throw "Missing release binary: $binary"
}

New-Item -ItemType Directory -Force $OutputDirectory | Out-Null
$outputDirectoryPath = (Resolve-Path $OutputDirectory).Path
$scriptPath = (Resolve-Path "packaging\windows\asterline-desktop.iss").Path

$downloadedBootstrapper = $false
if (-not $WebView2Bootstrapper) {
    $WebView2Bootstrapper = Join-Path ([IO.Path]::GetTempPath()) "MicrosoftEdgeWebview2Setup-$([Guid]::NewGuid().ToString('N')).exe"
    Invoke-WebRequest -Uri "https://go.microsoft.com/fwlink/p/?LinkId=2124703" -OutFile $WebView2Bootstrapper
    $downloadedBootstrapper = $true
}

$webView2BootstrapperPath = (Resolve-Path $WebView2Bootstrapper).Path
$signature = Get-AuthenticodeSignature $webView2BootstrapperPath
if ($signature.Status -ne "Valid" -or $signature.SignerCertificate.Subject -notmatch "Microsoft Corporation") {
    if ($downloadedBootstrapper) { Remove-Item -LiteralPath $webView2BootstrapperPath -Force }
    throw "The WebView2 bootstrapper does not have a valid Microsoft signature."
}

try {
    & $compilerPath `
        "/DMyAppVersion=$Version" `
        "/DSourceDir=$sourceDirectory" `
        "/DWebView2Bootstrapper=$webView2BootstrapperPath" `
        "/O$outputDirectoryPath" `
        $scriptPath
} finally {
    if ($downloadedBootstrapper -and (Test-Path $webView2BootstrapperPath)) {
        Remove-Item -LiteralPath $webView2BootstrapperPath -Force
    }
}

if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup failed with exit code $LASTEXITCODE."
}

$installer = Join-Path $outputDirectoryPath "asterline-desktop-$Version-x86_64-windows-setup.exe"
if (-not (Test-Path $installer)) {
    throw "Inno Setup did not produce the expected installer: $installer"
}

Write-Output $installer
