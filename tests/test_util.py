import bpy
import requests
import tempfile
from pathlib import Path
import logging
import platform


PKG_ID = "squishy_volumes_extension"


def addon_filename(version):
    just_number = version.lstrip("v")
    platform_name = get_platform()
    return f"{PKG_ID}-{just_number}-{platform_name}.zip"


def addon_url(version):
    zip_file = addon_filename(version)
    return f"https://github.com/Algebraic-UG/squishy_volumes/releases/download/{version}/{zip_file}"


def addon_filename_and_url(version):
    return addon_filename(version), addon_url(version)


def extension_url(version=None):
    if version is None:
        return "https://github.com/Algebraic-UG/squishy_volumes/releases/latest/download/index.json"
    else:
        return f"https://github.com/Algebraic-UG/squishy_volumes/releases/download/{version}/index.json"


def fetch_available_versions():
    url = "https://api.github.com/repos/Algebraic-UG/squishy_volumes/releases"
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    }

    response = requests.get(url, headers=headers, timeout=300)

    response.raise_for_status()
    data = response.json()

    return list(release["name"] for release in data)


def extension_repo_index(remote_url):
    next(
        i
        for i, repo in enumerate(bpy.context.preferences.extensions.repos)
        if repo.remote_url == remote_url
    )


def extension_repo_add(remote_url):
    bpy.ops.preferences.extension_repo_add(remote_url=remote_url)
    bpy.ops.extensions.repo_sync(repo_index=extension_repo_index(remote_url))


def extension_repo_remove(remote_url):
    bpy.ops.preferences.extension_repo_remove(
        index=extension_repo_index(remote_url),
        remove_files=True,
    )


def extension_install(remote_url):
    bpy.ops.extensions.package_install(
        repo_index=extension_repo_index(remote_url),
        pkg_id=PKG_ID,
    )


def extension_uninstall(remote_url):
    bpy.ops.extensions.package_uninstall(
        repo_index=extension_repo_index(remote_url),
        pkg_id=PKG_ID,
    )


def extension_module_name(remote_url):
    module = next(
        repo
        for repo in bpy.context.preferences.extensions.repos
        if repo.remote_url == remote_url
    ).module
    return f"bl_ext.{module}.{PKG_ID}"


def extension_enable(remote_url):
    bpy.ops.preferences.addon_enable(module=extension_module_name(remote_url))


def extension_disable(remote_url):
    bpy.ops.preferences.addon_disable(module=extension_module_name(remote_url))


def download_from_git(url, path):
    logging.info("Dowloading: %s to %s", url, path)
    session = requests.Session()
    session.headers.update({"Accept": "application/vnd.github+json"})
    with session.get(url=url, timeout=300, stream=True) as res:
        res.raise_for_status()
        with open(path, "wb") as f:
            for chunk in res.iter_content(chunk_size=8192):
                if chunk:
                    f.write(chunk)


def temp_dir_create():
    tmpdir = Path(tempfile.mkdtemp(prefix="squishy_volumes_test_"))
    logging.info("Test directory: %s", tmpdir)
    return tmpdir


def temp_dir_cleanup(tmpdir):
    for p in tmpdir.glob("*"):
        logging.info("Cleaning up: %s", p)
        p.unlink(missing_ok=True)
    tmpdir.rmdir()


def installed_addons():
    return [a for a in bpy.context.preferences.addons if PKG_ID in a.module]


def get_platform():
    name = platform.system()
    if name == "Linux":
        return "linux_x64"
    if name == "Darwin":
        return "macos_arm64"
    if name == "Windows":
        return "windows_x64"
    raise RuntimeError(f"Unknown platform: {name}")
