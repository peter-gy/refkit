import os
import platform
import subprocess
import sys
import sysconfig
from pathlib import Path

import pytest

from scripts import environment_contract as environment


@pytest.fixture
def repository(tmp_path: Path) -> Path:
    for name in (
        "pixi.toml",
        "Cargo.toml",
        "Makefile",
        "docs/package.json",
        "packages/refkit-js/rust/Cargo.lock",
    ):
        target = tmp_path / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes((environment.ROOT / name).read_bytes())
    return tmp_path


def test_current_environment_matches_package_locks(repository: Path) -> None:
    assert environment.validate(repository) == []


@pytest.mark.skipif(
    sys.platform != "linux" or os.environ.get("PIXI_PROJECT_ROOT") != str(environment.ROOT),
    reason="requires the Linux Pixi toolchain",
)
def test_linux_linker_does_not_embed_environment_path(tmp_path: Path) -> None:
    output = tmp_path / "probe.so"
    subprocess.run(
        [
            os.environ["CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER"],
            "-shared",
            "-x",
            "c",
            "/dev/null",
            "-o",
            str(output),
        ],
        check=True,
        capture_output=True,
    )
    assert os.environ["CONDA_PREFIX"].encode() not in output.read_bytes()


@pytest.mark.skipif(
    sys.platform != "darwin" or os.environ.get("PIXI_PROJECT_ROOT") != str(environment.ROOT),
    reason="requires the macOS Pixi toolchain and Apple SDK",
)
def test_macos_linker_uses_system_libraries_without_environment_path(tmp_path: Path) -> None:
    target = "AARCH64" if platform.machine() == "arm64" else "X86_64"
    source = tmp_path / "probe.c"
    source.write_text(
        '#include <iconv.h>\niconv_t probe(void) { return iconv_open("UTF-8", "UTF-8"); }\n'
    )
    output = tmp_path / "probe.dylib"
    subprocess.run(
        [
            os.environ[f"CARGO_TARGET_{target}_APPLE_DARWIN_LINKER"],
            "-dynamiclib",
            str(source),
            "-liconv",
            "-o",
            str(output),
        ],
        check=True,
        capture_output=True,
    )
    assert os.environ["CONDA_PREFIX"].encode() not in output.read_bytes()
    dependencies = subprocess.check_output(["otool", "-L", str(output)], text=True)
    assert "/usr/lib/libiconv.2.dylib" in dependencies


@pytest.mark.skipif(
    sys.platform != "darwin" or os.environ.get("PIXI_PROJECT_ROOT") != str(environment.ROOT),
    reason="requires the macOS Pixi toolchain and Python library",
)
def test_macos_runner_loads_python_without_embedded_environment_path(tmp_path: Path) -> None:
    target = "AARCH64" if platform.machine() == "arm64" else "X86_64"
    source = tmp_path / "probe.c"
    source.write_text(
        "#include <stdio.h>\nextern const char *Py_GetVersion(void);\n"
        "int main(void) { puts(Py_GetVersion()); return 0; }\n"
    )
    output = tmp_path / "probe"
    prefix = Path(os.environ["CONDA_PREFIX"])
    subprocess.run(
        [
            os.environ[f"CARGO_TARGET_{target}_APPLE_DARWIN_LINKER"],
            str(source),
            str(prefix / "lib" / sysconfig.get_config_var("LDLIBRARY")),
            "-o",
            str(output),
        ],
        check=True,
        capture_output=True,
    )
    assert str(prefix).encode() not in output.read_bytes()
    version = subprocess.check_output(
        [os.environ[f"CARGO_TARGET_{target}_APPLE_DARWIN_RUNNER"], str(output)], text=True
    )
    assert version.startswith(f"{sys.version_info.major}.{sys.version_info.minor}.")


@pytest.mark.parametrize(
    "before, after, message",
    [
        ('rust = "1.95.0.*"', 'rust = "1.96.0.*"', "rust must match"),
        (
            'rust-std-wasm32-unknown-unknown = "1.95.0.*"',
            'rust-std-wasm32-unknown-unknown = "1.96.0.*"',
            "rust-std-wasm32-unknown-unknown must match",
        ),
        ('pnpm = "11.10.0.*"', 'pnpm = "12.*"', "pnpm must match"),
        ('cargo-deny = "0.20.2.*"', 'cargo-deny = "0.21.*"', "cargo-deny must match"),
        ('cargo-shear = "1.13.4.*"', 'cargo-shear = "1.14.*"', "cargo-shear must match"),
        ("--version 0.2.126", "--version 0.2.127", "binding-tools must match"),
        ("--locked wasm-bindgen-cli", "wasm-bindgen-cli", "upstream Cargo lock"),
        ("[tasks]", '[pypi-dependencies]\npytest = "*"\n\n[tasks]', "belong to uv.lock"),
        (
            "[tasks]",
            '[target.linux-64.pypi-dependencies]\npytest = "*"\n\n[tasks]',
            "belong to uv.lock",
        ),
        (
            "[tasks]",
            '[feature.extra.pypi-dependencies]\npytest = "*"\n\n[tasks]',
            "belong to uv.lock",
        ),
        (
            "[tasks]",
            '[feature.extra.target.linux-64.pypi-dependencies]\npytest = "*"\n\n[tasks]',
            "belong to uv.lock",
        ),
    ],
)
def test_rejects_environment_drift(repository: Path, before: str, after: str, message: str) -> None:
    path = repository / "pixi.toml"
    source = path.read_text()
    assert before in source
    path.write_text(source.replace(before, after))
    assert any(message in error for error in environment.validate(repository))
