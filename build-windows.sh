#!/usr/bin/env bash
# Cross-compile the Windows portable exe from Linux.
#
# Uses a portable LLVM-MinGW (UCRT) toolchain unpacked without root, and
# statically links the CRT so the .exe has no mingw runtime dependencies.
#
# Output: target/x86_64-pc-windows-gnu/release/sheetpacker.exe
set -euo pipefail

TOOLCHAIN="${LLVM_MINGW:-/tmp/opencode/llvm-mingw}"

if [ ! -x "$TOOLCHAIN/bin/x86_64-w64-mingw32-clang" ]; then
  echo "error: LLVM-MinGW not found at $TOOLCHAIN" >&2
  echo "set LLVM_MINGW=/path/to/llvm-mingw" >&2
  exit 1
fi

export PATH="$TOOLCHAIN/bin:$PATH"
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-clang
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-clang
export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-clang++
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar

exec cargo build --release --target x86_64-pc-windows-gnu \
  --config 'target.x86_64-pc-windows-gnu.rustflags=["-C","target-feature=+crt-static"]' "$@"
