import bpy
import requests


def addon_url(platform, version):
    just_number = version.lstrip("v")
    return f"https://github.com/Algebraic-UG/squishy_volumes/releases/download/{version}/squishy_volumes_extension-{just_number}-{platform}.zip"


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

    response = requests.get(url, headers=headers)

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
        pkg_id="squishy_volumes_extension",
    )


def extension_uninstall(remote_url):
    bpy.ops.extensions.package_uninstall(
        repo_index=extension_repo_index(remote_url),
        pkg_id="squishy_volumes_extension",
    )


def extension_module_name(remote_url):
    module = next(
        repo
        for repo in bpy.context.preferences.extensions.repos
        if repo.remote_url == remote_url
    ).module
    return f"bl_ext.{module}.squishy_volumes_extension"


def extension_enable(remote_url):
    bpy.ops.preferences.addon_enable(module=extension_module_name(remote_url))


def extension_disable(remote_url):
    bpy.ops.preferences.addon_disable(module=extension_module_name(remote_url))
