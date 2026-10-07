param(
    [Parameter(Mandatory=$true)][string]$ClientRoot,
    [Parameter(Mandatory=$true)][string]$InputMap,
    [Parameter(Mandatory=$true)][string]$OutputMap,
    [Parameter(Mandatory=$true)][string]$PreviousProbe,
    [string]$Patcher = 'B:\targets\Cimmeria\debug\upk_patch.exe'
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $InputMap -PathType Leaf)) { throw "Missing input map: $InputMap" }
if (-not (Test-Path -LiteralPath $PreviousProbe -PathType Leaf)) { throw "Missing previous probe: $PreviousProbe" }
if (Test-Path -LiteralPath $OutputMap) { throw "Output already exists: $OutputMap" }
if (-not (Test-Path -LiteralPath $Patcher -PathType Leaf)) { throw "Missing upk_patch: $Patcher" }

$agnos = Join-Path $ClientRoot 'SGWGame\CookedPC\Maps\Agnos\Agnos-00000005.umap'
$shell = Join-Path $ClientRoot 'SGWGame\CookedPC\Maps\Agnos\Agnos-0002fff9.umap'
$library = Join-Path $ClientRoot 'SGWGame\CookedPC\Maps\Agnos_Library\Agnos_Library-00000000.umap'
$sgc = Join-Path $ClientRoot 'SGWGame\CookedPC\Maps\SGC\SGC-00000002.umap'
$castleRing = Join-Path $ClientRoot 'SGWGame\CookedPC\Maps\Castle\Castle-00090004.umap'
$harset = Join-Path $ClientRoot 'SGWGame\CookedPC\Maps\Harset\Harset-00000000.umap'
$waterEffects = Join-Path $ClientRoot 'SGWGame\CookedPC\Maps\Agnos\Agnos-0002fffa.umap'
$electricEffects = Join-Path $ClientRoot 'SGWGame\CookedPC\Maps\Agnos\Agnos-0001fffc.umap'
$steps = @(
    @{ name='sky'; source=$library; root=307; at='0,2200,-13814' },
    @{ name='archive-elbow'; source=$shell; root=290; at='-2048,2220,-128'; yaw=0 },
    @{ name='index-entry-fourway'; source=$shell; root=265; at='0,2220,-128'; yaw=0 },
    @{ name='fabrication-elbow'; source=$shell; root=290; at='2048,2220,-128'; yaw=90 },
    @{ name='west-gallery-threeway'; source=$shell; root=375; at='-2048,4268,-128'; yaw=0 },
    @{ name='index-fourway'; source=$shell; root=265; at='0,4268,-128'; yaw=0 },
    @{ name='east-encounter-threeway'; source=$shell; root=375; at='2048,4268,-128'; yaw=180 },
    @{ name='north-west-gallery-elbow'; source=$shell; root=290; at='-2048,6316,-128'; yaw=270 },
    @{ name='north-observation-fourway'; source=$shell; root=265; at='0,6316,-128'; yaw=0 },
    @{ name='north-east-encounter-elbow'; source=$shell; root=290; at='2048,6316,-128'; yaw=180 },
    @{ name='observation-ramp'; source=$shell; root=376; at='0,8620,768'; yaw=180 },
    @{ name='mezzanine-entry'; source=$shell; root=375; at='0,10924,768'; yaw=0 },
    @{ name='mezzanine-east-south'; source=$shell; root=290; at='2048,10924,768'; yaw=90 },
    @{ name='mezzanine-east-north'; source=$shell; root=290; at='2048,12972,768'; yaw=180 },
    @{ name='mezzanine-west-north'; source=$shell; root=290; at='0,12972,768'; yaw=270 },
    @{ name='south-garden-slab'; source=$sgc; root=1132; at='1205,-1630,-132' },
    @{ name='transfer-pad-visual-rig'; source=$castleRing; root='77,433,434'; at='0,-2100,-108' },
    @{ name='garden-south-wall-left'; source=$agnos; root=125; at='-450,-2900,-132'; yaw=0 },
    @{ name='garden-south-wall-right'; source=$agnos; root=125; at='450,-2900,-132'; yaw=0 },
    @{ name='garden-west-wall-a'; source=$agnos; root=125; at='-950,-2600,-132'; yaw=90 },
    @{ name='garden-west-wall-b'; source=$agnos; root=125; at='-950,-1700,-132'; yaw=90 },
    @{ name='garden-west-wall-c'; source=$agnos; root=125; at='-950,-800,-132'; yaw=90 },
    @{ name='garden-west-wall-d'; source=$agnos; root=125; at='-950,100,-128'; yaw=90 },
    @{ name='garden-west-wall-e'; source=$agnos; root=125; at='-950,900,-128'; yaw=90 },
    @{ name='garden-east-wall-a'; source=$agnos; root=125; at='650,-2600,-132'; yaw=90 },
    @{ name='garden-east-wall-b'; source=$agnos; root=125; at='650,-1700,-132'; yaw=90 },
    @{ name='garden-east-wall-c'; source=$agnos; root=125; at='650,-800,-132'; yaw=90 },
    @{ name='garden-east-wall-d'; source=$agnos; root=125; at='650,100,-128'; yaw=90 },
    @{ name='garden-east-wall-e'; source=$agnos; root=125; at='650,900,-128'; yaw=90 },
    @{ name='gate-shoulder-west'; source=$agnos; root=125; at='-900,1120,-128'; yaw=0 },
    @{ name='gate-shoulder-east'; source=$agnos; root=125; at='900,1120,-128'; yaw=0 },
    @{ name='archive-shelf-a'; source=$agnos; root=156; at='-4500,2660,-128' },
    @{ name='archive-shelf-b'; source=$agnos; root=314; at='-3700,2660,-128' },
    @{ name='archive-stasis'; source=$agnos; root=145; at='-4150,1940,-128' },
    @{ name='fabrication-mainframe'; source=$agnos; root=209; at='4550,2560,-128' },
    @{ name='fabrication-console'; source=$agnos; root=275; at='3650,1930,-128' },
    @{ name='fabrication-bench'; source=$agnos; root=161; at='4300,1830,-128' },
    @{ name='fabrication-monitor'; source=$agnos; root=200; at='4460,2600,-128' },
    @{ name='fabrication-corner-light'; source=$agnos; root=186; at='3220,2700,600' },
    @{ name='fabrication-electric-spark'; source=$electricEffects; root=6; at='4550,2550,80' },
    @{ name='west-link-terminal'; source=$agnos; root=275; at='-2520,4680,-128' },
    @{ name='east-link-bench'; source=$agnos; root=161; at='2520,4650,-128' },
    @{ name='north-gallery-stasis'; source=$agnos; root=145; at='-2430,6580,-128' },
    @{ name='north-gallery-shelves'; source=$agnos; root=273; at='2430,6590,-128' },
    @{ name='observation-terminal'; source=$agnos; root=301; at='0,6700,-128' },
    @{ name='index-corner-light'; source=$agnos; root=187; at='0,4100,600' },
    @{ name='encounter-corner-light'; source=$agnos; root=189; at='1900,4300,600' },
    @{ name='archive-corner-light'; source=$agnos; root=190; at='-4100,2500,600' },
    @{ name='garden-bench'; source=$agnos; root=161; at='350,-1740,-128' },
    @{ name='garden-plant-west-a'; source=$harset; root=771; at='-720,-2450,-132' },
    @{ name='garden-plant-west-b'; source=$harset; root=773; at='-690,-1350,-132' },
    @{ name='garden-plant-east-a'; source=$harset; root=771; at='380,-2520,-132' },
    @{ name='garden-plant-east-b'; source=$harset; root=773; at='350,-1450,-132' },
    @{ name='garden-plant-court-a'; source=$harset; root=771; at='-700,-450,-128' },
    @{ name='garden-plant-court-b'; source=$harset; root=773; at='350,-250,-128' },
    @{ name='garden-waterfall-top'; source=$waterEffects; root=6; at='-760,-1450,300' },
    @{ name='garden-waterfall-base'; source=$waterEffects; root=4; at='-760,-1450,-120' },
    @{ name='mezzanine-console'; source=$agnos; root=301; at='0,11200,768' },
    @{ name='mezzanine-shelves'; source=$agnos; root=156; at='2500,13000,768' },
    @{ name='power-generator'; source=$PreviousProbe; root=111; at='500,4650,-130' },
    @{ name='gallery-cover-west-a'; source=$PreviousProbe; root=113; at='-2530,4130,-128' },
    @{ name='gallery-cover-west-b'; source=$PreviousProbe; root=113; at='-1550,4660,-128' },
    @{ name='gallery-cover-north-a'; source=$PreviousProbe; root=113; at='-2520,6100,-128' },
    @{ name='gallery-cover-north-b'; source=$PreviousProbe; root=113; at='-1580,6630,-128' },
    @{ name='encounter-cover-a'; source=$PreviousProbe; root=113; at='1560,4100,-128' },
    @{ name='encounter-cover-b'; source=$PreviousProbe; root=113; at='2540,6610,-128' }
)

