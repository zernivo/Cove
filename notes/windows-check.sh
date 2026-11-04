#!/bin/bash
# Type-check Cove for Windows from the Linux sandbox (no linking).
# Needs: rustup target x86_64-pc-windows-msvc, apt clang + llvm-19, /tmp/shim headers (assert.h, string.h, stdlib.h stubs for ring).
# Self-installing: the sandbox home/apt state can be reset between sessions.
if ! command -v clang >/dev/null || [ ! -x /usr/lib/llvm-19/bin/llvm-rc ]; then sudo apt-get update -qq && sudo apt-get install -y -qq clang llvm-19 gcc >/dev/null; fi
if [ ! -x "$HOME/.cargo/bin/cargo" ]; then curl -sSf --cacert "$SSL_CERT_FILE" https://sh.rustup.rs | sh -s -- -y -q --profile minimal --no-modify-path --default-toolchain stable; fi
"$HOME/.cargo/bin/rustup" default stable >/dev/null 2>&1; "$HOME/.cargo/bin/rustup" target add x86_64-pc-windows-msvc >/dev/null 2>&1
mkdir -p /tmp/bin /tmp/shim
for t in llvm-rc llvm-ar; do ln -sf /usr/lib/llvm-19/bin/$t /tmp/bin/$t; done
[ -f /tmp/shim/assert.h ] || printf '#pragma once\n#define assert(x) ((void)0)\n#define static_assert _Static_assert\n' > /tmp/shim/assert.h
[ -f /tmp/shim/string.h ] || printf '#pragma once\n#include <stddef.h>\nvoid *memcpy(void*,const void*,size_t);void *memset(void*,int,size_t);void *memmove(void*,const void*,size_t);int memcmp(const void*,const void*,size_t);\n' > /tmp/shim/string.h
[ -f /tmp/shim/stdlib.h ] || printf '#pragma once\n#include <stddef.h>\nvoid abort(void);void *malloc(size_t);void free(void*);\nunsigned long _byteswap_ulong(unsigned long);unsigned long long _byteswap_uint64(unsigned long long);unsigned short _byteswap_ushort(unsigned short);\n' > /tmp/shim/stdlib.h
export PATH=/tmp/bin:$HOME/.cargo/bin:$PATH CC_x86_64_pc_windows_msvc=clang CFLAGS_x86_64_pc_windows_msvc="--target=x86_64-pc-windows-msvc -ffreestanding -I/tmp/shim -Wno-error" AR_x86_64_pc_windows_msvc=llvm-ar
cd "$(dirname "$(readlink -f "$0")")/../src-tauri" && CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}" cargo check -j "${JOBS:-2}" --target x86_64-pc-windows-msvc
