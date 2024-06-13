package main

import (
	"fmt"
	"os"
	"strings"

	"github.com/gdamore/tcell/v2"
	"github.com/rivo/tview"
	"github.com/suyashkumar/dicom"
	"github.com/suyashkumar/dicom/pkg/tag"
)

type DatasetEntry struct {
	filename string
	dataset  dicom.Dataset
}

var helpText = `Navigation

Global

- q - quit
- 1 - sort tree by filenames - under each filename entry the corresponding tags are located
- 2 - sort tree by tags - under each tag the corresponding filenames are located with its values
- 3 - sort tree by tags and show only the tags which contains different tag values per file
- / - enter command line with search
- : - enter command line with command
- ? - help view

Treeview

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
`

func addAndShowHelpPage(pages *tview.Pages) {
	viewName := "help"
	helpView := tview.NewTextView().SetText(string(helpText))
	helpView.
		SetTitle("Help").
		SetTitleAlign(tview.AlignCenter).
		SetBorder(true).
		SetBorderPadding(1, 1, 1, 1)
	helpView.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		switch event.Key() {
		case tcell.KeyEsc:
			pages.RemovePage(viewName)
			return nil
		case tcell.KeyRune:
			switch event.Rune() {
			case 'q':
				pages.RemovePage(viewName)
				return nil
			}
		}
		return event
	})
	width, height := 120, 40
	grid := tview.NewGrid().
		SetColumns(0, width, 0).
		SetRows(0, height, 0).
		AddItem(helpView, 1, 1, 1, 1, 0, 0, true)
	pages.AddAndSwitchToPage(viewName, grid, true).ShowPage("main")
}

func addAndShowTagEditingPage(pages *tview.Pages, element *dicom.Element) {
	viewName := "TagEditView"

	newValue := ""
	form := tview.NewForm().
		SetItemPadding(0).
		SetFieldBackgroundColor(tcell.ColorDarkBlue).
		SetButtonBackgroundColor(tcell.ColorDarkBlue).
		AddTextView("Tag", fmt.Sprintf("%04x | %04x", element.Tag.Group, element.Tag.Element), 0, 1, false, false).
		AddTextView("Name", getTagName(element), 0, 1, false, false).
		AddTextView("VR", element.RawValueRepresentation, 0, 1, false, false).
		AddTextView("Length", fmt.Sprint(element.ValueLength), 0, 1, false, false).
		AddInputField("Value", getValueString(element), 0, nil, func(text string) {
			newValue = text
		}).
		AddButton("Save", func() {
			stringArray := []string{newValue}
			element.Value, _ = dicom.NewValue(stringArray)
			pages.RemovePage(viewName)
		}).
		AddButton("Cancel", func() {
			pages.RemovePage(viewName)
		})
	form.SetBorder(true).
		SetTitle("Edit Tag Value").
		SetTitleAlign(tview.AlignCenter)
	form.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		switch event.Key() {
		case tcell.KeyEsc:
			pages.RemovePage(viewName)
			return nil
		}
		return event
	})

	modal := func(p tview.Primitive, width, height int) tview.Primitive {
		return tview.NewGrid().
			SetColumns(0, width, 0).
			SetRows(0, height, 0).
			AddItem(p, 1, 1, 1, 1, 0, 0, true)
	}
	pages.AddAndSwitchToPage(viewName, modal(form, 64, 11), true).ShowPage("main")
}

func parseDicomFiles(path string) ([]DatasetEntry, error) {
	datasetsWithFilename := make([]DatasetEntry, 0)
	pathInfo, err := os.Stat(path)
	if err != nil {
		return datasetsWithFilename, err
	}

	if pathInfo.IsDir() {
		dir := pathInfo.Name()
		files, err := os.ReadDir(dir)
		if err != nil {
			return datasetsWithFilename, err
		}

		for _, f := range files {
			if f.IsDir() {
				continue
			}
			dataset, err := dicom.ParseFile(dir+"/"+f.Name(), nil)
			if err != nil {
				return datasetsWithFilename, err
			}
			datasetsWithFilename = append(datasetsWithFilename, DatasetEntry{f.Name(), dataset})
		}
	} else {
		dataset, err := dicom.ParseFile(path, nil)
		if err != nil {
			return datasetsWithFilename, err
		}
		datasetsWithFilename = append(datasetsWithFilename, DatasetEntry{pathInfo.Name(), dataset})
	}

	return datasetsWithFilename, err
}