$scratch = Join-Path ([IO.Path]::GetDirectoryName($OutputMap)) ([IO.Path]::GetFileNameWithoutExtension($OutputMap) + '.assembling.umap')
if (Test-Path -LiteralPath $scratch) { throw "Scratch output already exists: $scratch" }
$current = $InputMap
try {
    for ($i = 0; $i -lt $steps.Count; $i++) {
        $step = $steps[$i]
        $next = if ($i -eq $steps.Count - 1) { $OutputMap } else { "$scratch.$i" }
        Write-Host "[$($i + 1)/$($steps.Count)] $($step.name)"
        $argsForClone = @('clone-objects', $current, $step.source, $next, '--roots', $step.root, '--first-at', $step.at, '--strip-lightmaps')
        if ($step.ContainsKey('yaw')) { $argsForClone += @('--yaw-degrees', $step.yaw) }
        & $Patcher @argsForClone
        if ($LASTEXITCODE -ne 0) { throw "Clone failed: $($step.name)" }
        if ($current -like "$scratch.*") { Remove-Item -LiteralPath $current }
        $current = $next
    }
    & $Patcher audit-names $OutputMap
    if ($LASTEXITCODE -ne 0) { throw 'Property-name audit failed' }
} finally {
    if ($current -like "$scratch.*" -and (Test-Path -LiteralPath $current)) { Remove-Item -LiteralPath $current }
}
