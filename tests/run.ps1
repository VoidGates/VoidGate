# Runs the root suite: API contract, then the browser detector and the two languages.
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

cargo test
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

node --test frontend/product.test.ts
exit $LASTEXITCODE
