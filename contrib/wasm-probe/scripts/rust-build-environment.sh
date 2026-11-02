#!/usr/bin/env bash
# We preserve Cargo's flag precedence and add reproducible compiler path mappings.
rust_sysroot="$(rustc --print sysroot)"
encoded_rust_flags="$(python3 - "$proof_directory" "$rust_sysroot" <<'PY'
import os
import sys

separator = chr(31)
if 'CARGO_ENCODED_RUSTFLAGS' in os.environ:
    encoded_flags = os.environ['CARGO_ENCODED_RUSTFLAGS']
    inherited_flags = encoded_flags.split(separator) if encoded_flags else []
else:
    inherited_flags = os.environ.get('RUSTFLAGS', '').split()

path_mappings = [
    (os.path.expanduser('~'), '/host'),
    (os.environ.get('CARGO_HOME'), '/cargo'),
    (os.environ.get('CARGO_TARGET_DIR'), '/build'),
    (os.path.dirname(os.path.dirname(sys.argv[1])), '/workspace'),
    (sys.argv[2], '/toolchain'),
]
for source_path, replacement in path_mappings:
    if source_path:
        inherited_flags.append(f'--remap-path-prefix={source_path}={replacement}')
print(separator.join(inherited_flags), end='')
PY
)"
export CARGO_ENCODED_RUSTFLAGS="$encoded_rust_flags"
