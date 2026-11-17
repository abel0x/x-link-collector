# Packaging

Notes for putting x-link-collector into a distribution's package manager.
The Arch User Repository package `x-link-collector` follows them.

## Layout

The receiver looks for the `tools/` and `extension/` folders next to its own
binary, and also under `<prefix>/share/x-link-collector` (and
`<prefix>/lib/x-link-collector`) for a binary in `<prefix>/bin`. So:

```
/usr/bin/x-link-receiver
/usr/bin/x-download        -> /usr/share/x-link-collector/tools/x-download
/usr/bin/x-flatten         -> /usr/share/x-link-collector/tools/x-flatten
/usr/share/x-link-collector/tools/
/usr/share/x-link-collector/extension/
/usr/lib/systemd/user/x-link-receiver.service
```

The panel then shows `/usr/share/x-link-collector/extension` as the folder to
load the extension from; a read-only folder is fine for that.

## Building

```sh
cd receiver
X_LINK_COLLECTOR_PACKAGED=1 cargo build --release --locked
```

The receiver has no dependencies, so the build needs no network.
`X_LINK_COLLECTOR_PACKAGED` keeps the build folder's path out of the binary:
otherwise it remembers its source checkout, where the tools are for anyone who
ran `make install`, and a package's build folder is gone once it is installed.

## yt-dlp and gallery-dl

Either from the distribution, as optional dependencies: `x-download` uses the
ones on `PATH` when it has no virtualenv of its own. Or from the panel's
**Install or update** button, which makes a virtualenv: in `tools/.venv` when
that folder is writable, and otherwise in the user's data folder
(`$XDG_DATA_HOME/x-link-collector/venv`, usually
`~/.local/share/x-link-collector/venv`). Nothing is written under `/usr`.

ffmpeg is needed when X serves a video's picture and sound apart.

## The user service

`packaging/systemd/x-link-receiver.service` starts `%h/.local/bin/x-link-receiver`,
where `make install` puts it. A package installs it with that line pointing at
its own binary:

```sh
sed 's|^ExecStart=.*|ExecStart=/usr/bin/x-link-receiver|' \
  packaging/systemd/x-link-receiver.service > x-link-receiver.service
```

Each user then turns it on for themselves:

```sh
systemctl --user enable --now x-link-receiver
```

## Settings and data

Nothing is kept beside the program. Settings, the download history, the
capture log and the panel key are in `$XDG_CONFIG_HOME/x-link-collector/`;
links and media go where the settings say, by default the user's desktop.
