package main

import (
	"fmt"
	"os"

	"github.com/rivo/tview"
	"github.com/suyashkumar/dicom"
	"github.com/suyashkumar/dicom/pkg/tag"
)

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
		// os.Exit(1)
	} else {
		dataset, err := dicom.ParseFile(path, nil)
		if err != nil {
			return datasetsWithFilename, err
		}
		datasetsWithFilename = append(datasetsWithFilename, DatasetEntry{pathInfo.Name(), dataset})
	}

	return datasetsWithFilename, err
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

			var tagName string
			if tagInfo, err := tag.Find(e.Tag); err == nil {
				tagName = tagInfo.Name
			}

			elementText := fmt.Sprintf("\t%04x %s", e.Tag.Element, tagName)
			elementNode := tview.NewTreeNode(elementText).SetSelectable(true).SetReference(e)
			currentGroupNode.AddChild(elementNode)
		}
	}

	return tree, root
}

func sortTreeByTag(rootDir string, tree *tview.TreeView, datasetsWithFilename []DatasetEntry) (*tview.TreeView, *tview.TreeNode) {
	if len(datasetsWithFilename) == 1 {
		return sortTreeByFilename(rootDir, tree, datasetsWithFilename) // sortying by tag doesn't make sense for single file
	}

	if tree.GetRoot() != nil {
		tree.GetRoot().ClearChildren()
	}

	root := tview.NewTreeNode(rootDir).SetSelectable(true)
	tree.SetRoot(root).SetCurrentNode(root)
	groupNodesByGroupTag := make(map[uint16]*tview.TreeNode)
	tagNodesByTag := make(map[tag.Tag]*tview.TreeNode)
	for _, entry := range datasetsWithFilename {
		for _, e := range entry.dataset.Elements {
			currentGroupNode, ok := groupNodesByGroupTag[e.Tag.Group]
			if !ok {
				// currentGroup = e.Tag.Group
				groupTagText := fmt.Sprintf("%04x/", e.Tag.Group)
				currentGroupNode = tview.NewTreeNode(groupTagText).SetSelectable(true)
				root.AddChild(currentGroupNode)
				groupNodesByGroupTag[e.Tag.Group] = currentGroupNode
			}

			tagNode, ok := tagNodesByTag[e.Tag]
			if !ok {
				var tagName string
				if tagInfo, err := tag.Find(e.Tag); err == nil {
					tagName = tagInfo.Name
				}
				elementText := fmt.Sprintf("\t%04x %s/", e.Tag.Element, tagName)
				tagNode = tview.NewTreeNode(elementText).SetSelectable(true).SetReference(e)
				currentGroupNode.AddChild(tagNode)
				tagNodesByTag[e.Tag] = tagNode
			}

			elementNode := tview.NewTreeNode(entry.filename).SetSelectable(true).SetReference(e)
			tagNode.AddChild(elementNode)
		}
	}
	return tree, root
}

func sortTreeByTagUnique(rootDir string, tree *tview.TreeView, datasetsWithFilename []DatasetEntry) (*tview.TreeView, *tview.TreeNode) {
	if len(datasetsWithFilename) == 1 {
		return sortTreeByFilename(rootDir, tree, datasetsWithFilename) // sortying by tag doesn't make sense for single file
	}

	if tree.GetRoot() != nil {
		tree.GetRoot().ClearChildren()
	}

	root := tview.NewTreeNode(rootDir).SetSelectable(true)
	tree.SetRoot(root).SetCurrentNode(root)

	valuesByTag := make(map[tag.Tag]map[string]bool)
	for _, entry := range datasetsWithFilename {
		for _, e := range entry.dataset.Elements {
			_, ok := valuesByTag[e.Tag]
			if !ok {
				valuesByTag[e.Tag] = make(map[string]bool)
			}
			valuesByTag[e.Tag][e.Value.String()] = true
		}
	}

	groupNodesByGroupTag := make(map[uint16]*tview.TreeNode)
	tagNodesByTag := make(map[tag.Tag]*tview.TreeNode)
	for _, entry := range datasetsWithFilename {
		for _, e := range entry.dataset.Elements {
			currentGroupNode, ok := groupNodesByGroupTag[e.Tag.Group]
			if !ok {
				// currentGroup = e.Tag.Group
				groupTagText := fmt.Sprintf("%04x/", e.Tag.Group)
				currentGroupNode = tview.NewTreeNode(groupTagText).SetSelectable(true)
				root.AddChild(currentGroupNode)
				groupNodesByGroupTag[e.Tag.Group] = currentGroupNode
			}

			valuesForTag := valuesByTag[e.Tag]
			if len(valuesForTag) > 1 {
				// fmt.Printf("multiple values for tag %v\n", e.Tag)
				tagNode, ok := tagNodesByTag[e.Tag]
				if !ok {
					var tagName string
					if tagInfo, err := tag.Find(e.Tag); err == nil {
						tagName = tagInfo.Name
					}
					elementText := fmt.Sprintf("\t%04x %s/", e.Tag.Element, tagName)
					tagNode = tview.NewTreeNode(elementText).SetSelectable(true).SetReference(e)
					currentGroupNode.AddChild(tagNode)
					tagNodesByTag[e.Tag] = tagNode
				}

				elementNode := tview.NewTreeNode(entry.filename).SetSelectable(true).SetReference(e)
				tagNode.AddChild(elementNode)
			}
		}
	}
	return tree, root
}
