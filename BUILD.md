# Runik Desktop — Build Guide

## Prerequisites

- Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Node.js 18+
- Tauri CLI: `npm install` (in runik-desktop/)
- create-dmg (Mac): `brew install create-dmg`
- ImageMagick (icons): `brew install imagemagick`

## Icons

Source: `claude-ia/marciano-runik-1024x1024_1.png` (1024x1024)

Regenerate all icons from source:

```bash
SRC="../claude-ia/marciano-runik-1024x1024_1.png"
ICONS="src-tauri/icons"

# PNG sizes
sips -z 32 32 "$SRC" --out "$ICONS/32x32.png"
sips -z 128 128 "$SRC" --out "$ICONS/128x128.png"
sips -z 256 256 "$SRC" --out "$ICONS/128x128@2x.png"
sips -z 1024 1024 "$SRC" --out "$ICONS/icon.png"

# Windows squares
for s in 30 44 71 89 107 142 150 284 310; do
  sips -z $s $s "$SRC" --out "$ICONS/Square${s}x${s}Logo.png"
done
sips -z 50 50 "$SRC" --out "$ICONS/StoreLogo.png"

# macOS .icns
ICONSET="/tmp/runik.iconset"
mkdir -p "$ICONSET"
sips -z 16 16 "$SRC" --out "$ICONSET/icon_16x16.png"
sips -z 32 32 "$SRC" --out "$ICONSET/icon_16x16@2x.png"
sips -z 32 32 "$SRC" --out "$ICONSET/icon_32x32.png"
sips -z 64 64 "$SRC" --out "$ICONSET/icon_32x32@2x.png"
sips -z 128 128 "$SRC" --out "$ICONSET/icon_128x128.png"
sips -z 256 256 "$SRC" --out "$ICONSET/icon_128x128@2x.png"
sips -z 256 256 "$SRC" --out "$ICONSET/icon_256x256.png"
sips -z 512 512 "$SRC" --out "$ICONSET/icon_256x256@2x.png"
sips -z 512 512 "$SRC" --out "$ICONSET/icon_512x512.png"
sips -z 1024 1024 "$SRC" --out "$ICONSET/icon_512x512@2x.png"
iconutil -c icns "$ICONSET" -o "$ICONS/icon.icns"

# Windows .ico
magick "$ICONS/icon.png" -define icon:auto-resize=256,128,64,48,32,16 "$ICONS/icon.ico"
```

## Build — Mac ARM (Apple Silicon)

```bash
cd runik-desktop

# 1. Compile
npm run tauri build

# 2. Sign the .app (ad-hoc, no Apple Developer cert)
codesign --force --deep --sign - "src-tauri/target/release/bundle/macos/Runik AI.app"

# 3. Create DMG with standard Mac layout
create-dmg \
  --volname "Runik AI" \
  --volicon "src-tauri/icons/icon.icns" \
  --window-pos 200 120 \
  --window-size 600 400 \
  --icon-size 100 \
  --icon "Runik AI.app" 150 190 \
  --app-drop-link 450 190 \
  --no-internet-enable \
  "Runik-AI-mac-arm.dmg" \
  "src-tauri/target/release/bundle/macos/Runik AI.app"
```

Output: `Runik-AI-mac-arm.dmg` (~3.7 MB)

## Build — Mac Intel (x86_64)

```bash
# 1. Add target (one time only)
rustup target add x86_64-apple-darwin

# 2. Compile
npm run tauri build -- --target x86_64-apple-darwin

# 3. Sign
codesign --force --deep --sign - "src-tauri/target/x86_64-apple-darwin/release/bundle/macos/Runik AI.app"

# 4. Create DMG
create-dmg \
  --volname "Runik AI" \
  --volicon "src-tauri/icons/icon.icns" \
  --window-pos 200 120 \
  --window-size 600 400 \
  --icon-size 100 \
  --icon "Runik AI.app" 150 190 \
  --app-drop-link 450 190 \
  --no-internet-enable \
  "Runik-AI-mac-intel.dmg" \
  "src-tauri/target/x86_64-apple-darwin/release/bundle/macos/Runik AI.app"
```

Output: `Runik-AI-mac-intel.dmg` (~3.7 MB)

## Build — Windows (requires Windows machine or CI)

Cannot cross-compile from Mac. Options:

### Option A: GitHub Actions (recommended)

Add `.github/workflows/build-windows.yml`:

```yaml
name: Build Windows
on: workflow_dispatch
jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - uses: dtolnay/rust-toolchain@stable
      - run: npm install
        working-directory: runik-desktop
      - run: npx tauri build
        working-directory: runik-desktop
      - uses: actions/upload-artifact@v4
        with:
          name: windows-installer
          path: runik-desktop/src-tauri/target/release/bundle/nsis/*.exe
```

Output: `.exe` installer (NSIS) + `.msi`

### Option B: Windows VM / Remote machine

```powershell
# Install Rust: https://rustup.rs
# Install Node.js 20+
cd runik-desktop
npm install
npx tauri build
# Output: src-tauri/target/release/bundle/nsis/Runik AI_0.3.0_x64-setup.exe
```

## Build — Linux (requires Linux machine or CI)

Cannot cross-compile from Mac. Options:

### Option A: GitHub Actions (recommended)

Add `.github/workflows/build-linux.yml`:

```yaml
name: Build Linux
on: workflow_dispatch
jobs:
  build:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - uses: dtolnay/rust-toolchain@stable
      - run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
      - run: npm install
        working-directory: runik-desktop
      - run: npx tauri build
        working-directory: runik-desktop
      - uses: actions/upload-artifact@v4
        with:
          name: linux-packages
          path: |
            runik-desktop/src-tauri/target/release/bundle/deb/*.deb
            runik-desktop/src-tauri/target/release/bundle/appimage/*.AppImage
```

Output: `.deb` + `.AppImage`

### Option B: Linux VM / VPS

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Install Node.js 20+

cd runik-desktop
npm install
npx tauri build
# Output: src-tauri/target/release/bundle/deb/*.deb
#         src-tauri/target/release/bundle/appimage/*.AppImage
```

## Deploy to VPS

```bash
# Upload DMGs
scp Runik-AI-mac-arm.dmg root@runikapp.com:/root/landing/downloads/runik-desktop-mac-arm.dmg
scp Runik-AI-mac-intel.dmg root@runikapp.com:/root/landing/downloads/runik-desktop-mac-intel.dmg

# Future Windows/Linux:
# scp Runik-AI-setup.exe root@runikapp.com:/root/landing/downloads/runik-desktop-win.exe
# scp Runik-AI.AppImage root@runikapp.com:/root/landing/downloads/runik-desktop-linux.AppImage
```

Download page: https://runikapp.com/desktop.html

## User install notes

### Mac (unsigned app)
After copying to Applications, users may need:
```bash
xattr -cr "/Applications/Runik AI.app"
```

### Proper signing (future)
To avoid Gatekeeper warnings, sign with an Apple Developer ID ($99/year):
```bash
codesign --force --deep --sign "Developer ID Application: Your Name (TEAMID)" "Runik AI.app"
# Then notarize:
xcrun notarytool submit "Runik-AI.dmg" --apple-id "you@email.com" --team-id "TEAMID" --password "app-specific-password"
```
