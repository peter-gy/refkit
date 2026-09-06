"""PEP 517 delegate that selects one wheel built by Maturin."""

from __future__ import annotations

from collections.abc import Sequence
from pathlib import Path

ConfigSettings = dict[str, object] | None


def build_wheel(
    wheel_directory: str,
    config_settings: ConfigSettings = None,
    metadata_directory: str | None = None,
) -> str:
    """Return the single prebuilt wheel in `wheel_directory`."""

    del config_settings, metadata_directory
    wheels = sorted(Path(wheel_directory).glob("*.whl"))
    if len(wheels) != 1:
        raise RuntimeError(f"Expected one prebuilt wheel in {wheel_directory}, found {len(wheels)}")
    return wheels[0].name


def build_sdist(
    sdist_directory: str,
    config_settings: ConfigSettings = None,
) -> str:
    """Reject source builds through the prebuilt-wheel delegate."""

    del sdist_directory, config_settings
    raise RuntimeError("The prebuilt-wheel delegate supports wheel augmentation")


def build_editable(
    wheel_directory: str,
    config_settings: ConfigSettings = None,
    metadata_directory: str | None = None,
) -> str:
    """Reject editable builds through the prebuilt-wheel delegate."""

    del wheel_directory, config_settings, metadata_directory
    raise RuntimeError("The prebuilt-wheel delegate supports wheel augmentation")


def get_requires_for_build_wheel(
    config_settings: ConfigSettings = None,
) -> Sequence[str]:
    """Return no additional requirements for a prebuilt wheel."""

    del config_settings
    return ()


get_requires_for_build_sdist = get_requires_for_build_wheel
get_requires_for_build_editable = get_requires_for_build_wheel
