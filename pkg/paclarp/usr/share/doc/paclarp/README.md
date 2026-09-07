# paclarp

## Installation

Build an Arch package from a checkout:

```sh
git archive --format=tar.gz --prefix=paclarp-0.1.0/ HEAD > paclarp-0.1.0.tar.gz
makepkg -f
sudo pacman -U ./paclarp-0.1.0-1-x86_64.pkg.tar.zst
```

This installs `/usr/bin/paclarp`. For a local development build, use:

```sh
cargo build --release
install -Dm755 target/release/paclarp ~/.local/bin/paclarp
```

On first run paclarp creates `~/.config/paclarp/config.jsonc` (or
`$XDG_CONFIG_HOME/paclarp/config.jsonc`). It never overwrites an existing
configuration. To route ordinary `pacman` commands through paclarp, add the
following to your shell startup file and start a new shell:

```sh
alias pacman='paclarp --'
```

You can print the alias at any time with `paclarp --print-alias`.

`paclarp` runs pacman and transforms its output using a JSONC configuration file. All pacman arguments are forwarded unchanged:

```sh
paclarp -- -Syu
paclarp --config ./config.jsonc -- -Qs firefox
paclarp --print-default-config
```

The default file is `~/.config/paclarp/config.jsonc` (or `$XDG_CONFIG_HOME/paclarp/config.jsonc`). On first launch paclarp creates this file and prints the optional shell alias. The backend defaults to `/usr/bin/pacman` so the alias cannot recurse. Print the alias again with `paclarp --print-alias`.

Rules are applied in order to each output line; `pattern` is a regular expression and `replacement` uses regex capture references such as `$1`. Set `color` to `always`, `never`, or `auto` and optionally set a rule-level ANSI SGR code (for example `"31;1"`). Progress lines containing `NN%` can be formatted with `progress`; Kitty-compatible frames can be enabled with `kitty.frames`.

Use `text_color` for the base ANSI SGR color of a stream (for example `"37"`). Add ordered `highlights` entries to override it for matching lines:

```jsonc
"stdout": {
  "color": "auto",
  "text_color": "37",
  "highlights": [
    { "pattern": "^warning:", "color": "33;1" },
    { "pattern": "^error:", "color": "31;1" }
  ]
}
```

Example:

```jsonc
{
  "pacman": "pacman",
  "stdout": {
    "prefix": "[pacman] ",
    "color": "auto",
    "rules": [
      { "pattern": "^::", "replacement": "$0", "color": "36;1" },
      { "pattern": "^(warning:.*)$", "replacement": "WARN: $1", "color": "33" }
    ]
  },
  "stderr": { "color": "never", "rules": [] }
}
```

The child process exit code is preserved. Invalid regex rules are ignored so one optional customization cannot prevent pacman from running.

## Configuration reference

The configuration is JSONC: standard JSON plus `//` and block comments. Generate
the complete built-in schema with `paclarp --print-default-config`.

### Stream formatting

`stdout` and `stderr` support:

- `prefix` and `suffix`: text added to every rendered line.
- `color`: `auto`, `always`, or `never`.
- `text_color`: default ANSI SGR code, such as `"37"` or `"38;5;245"`.
- `highlights`: ordered regex/color overrides.
- `rules`: ordered regex replacements with `$0`, `$1`, and other captures.

Example:

```jsonc
"stderr": {
  "color": "always",
  "text_color": "37",
  "highlights": [
    { "pattern": "^warning:", "color": "33;1" },
    { "pattern": "^error:", "color": "31;1" }
  ],
  "rules": [
    { "pattern": "^warning:", "replacement": "⚠ $0" }
  ]
}
```

### Progress bars and images

Progress recognizes percentages such as `42%` in pacman output. Configure its
shape with `width`, `filled`, `empty`, and `template`; templates support
`{bar}`, `{percent}`, and `{image}`. Kitty frames are enabled only on Kitty
terminals and silently fall back elsewhere:

```jsonc
"stdout": {
  "progress": {
    "enabled": true,
    "width": 24,
    "filled": "█",
    "empty": "·",
    "template": "Downloading [{bar}] {percent}%"
  },
  "kitty": {
    "enabled": true,
    "frames": ["~/.config/paclarp/a.png", "~/.config/paclarp/b.png"],
    "interval_ms": 120,
    "width": 32,
    "height": 32
  }
}
```

### UI styles, themes, and animation

The `ui` section controls higher-level presentation:

```jsonc
"ui": {
  "style": "dashboard", // minimal, verbose, dashboard, compact
  "verbosity": 2,
  "animation": true,
  "fps": 12,
  "sound": false,
  "spinner": ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴"],
  "operation_icons": {
    "download": "↓",
    "install": "+",
    "remove": "−"
  },
  "theme": {
    "package": "36",
    "download": "34",
    "install": "32",
    "warning": "33;1",
    "error": "31;1",
    "border": "90",
    "status": "37"
  }
}
```

`verbosity` is `0` for quiet filtering, `1` for normal output, and higher
values for future/dashboard detail. Set `animation` to `false` for CI or
log files. ANSI colors automatically degrade when stdout is not a TTY, while
Kitty image escape sequences are emitted only when `TERM=xterm-kitty`.

### Backend and command usage

The `pacman` field selects the backend executable; it defaults to
`/usr/bin/pacman` to avoid recursion through the shell alias. Arguments after
`--` are passed unchanged:

```sh
paclarp -- -Syu
paclarp -- -S --needed firefox
paclarp --config ./work.jsonc -- -Rns old-package
```

## Terminal UI

The `ui` section accepts `style` (`minimal`, `verbose`, `dashboard`, or `compact`), `verbosity`, global `animation`, animation `fps`, `sound`, custom spinner frames, operation icons, and ANSI SGR theme colors for packages, downloads, installation, warnings, errors, borders, and status text. Existing stream rules remain valid.

paclarp classifies pacman output into transaction, download, installation, hook, completion, and failure events before rendering. Terminal capabilities are detected from TTY state, `TERM`, `SSH_CONNECTION`, and `CI`; Kitty images are emitted only under Kitty and otherwise fall back to configured text progress bars.

The current configuration schema can always be generated with:

```sh
paclarp --print-default-config
```

For Arch Linux package creation and `pacman -U` installation, see [packaging.md](packaging.md).