func writeDatasetToFile(dataset dicom.Dataset, filename string) error {
	file, err := os.Create(filename)
	if err != nil {
		return err
	}
	defer file.Close()
	if err = dicom.Write(file, dataset); err != nil {
		return err
	}
	return nil
}

func isTagNode(node *tview.TreeNode) bool {
	return node.GetReference() != nil
}

func updateTagValue(node *tview.TreeNode, newValue string) {
	if isTagNode(node) {
		e := node.GetReference().(*dicom.Element)
		stringArray := []string{newValue}
		e.Value, _ = dicom.NewValue(stringArray)
	}
}

func findNodeRecursive(tree *tview.TreeView, searchText string) ([]*tview.TreeNode, int) {
	findPred := func(node *tview.TreeNode) bool {
		return strings.Contains(strings.ToLower(node.GetText()), searchText)
	}

	foundNodes := make([]*tview.TreeNode, 0)
	foundIndex := -1
	tree.GetRoot().Walk(func(node, parent *tview.TreeNode) bool {
		if findPred(node) {
			foundNodes = append(foundNodes, node)
		}
		if tree.GetCurrentNode() == node {
			if len(foundNodes) > 0 {
				foundIndex = len(foundNodes) - 1
			} else {
				foundIndex = 0
			}
		}
		return true
	})

	return foundNodes, foundIndex
}

func collapseAllChildren(node *tview.TreeNode) {
	for _, child := range node.GetChildren() {
		child.CollapseAll()
	}
}

func collapseAllRecursive(node *tview.TreeNode) {
	for _, child := range node.GetChildren() {
		child.CollapseAll()
		collapseAllRecursive(child)
	}
}

func collapseAllLeaves(node *tview.TreeNode) {
	canCollapse := true
	for _, child := range node.GetChildren() {
		if len(child.GetChildren()) > 0 {
			collapseAllLeaves(child)
			canCollapse = false
		}
	}
	if canCollapse {
		node.CollapseAll()
	}
}

func collectAllVisible(tree *tview.TreeView) []*tview.TreeNode {
	foundNodes, _ := collectAllVisibleNodesWithPred(tree, func(node *tview.TreeNode) bool { return true }, nil)
	return foundNodes
}

// collects all nodes visible nodes that pass the 'findPred' predicate and additionally returns the index of the node that passed the 'findIdxPred'
func collectAllVisibleNodesWithPred(tree *tview.TreeView, findPred func(node *tview.TreeNode) bool, findIdxPred func(node *tview.TreeNode) bool) ([]*tview.TreeNode, int) {
	foundNodes := make([]*tview.TreeNode, 0)
	foundIndex := -1
	tree.GetRoot().Walk(func(node, parent *tview.TreeNode) bool {
		if findPred(node) {
			foundNodes = append(foundNodes, node)
			if findIdxPred != nil && findIdxPred(node) {
				foundIndex = len(foundNodes) - 1
			}
		}
		return node.IsExpanded()
	})

	return foundNodes, foundIndex
}

func collectSiblings(tree *tview.TreeView, refNode *tview.TreeNode) []*tview.TreeNode {
	foundNodes := make([]*tview.TreeNode, 0)
	tree.GetRoot().Walk(func(node, parent *tview.TreeNode) bool {
		if node == refNode {
			if node == tree.GetRoot() {
				foundNodes = append(foundNodes, node)
			} else {
				foundNodes = parent.GetChildren()
			}
			return false
		}
		return true
	})

	return foundNodes
}

func getParent(tree *tview.TreeView, refNode *tview.TreeNode) *tview.TreeNode {
	var foundNode *tview.TreeNode
	tree.GetRoot().Walk(func(node, parent *tview.TreeNode) bool {
		if node == refNode {
			foundNode = parent
			return false
		}
		return true
	})
	return foundNode
}

func expandPathToNode(tree *tview.TreeView, node *tview.TreeNode) {
	if node == tree.GetRoot() {
		node.Expand()
		return
	}

	parent := getParent(tree, node)
	if parent != nil {
		expandPathToNode(tree, parent)
	} else {
		node.Expand()
	}
	node.Expand()
}

func expandCurrentAndAllSiblings(tree *tview.TreeView) {
	siblings := collectSiblings(tree, tree.GetCurrentNode())
	for _, sibling := range siblings {
		sibling.Expand()
	}
}

func collapseCurrentAndAllSiblings(tree *tview.TreeView) {
	siblings := collectSiblings(tree, tree.GetCurrentNode())
	for _, sibling := range siblings {
		sibling.Collapse()
	}
}

