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
    parser.add_argument("--version", choices=versions)

    args = parser.parse_args()

    if args.local_json:
        path = Path(args.local_json).expanduser().resolve()
        file_url = urllib.parse.urljoin("file:", urllib.request.pathname2url(str(path)))
        URL = file_url
    else:
        URL = extension_url(args.version)

    test_factory_clean()

    bpy.context.preferences.system.use_online_access = True

    test_install(URL)

    # TODO

    test_uninstall(URL)
