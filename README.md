`wd` is a Terminal-UI (TUI) viewer for massive, spammy logfiles (and less annoying files, too).

# Usage
`wd mylog.txt`

Keybindings are the same as less/vim:
  - `j` and `k` as arrow-keys for navigating up and down.
  - `gg` to go to beginning, `G` to go to end
    - Try pressing `g` once and reading the help of the menu that pops up :-)
    You can go to a particular timestamp in the file, or shift ahead by 5 minutes...assuming your timestamps were successfully auto-parsed.
  - `l` opens a log of wd's operations, to peek under the hood.

## Provided magic:

# What's up with the name?
`wd` is named in the tradition of `less`. `less` is often used for massive files due to its limited resource usage
and responsive performance; we target using no more resources than less would have, and often less for common log viewing workflows.
Offline, if you wanted `less rust`, you would use WD-40, hence the name of this app.