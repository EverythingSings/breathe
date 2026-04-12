$ErrorActionPreference = "Stop"

Write-Host ":: clippy"
cargo clippy --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ":: build"
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ":: deploy"
Copy-Item target\release\breathe.exe ~\bin\breathe.exe

Write-Host ":: done"
