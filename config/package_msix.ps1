# Script de empaquetado y firma MSIX para "Limpiador de CSV"
# Desarrolladora: Victoria Sanjurjo
# Publisher ID: CN=CF220D35-0C1C-408C-BCB2-E85AA65BC95B

$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$releaseExe = Join-Path $projectRoot "src-tauri\target\release\limpiador-de-csv.exe"
$manifestPath = Join-Path $projectRoot "config\AppxManifest.xml"
$iconsDir = Join-Path $projectRoot "src-tauri\icons"
$outputsDir = Join-Path $projectRoot "outputs"
$stagingDir = Join-Path $projectRoot "src-tauri\target\msix_staging"

if (-not (Test-Path $releaseExe)) {
    throw "No se encontró el binario release en: $releaseExe. Ejecuta primero cargo build --release."
}

# Localizar makeappx.exe y signtool.exe del Windows SDK
$makeappx = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\makeappx.exe" | Sort-Object FullName -Descending | Select-Object -First 1
$signtool = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe" | Sort-Object FullName -Descending | Select-Object -First 1

if (-not $makeappx -or -not $signtool) {
    throw "No se encontraron makeappx.exe o signtool.exe en Windows Kits."
}

Write-Host "Usando MakeAppx: $($makeappx.FullName)"
Write-Host "Usando SignTool: $($signtool.FullName)"

# Preparar carpeta de staging para MSIX
if (Test-Path $stagingDir) {
    Remove-Item $stagingDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $stagingDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stagingDir "Assets") | Out-Null
New-Item -ItemType Directory -Force -Path $outputsDir | Out-Null

# Copiar ejecutable y AppxManifest.xml
Copy-Item $releaseExe (Join-Path $stagingDir "limpiador-de-csv.exe") -Force
Copy-Item $releaseExe (Join-Path $outputsDir "LimpiadorDeCSV.exe") -Force
Copy-Item $manifestPath (Join-Path $stagingDir "AppxManifest.xml") -Force

# Copiar recursos visuales requeridos por AppxManifest.xml
$requiredLogos = @(
    "StoreLogo.png",
    "Square44x44Logo.png",
    "Square71x71Logo.png",
    "Square150x150Logo.png",
    "Square310x310Logo.png",
    "Wide310x150Logo.png"
)
foreach ($logo in $requiredLogos) {
    Copy-Item (Join-Path $iconsDir $logo) (Join-Path $stagingDir "Assets\$logo") -Force
}

$msixPath = Join-Path $outputsDir "LimpiadorDeCSV_1.0.0.0_x64.msix"
if (Test-Path $msixPath) {
    Remove-Item $msixPath -Force
}

# Empaquetar el MSIX con makeappx.exe
& $makeappx.FullName pack /d $stagingDir /p $msixPath /o
if ($LASTEXITCODE -ne 0) {
    throw "makeappx.exe falló con código $LASTEXITCODE"
}

# Crear o reutilizar certificado autofirmado para CN=CF220D35-0C1C-408C-BCB2-E85AA65BC95B
$publisherSubject = "CN=CF220D35-0C1C-408C-BCB2-E85AA65BC95B"
$cert = Get-ChildItem Cert:\CurrentUser\My | Where-Object { $_.Subject -eq $publisherSubject } | Select-Object -First 1

if (-not $cert) {
    Write-Host "Generando certificado autofirmado para $publisherSubject..."
    $cert = New-SelfSignedCertificate `
        -Type Custom `
        -Subject $publisherSubject `
        -KeyUsage DigitalSignature `
        -FriendlyName "Victoria Sanjurjo - Limpiador de CSV MSIX Cert" `
        -CertStoreLocation "Cert:\CurrentUser\My" `
        -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
}

# Exportar certificado público (.cer) en outputs/ para facilitar su instalación si se desea probar el MSIX
$cerPath = Join-Path $outputsDir "VictoriaSanjurjo_LimpiadorDeCSV.cer"
Export-Certificate -Cert $cert -FilePath $cerPath -Force | Out-Null

# Firmar el paquete MSIX usando el huella digital (Thumbprint) en el almacén CurrentUser\My
& $signtool.FullName sign /fd SHA256 /sha1 $cert.Thumbprint /s My $msixPath
if ($LASTEXITCODE -ne 0) {
    throw "signtool.exe falló con código $LASTEXITCODE"
}

Write-Host "Paquete MSIX creado y firmado exitosamente en: $msixPath"

