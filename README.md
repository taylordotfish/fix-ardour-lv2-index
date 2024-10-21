fix-ardour-lv2-index
====================

If a new version of an [LV2 plugin] changes the index of some of its
parameters, automation data for those parameters in existing [Ardour] sessions
may become corrupted or lost (see [Ardour issue 9825]).

[LV2 plugin]: https://lv2plug.in
[Ardour]: https://ardour.org
[Ardour issue 9825]: https://tracker.ardour.org/view.php?id=9825

fix-ardour-lv2-index provides a workaround for this issue by patching Ardour
session files to fix the incorrect indices.

Building
--------

Ensure the following dependencies are installed:

* [Rust] 1.74 or later
* [Lilv] \(development version; e.g., `liblilv-dev` on Debian/Ubuntu)
* [Git]

[Rust]: https://www.rust-lang.org/tools/install
[Lilv]: https://drobilla.net/software/lilv.html
[Git]: https://git-scm.com

Download the source code:

```bash
git clone https://github.com/taylordotfish/fix-ardour-lv2-index
cd fix-ardour-lv2-index
```

Build and install the program:

```bash
cargo install --path .
```

This will install a program named `fix-ardour-lv2-index` in `~/.cargo/bin`. If
that directory is in your `PATH`, you can run the program simply by typing
`fix-ardour-lv2-index` in your shell.

<details>
<summary>Run without installing</summary>

You can also build and run the program without installing:

```bash
cargo build --release
./target/release/fix-ardour-lv2-index --help
```
</details>

Usage
-----

Run `fix-ardour-lv2-index` with the path to an Ardour session file (`.ardour`
extension):

```bash
fix-ardour-lv2-index /path/to/your-session/your-session.ardour
```

This patches the session file, fixing the incorrect LV2 indices, and saves a
backup of the original session in `your-session.ardour.orig`.

See `fix-ardour-lv2-index --help` for a full list of options.

License
-------

fix-ardour-lv2-index is licensed under version 3 of the GNU General Public
License, or (at your option) any later version. See [LICENSE](LICENSE).
