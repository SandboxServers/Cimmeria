function Get-CimmeriaDatabaseState {
    <#
    .SYNOPSIS
        Reads what is loaded in the game database: schema, seed data and accounts.
    .DESCRIPTION
        SchemaExists is true when public.account exists (the player data this tool must
        keep). Throws when the database cannot be queried. SeedComplete also needs the test account, shards and resources data.
        HasAccounts is true when public.account has any row.
    .OUTPUTS
        Object with SchemaExists, SeedComplete and HasAccounts.
    #>
    param(
        [Parameter(Mandatory)][string]$PsqlExe,
        [Parameter(Mandatory)][int]$Port,
        [switch]$UseDocker,
        [string]$Database = 'sgw'
    )

    $hostArgs = if ($UseDocker) { @("-h", "localhost") } else { @() }
    $query = { param($sql) "$(& $PsqlExe @hostArgs -p $Port -U "w-testing" -d $Database -tAc $sql 2>$null)" }
    $state = [pscustomobject]@{ SchemaExists = $false; SeedComplete = $false; HasAccounts = $false }

    # Fail closed: a failed query (wrong role, server down) must not read as "no schema",
    # because -Force drops a database that looks empty. Only this first query is strict;
    # the seed checks below may fail on a half-loaded schema and then mean "incomplete".
    $accountTable = & $PsqlExe @hostArgs -p $Port -U "w-testing" -d $Database -tAc "SELECT to_regclass('public.account') IS NOT NULL" 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Cannot read the state of database '$Database' (psql exit $LASTEXITCODE): $("$accountTable".Trim())" }

    if ("$accountTable".Trim() -eq 't') {
        $state.SchemaExists = $true
        Write-Status "Schema exists (account table found)." "DarkGray"

        $testAccountCheck = & $query "SELECT 1 FROM account WHERE account_id = 2"
        $shardsCheck = & $query "SELECT 1 FROM shards WHERE shard_id = 1"
        $resourcesDataCheck = & $query "SELECT 1 FROM resources.worlds LIMIT 1"
        if (($testAccountCheck -match '1') -and ($shardsCheck -match '1') -and ($resourcesDataCheck -match '1')) {
            $state.SeedComplete = $true
            Write-Status "Seed data exists (test account, shards, resources data found)." "DarkGray"
        } else {
            Write-Status "Schema loaded but seed data appears incomplete." "Yellow"
        }
        $state.HasAccounts = [bool]((& $query "SELECT 1 FROM account LIMIT 1") -match '1')
    }
    return $state
}
