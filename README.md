# ProbeConfigurator

Cross-platform desktop app for managing SEGGER J-Link probes, built with **Tauri** (Rust backend) and **React + TypeScript** (frontend).

## Release and download

**Installers** are published under the repo’s **Releases** page on GitHub (open the repository, then **Releases** in the sidebar). New builds are produced when you push a version tag (details in **CI/CD** below). Each run creates a **draft** release — **Publish release** when you are ready for it to be public.

**Ship a version (maintainers):**

1. Ensure **`main`** is green in Actions (**CI** workflow).
2. Set the same **semver** in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`, commit, and push to `main` if you bumped the version.
3. Create and push the tag (example for first public `1.0.0`):
   ```bash
   git checkout main
   git pull
   git tag v1.0.0
   git push origin v1.0.0
   ```
4. Open **Actions** → **Release** and wait for all four builds (Windows, Linux, macOS Intel, macOS Apple Silicon).
5. Open **Releases** → the new **draft** → add notes → **Publish release**.

If the tag already exists, use a new version (e.g. `v1.0.1`) instead of re-pushing the same tag.

## Technology stack

| Area | Technologies |
|------|----------------|
| **Desktop shell** | [Tauri](https://tauri.app/) 2 — native window, system webview, Rust ↔ web IPC |
| **Frontend** | [React](https://react.dev/) 18, [TypeScript](https://www.typescriptlang.org/) 5 |
| **Frontend build** | [Vite](https://vitejs.dev/) 6, [@vitejs/plugin-react](https://github.com/vitejs/vite-plugin-react) |
| **UI styling** | [Tailwind CSS](https://tailwindcss.com/) 3, PostCSS, Autoprefixer |
| **Backend** | Rust (2021 edition) — `src-tauri` crate; async runtime: **Tokio** |
| **Tauri bridge** | `@tauri-apps/api` v2 — `invoke` (commands), `listen` (events), shell plugin |
| **State management** | [Zustand](https://zustand-demo.pmnd.rs/) 5 (`src/renderer/store/probeStore.ts`) |
| **Database** | *None* — no embedded DB, no local SQL; probe data is read from J-Link CLI output and held in memory |
| **Serialization / errors** | **serde** / **serde_json** (Rust), shared TS types in `src/shared/types.ts` |
| **HTTP / downloads** | **reqwest** (Rust) where used; SEGGER pages also driven via hidden **WebviewWindow** |
| **Logging** | **tauri-plugin-log**, Rust `log` crate |
| **Platform targets** | Windows (WebView2), macOS (WebKit), Linux (WebKitGTK), via Tauri + OS-specific Rust in `src-tauri/src/platform/` |
| **External tools** | SEGGER **J-Link Commander** (`JLink` / `JLinkExe`) for detect, scan, firmware, nickname, USB driver scripts |

## Features

- Detect J-Link installation and read version
- Download and install J-Link from SEGGER when the software is missing
- Scan connected probes (serial, product, nickname, firmware, USB driver mode)
- Update probe firmware and set or clear probe nickname
- **Switch the selected probe’s USB stack** between **WinUSB** and **SEGGER** driver modes (via J-Link Commander scripts — no separate Configurator app required)

## How the application works

1. **Startup** — `App.tsx` calls `checkInstallation()`, which invokes `detect_and_scan`. The Rust command resolves whether J-Link is installed (`jlink::detect`), stores the resolved `JLink`/`JLinkExe` path in app state, and if installed runs `scan_probes` once so the UI can show probes immediately.
2. **No J-Link** — If detection says not installed, the UI shows `InstallJLink.tsx`. That flow may invoke `scan_for_installer`, `download_jlink`, `install_jlink`, and related cancel commands; downloads use a hidden WebView plus platform-specific follow-up (e.g. Windows file polling).
3. **J-Link present** — `Dashboard.tsx` is shown. The user refreshes or selects a probe; actions call Tauri commands: `scan_probes`, `update_firmware`, `set_nickname`, and `switch_usb_driver` (frontend modes `winUsb` / `segger` map to WebUSB / WinUSB enable-disable scripts in `jlink::usb_driver` and `jlink::scripts`).
4. **State** — `probeStore` (Zustand) holds installation status, probe list, selection, and status for firmware, nickname, and USB driver operations. Errors from `invoke` surface in the store and UI.

## Development prerequisites

This project is a **Tauri 2** app. You need a working **Node.js** toolchain for the UI and a working **Rust** toolchain for the native shell. Follow the official checklist first, then install JS dependencies.

### Everyone (contributors building from source)

| Requirement | Notes |
|-------------|--------|
| **Node.js** | **LTS**, v20 or newer (matches `package.json` / CI expectations). [nodejs.org](https://nodejs.org/) |
| **Yarn** | **Classic Yarn v1** (`yarn --version` ≈ 1.22.x). Install: `npm install -g yarn` if needed. The repo uses `yarn.lock`. |
| **Rust** | **Stable** channel via [rustup](https://rustup.rs/). After install: `rustc --version` and `cargo --version` should work in the same terminal you use for development. |
| **Tauri CLI** | **`@tauri-apps/cli`** (devDependency). Scripts use **`yarn tauri dev`** / **`yarn tauri build`**, which run the local CLI after `yarn install`. Optionally install globally: `cargo install tauri-cli` and use `cargo tauri …` instead. |
| **SEGGER J-Link** | Not required to *compile* the app, but required to *use* probe features. The install screen can download J-Link on supported platforms. |

**Verify your environment** (from any directory):

```bash
node --version    # expect v20.x or newer
yarn --version    # expect 1.22.x
rustc --version
cargo --version
```

### Platform-specific (Tauri / WebView)

Install the system dependencies Tauri expects on your OS. The authoritative list is here:

**[https://tauri.app/start/prerequisites/](https://tauri.app/start/prerequisites/)**

Summary (always double-check the doc above for your exact OS version):

- **Windows** — **Microsoft C++ Build Tools** (MSVC) for Rust `*-pc-windows-msvc` targets; **WebView2** (Evergreen Runtime is usually already present on recent Windows 10/11).
- **macOS** — **Xcode Command Line Tools** (`xcode-select --install`).
- **Linux** — WebKitGTK and related packages (e.g. on Debian/Ubuntu families: `libwebkit2gtk`, `libgtk-3`, build essentials). Use Tauri’s Linux section for the current package list.

After OS deps are satisfied, you should be able to compile the crate in `src-tauri` without linker errors (e.g. `cd src-tauri && cargo check`).

---

## Build and run

### For end users (running the app)

- **Pre-built installers** — If releases are published (e.g. `.exe`, `.msi`, `.dmg`, `.AppImage`, `.deb`), prefer installing those. You do **not** need Node or Rust unless you build from source.
- **Runtime expectation** — The app manages SEGGER J-Link **software** (detect / optional download / install). You still need J-Link-compatible **hardware** when working with probes.
- **Permissions** — J-Link installation or driver changes may trigger **UAC** (Windows) or **administrator** prompts (macOS/Linux), depending on the installer and OS policy.

### For developers (this repository)

Work from the **repository root** (the folder that contains `package.json` and `src-tauri/`).

#### 1. Install JavaScript dependencies

```bash
yarn install
```

#### 2. Day-to-day development (full app)

```bash
yarn tauri:dev
```

What this does (see `src-tauri/tauri.conf.json`):

1. Runs **`yarn dev`** — starts the **Vite** dev server (default `http://localhost:5173/`).
2. Runs **`tauri dev`** (via `yarn tauri:dev`) — compiles the Rust crate and opens the desktop window pointed at that URL.

