"""Build the PyWebview application via PyInstaller."""

from __future__ import annotations

import argparse
import importlib.util
import os
import platform
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

REPO_ROOT = Path(__file__).resolve().parents[1]
ENTRY_POINT = REPO_ROOT / "webview_UI" / "app.py"
ENTRY_POINT_OCR = REPO_ROOT / "webview_UI" / "app_ocr.py"
ASSETS_DIR = REPO_ROOT / "webview_UI" / "assets"
ASSET_BUNDLE_TARGET = "webview_assets"
OPTIONAL_RESOURCE_FILES: tuple[tuple[Path, str], ...] = (
    (REPO_ROOT / "webview_UI" / "character_preset.json", "."),
    (REPO_ROOT / "webview_UI" / "user_counts_data.json", "."),
)
OCR_RESOURCE_FILES: tuple[tuple[Path, str], ...] = (
    (REPO_ROOT / "policy_core" / "ocr" / "upgrade_page_logo.png", "policy_core/ocr"),
)


@dataclass(frozen=True)
class BuildVariant:
    name: str
    entry_point: Path
    extra_args: list[str] = field(default_factory=list)
    extra_data: tuple[tuple[Path, str], ...] = ()


def format_add_data(source: Path, destination: str) -> str:
    """Return a platform-correct add-data argument for PyInstaller."""

    separator = ";" if os.name == "nt" else ":"
    return f"{source}{separator}{destination}"


def resolve_pyinstaller_command() -> list[str]:
    """Return the command used to invoke PyInstaller."""

    candidates = ["pyinstaller"]
    exe_dir = Path(sys.executable).parent
    candidates.append(str(exe_dir / "pyinstaller"))
    candidates.append(str(exe_dir / "pyinstaller.exe"))

    venv_dir = os.environ.get("VIRTUAL_ENV")
    if venv_dir:
        candidates.append(str(Path(venv_dir) / "bin" / "pyinstaller"))
        candidates.append(str(Path(venv_dir) / "Scripts" / "pyinstaller.exe"))

    # Common local virtual environment name.
    local_venv = REPO_ROOT / ".venv"
    candidates.append(str(local_venv / "bin" / "pyinstaller"))
    candidates.append(str(local_venv / "Scripts" / "pyinstaller.exe"))

    for candidate in candidates:
        path = shutil.which(candidate)
        if path:
            try:
                subprocess.run([path, "--version"], check=True, capture_output=True, text=True)
            except (OSError, subprocess.CalledProcessError):
                continue
            return [path]

    raise SystemExit("PyInstaller is not available. Install it with 'pip install pyinstaller' first.")


def collect_onnxocr_data() -> tuple[tuple[Path, str], ...]:
    """Locate non-Python data files bundled with the onnxocr package."""

    try:
        spec = importlib.util.find_spec("onnxocr")
    except Exception:
        return ()
    if not spec or not spec.origin:
        return ()
    package_dir = Path(spec.origin).resolve().parent
    if not package_dir.exists():
        return ()

    data_files: list[tuple[Path, str]] = []
    for path in package_dir.rglob("*"):
        if path.is_dir():
            continue
        if path.suffix in {".py", ".pyc", ".pyo"}:
            continue
        relative = path.relative_to(package_dir)
        dest = "onnxocr"
        if relative.parent.as_posix() != ".":
            dest = f"{dest}/{relative.parent.as_posix()}"
        data_files.append((path, dest))
    return tuple(data_files)


def build_command(
    base_cmd: list[str],
    args: argparse.Namespace,
    *,
    name: str,
    entry_point: Path,
    extra_args: Iterable[str] = (),
    extra_data: Iterable[tuple[Path, str]] = (),
) -> tuple[list[str], dict[str, str]]:
    """Construct the PyInstaller command and accompanying environment."""

    command = base_cmd + [str(entry_point), "--name", name, "--noconfirm"]

    if args.clean:
        command.append("--clean")
    if not args.console:
        command.append("--windowed")
    if args.onefile:
        command.append("--onefile")

    command.extend(["--add-data", format_add_data(ASSETS_DIR, ASSET_BUNDLE_TARGET)])
    if args.bundle_config:
        for source, destination in OPTIONAL_RESOURCE_FILES:
            if not source.exists():
                print(f"Warning: requested bundle for missing resource {source}")
                continue
            command.extend(["--add-data", format_add_data(source, destination)])
    for source, destination in extra_data:
        if not source.exists():
            print(f"Warning: requested bundle for missing resource {source}")
            continue
        command.extend(["--add-data", format_add_data(source, destination)])

    env = os.environ.copy()

    for extra in args.pyinstaller_args:
        command.append(extra)
    for extra in extra_args:
        command.append(extra)

    return command, env


