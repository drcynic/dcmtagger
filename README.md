# A simple DICOM tag viewer

## Navigation

### Global

```
  q/Esc                - Quit
  1                    - Sort tree by filename
  2                    - Sort tree by tags
  3                    - Sort tree by tags, only showing tags with different values
  /                    - Enter search mode
  ?                    - Show help
```

### Tree view browsing

```
  Enter/Space          - Toggle expand/collapse
  j/↓/ctrl+n           - Move down visible tree structure over all hierarchy levels
  k/↑/ctrl+p           - Move up visible tree structure over all hierarchy levels
  h/←                  - Move to parent or close node
  l/→                  - Expand node or move to first child
  H/shift+←            - Move to parent
  L/shift+→            - Move to next child (expand if collapsed)
  J/shift+↓            - Move to next sibling (same level)
  K/shift+↑            - Move to previous sibling (same level)
  g                    - Move to first element
  G                    - Move to last element
  0/^                  - Move to first sibling
  $                    - Move to last sibling
  c                    - Collapse current node and siblings
  e                    - Expand current node and siblings
  E                    - Expand current node recursively
  C                    - Collapse current node recursively

  ctrl+u               - Move half page up
  ctrl+d               - Move half page down
  ctrl+f/page-down     - Move page down
  ctrl+b/page-up       - Move page up

  n                    - search for next occurence if search text present
  N                    - search for prev occurence if search text present
```

### Search

```
  Esc                  - Leave and clear search text
  Enter                - Leave and keep search text
```
