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

func collapseAll(tree *tview.TreeView) {
	for _, child := range tree.GetRoot().GetChildren() {
		child.CollapseAll()
	}
}

func collapseAllRecursive(node *tview.TreeNode) {
	for _, child := range node.GetChildren() {
		child.CollapseAll()
		collapseAllRecursive(child)
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
				groupTagText := fmt.Sprintf("%04x", e.Tag.Group)
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
				elementText := fmt.Sprintf("\t%04x %s", e.Tag.Element, tagName)
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
