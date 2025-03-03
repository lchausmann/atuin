# Key binding

## Shell

By default, Atuin will rebind both <kbd>Ctrl-r</kbd> and the up arrow.

You can also disable either the up-arrow or <kbd>Ctrl-r</kbd> bindings individually, by passing
`--disable-up-arrow` or `--disable-ctrl-r` to the call to `atuin init`:

```
# Bind ctrl-r but not up arrow
eval "$(atuin init zsh --disable-up-arrow)"

# Bind up-arrow but not ctrl-r
eval "$(atuin init zsh --disable-ctrl-r)"
```

If you do not want either key to be bound, either pass both `--disable` arguments, or set the
environment variable `ATUIN_NOBIND` to any value before the call to `atuin init`:

```
## Do not bind any keys
# Either:
eval "$(atuin init zsh --disable-up-arrow --disable-ctrl-r)"

# Or:
export ATUIN_NOBIND="true"
eval "$(atuin init zsh)"
```

You can then choose to bind Atuin if needed, do this after the call to init.

### zsh

Atuin defines the ZLE widget "\_atuin_search_widget"

```
export ATUIN_NOBIND="true"
eval "$(atuin init zsh)"

bindkey '^r' _atuin_search_widget

# depends on terminal mode
bindkey '^[[A' _atuin_search_widget
bindkey '^[OA' _atuin_search_widget
```

### bash

```
export ATUIN_NOBIND="true"
eval "$(atuin init bash)"

# bind to ctrl-r, add any other bindings you want here too
bind -x '"\C-r": __atuin_history'
```

### fish

```
set -gx ATUIN_NOBIND "true"
atuin init fish | source

# bind to ctrl-r in normal and insert mode, add any other bindings you want here too
bind \cr _atuin_search
bind -M insert \cr _atuin_search
```

# nu

```
set-env ATUIN_NOBIND = true
atuin init nu | save -f ~/.local/share/atuin/init.nu #make sure you created the directory beforehand with `mkdir ~/.local/share/atuin/init.nu`
source ~/.local/share/atuin/init.nu

#bind to ctrl-r in emacs, vi_normal and vi_insert modes, add any other bindings you want here too
let-env config = (
    $env.config | upsert keybindings (
        $env.config.keybindings
        | append {
            name: atuin
            modifier: control
            keycode: char_r
            mode: [emacs, vi_normal, vi_insert]
            event: { send: executehostcommand cmd: (_atuin_search_cmd) }
        }
    )
)
```


## Atuin UI shortcuts

| Shortcut                                  | Action                                                                        |
| ----------------------------------------- | ----------------------------------------------------------------------------- |
| ctrl + r                                  | Cycle through filter modes                                                    |
| ctrl + s                                  | Cycle through search modes                                                    |
| alt + 1 to alt + 9                        | Select item by the number located near it                                     |
| ctrl + c / ctrl + d / ctrl + g / esc      | Return original                                                               |
| ctrl + ⬅︎ / alt + b                       | Move the cursor to the previous word                                          |
| ctrl + ➡️ / alt + f                       | Move the cursor to the next word                                              |
| ctrl + h / ctrl + b / ⬅︎                  | Move the cursor to the left                                                   |
| ctrl + l / ctrl + f / ➡️                  | Move the cursor to the right                                                  |
| ctrl + a / home                           | Move the cursor to the start of the line                                      |
| ctrl + e / end                            | Move the cursor to the end of the line                                        |
| ctrl + backspace / ctrl + alt + backspace | Remove the previous word / remove the word just before the cursor             |
| ctrl + delete / ctrl + alt + delete       | Remove the next word or the word just after the cursor                        |
| ctrl + w                                  | Remove the word before the cursor even if it spans across the word boundaries |
| ctrl + u                                  | Clear the current line                                                        |
| ctrl + n / ctrl + j / ⬆                  | Select the next item on the list                                              |
| ctrl + p / ctrl + k / ⬇                  | Select the previous item on the list                                          |
| page down                                 | Scroll search results one page down                                           |
| page up                                   | Scroll search results one page up                                             |
| enter                                     | Select highlighted item                                                       |
| ⬇ (with no entry selected)               | Return original or return query depending on settings                         |
| ⬇                                        | Select the next item on the list                                              |

