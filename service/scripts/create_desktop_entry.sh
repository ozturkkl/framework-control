#!/bin/bash
set -e

# Detect actual user home when running as root (system service)
detect_user_home() {
    # Check if running via sudo
    if [ -n "$SUDO_USER" ]; then
        echo "/home/$SUDO_USER"
        return 0
    fi

    # Scan /run/user/ for active sessions with UID >= 1000
    if [ -d /run/user ]; then
        for dir in /run/user/*; do
            if [ -d "$dir" ]; then
                uid=$(basename "$dir")
                if [ "$uid" -ge 1000 ] 2>/dev/null; then
                    username=$(id -un "$uid" 2>/dev/null || echo "")
                    if [ -n "$username" ]; then
                        echo "/home/$username"
                        return 0
                    fi
                fi
            fi
        done
    fi

    # Fallback to HOME
    if [ -n "$HOME" ]; then
        echo "$HOME"
        return 0
    fi

    return 1
}

# Get actual user home
user_home=$(detect_user_home)
if [ -z "$user_home" ]; then
    echo "Error: Could not determine user home directory" >&2
    exit 1
fi

# Template variables
port="{PORT}"
icon_temp_path="{ICON}"

# Construct paths
url="http://127.0.0.1:$port"
desktop_file="$user_home/.local/share/applications/framework-control.desktop"
icon_dir="$user_home/.local/share/framework-control/assets"
icon_path="$icon_dir/framework-control.png"

# Ensure directories exist
mkdir -p "$(dirname "$desktop_file")"
mkdir -p "$icon_dir"

# Copy icon from temp location to user's home
if [ -f "$icon_temp_path" ]; then
    cp "$icon_temp_path" "$icon_path"
    rm -f "$icon_temp_path"
fi

# Create .desktop file content
cat > "$desktop_file" << EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Framework Control
Comment=Framework Control - Local service UI
Exec=xdg-open $url
Icon=$icon_path
Terminal=false
Categories=Utility;System;
StartupNotify=true
EOF

# Make it executable
chmod +x "$desktop_file"

echo "Desktop entry created successfully at: $desktop_file"
