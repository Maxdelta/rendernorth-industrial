param([switch]$SkipBuild)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Version = (Get-Content (Join-Path $Root "package.json") -Raw | ConvertFrom-Json).version
$Artifacts = Join-Path $Root "release-artifacts"
$Release = Join-Path $Root "src-tauri\target\release"

if (-not $SkipBuild) {
    Push-Location $Root
    try { npm run tauri build -- --bundles nsis }
    finally { Pop-Location }
}

$Executable = Join-Path $Release "rendernorth-industrial.exe"
if (-not (Test-Path -LiteralPath $Executable)) { throw "Portable executable not found: $Executable" }

New-Item -ItemType Directory -Force -Path $Artifacts | Out-Null
$PortableFolder = Join-Path $Artifacts "RenderNorth-Industrial-$Version-Windows-Portable"
if (Test-Path -LiteralPath $PortableFolder) { Remove-Item -LiteralPath $PortableFolder -Recurse -Force }
New-Item -ItemType Directory -Force -Path $PortableFolder | Out-Null
Copy-Item -LiteralPath $Executable -Destination $PortableFolder
Copy-Item -LiteralPath (Join-Path $Root "README.md") -Destination $PortableFolder
Copy-Item -LiteralPath (Join-Path $Root "docs\PRIVATE_BETA_ONBOARDING.md") -Destination $PortableFolder

$Zip = "$PortableFolder.zip"
if (Test-Path -LiteralPath $Zip) { Remove-Item -LiteralPath $Zip -Force }
Compress-Archive -Path (Join-Path $PortableFolder "*") -DestinationPath $Zip -CompressionLevel Optimal

$Installer = Get-ChildItem -Path (Join-Path $Release "bundle\nsis") -Filter "*.exe" | Select-Object -First 1
if (-not $Installer) { throw "NSIS installer was not produced." }

Write-Host "Windows installer: $($Installer.FullName)"
Write-Host "Portable ZIP: $Zip"
