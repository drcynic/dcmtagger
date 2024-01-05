# A simple DICOM tag viewer


## Navigation

### Global

- q - quit
- 1 - sort tree by filenames - under each filename entry the corresponding tags are located
- 2 - sort tree by tags - under each tag the corresponding filenames are located with its values
- 3 - sort tree by tags and show only the tags which contains different tag values per file
- / - enter command line with search
- : - enter command line with command

### Treeview

- j,↓ - move down in visible tree structure over all hierarchy levels
- k, ↑ - move up in visible tree structure over all hierarchiy levels
- shift + j, shift + ↓ - move down all nodes in current hierarchy level - skips other hierarchy levels
- shift + k, shift + ↑ - move up all nodes in current hierarchy level - skips other hierarchy levels
- h, ← - move to next parent
- l, → - move to next child - if current node is collapsed it will be expanded

- c - collapse current node if collapsable
- e - expand current node if expandable
- space, enter - toggle collapse state of current node
- shift + c - collapse recursively current node if collapse
- shift + e - expand recursively current node if expandable

- g - go to first node (root)
- shift + g - go to last visible node
- ctrl + u - half screen up in visible tree structure
- ctrl + d - half screen down in visible tree structure

### Commandline

