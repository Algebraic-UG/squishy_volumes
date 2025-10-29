import bpy
import argparse
import logging

from test_util import (
    PKG_ID,
    addon_filename_and_url,
    download_from_git,
    fetch_available_versions,
    installed_addons,
    temp_dir_cleanup,
    temp_dir_create,
)


def test_factory_clean():
    assert not installed_addons()


def test_install_uninstall(path):
    bpy.ops.extensions.package_install_files(repo="user_default", filepath=str(path))
    assert len(installed_addons()) == 1
    bpy.ops.extensions.package_uninstall(
        repo_index=next(
            i
            for i, repo in enumerate(bpy.context.preferences.extensions.repos)
            if repo.module == "user_default"
        ),
        pkg_id=PKG_ID,
    )
    test_factory_clean()


if __name__ == "__main__":
    logging.getLogger().setLevel(logging.DEBUG)

    versions = fetch_available_versions()

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--version-new",
        choices=versions,
        default=versions[0],
    )
    parser.add_argument(
        "--version-old",
        choices=versions,
        default=versions[1],
    )

    args = parser.parse_args()

    tmpdir = temp_dir_create()
    filename_new, url_new = addon_filename_and_url(args.version_new)
    filename_old, url_old = addon_filename_and_url(args.version_old)
    path_new = tmpdir / filename_new
    path_old = tmpdir / filename_old

    try:
        test_factory_clean()
        download_from_git(url_new, path_new)
        download_from_git(url_old, path_old)
        test_install_uninstall(path_new)
        test_install_uninstall(path_old)
    finally:
        temp_dir_cleanup(tmpdir)
