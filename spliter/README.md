# spliter

Pulls every downloaded file out of its per-handle folder and lays them side by
side in one directory.

```sh
./x-flatten --dry-run     # see what would move, touch nothing
./x-flatten               # move them
./x-flatten --undo        # put them back
```

Default: `<desktop>/x-media/<handle>/<file>` → `<desktop>/x-media/x-media/<file>`

```
x-media/                          x-media/
├── littlebambiboii/              └── x-media/
│   └── 2093332826263376118-1.mp4     ├── 2093332826263376118-1.mp4
├── itssefa9/               ->        ├── 2093373181234987229-1.mp4
│   └── 2093373181234987229-1.mp4     ├── 2093054732000440628-1.mp4
└── TwinkLeo07/                       └── 2093055318041260437-1.jpg
    ├── 2093054732000440628-1.mp4
    └── 2093055318041260437-1.jpg
```

The per-handle folders are removed once empty. `.x-download-state.json` stays
where it is, so the downloader still knows what it has fetched.

## It is reversible

Each run writes `.x-flatten-manifest.json` into the destination, recording where
every file came from. `--undo` walks it back exactly — verified on 620 files:
all 620 returned to their original 119 folders, and the empty flat folder was
cleaned up afterwards.

Moving 600 files with no way back is not a thing to ship, hence the manifest.

## Two things it is careful about

**The destination sits inside the source.** `x-media/x-media` is under
`x-media`, so a naive scan would find the already-moved files on a second run
and move them onto themselves. The destination subtree is excluded from the
scan; running it twice reports `no files to move`.

**Names never overwrite.** A tweet id is unique, so `<tweet id>-<n>.<ext>` is
too, and collisions should not happen. If one somehow does, the second file
becomes `<name>~2.<ext>` rather than replacing the first.

## Options

```
--src DIR           folder to flatten          (default: <desktop>/x-media)
--dest DIR          where the flat files go    (default: <src>/x-media)
--copy              copy instead of moving; needs twice the disk space
--prefix-handle     keep the poster in the name: <handle>-<tweet id>-1.mp4
--videos-only       leave images in their folders
--dry-run           list what would happen, change nothing
--undo              reverse the last move using the manifest
```

Flattening loses the poster's name, since that only lived in the folder. Use
`--prefix-handle` if you want it kept:

```
historyinmemes-1790637656616943991-1.mp4
```

## A note on the default destination

Putting the flat folder inside `x-media` is what was asked for and it works, but
`x-media/x-media` reads oddly and the downloader's `--status` counts files
recursively, so it will keep counting them. Somewhere outside is tidier:

```sh
./x-flatten --dest ~/Videolar/x-hepsi
```
