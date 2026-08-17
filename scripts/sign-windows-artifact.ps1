param(
    [Parameter(Mandatory = $true)]
    [string]$ArtifactPath,

    [Parameter(Mandatory = $true)]
    [string]$CertificatePath,

    [Parameter(Mandatory = $true)]
    [string]$CertificatePassword,

    [Parameter(Mandatory = $true)]
    [string]$ExpectedSubject,

    [string]$TimestampUrl = "http://timestamp.digicert.com"
)

$ErrorActionPreference = "Stop"

$artifact = (Resolve-Path -LiteralPath $ArtifactPath).Path
$certificate = (Resolve-Path -LiteralPath $CertificatePath).Path

$signTool = Get-Command "signtool.exe" -ErrorAction SilentlyContinue
if ($signTool) {
    $signToolPath = $signTool.Path
} else {
    $kitsRoot = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
    $signToolPath = Get-ChildItem -LiteralPath $kitsRoot -Filter signtool.exe -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}

if (-not $signToolPath) {
    throw "signtool.exe was not found. Install the Windows SDK signing tools."
}

& $signToolPath sign /fd SHA256 /f $certificate /p $CertificatePassword /tr $TimestampUrl /td SHA256 $artifact
if ($LASTEXITCODE -ne 0) {
    throw "Authenticode signing failed for $artifact."
}

& $signToolPath verify /pa /all /v $artifact
if ($LASTEXITCODE -ne 0) {
    throw "Authenticode verification failed for $artifact."
}

$signature = Get-AuthenticodeSignature -FilePath $artifact
if ($signature.Status -ne "Valid") {
    throw "Windows reported an invalid Authenticode signature for ${artifact}: $($signature.StatusMessage)"
}

$actualSubject = $signature.SignerCertificate.Subject
if (-not [string]::Equals($actualSubject.Trim(), $ExpectedSubject.Trim(), [StringComparison]::OrdinalIgnoreCase)) {
    throw "Authenticode publisher mismatch for ${artifact}. Expected '$ExpectedSubject', got '$actualSubject'."
}

Write-Output $artifact