func expandOrMoveToFirstChild(tree *tview.TreeView) {
	currentNode := tree.GetCurrentNode()
	if len(currentNode.GetChildren()) > 0 {
		if currentNode.IsExpanded() {
			tree.SetCurrentNode(currentNode.GetChildren()[0])
		} else {
			currentNode.Expand()
		}
	}
}

func collapseOrMoveToParent(tree *tview.TreeView) {
	currentNode := tree.GetCurrentNode()
	if len(currentNode.GetChildren()) > 0 && currentNode.IsExpanded() {
		currentNode.Collapse()
	} else {
		moveToParent(tree)
	}
}

func moveToFirstChild(tree *tview.TreeView) {
	currentNode := tree.GetCurrentNode()
	if len(currentNode.GetChildren()) > 0 {
		currentNode.SetExpanded(true)
		tree.SetCurrentNode(currentNode.GetChildren()[0])
	}
}

func moveToParent(tree *tview.TreeView) {
	parent := getParent(tree, tree.GetCurrentNode())
	if parent != nil {
		tree.SetCurrentNode(parent)
	}
}

func moveToFirstSibling(tree *tview.TreeView) {
	siblings := collectSiblings(tree, tree.GetCurrentNode())
	if len(siblings) > 0 {
		tree.SetCurrentNode(siblings[0])
	}
}

func moveToLastSibling(tree *tview.TreeView) {
	siblings := collectSiblings(tree, tree.GetCurrentNode())
	if len(siblings) > 0 {
		tree.SetCurrentNode(siblings[len(siblings)-1])
	}
}

func getIsLevelPredicate(level int) func(node *tview.TreeNode) bool {
	return func(node *tview.TreeNode) bool {
		return node.GetLevel() == level
	}
}

func moveUpSameLevel(tree *tview.TreeView) {
	currentNode := tree.GetCurrentNode()
	isLevelPred := getIsLevelPredicate(currentNode.GetLevel())
	isSameNode := func(node *tview.TreeNode) bool { return node == currentNode }
	nodesWithLevel, currentNodeIdx := collectAllVisibleNodesWithPred(tree, isLevelPred, isSameNode)
	if currentNodeIdx > 0 {
		tree.SetCurrentNode(nodesWithLevel[currentNodeIdx-1])
	}
}

func moveDownSameLevel(tree *tview.TreeView) {
	currentNode := tree.GetCurrentNode()
	isLevelPred := getIsLevelPredicate(currentNode.GetLevel())
	isSameNode := func(node *tview.TreeNode) bool { return node == currentNode }
	nodesWithLevel, currentNodeIdx := collectAllVisibleNodesWithPred(tree, isLevelPred, isSameNode)
	if currentNodeIdx < len(nodesWithLevel)-1 {
		tree.SetCurrentNode(nodesWithLevel[currentNodeIdx+1])
	}
}

func jumpToRoot(tree *tview.TreeView) {
	tree.SetCurrentNode(tree.GetRoot())
}

func jumpToLastVisibleNode(tree *tview.TreeView) {
	nodes := collectAllVisible(tree)
	tree.SetCurrentNode(nodes[len(nodes)-1])
}

func jumpToNextFoundNode(searchText string, tree *tview.TreeView) {
	jumpToNthFoundNode(searchText, 1, tree)
}

func jumpToPrevFoundNode(searchText string, tree *tview.TreeView) {
	jumpToNthFoundNode(searchText, -1, tree)
}

func jumpToNthFoundNode(searchText string, offset int, tree *tview.TreeView) {
	if len(searchText) > 1 {
		foundNodes, currentIdx := findNodeRecursive(tree, searchText)
		len := len(foundNodes)
		if len > 0 {
			newNode := foundNodes[(currentIdx+len+offset)%len]
			if newNode != tree.GetCurrentNode() {
				tree.SetCurrentNode(newNode)
				expandPathToNode(tree, newNode)
			}
		}
	}
}

