package main

import (
	"fmt"
	"os"

	"github.com/alexflint/go-arg"
	"github.com/gdamore/tcell/v2"
	"github.com/rivo/tview"
	"github.com/suyashkumar/dicom"
	"github.com/suyashkumar/dicom/pkg/tag"
	"github.com/suyashkumar/dicom/pkg/vrraw"
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
	app := tview.NewApplication()
	rootDir := ""
	root := tview.NewTreeNode(rootDir).SetSelectable(false)
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

	tree.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		switch event.Rune() {
		case 'E':
			for _, child := range root.GetChildren() {
				child.ExpandAll()
			}
			return nil
		case 'C':
			for _, child := range root.GetChildren() {
				child.CollapseAll()
			}
			return nil
		case 'q':
			app.Stop()
			return nil
		}

		return event
	})

	if err := app.SetRoot(tree, true).Run(); err != nil {
		panic(err)
	}
}

func myFunc(param int) int {
	return param + 4
}
