import bpy
import argparse
import logging

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
