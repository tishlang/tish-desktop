$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$Out = Join-Path $Root "dist\release\windows-msstore"
New-Item -ItemType Directory -Force -Path $Out | Out-Null
Write-Host "[package-msstore] draft — emit MSIX into $Out for Partner Center"
