# Troubleshooting

Here are solutions to some common problems.

First, if you are not using the latest release of Squishy Volumes, please have a look at the [changelogs of newer releases](https://github.com/Algebraic-UG/squishy_volumes/releases). Your problem might already be solved in a newer release.

If you can't find a solution to your problem here, please consider the [get-help Discord channel](https://discord.gg/nXWqTMZgTE) and opening an [issue on Github](https://github.com/Algebraic-UG/squishy_volumes/issues).

You can also just message me (Vollkornaffe) on Discord directly, I'm happy to help.

> It is never the user's fault.

I swear by this. Yet certain problems are so hard to solve 'completely' that we have to cut some corners (at least for now) and this is why this chapter exists.

## No Squishy Volumes UI 

If you cannot find the Squishy Volumes [UI](../getting_started/discover_ui.md):

Please make sure that you have [installed](../getting_started/installation.md) Squishy Volumes and that you are in [Object Mode](https://docs.blender.org/manual/en/latest/editors/3dview/modes.html).

If you have several other Add-ons installed, you might need to scroll down in the sidebar. 

## Installation

Here are some problems related to [installing](../getting_started/installation.md) Squishy Volumes.

### Verify

Please try to verify that Squishy Volumes is installed in Edit > Preferences (Ctrl + ,).

<img src="verify_installed.png">

### Multiple 

This can happen by mixing installation methods.

If there are multiple installations of Squishy Volumes present, remove duplicates.

<img src="uninstall_duplicate.png">

### Orphan

This can happen when 'extension.blender.org' was chosen during install.

<center><img src="extension_platform.png"></center>

<img src="orphan.png">

Please uninstall and try again with 'User Default'.


### Unsupported Blender Version 

Make sure your Blender version is supported.

You'll see this when you try to install Squishy Volumes in an unsupported Blender version like 4.5.12:

<img src="unsupported_blender.png">
