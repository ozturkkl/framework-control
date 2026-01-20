#!/bin/bash
set -e

# Template variables (user_home passed from Rust)
user_home="{USER_HOME}"
port="{PORT}"

# Construct paths
url="http://127.0.0.1:$port"
desktop_file="$user_home/.local/share/applications/framework-control.desktop"
icon_dir="$user_home/.local/share/framework-control/assets"
icon_path="$icon_dir/framework-control.png"

# Ensure desktop applications directory exists
mkdir -p "$(dirname "$desktop_file")"

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
