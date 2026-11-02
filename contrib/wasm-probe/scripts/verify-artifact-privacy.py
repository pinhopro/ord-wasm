"""Rejects embedded host home paths in generated artifact bytes without editing them."""

import os
from pathlib import Path
import sys

home_directory = os.path.expanduser('~').encode()
for artifact_path in Path(sys.argv[1]).iterdir():
    if not artifact_path.is_file():
        continue
    artifact_bytes = artifact_path.read_bytes()
    for forbidden_prefix in [home_directory, b'/Users/', b'/home/']:
        if forbidden_prefix in artifact_bytes:
            raise SystemExit(f'Host path detected in generated artifact: {artifact_path.name}')
print('Generated artifact byte privacy check passed')
