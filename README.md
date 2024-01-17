# A simple DICOM tag viewer


## Navigation

### Global

- q - quit
- 1 - sort tree by filenames - under each filename entry the corresponding tags are located
- 2 - sort tree by tags - under each tag the corresponding filenames are located with its values
- 3 - sort tree by tags and show only the tags which contains different tag values per file
- / - enter command line with search
- : - enter command line with command
- ? - help view

### Treeview

- j,↓ - move down in visible tree structure over all hierarchy levels
- k, ↑ - move up in visible tree structure over all hierarchy levels
- shift + j, shift + ↓ - move down in current hierarchy level - skips other hierarchy levels
- shift + k, shift + ↑ - move up in current hierarchy level - skips other hierarchy levels
- h, ← - if branch node and expanded: collapse, if leaf or collapsed: move to parent if possible
- l, → - if branch node and collapsed: expand node, if branch node and expanded: move to first child
- shift + H, shift + ← - move to next parent
- shift + l, shift + → - move to next child - if current node is collapsed it will be expanded
- 0, ^ - move to first sibling in current hierachy level
- $ - move to last sibling in current hierachy level

- space, enter - toggle collapse state of current node
- c - collapse current node and all its siblings
- e - expand current node and all its siblings
- shift + c - collapse current node recursively
- shift + e - expand current node recursively

- g, home - go to first node (root)
- shift + g, end - go to last visible node
- ctrl + u - half screen up
- ctrl + d - half screen down
- ctrl + f, page-down - one screen down
- ctrl + b, page-up - one screen up

- n - search for next occurence if search text present
- N - search for prev occurence if search text present

### Commandline

