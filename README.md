# pacwrap

`pacwrap` runs pacman and transforms its output using a JSONC configuration file. All pacman arguments are forwarded unchanged:

```sh
pacwrap -- -Syu
pacwrap --config ./config.jsonc -- -Qs firefox
pacwrap --print-default-config
```

The default file is `~/.config/pacwrap/config.jsonc` (or `$XDG_CONFIG_HOME/pacwrap/config.jsonc`). On first launch pacwrap creates this file and prints the optional shell alias. The backend defaults to `/usr/bin/pacman` so the alias cannot recurse. Print the alias again with `pacwrap --print-alias`.

Rules are applied in order to each output line; `pattern` is a regular expression and `replacement` uses regex capture references such as `$1`. Set `color` to `always`, `never`, or `auto` and optionally set a rule-level ANSI SGR code (for example `"31;1"`). Progress lines containing `NN%` can be formatted with `progress`; Kitty-compatible frames can be enabled with `kitty.frames`.

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
