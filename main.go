package main

import (
	"fmt"
	"os"

	"github.com/alexflint/go-arg"
	"github.com/suyashkumar/dicom"
	"github.com/suyashkumar/dicom/pkg/tag"
	"github.com/suyashkumar/dicom/pkg/vrraw"

	"github.com/rivo/tview"
)

var (
	version = "unknown"
	commit  string
)

type args struct {
	Input string `arg:"positional" help:"The DICOM input file"`
}

func (args) Version() string { return "Version " + version + " (" + commit + ")" }

func main() {
	var args args
	p := arg.MustParse(&args)
	if args.Input == "" {
		p.Fail("missing DICOM input file")
	}

	dataset, err := dicom.ParseFile(args.Input, nil)
	if err != nil {
		fmt.Printf("Could not read DICOM file: '%s'\n", err.Error())
		os.Exit(1)
	}

	// create tree nodes with dicom tags
	rootDir := "."
	root := tview.NewTreeNode(rootDir)
	tree := tview.NewTreeView().SetRoot(root).SetCurrentNode(root)
	var currentGroupNode *tview.TreeNode
	var currentGroup uint16
	for _, e := range dataset.Elements {
		if currentGroup != e.Tag.Group {
			currentGroup = e.Tag.Group
			groupTagText := fmt.Sprintf("%04x", e.Tag.Group)
			currentGroupNode = tview.NewTreeNode(groupTagText).SetSelectable(true)
			root.AddChild(currentGroupNode)
			//fmt.Printf("%s\n", groupTagText)
		}

		var tagName string
		if tagInfo, err := tag.Find(e.Tag); err == nil {
			tagName = tagInfo.Name
		}

		var value string
		if e.RawValueRepresentation != vrraw.Sequence && e.ValueLength < 150 {
			value = e.Value.String()
		}
		elementText := fmt.Sprintf("\t%04x %s (%s): %s", e.Tag.Element, tagName, e.RawValueRepresentation, value)
		elementNode := tview.NewTreeNode(elementText).SetSelectable(true)
		currentGroupNode.AddChild(elementNode)
		//fmt.Printf("\t%s\n", elementText)
	}

	tree.SetSelectedFunc(func(node *tview.TreeNode) {
		node.SetExpanded(!node.IsExpanded())
	})

	if err := tview.NewApplication().SetRoot(tree, true).Run(); err != nil {
		panic(err)
	}
}

func myFunc(param int) int {
	return param + 4
}
