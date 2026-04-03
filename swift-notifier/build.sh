#!/bin/bash
# Build ClaudeNotifier.app bundle
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/build"
APP_DIR="$BUILD_DIR/ClaudeNotifier.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

# Clean
rm -rf "$BUILD_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

# Compile Swift
echo "Compiling ClaudeNotifier..."
swiftc \
    -O \
    -o "$MACOS_DIR/ClaudeNotifier" \
    "$SCRIPT_DIR/ClaudeNotifier.swift" \
    -framework Cocoa \
    -framework UserNotifications

# Create Info.plist (NSPrincipalClass is critical for notification icon)
cat > "$CONTENTS_DIR/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.claude-notify.desktop</string>
    <key>CFBundleName</key>
    <string>Claude Notify</string>
    <key>CFBundleDisplayName</key>
    <string>Claude Notify</string>
    <key>CFBundleExecutable</key>
    <string>ClaudeNotifier</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>LSBackgroundOnly</key>
    <false/>
    <key>LSUIElement</key>
    <true/>
    <key>NSUserNotificationAlertStyle</key>
    <string>banner</string>
</dict>
</plist>
PLIST

# Copy icon
ICON_SOURCE="$SCRIPT_DIR/claude_icon.icns"
if [ -f "$ICON_SOURCE" ]; then
    cp "$ICON_SOURCE" "$RESOURCES_DIR/AppIcon.icns"
    echo "Icon copied."
fi

# Ad-hoc code sign (required for UNUserNotificationCenter to trust the app)
echo "Code signing..."
codesign --force --deep --sign - "$APP_DIR" 2>&1

echo "Build complete: $APP_DIR"
echo "Binary size: $(du -h "$MACOS_DIR/ClaudeNotifier" | cut -f1)"
