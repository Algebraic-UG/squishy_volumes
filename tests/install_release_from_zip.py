# SPDX-License-Identifier: GPL-3.0-or-later
#
# This file is part of the Squishy Volumes extension.
# Copyright (C) 2025  Algebraic UG (haftungsbeschränkt)
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.


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
    test_factory_clean,
)


def test_install(path):
    bpy.ops.extensions.package_install_files(repo="user_default", filepath=str(path))
    assert len(installed_addons()) == 1


def test_uninstall():
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
    parser.add_argument("--local-zip")
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

    if args.local_zip:
        url_new = None  # pylint: disable=invalid-name
        path_new = args.local_zip
    else:
        filename_new, url_new = addon_filename_and_url(args.version_new)
        path_new = tmpdir / filename_new

    filename_old, url_old = addon_filename_and_url(args.version_old)
    path_old = tmpdir / filename_old

    try:
        test_factory_clean()

        # do this just once, we need to cleanup later
        if not args.local_zip:
            download_from_git(url_new, path_new)
        download_from_git(url_old, path_old)

        # just normal install of both old and new
        test_install(path_old)
        test_uninstall()
        test_install(path_new)
        test_uninstall()

        # update old -> new
        test_install(path_old)
        test_install(path_new)
        test_uninstall()

        # update new -> old -> new
        test_install(path_new)
        test_install(path_old)
        test_install(path_new)
        test_uninstall()

    finally:
        temp_dir_cleanup(tmpdir)
