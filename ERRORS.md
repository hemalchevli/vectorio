# Error Log

## Failed Approaches and Resolutions

### 1. Windows Toolchain: MSVC build tools unavailable
- **What failed**: Default `stable-x86_64-pc-windows-msvc` toolchain failed because `link.exe` was missing and installing full MSVC build tools required administrator privileges.
- **What worked**: Installed `BrechtSanders.WinLibs.POSIX.UCRT` via `winget` (portable MinGW-w64 GCC/G++ suite) and switched toolchain to `stable-x86_64-pc-windows-gnu`. Ensured WinLibs `bin` directory was prepended to `PATH` so `gcc`, `g++`, `as`, and `dlltool` are discoverable by `cc-rs` and `windows-sys`.
- **Note for next time**: On Windows environments without admin privileges, use `x86_64-pc-windows-gnu` combined with WinLibs UCRT MinGW instead of MSVC.

### 2. Windows CRLF Line Endings in String Parsing Tests
- **What failed**: Test `i18n::tests::every_key_used_in_the_code_is_translated_and_none_is_left_over` failed with 100+ false missing keys because git checked out source files with `\r\n`, breaking hardcoded multiline splits (`split("\n    }\n")`).
- **What worked**: Normalized source contents with `.replace("\r\n", "\n")` in `used_keys()` before parsing.
- **Note for next time**: When tests read source files or raw text on Windows, normalize line endings to `\n` to be agnostic of git's `core.autocrlf`.

### 3. Cross-platform Floating Point Epsilon / Grid DDA Collinearity
- **What failed**:
  - `scene::tests::edge_on_face_does_not_occlude` failed due to strict `assert_eq!` between projected and unprojected points with ~1 ULP roundoff (`1.42e-14` delta).
  - `scene::tests::whole_line_is_atomic_for_occlusion_and_paper_clipping` failed because DDA horizontal/vertical line check `d.y.abs() < 1e-14` failed on `1.42e-14`, causing a horizontal segment to step vertically across grid cells and miss occluders.
  - `volumetric::continuity_tests::perspective_has_vanishing_points` failed on `assert_eq!` for `10.000000000000004` vs `10.0`.
- **What worked**:
  - Adjusted DDA collinearity threshold in `crates/vectorio-core/src/scene/hidden.rs` from `1e-14` to `1e-10`.
  - Used approximate distance / epsilon tolerance checks (`< 1e-7`, `< 1e-9`) in tests instead of bitwise `assert_eq!`.
- **Note for next time**: Avoid `assert_eq!` on floating-point projections or transformations across platforms. DDA collinear checks should allow for standard IEEE 754 double precision accumulation (~`1e-10`).

