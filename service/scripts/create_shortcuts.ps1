$WshShell = New-Object -ComObject WScript.Shell
$url = 'http://127.0.0.1:{PORT}'
$iconPath = '{ICON}'

function Get-AppPathFromRegistry($exe) {
    try {
        $k = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Software\Microsoft\Windows\CurrentVersion\App Paths\$exe")
        if ($k) {
            $v = $k.GetValue("")
            if ($v -and (Test-Path $v)) { return $v }
        }
        $k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey("SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\$exe")
        if ($k) {
            $v = $k.GetValue("")
            if ($v -and (Test-Path $v)) { return $v }
        }
    }
    catch {}
    return $null
}

# Resolve Chrome path
$chromePath = Get-AppPathFromRegistry 'chrome.exe'
if (-not $chromePath) {
    $cands = @(
        "$env:LOCALAPPDATA\Google\Chrome\Application\chrome.exe",
        "$env:ProgramFiles\Google\Chrome\Application\chrome.exe",
        "$env:ProgramFiles(x86)\Google\Chrome\Application\chrome.exe"
    )
    foreach ($c in $cands) { if (Test-Path $c) { $chromePath = $c; break } }
}

# Resolve Edge path
$edgePath = Get-AppPathFromRegistry 'msedge.exe'
if (-not $edgePath) {
    $cands = @(
        "$env:LOCALAPPDATA\Microsoft\Edge\Application\msedge.exe",
        "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe",
        "$env:ProgramFiles(x86)\Microsoft\Edge\Application\msedge.exe"
    )
    foreach ($c in $cands) { if (Test-Path $c) { $edgePath = $c; break } }
}

# Resolve Brave path
$bravePath = Get-AppPathFromRegistry 'brave.exe'
if (-not $bravePath) {
    $cands = @(
        "$env:LOCALAPPDATA\BraveSoftware\Brave-Browser\Application\brave.exe",
        "$env:ProgramFiles\BraveSoftware\Brave-Browser\Application\brave.exe",
        "$env:ProgramFiles(x86)\BraveSoftware\Brave-Browser\Application\brave.exe"
    )
    foreach ($c in $cands) { if (Test-Path $c) { $bravePath = $c; break } }
}

# Determine which browser to use

$useAppMode = $false
if (Test-Path $edgePath) {
    $targetPath = $edgePath
    $arguments = "--app=`"$url`""
    $useAppMode = $true
}
elseif (Test-Path $chromePath) {
    $targetPath = $chromePath
    $arguments = "--app=`"$url`""
    $useAppMode = $true
}
elseif (Test-Path $bravePath) {
    $targetPath = $bravePath
    $arguments = "--app=`"$url`""
    $useAppMode = $true
}
else {
    # No app-mode browser found; we'll create .url InternetShortcuts instead
    $targetPath = $null
    $arguments = $null
}



# Resolve output paths
$startMenuLnk = '{START_MENU}'
$desktopLnk = '{DESKTOP}'
$startMenuUrl = [System.IO.Path]::ChangeExtension($startMenuLnk, '.url')
$desktopUrl = [System.IO.Path]::ChangeExtension($desktopLnk, '.url')

# Ensure parent directories exist
$dirs = @((Split-Path -Parent $startMenuLnk), (Split-Path -Parent $desktopLnk))
foreach ($d in $dirs) { if ($d -and -not (Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null } }

if ($useAppMode) {
    # Start Menu shortcut (.lnk, app-mode)
    $StartMenuShortcut = $WshShell.CreateShortcut($startMenuLnk)
    $StartMenuShortcut.TargetPath = $targetPath
    $StartMenuShortcut.Arguments = $arguments
    $StartMenuShortcut.WindowStyle = 1  # Normal
    $StartMenuShortcut.IconLocation = "$iconPath,0"
    $StartMenuShortcut.Description = 'Framework Control - Local service UI'
    $StartMenuShortcut.Save()

    # Desktop shortcut (.lnk, app-mode)
    $DesktopShortcut = $WshShell.CreateShortcut($desktopLnk)
    $DesktopShortcut.TargetPath = $targetPath
    $DesktopShortcut.Arguments = $arguments
    $DesktopShortcut.WindowStyle = 1  # Normal
    $DesktopShortcut.IconLocation = "$iconPath,0"
    $DesktopShortcut.Description = 'Framework Control - Local service UI'
    $DesktopShortcut.Save()

    Write-Host "Shortcuts created successfully using app-mode: $targetPath"
}
else {
    # Fallback: create Internet Shortcut (.url) which opens via the default browser reliably
    $urlContent = "[InternetShortcut]`nURL=$url`nIconFile=$iconPath`nIconIndex=0`n"
    Set-Content -Path $startMenuUrl -Value $urlContent -Encoding ASCII -Force
    Set-Content -Path $desktopUrl -Value $urlContent -Encoding ASCII -Force
    Write-Host "Internet shortcuts (.url) created for default browser"
}


