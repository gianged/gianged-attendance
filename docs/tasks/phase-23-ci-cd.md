# Phase 23: CI/CD with Release Please

## Objective

Set up GitHub Actions with Release Please to automate releases based on conventional commits.

---

## How It Works

1. Push commits with conventional prefixes (`feat:`, `fix:`, `chore:`, etc.)
2. Release Please automatically creates/updates a "Release PR"
3. When Release PR is merged, it:
   - Bumps version in Cargo.toml
   - Generates CHANGELOG.md
   - Creates GitHub Release with tag
   - Triggers build workflow to attach MSI

---

## Conventional Commit Prefixes

| Prefix | Version Bump | Example |
|--------|--------------|---------|
| `feat:` | Minor (0.1.0 → 0.2.0) | `feat: add employee search` |
| `fix:` | Patch (0.1.0 → 0.1.1) | `fix: correct date parsing` |
| `feat!:` or `BREAKING CHANGE:` | Major (0.1.0 → 1.0.0) | `feat!: new config format` |
| `chore:`, `docs:`, `refactor:` | No release | `chore: update deps` |

---

## Tasks

### 23.1 CI Workflow

**`.github/workflows/ci.yml`**

Runs checks on every PR:

```yaml
name: CI

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: cargo fmt --check

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Check
        run: cargo check
```

### 23.2 Release Please Workflow

**`.github/workflows/release-please.yml`**

```yaml
name: Release Please

on:
  push:
    branches: [main]

permissions:
  contents: write
  pull-requests: write

jobs:
  release-please:
    runs-on: ubuntu-latest
    outputs:
      release_created: ${{ steps.release.outputs.release_created }}
      tag_name: ${{ steps.release.outputs.tag_name }}
    steps:
      - uses: googleapis/release-please-action@v4
        id: release
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          release-type: rust
```

### 23.3 Build Workflow

**`.github/workflows/build.yml`**

Builds and uploads MSI when a release is published:

```yaml
name: Build

on:
  release:
    types: [published]
  workflow_dispatch:

jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install WiX
        run: |
          dotnet tool install --global wix --version 4.0.5
          wix extension add WixToolset.UI.wixext/4.0.5 -g

      - name: Install cargo-wix
        run: cargo install cargo-wix

      - name: Initialize WiX
        run: cargo wix init --force

      - name: Build MSI
        run: cargo wix --nocapture

      - name: Upload MSI to Release
        uses: softprops/action-gh-release@v2
        with:
          files: target/wix/*.msi
```

### 23.4 Release Please Config

**`release-please-config.json`**

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "release-type": "rust",
  "packages": {
    ".": {
      "changelog-path": "CHANGELOG.md",
      "bump-minor-pre-major": true,
      "bump-patch-for-minor-pre-major": true
    }
  }
}
```

**`.release-please-manifest.json`**

```json
{
  ".": "0.1.0"
}
```

---

## Workflow Diagram

```
Developer commits
       ↓
feat: add feature  ──→  Release Please creates/updates PR
fix: bug fix       ──→  "chore(main): release 0.2.0"
       ↓
Merge Release PR
       ↓
GitHub Release created (v0.2.0)
       ↓
Build workflow triggered
       ↓
MSI uploaded to release assets
```

---

## Deliverables

- [ ] `.github/workflows/ci.yml` - PR checks
- [ ] `.github/workflows/release-please.yml` - Release automation
- [ ] `.github/workflows/build.yml` - MSI build on release
- [ ] `release-please-config.json` - Configuration
- [ ] `.release-please-manifest.json` - Version tracking
- [ ] First successful release via Release Please