def cleanup_previous_outputs(name: str, onefile: bool) -> None:
    """Remove previous build/dist outputs that might interfere with a new build."""

    dist_dir = REPO_ROOT / "dist"
    build_dir = REPO_ROOT / "build"

    targets: list[Path] = []
    spec_file = REPO_ROOT / f"{name}.spec"
    if spec_file.exists():
        spec_file.unlink()

    if dist_dir.exists():
        if onefile:
            # Onefile artefacts vary by platform; keep removal conservative.
            candidates = [
                dist_dir / (name + ".exe"),
                dist_dir / name,
            ]
            if platform.system().lower() == "darwin":
                candidates.append(dist_dir / f"{name}.app")
            targets.extend(candidates)
        else:
            targets.append(dist_dir / name)
            targets.append(dist_dir / f"{name}.app")
            # Remove symlink-friendly directories inside the .app bundle if they remain.
            targets.append(dist_dir / f"{name}.app/Contents/Frameworks/webview")

    if build_dir.exists():
        targets.append(build_dir / name)
        targets.append(build_dir / (name + ".app"))

    for target in targets:
        if not target.exists():
            continue
        if target.is_dir():
            shutil.rmtree(target, ignore_errors=True)
        else:
            try:
                target.unlink()
            except FileNotFoundError:
                pass

    # Cleanup empty dist/build directories that may have been left behind.
    for root in (dist_dir, build_dir):
        try:
            if root.exists() and not any(root.iterdir()):
                root.rmdir()
        except OSError:
            pass


def resolve_target(value: str | None) -> str:
    if value:
        return value
    system = platform.system().lower()
    if system == "darwin":
        return "macos"
    if system == "windows":
        return "windows"
    return "linux"


def determine_variants(args: argparse.Namespace, target: str) -> list[BuildVariant]:
    """Return the list of build variants to generate."""

    base_name = args.name
    if args.target:
        base_name = f"{base_name}-{target}"

    onnxocr_data = collect_onnxocr_data()

    variants = [
        BuildVariant(
            name=base_name,
            entry_point=ENTRY_POINT,
            extra_args=["--exclude-module", "onnxocr"],
            extra_data=(),
        ),
        BuildVariant(
            name=f"{base_name}_ocr",
            entry_point=ENTRY_POINT_OCR,
            extra_args=[],
            extra_data=OCR_RESOURCE_FILES + onnxocr_data,
        ),
    ]
    return variants


def parse_args(argv: Iterable[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the WuWa Echo Policy Calculator PyWebview app.")
    parser.add_argument(
        "--name",
        default="WuWaEchoCalculator",
        help="Base name for generated executables/app bundles (default: %(default)s).",
    )
    parser.add_argument(
        "--target",
        choices=("windows", "macos", "linux"),
        default=None,
        help="Target platform for the build (default: current platform).",
    )
    parser.add_argument(
        "--onefile",
        action="store_true",
        help="Generate a single-file executable (extracts to a temp directory on launch).",
    )
    parser.add_argument(
        "--console",
        action="store_true",
        help="Keep the console window (default hides console).",
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        help="Request PyInstaller to clean previous build artifacts before building.",
    )
    parser.add_argument(
        "--pyinstaller-arg",
        dest="pyinstaller_args",
        action="append",
        default=[],
        help="Extra argument to forward to PyInstaller (can be repeated).",
    )
    parser.add_argument(
        "--bundle-config",
        action="store_true",
        help="Bundle character presets and user count JSON files with the executable.",
    )
    return parser.parse_args(list(argv) if argv is not None else None)


def main(argv: Iterable[str] | None = None) -> None:
    if not ENTRY_POINT.exists():
        raise FileNotFoundError(f"Cannot find webview_UI entry point at {ENTRY_POINT!s}")
    if not ENTRY_POINT_OCR.exists():
        raise FileNotFoundError(f"Cannot find webview_UI entry point at {ENTRY_POINT_OCR!s}")
    if not ASSETS_DIR.exists():
        raise FileNotFoundError(f"Cannot locate webview_UI assets at {ASSETS_DIR!s}")
    args = parse_args(argv)
    if args.bundle_config:
        missing_resources = [
            resource
            for resource, _dest in OPTIONAL_RESOURCE_FILES
            if not resource.exists()
        ]
        if missing_resources:
            print(
                "Optional resources missing and will not be bundled:",
                ", ".join(str(path) for path in missing_resources),
            )

    target = resolve_target(args.target)
    variants = determine_variants(args, target)
    base_cmd = resolve_pyinstaller_command()
    for variant in variants:
        command, env = build_command(
            base_cmd,
            args,
            name=variant.name,
            entry_point=variant.entry_point,
            extra_args=variant.extra_args,
            extra_data=variant.extra_data,
        )
        cleanup_previous_outputs(variant.name, args.onefile)

        print(f"Building PyWebview app ({variant.name}) with command:")
        print("  " + " ".join(command))

        try:
            subprocess.run(command, check=True, cwd=REPO_ROOT, env=env)
        except subprocess.CalledProcessError as exc:
            raise SystemExit(exc.returncode) from exc

        output_dir = REPO_ROOT / "dist" / variant.name
        if args.onefile:
            output_dir = REPO_ROOT / "dist"
        if output_dir.exists():
            print(f"Build complete. Artifacts available in: {output_dir}")
        else:
            print("Build script finished, but PyInstaller did not create the expected output directory.")


if __name__ == "__main__":
    main(sys.argv[1:])
