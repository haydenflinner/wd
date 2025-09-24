# wd
`wd` is a Terminal-UI (TUI) viewer for massive, spammy logfiles (and less annoying logs, too).

It tries to parse timestamps from the beginning of your log lines, and supports filtering lines
with regexes, as well as navigating the logfile by timestamp.

## Installation
```bash
git clone https://github.com/haydenflinner/wd.git && cd wd && cargo install
```

## Usage
`wd mylog.txt`

### Keybindings
Keybindings are the same as less/vim, plus our additional features of filtering, seeking, and going to timestamps.
  - `j` and `k` as arrow-keys for navigating up and down. `PGUP`/`PGDOWN` work as expected.
  - `gg` to go to beginning, `G` to go to end
      - Try pressing `g` once and reading the help of the menu that pops up `:-)`
        You can go to a particular timestamp in the file, or shift ahead by 5 minutes...assuming your timestamps were successfully auto-parsed.
  - `f` opens the filtering menu, which you can use to "filter-in" (whitelist) or filter-out (blacklist). Filters are ORed together rather than applied in sequence, this is open to change if you submit a PR (since we could use `|` in regex filter to make one regex with OR), because we currently don't support an iterative filtering-down.
  - `/` opens a search, and `n`/`N` navigates the results.
  - `s` uses the Drain algorithm to try to skip until "new-looking" log content is seen. That is, if you're looking at a big screen full of similar looking "spam", you can press `s` to let `wd` attempt to seek to the first log line that looks different than the current screen's contents.
  - `l` opens a log of wd's operations, to peek under the hood.

## Future Work
  - `h`/`?` should open a help menu.
  - `SPC` should open a menu with the above j/g/l/f prompts or named cmds, like Linear or Spacemacs.
  - Fix the occasional crashes
  - Use a segment-tree/BVH/Rope-like datastructure to make operations over filtered data as effecient as they can obviously be. Pathalogical case: Filtering out the majority of a very spammy file, such that on-screen is rendered chunks from across the 10GB+ file, and then scroll up and down some lines.

## What's up with the name?
`wd` is named in the tradition of [less](https://en.wikipedia.org/wiki/Less_(Unix)), which is in turn named after [more](https://en.wikipedia.org/wiki/More_(command)). `less` is often used for massive files due to its limited resource usage
and responsive performance; we target using no more resources than `less` would have, and often less for common log viewing workflows.

Offline, if you wanted `less rust`, you would use WD-40, hence the name of this app.