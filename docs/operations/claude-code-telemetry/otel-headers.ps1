# Claude Code `otelHeadersHelper` for the colo SigNoz OTLP endpoint.
#
# Prints the Cloudflare Access service-token headers as one JSON object on
# stdout. Claude Code runs it at startup and every 29 minutes, so the secret
# never sits in a settings file. Runbook: docs/operations/claude-code-telemetry.md.
#
# Needs: the Azure CLI logged in with read access to the project Key Vault, and
# the vault's name in the user environment variable CIMMERIA_KEY_VAULT.
# On any failure it exits non-zero with nothing on stdout; Claude Code then
# exports without headers, the Access edge answers 403, and nothing else breaks.

$ErrorActionPreference = 'Stop'

$vault = $env:CIMMERIA_KEY_VAULT
if (-not $vault) {
    [Console]::Error.WriteLine('otel-headers: CIMMERIA_KEY_VAULT is not set')
    exit 1
}

function Get-VaultSecret([string] $name) {
    $value = az keyvault secret show --vault-name $vault --name $name --query value --output tsv 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $value) {
        throw "secret '$name' could not be read from the vault"
    }
    return $value.Trim()
}

try {
    $headers = [ordered]@{
        'CF-Access-Client-Id'     = Get-VaultSecret 'claude-code-otel-cf-client-id'
        'CF-Access-Client-Secret' = Get-VaultSecret 'claude-code-otel-cf-client-secret'
    }
} catch {
    [Console]::Error.WriteLine("otel-headers: $($_.Exception.Message)")
    exit 1
}

$headers | ConvertTo-Json -Compress
