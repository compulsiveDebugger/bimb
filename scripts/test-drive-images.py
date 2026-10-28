"""Exercise synthetic FAT32/exFAT images on macOS, including read-only mounts.

Requires access to DiskImages services. Uses only new temporary images, never physical disks.
"""
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile

REPO = Path(__file__).resolve().parent.parent


def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def attach(image, mount, read_only=False):
    args = ["hdiutil", "attach", str(image), "-nobrowse", "-mountpoint", str(mount), "-plist"]
    if read_only:
        args.append("-readonly")
    result = run(*args, stdout=subprocess.PIPE)
    plistlib.loads(result.stdout)  # Verify that attachment returned a valid result.


with tempfile.TemporaryDirectory(prefix="bimb-drive-tests-") as temporary:
    temporary = Path(temporary)
    for name, filesystem in [("fat32", "MS-DOS FAT32"), ("exfat", "ExFAT")]:
        image = temporary / f"{name}.dmg"
        mount = temporary / f"mount-{name}"
        mount.mkdir()
        run("hdiutil", "create", "-size", "96m", "-fs", filesystem,
            "-volname", f"BimbTest-{name}", str(image))
        for read_only in [False, True]:
            attach(image, mount, read_only)
            try:
                if not read_only:
                    shutil.copyfile(REPO / "crates/bimb-core/tests/fixtures/mountain.jpg", mount / "mountain.jpg")
                env = {**os.environ, "BIMB_TEST_DRIVE_FIXTURE": str(mount),
                       "BIMB_TEST_READ_ONLY": str(read_only).lower()}
                run("sh", str(REPO / "scripts/cargo.sh"), "test", "-p", "bimb-core", "--test", "drives",
                    "mounted_drive_fixture_is_read_without_writing_originals", "--", "--ignored", "--nocapture", env=env)
            finally:
                run("hdiutil", "detach", str(mount))
