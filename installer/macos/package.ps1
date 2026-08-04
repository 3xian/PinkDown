param(
    [Parameter(Mandatory = $true)]
    [string] $Binary,

    [Parameter(Mandatory = $true)]
    [string] $ArtifactName,

    [Parameter(Mandatory = $true)]
    [string] $Version
)

$ErrorActionPreference = 'Stop'

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
$artifact = Join-Path $dist $ArtifactName

if ([IO.Path]::GetExtension($ArtifactName) -ne '.zip' -or
    [IO.Path]::GetFileName($ArtifactName) -ne $ArtifactName) {
    throw 'ArtifactName must be a .zip filename without a directory.'
}

New-Item -ItemType Directory -Path $dist -Force | Out-Null
foreach ($path in @($bundle, $iconset)) {
    if (Test-Path -LiteralPath $path) {
        Remove-Item -LiteralPath $path -Recurse -Force
    }
}
if (Test-Path -LiteralPath $artifact) {
    Remove-Item -LiteralPath $artifact -Force
}

New-Item -ItemType Directory -Path $macOs, $resources, $iconset -Force | Out-Null
Copy-Item -LiteralPath $binaryPath -Destination (Join-Path $macOs 'pinkdown')
& chmod '+x' (Join-Path $macOs 'pinkdown')
Copy-Item -LiteralPath $plistSource -Destination (Join-Path $contents 'Info.plist')

$plist = Join-Path $contents 'Info.plist'
& /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $Version" $plist
& /usr/libexec/PlistBuddy -c "Set :CFBundleVersion $Version" $plist

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
    & sips -z $icon.Size $icon.Size $iconSource --out $output | Out-Null
}

& iconutil -c icns $iconset -o (Join-Path $resources 'PinkDown.icns')
& plutil -lint $plist | Out-Null
& ditto -c -k --sequesterRsrc --keepParent $bundle $artifact

if (!(Test-Path -LiteralPath $artifact)) {
    throw "macOS bundle archive was not created: $artifact"
}

Remove-Item -LiteralPath $bundle, $iconset -Recurse -Force
