# Phase 22: Build and Packaging

## Objective

Configure build process for Windows executable with icon and prepare for distribution.

---

## Tasks

### 22.1 Build Script

**`build.rs`**

```rust
fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "GiangEd Attendance");
        res.set("FileDescription", "Mini ERP for attendance management");
        res.set("LegalCopyright", "Copyright 2025");
        res.compile().expect("Failed to compile Windows resources");
    }
}
```

### 22.2 Update Cargo.toml

```toml
[package]
name = "gianged-attendance"
version = "0.1.0"
edition = "2021"
description = "Mini ERP desktop app for staff and attendance management"
authors = ["GiangEd"]
license = "MIT"

[build-dependencies]
winres = "0.1"

# Optimize release build
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

### 22.3 Create Icon

Create `assets/icon.ico`:
- Use any icon generator or design tool
- Should include multiple sizes: 16x16, 32x32, 48x48, 256x256
- Place in `assets/` directory

### 22.4 Build Commands

**Development build:**
```bash
cargo build
```

**Release build:**
```bash
cargo build --release
```

**Output location:**
- Debug: `target/debug/gianged-attendance.exe`
- Release: `target/release/gianged-attendance.exe` (~15-20 MB)

### 22.5 MSI Installer (cargo-wix)

**Prerequisites (one-time setup on Windows):**

```powershell
# Install WiX Toolset
winget install WixToolset.WiX

# Install cargo-wix
cargo install cargo-wix
```

**Initialize WiX configuration:**

```bash
cargo wix init
```

This creates `wix/main.wxs` and `wix/License.rtf`.

**Customize `wix/main.wxs`** to include additional files:
- `config.example.toml`
- `database.sql`

**Build MSI:**

```bash
cargo wix
```

Output: `target/wix/gianged-attendance-0.1.0-x86_64.msi`

**MSI includes:**
- Application executable with icon
- Start Menu shortcut
- Add/Remove Programs entry
- Clean uninstall support

### 22.6 Portable Distribution (Alternative)

For ZIP distribution without installer:

```
gianged-attendance-v0.1.0/
├── gianged-attendance.exe
├── config.example.toml
├── README.txt
└── database.sql
```

### 22.7 GitHub Actions (Optional)

**`.github/workflows/build.yml`**

```yaml
name: Build

on:
  push:
    branches: [main]
    tags: ['v*']
  pull_request:
    branches: [main]

jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build
        run: cargo build --release

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: gianged-attendance-windows
          path: target/release/gianged-attendance.exe

  release:
    needs: build-windows
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/v')
    steps:
      - name: Download artifacts
        uses: actions/download-artifact@v4

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            gianged-attendance-windows/gianged-attendance.exe
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### 22.8 Version Bump Script

**`scripts/bump-version.sh`**

```bash
#!/bin/bash
NEW_VERSION=$1

if [ -z "$NEW_VERSION" ]; then
    echo "Usage: ./bump-version.sh 0.2.0"
    exit 1
fi

# Update Cargo.toml
sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml

# Commit and tag
git add Cargo.toml
git commit -m "Bump version to $NEW_VERSION"
git tag "v$NEW_VERSION"

echo "Version bumped to $NEW_VERSION"
echo "Run 'git push && git push --tags' to publish"
```

---

## Deliverables

- [x] build.rs with Windows resources
- [x] Release profile optimization (Cargo.toml)
- [ ] Application icon (assets/icon.ico)
- [ ] WiX configuration (cargo wix init)
- [ ] MSI installer build
- [ ] GitHub Actions workflow (optional)
- [ ] Version bump script
- [ ] Successful release build
- [ ] Final executable size check (~15-20 MB)
