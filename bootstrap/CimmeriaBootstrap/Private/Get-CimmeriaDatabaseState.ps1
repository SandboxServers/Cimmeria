function Get-CimmeriaDatabaseState {
    <#
    .SYNOPSIS
        Reads what is loaded in the game database: schema, seed data and accounts.
    .DESCRIPTION
        SchemaExists is true when public.account exists (the player data this tool must
        keep). SeedComplete also needs the test account, shards and resources data.
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

    if ((& $query "SELECT 1 FROM information_schema.tables WHERE table_name='account'") -match '1') {
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
