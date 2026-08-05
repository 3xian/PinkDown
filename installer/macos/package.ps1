param(
    [Parameter(Mandatory = $true)]
    [string] $Binary,

    [Parameter(Mandatory = $true)]
    [string] $ArtifactName,

    [Parameter(Mandatory = $true)]
    [string] $Version
)

$ErrorActionPreference = 'Stop'

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Name,
        [Parameter(Mandatory = $true)]
        [scriptblock] $Command
    )
    & $Command
    if ($null -ne $LASTEXITCODE -and $LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$binaryPath = (Resolve-Path $Binary).Path
$iconSource = Join-Path $repoRoot 'assets/pinkdown-macos-icon.png'
$plistSource = Join-Path $PSScriptRoot 'Info.plist'
$dist = Join-Path $repoRoot 'dist'
$bundle = Join-Path $dist 'PinkDown.app'
$contents = Join-Path $bundle 'Contents'
$macOs = Join-Path $contents 'MacOS'
$resources = Join-Path $contents 'Resources'
$iconset = Join-Path $dist 'PinkDown.iconset'
$stage = Join-Path $dist 'dmg-stage'
$artifact = Join-Path $dist $ArtifactName
$applicationsLink = Join-Path $stage 'Applications'

if ([IO.Path]::GetExtension($ArtifactName) -ne '.dmg' -or
    [IO.Path]::GetFileName($ArtifactName) -ne $ArtifactName) {
    throw 'ArtifactName must be a .dmg filename without a directory.'
}

New-Item -ItemType Directory -Path $dist -Force | Out-Null
foreach ($path in @($bundle, $iconset, $stage)) {
    if (Test-Path -LiteralPath $path) {
        Remove-Item -LiteralPath $path -Recurse -Force
    }
}
if (Test-Path -LiteralPath $artifact) {
    Remove-Item -LiteralPath $artifact -Force
}

New-Item -ItemType Directory -Path $macOs, $resources, $iconset -Force | Out-Null
Copy-Item -LiteralPath $binaryPath -Destination (Join-Path $macOs 'pinkdown')
Invoke-Native -Name 'chmod' -Command { & chmod '+x' (Join-Path $macOs 'pinkdown') }
Copy-Item -LiteralPath $plistSource -Destination (Join-Path $contents 'Info.plist')

$plist = Join-Path $contents 'Info.plist'
Invoke-Native -Name 'PlistBuddy version' -Command {
    & /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $Version" $plist
}
Invoke-Native -Name 'PlistBuddy build' -Command {
    & /usr/libexec/PlistBuddy -c "Set :CFBundleVersion $Version" $plist
}

$iconSizes = @(
    @{ Name = 'icon_16x16.png'; Size = 16 },
    @{ Name = 'icon_16x16@2x.png'; Size = 32 },
    @{ Name = 'icon_32x32.png'; Size = 32 },
    @{ Name = 'icon_32x32@2x.png'; Size = 64 },
    @{ Name = 'icon_128x128.png'; Size = 128 },
    @{ Name = 'icon_128x128@2x.png'; Size = 256 },
    @{ Name = 'icon_256x256.png'; Size = 256 },
    @{ Name = 'icon_256x256@2x.png'; Size = 512 },
    @{ Name = 'icon_512x512.png'; Size = 512 },
    @{ Name = 'icon_512x512@2x.png'; Size = 1024 }
)

foreach ($icon in $iconSizes) {
    $output = Join-Path $iconset $icon.Name
    Invoke-Native -Name "sips $($icon.Name)" -Command {
        & sips -z $icon.Size $icon.Size $iconSource --out $output | Out-Null
    }
}

Invoke-Native -Name 'iconutil' -Command {
    & iconutil -c icns $iconset -o (Join-Path $resources 'PinkDown.icns')
}
Invoke-Native -Name 'plutil' -Command {
    & plutil -lint $plist | Out-Null
}

# Stage a drag-to-Applications disk image layout.
New-Item -ItemType Directory -Path $stage -Force | Out-Null
Copy-Item -LiteralPath $bundle -Destination (Join-Path $stage 'PinkDown.app') -Recurse
Invoke-Native -Name 'ln' -Command {
    & ln -s '/Applications' $applicationsLink
}
if (!(Test-Path -LiteralPath $applicationsLink)) {
    throw "Applications symlink was not created: $applicationsLink"
}

Invoke-Native -Name 'hdiutil create' -Command {
    & hdiutil create `
        -volname 'PinkDown' `
        -srcfolder $stage `
        -ov `
        -format UDZO `
        $artifact
}

if (!(Test-Path -LiteralPath $artifact)) {
    throw "macOS DMG was not created: $artifact"
}

Remove-Item -LiteralPath $bundle, $iconset, $stage -Recurse -Force