The **first** run can take several minutes while Cargo downloads and compiles dependencies; later runs are much faster.

**Important:** Use `yarn tauri:dev` whenever you exercise the backend (`invoke`, events, filesystem, J-Link CLI). **`yarn dev` alone** only serves the React app — **Tauri IPC is unavailable**, so startup and probe flows will fail or look broken.

#### 3. Frontend only (optional, limited)

```bash
yarn dev
```

Useful for quick UI/CSS passes when you do not need Rust — **not** enough for end-to-end probe or download testing.

#### 4. Production build (release-style output)

```bash
yarn tauri:build
```

This runs, in order:

1. **`yarn build`** — TypeScript + Vite production bundle (output is `out/renderer` at repo root, as referenced from `src-tauri/tauri.conf.json`).
2. **`tauri build`** (via `yarn tauri:build`) — release native binary and platform bundles (exact artifacts depend on OS and Tauri bundle settings).

Outputs land under **`src-tauri/target/release/`** plus installer/bundle files Tauri emits for your platform.

| Command | Typical use |
|--------|----------------|
| `yarn dev` | Vite only; no Tauri |
| `yarn tauri:dev` | **Recommended** full-stack development |
| `yarn build` | Frontend bundle only (also invoked automatically before `tauri build`) |
| `yarn tauri:build` | Shippable app / installers |

## Usage (high level)

- **First run without J-Link:** use the install screen to download and install SEGGER J-Link, then restart or let the app continue as detection succeeds.
- **Dashboard:** connect probes, select a row, then use refresh, firmware update, nickname, or **Switch to WinUSB** / **Switch to SEGGER** as needed. A replug may be required after a driver switch.

## Project structure

Omitted from the tree: **`node_modules/`**, **`src-tauri/target/`** (Cargo artifacts), and other generated or ignored paths from `.gitignore`.

