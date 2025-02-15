# zed-hyprlang-extension

A Zed extension which adds syntax highlighting hyprlang and the [hyprls language server](https://github.com/hyprland-community/hyprls) for all parts of the [hypr-ecosystem](https://wiki.hyprland.org/Hypr-Ecosystem/) which are currently supported by the language server.

> At the moment hyprls only provides language server functionality to the main hyprland config file but there is an [issue for broader support of the hypr-ecosystem](https://github.com/hyprland-community/hyprls/issues/5)

## Installation

At the moment it's only possible to install the extension as an dev extension. For this you'll have to have `cargo` installed and the `wasm32-wasip1` target ond the stable rust toolchain set up. Additionally, you'll need to clone this repository to some location from where you can install it as a dev extension.
A more detailed tutorial about this can be found in the [Zed docs](https://zed.dev/docs/extensions/developing-extensions#developing-an-extension-locally).

### Language server

To use the hyprls language server, you'll have to have the language server installed and available in `$PATH`. A detailed installation guide can be found in the [hyprls repository](https://github.com/hyprland-community/hyprls?tab=readme-ov-file#installation).

## Setup

Once installed, you need to edit your `~/.config/zed/settings.json` to contain the following file types mappings:
```json
"file_types": {
    "Hyprland Config": [ "hyprland.conf", "hyprlandd.conf", "hyprland.hl", "hyprlandd.hl" ]
}
```

By default every file ending in `.hl` or `.conf` will have hyprlang syntax highlighting. Should you want to change the default syntax highlighting for `.conf` files you would need to add another rule to the `file_types` object.

> The whole pattern matching is a bit unfortunate since an extension is only allowed to specify a suffix but not a glob to match filenames. Additionally, the `file_types` rules are stored in an hash map which can result in non-deterministic behavior!