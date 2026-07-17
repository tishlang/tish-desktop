# Sign Windows binaries (Azure Trusted Signing or PFX). Draft — no-op without secrets.
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Write-Host "[sign-windows] draft — configure AZURE_* or WINDOWS_PFX secrets"
if (-not $env:WINDOWS_PFX -and -not $env:AZURE_CODE_SIGNING_ACCOUNT) {
  Write-Host "[sign-windows] no signing credentials — skip"
  exit 0
}
Write-Host "[sign-windows] TODO: invoke signtool / Azure Trusted Signing on dist/release/win32"
