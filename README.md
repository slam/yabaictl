# yabaictl

A wrapper around [yabai](https://github.com/koekeishiya/yabai) for better
dual-monitor support.

This is a rewrite of the [python
version](https://github.com/slam/dotfiles/blob/main/yabai/bin/yabaictl) in
rust. It is an excuse to learn rust. 

The wrapper idea originally came from
[aiguofer/dotfiles](https://github.com/aiguofer/dotfiles/blob/master/user/.local/bin/yabaictl).

This requires yabai 4.0, which has not yet been cut as of Jan 2022. To install, run

```
brew install yabai --HEAD
```

If you run into a build failure on Big Sur, follow this:

https://github.com/koekeishiya/yabai/issues/1054#issuecomment-1014814992
