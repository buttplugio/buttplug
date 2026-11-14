# Building on Windows

`cargo build` is all that is required to *compile* this workspace on Windows. What follows is
the prerequisite setup the top-level README assumes, plus a wrapper script that enforces it.

## Prerequisites

| Requirement | Why |
| --- | --- |
| Rust **MSVC** toolchain, rustc >= 1.85 | `edition = "2024"` and `resolver = "3"` in the root manifest. The GNU toolchain compiles most of the tree then fails in `sdl3-sys` and `windows`. |
| Visual Studio 2022 Build Tools, workload `Microsoft.VisualStudio.Workload.VCTools` | Provides MSVC x64 and the linker. `crates/buttplug_server_hwmgr_sdl_gamepad` builds SDL3 from source through `cmake`. |
| CMake | Required by `sdl3-sys`'s build script. The copy bundled with VS under `Common7\IDE\CommonExtensions\Microsoft\CMake\` is preferred over any system CMake. |
| Ninja (optional) | Faster SDL3 build. Falls back to the Visual Studio generator. |

Install the toolchain directly. The `winget` package for rustup has been observed to report
success without actually installing anything:

    Invoke-WebRequest https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe -OutFile "$env:TEMP\rustup-init.exe"
    & "$env:TEMP\rustup-init.exe" -y --default-toolchain stable-x86_64-pc-windows-msvc --profile default

Then the C++ workload, in an elevated shell:

    winget install --id Microsoft.VisualStudio.2022.BuildTools --source winget --accept-package-agreements --accept-source-agreements --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

Open a **new** shell afterwards so `PATH` picks up `%USERPROFILE%\.cargo\bin`.