```text
.
├── .github/
│   └── workflows/              # CI (main/PR) + release (version tags)
├── scripts/
│   └── gen_icon_png.py         # Optional: generate a 1024² PNG for `tauri icon`
├── index.html                  # Vite HTML entry
├── package.json
├── yarn.lock
├── .yarnrc.yml
├── vite.config.ts
├── tsconfig.json
├── tsconfig.node.json
├── tailwind.config.js
├── postcss.config.js
├── LICENSE
├── README.md
├── .gitignore
│
├── src/
│   ├── shared/
│   │   └── types.ts            # Shared TS types, COMMANDS, EVENTS
│   └── renderer/
│       ├── main.tsx            # React DOM root
│       ├── App.tsx             # checking → InstallJLink | Dashboard
│       ├── styles.css
│       ├── assets/
│       │   └── index.css       # Tailwind / base styles entry
│       ├── components/
│       │   └── ProbeTable.tsx
│       ├── pages/
│       │   ├── Dashboard.tsx
│       │   └── InstallJLink.tsx
│       └── store/
│           └── probeStore.ts   # Zustand + invoke / listeners
│
└── src-tauri/
    ├── Cargo.toml
    ├── Cargo.lock
    ├── build.rs                # Tauri build hook
    ├── tauri.conf.json         # App id, windows, bundle
    ├── capabilities/
    │   └── default.json        # Tauri 2 capability / permission config
    ├── .gitignore
    └── src/
        ├── main.rs             # Binary entry (calls lib)
        ├── lib.rs              # Tauri builder, plugins, invoke_handler
        ├── error.rs            # AppError / AppResult
        ├── state.rs            # JLinkState — cached J-Link executable path
        │
        ├── commands/
        │   ├── mod.rs
        │   ├── probe.rs        # detect_and_scan, scan_probes, switch_usb_driver, …
        │   └── download.rs     # scan_for_installer, download_jlink, install_jlink, …
        │
        ├── download/
        │   ├── mod.rs
        │   ├── types.rs        # DownloadConfig, progress DTOs, scan results
        │   ├── webview.rs      # Hidden WebviewWindow SEGGER flow
        │   ├── poll.rs         # Windows: poll .tmp until stable → rename
        │   └── installer.rs    # Run platform installers (elevated where needed)
        │
        ├── jlink/
        │   ├── mod.rs
        │   ├── types.rs        # Probe, CLI result types
        │   ├── detect.rs       # Locate J-Link installation
        │   ├── scan.rs         # Enumerate probes via Commander
        │   ├── runner.rs       # Spawn J-Link, parse version banner
        │   ├── scripts.rs      # Commander script strings (firmware, nickname, USB)
        │   ├── firmware.rs     # Firmware update session
        │   ├── nickname.rs     # Set/clear nickname
        │   └── usb_driver.rs   # WinUSB ↔ SEGGER driver switch
        │
        └── platform/
            ├── mod.rs          # platform::config() — jlink_bin + search dirs
            ├── windows.rs
            ├── macos.rs
            └── linux.rs
```

## Architecture notes

- The UI talks to Rust only through **`invoke`** with names from `COMMANDS` in `src/shared/types.ts`.
- Long-running work (CLI, file IO) runs in **`spawn_blocking`** from async command handlers.
- Downloader progress and completion are signaled with events such as `download://progress`, `download://completed`, and `download://cancelled` (consumed in `InstallJLink.tsx`).

## CI/CD (GitHub Actions)

Workflows live under [`.github/workflows/`](.github/workflows/).

| Workflow | When it runs | What it does |
|----------|----------------|--------------|
| **`ci.yml`** | Push or pull request to **`main`** | Builds the frontend and runs **`yarn tauri:build`** on **Ubuntu 22.04**, **Windows**, and **macOS**. Uploads **`src-tauri/target/release/bundle/`** as a workflow artifact per OS (installers vary by platform). |
| **`release.yml`** | Push a **Git tag** matching **`v*`** (e.g. `v1.0.1`) | Uses **[`tauri-action`](https://github.com/tauri-apps/tauri-action)** to build (including macOS **x64** and **ARM** targets) and attach binaries to a **draft** GitHub Release named after the tag. |

**Repository setting (required for releases):** GitHub → **Settings** → **Actions** → **General** → **Workflow permissions** → enable **Read and write permissions**. Without this, `release.yml` may fail when creating the release or uploading assets.

Step-by-step tagging and publishing are in **[Release and download](#release-and-download)** above. After the workflow finishes, installers appear under the draft release **Assets** (e.g. `.exe`/`.msi`, `.dmg`, `.deb`/`.AppImage`, depending on platform). Unsigned builds may trigger SmartScreen or Gatekeeper warnings until you add code signing.

**Version bumps (next releases):** keep the same semver in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` (no `v` prefix), commit on `main`, then push a **new** tag (e.g. `v1.0.1`).

Icons for bundling are under **`src-tauri/icons/`**. To regenerate from a 1024×1024 source PNG later: place it at `resources/icon-source.png` (un-ignore that path in `.gitignore` if needed), then from `src-tauri/` run `npx @tauri-apps/cli icon ../resources/icon-source.png`. The repo includes **`scripts/gen_icon_png.py`** as a fallback to generate a solid-color 1024×1024 PNG for that step.

## Troubleshooting

- **Blank or stuck “Checking J-Link…”** — Ensure you run **`yarn tauri:dev`**, not only `yarn dev`, so `detect_and_scan` can run.
- **Probes not listed** — Confirm the probe is USB-connected, J-Link software is detected, then use refresh.
- **Driver switch seems unchanged** — Unplug and replug the probe; the backend sends J-Link script sequences that may require a short reboot.

## Known limitations

- macOS/Linux install and driver flows have less field testing than Windows.
- Code signing and auto-update are not configured.

## License

MIT
