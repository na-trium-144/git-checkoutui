# git-checkoutui

```sh
cargo install --git https://github.com/na-trium-144/git-checkoutui
```

View a list of git branches and select and checkout them.

```sh
git checkoutui
```

![screenshot](screenshot.png)

Feel free to set your own aliases in .gitconfig and use them.

```ini
[alias]
ct = checkoutui
```

```sh
git ct
```

This TUI does not fetch remotes, it only looks at local remote branch information. Please run `git fetch -p` manually beforehand.
