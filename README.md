# yabaictl

A wrapper around [yabai](https://github.com/koekeishiya/yabai) for better
dual-monitor support.

This is a rewrite of the [python
version](https://github.com/slam/dotfiles/blob/main/yabai/bin/yabaictl) in
rust. It is an excuse to learn rust. 

The wrapper idea originally came from
[aiguofer/dotfiles](https://github.com/aiguofer/dotfiles/blob/master/user/.local/bin/yabaictl).

This requires yabai 4.0.2+.

In 4.0.2, the yabai client/server message format [has
changed](https://github.com/koekeishiya/yabai/commit/ef51c64d50d152c5b88c43b4bed73dd02da7d7cb#).
`yabaictl` only supports the new format.
