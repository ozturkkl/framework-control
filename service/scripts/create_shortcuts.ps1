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
    } catch {}
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

# Determine which browser to use
if (Test-Path $chromePath) {
    $targetPath = $chromePath
    $arguments = "--app=`"$url`""
} elseif (Test-Path $edgePath) {
    $targetPath = $edgePath
    $arguments = "--app=`"$url`""
} else {
    # Fallback to explorer (opens default browser without console flash)
    $targetPath = "$env:WINDIR\explorer.exe"
    $arguments = $url
}

# Start Menu shortcut
$StartMenuShortcut = $WshShell.CreateShortcut('{START_MENU}')
$StartMenuShortcut.TargetPath = $targetPath
$StartMenuShortcut.Arguments = $arguments
$StartMenuShortcut.WindowStyle = 1  # Normal
$StartMenuShortcut.IconLocation = "$iconPath,0"
$StartMenuShortcut.Description = 'Framework Control - Local service UI'
$StartMenuShortcut.Save()

# Desktop shortcut
$DesktopShortcut = $WshShell.CreateShortcut('{DESKTOP}')
$DesktopShortcut.TargetPath = $targetPath
$DesktopShortcut.Arguments = $arguments
$DesktopShortcut.WindowStyle = 1  # Normal
$DesktopShortcut.IconLocation = "$iconPath,0"
$DesktopShortcut.Description = 'Framework Control - Local service UI'
$DesktopShortcut.Save()

Write-Host "Shortcuts created successfully using: $targetPath"


