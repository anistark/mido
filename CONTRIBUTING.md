# Contributing to mido

Thanks for helping. The contributor guide lives in the docs: [docs/contributing.md](docs/contributing.md), or online at https://anistark.github.io/mido/contributing/. It covers the rendering pipeline, how to add a block type, how to write snapshot tests, and how the key reference and docs are checked.

The short version:

```sh
git clone https://github.com/anistark/mido
cd mido
just check
```

Open an issue before a large change, keep `CHANGELOG.md` updated in the same commit, and run `just keys` after editing the keymap.
