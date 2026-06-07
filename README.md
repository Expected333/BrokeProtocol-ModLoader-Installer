# ModLoader Setup — BROKE PROTOCOL

Graphical installer (single self-contained `.exe`) that injects **ModLoader**
and **Harmony** into a BROKE PROTOCOL installation.

## What the installer does

1. **Detects the game** automatically via Steam (Windows registry +
   `libraryfolders.vdf`, so it works even if the game lives on another drive).
   A *Browse…* button is provided as a fallback.
2. **Patches `BrokeProtocol_Data/ScriptingAssemblies.json`**: appends
   `!_0Harmony.dll` and `ModLoader.dll` (type `16`) to the list without
   touching anything else (resilient to game updates).
3. **Patches `BrokeProtocol_Data/RuntimeInitializeOnLoads.json`**: registers
   the entry point `ModLoader.Core.Start`.
4. **Copies `ModLoader.dll` and `!_0Harmony.dll`** to `BrokeProtocol_Data/Managed`.

Both DLLs are **embedded into the `.exe`**: no external file is required to
distribute the installer.

### Safety / robustness

- **Backup**: every JSON file is copied to `*.modloader.bak` before modification.
- **Idempotent**: running the installer again creates no duplicates.
- **Uninstall**: a button restores the `.bak` files and removes the added DLLs.
- **UAC elevation**: the installer requests administrator rights at launch
  (required to write into `C:\Program Files (x86)\Steam\...`).

## Build

```sh
cargo build --release
```

The binary is produced at `target/release/ModLoaderSetup.exe`.

On every build, `build.rs` automatically refreshes the bundled DLLs from
`../ModLoader/ModLoader/bin/Debug` into `assets/` before embedding them. If
that folder does not exist (building on another machine), the existing copies
in `assets/` are kept as-is.

## Test

```sh
cargo test
```

- Unit tests (`src/`): Steam `.vdf` parsing, idempotent JSON merging.
- Integration test (`tests/integration.rs`): full install → reinstall → uninstall
  cycle against a temporary fake game folder.

## Architecture

| File                 | Role                                                        |
| -------------------- | ----------------------------------------------------------- |
| `src/main.rs`        | Graphical interface (egui/eframe).                          |
| `src/lib.rs`         | Library entry point (testable logic).                       |
| `src/steam.rs`       | Game folder detection via Steam.                            |
| `src/installer.rs`   | Backup, JSON merge, DLL copy, uninstall.                    |
| `src/assets.rs`      | Embedded DLLs (`include_bytes!`).                           |
| `build.rs`           | Refresh DLLs + embed Windows manifest (UAC/DPI).            |