func sortTreeByFilename(rootDir string, tree *tview.TreeView, datasetsWithFilename []DatasetEntry) (*tview.TreeView, *tview.TreeNode) {
	if tree.GetRoot() != nil {
		tree.GetRoot().ClearChildren()
	}
	root := tview.NewTreeNode(rootDir).SetSelectable(true)
	tree.SetRoot(root).SetCurrentNode(root)

	for _, entry := range datasetsWithFilename {
		fileNode := tview.NewTreeNode(entry.filename).SetSelectable(true)
		if len(datasetsWithFilename) == 1 {
			tree.SetRoot(fileNode) // only one file, so this name is root then
		} else {
			root.AddChild(fileNode)
		}

		var currentGroupNode *tview.TreeNode
		var currentGroup uint16
		for _, e := range entry.dataset.Elements {
			if currentGroup != e.Tag.Group {
				currentGroup = e.Tag.Group
				groupTagText := fmt.Sprintf("%04x", e.Tag.Group)
				currentGroupNode = tview.NewTreeNode(groupTagText).SetSelectable(true)
				fileNode.AddChild(currentGroupNode)
			}

			tagName := getTagName(e)
			value := getValueString(e)
			elementText := fmt.Sprintf("\t%04x %s (%s, %d): %s", e.Tag.Element, tagName, e.RawValueRepresentation, e.ValueLength, value)
			elementNode := tview.NewTreeNode(elementText).SetSelectable(true).SetReference(e)
			currentGroupNode.AddChild(elementNode)
		}
	}

	return tree, root
}

func sortTreeByTags(rootDir string, tree *tview.TreeView, datasetsWithFilename []DatasetEntry, minDiffValuesPerTag int) (*tview.TreeView, *tview.TreeNode) {
	if len(datasetsWithFilename) == 1 {
		return sortTreeByFilename(rootDir, tree, datasetsWithFilename) // sortying by tag doesn't make sense for single file
	}

	if tree.GetRoot() != nil {
		tree.GetRoot().ClearChildren()
	}

	root := tview.NewTreeNode(rootDir).SetSelectable(true)
	tree.SetRoot(root).SetCurrentNode(root)

	// todo: this is always the same, calculate at startup and store
	valuesByTag := make(map[tag.Tag]map[string]bool)
	valueLengthsByTag := make(map[tag.Tag]map[uint32]bool)
	for _, entry := range datasetsWithFilename {
		for _, e := range entry.dataset.Elements {
			_, ok := valuesByTag[e.Tag]
			if !ok {
				valuesByTag[e.Tag] = make(map[string]bool)
			}
			valuesByTag[e.Tag][e.Value.String()] = true

			_, ok = valueLengthsByTag[e.Tag]
			if !ok {
				valueLengthsByTag[e.Tag] = make(map[uint32]bool)
			}
			valueLengthsByTag[e.Tag][e.ValueLength] = true
		}
	}

	groupNodesByGroupTag := make(map[uint16]*tview.TreeNode)
	tagNodesByTag := make(map[tag.Tag]*tview.TreeNode)
	for _, entry := range datasetsWithFilename {
		for _, e := range entry.dataset.Elements {
			currentGroupNode, ok := groupNodesByGroupTag[e.Tag.Group]
			if !ok {
				groupTagText := fmt.Sprintf("%04x/", e.Tag.Group)
				currentGroupNode = tview.NewTreeNode(groupTagText).SetSelectable(true)
				root.AddChild(currentGroupNode)
				groupNodesByGroupTag[e.Tag.Group] = currentGroupNode
			}

			valuesForTag := valuesByTag[e.Tag]
			if len(valuesForTag) > minDiffValuesPerTag {
				tagNode, ok := tagNodesByTag[e.Tag]
				if !ok {
					tagName := getTagName(e)
					valueLengthsByTag := valueLengthsByTag[e.Tag]
					valueLengthText := ""
					if len(valueLengthsByTag) == 1 {
						valueLengthText = fmt.Sprintf(", %d", e.ValueLength)
					}
					elementText := fmt.Sprintf("\t%04x %s (%s%s)/", e.Tag.Element, tagName, e.RawValueRepresentation, valueLengthText)
					tagNode = tview.NewTreeNode(elementText).SetSelectable(true).SetReference(e)
					currentGroupNode.AddChild(tagNode)
					tagNodesByTag[e.Tag] = tagNode
				}

				value := getValueString(e)
				elementText := fmt.Sprintf("\t %s (%d)\t - %s", value, e.ValueLength, entry.filename)
				elementNode := tview.NewTreeNode(elementText).SetSelectable(true).SetReference(e)
				tagNode.AddChild(elementNode)
			}
		}
	}
	return tree, root
}

func getTagName(e *dicom.Element) string {
	var tagName string
	if tagInfo, err := tag.Find(e.Tag); err == nil {
		tagName = tagInfo.Name
	}
	return tagName
}

func getValueString(e *dicom.Element) string {
	value := e.Value.String()
	if e.Value.ValueType() == dicom.Strings {
		valueList := e.Value.GetValue().([]string)
		if len(valueList) == 1 {
			value = valueList[0]
		}
	}
	const maxLength = 50
	if len(value) > maxLength {
		value = value[:maxLength-4] + "...]"
	}

	return value
}
