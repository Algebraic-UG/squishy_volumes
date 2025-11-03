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
import urllib
from pathlib import Path

from test_util import (
    extension_install,
    extension_repo_add,
    extension_repo_remove,
    extension_repo_update_url,
    extension_url,
    fetch_available_versions,
    installed_addons,
    test_factory_clean,
)


def test_install(remote_url):
    extension_repo_add(remote_url)
    extension_install(remote_url)
    assert len(installed_addons()) == 1


def test_uninstall(remote_url):
    extension_repo_remove(remote_url)
    test_factory_clean()


def test_update(url_old, url_new):
    extension_repo_update_url(url_old, url_new)
    extension_install(url_new)


if __name__ == "__main__":
    logging.getLogger().setLevel(logging.DEBUG)

    versions = fetch_available_versions()

    parser = argparse.ArgumentParser()
    parser.add_argument("--local-json")
    parser.add_argument(
        "--version-new",
        choices=versions,
    )
    parser.add_argument(
        "--version-old",
        choices=versions,
        default=versions[1],
    )

    args = parser.parse_args()

    if args.local_json:
        path = Path(args.local_json).expanduser().resolve()
        file_url = urllib.parse.urljoin("file:", urllib.request.pathname2url(str(path)))
        URL_NEW = file_url
    else:
        URL_NEW = extension_url(args.version_new)

    URL_OLD = extension_url(args.version_old)

    test_factory_clean()

    bpy.context.preferences.system.use_online_access = True

    # just normal install of both old and new
    test_install(URL_OLD)
    test_uninstall(URL_OLD)
    test_install(URL_NEW)
    test_uninstall(URL_NEW)

    # update old -> new
    test_install(URL_OLD)
    test_update(URL_OLD, URL_NEW)
    test_uninstall(URL_NEW)

    # update new -> old -> new
    test_install(URL_NEW)
    test_update(URL_NEW, URL_OLD)
    test_update(URL_OLD, URL_NEW)
    test_uninstall(URL_NEW)
