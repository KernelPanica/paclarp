# Arch package

`PKGBUILD` produces a native package installable with `pacman -U`. From a
clean checkout:

```sh
git archive --format=tar.gz --prefix=paclarp-0.1.0/ HEAD > paclarp-0.1.0.tar.gz
makepkg -f
sudo pacman -U ./paclarp-0.1.0-1-x86_64.pkg.tar.zst
```

The package installs `/usr/bin/paclarp`; first launch creates the user JSONC
configuration under `~/.config/paclarp/config.jsonc`.
