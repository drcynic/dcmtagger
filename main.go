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
)

type args struct {
	Input string `arg:"positional" help:"The DICOM input file"`
}

func (args) Version() string { return "Version " + version }

type EditMode int

const (
	TreeMode EditMode = iota
	CmdlineMode
)

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
	rootDir := args.Input
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

		elementText := fmt.Sprintf("\t%04x %s", e.Tag.Element, tagName)
		elementNode := tview.NewTreeNode(elementText).SetSelectable(true).SetReference(e)
		currentGroupNode.AddChild(elementNode)
		//fmt.Printf("\t%s\n", elementText)
	}

	tagDescriptionViews := tagDescView()
	cmdline := tview.NewInputField()

	mainGrid := tview.NewGrid().
		SetRows(-1, 1).
		SetColumns(-1, -2).
		SetBorders(true).
		AddItem(tree, 0, 0, 1, 1, 0, 0, true).
		AddItem(tagDescriptionViews.grid, 0, 1, 1, 1, 0, 0, false).
		AddItem(cmdline, 1, 0, 1, 2, 0, 0, false)

	tree.SetSelectedFunc(func(node *tview.TreeNode) {
		node.SetExpanded(!node.IsExpanded())
	})

	tree.SetChangedFunc(func(node *tview.TreeNode) {
		if len(node.GetChildren()) > 0 {
			tagDescriptionViews.tagNameView.SetText("")
			tagDescriptionViews.vrView.SetText("")
			tagDescriptionViews.lengthView.SetText("")
			tagDescriptionViews.valueView.SetText("")
		} else {
			e := node.GetReference().(*dicom.Element)
			var tagName string
			if tagInfo, err := tag.Find(e.Tag); err == nil {
				tagName = tagInfo.Name
			}
			tagDescriptionViews.tagNameView.SetText(tagName)
			tagDescriptionViews.vrView.SetText(e.RawValueRepresentation)
			tagDescriptionViews.lengthView.SetText(fmt.Sprint(e.ValueLength))

			var value string
			if e.RawValueRepresentation != vrraw.Sequence && e.ValueLength < 150 {
				value = e.Value.String()
			}
			tagDescriptionViews.valueView.SetText(value)
			//elementText := fmt.Sprintf("\t%04x %s (%s): %s", e.Tag.Element, tagName, e.RawValueRepresentation, value)
		}
	})

	// key handlings
	tree.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		switch event.Rune() {
		case '/':
			app.SetFocus(cmdline)
			cmdline.SetText("/")
			return nil
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
		case 'J':
			// todo: jump to next node on parent level or if on expandable node to next on same level
			return nil
		case 'h':
			currentNode := tree.GetCurrentNode()
			numChildren := len(currentNode.GetChildren())
			if numChildren == 0 || !currentNode.IsExpanded() {
				return tcell.NewEventKey(tcell.KeyRune, 'K', tcell.ModNone)
			} else {
				currentNode.Collapse()
				return nil
			}
		case 'l':
			currentNode := tree.GetCurrentNode()
			if len(currentNode.GetChildren()) > 0 && !currentNode.IsExpanded() {
				currentNode.SetExpanded(true)
			}
			return nil
		case 'q':
			app.Stop()
			return nil
		}

		return event
	})

	cmdline.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		switch event.Key() {
		case tcell.KeyEsc:
			cmdline.SetText("")
			app.SetFocus(tree)
			return nil
		}
		return event
	})

	if err := app.SetRoot(mainGrid, true).Run(); err != nil {
		panic(err)
	}
}

type tagDescViews struct {
	grid                                       *tview.Grid
	tagNameView, vrView, lengthView, valueView *tview.TextView
}

func tagDescView() *tagDescViews {

	grid := tview.NewGrid().
		SetRows(2, 1, 1, 1, -1).
		SetColumns(-1, -4)

	header := tview.NewTextView().
		SetTextAlign(tview.AlignCenter).
		SetText("<tag name here>")

	vrLabel := tview.NewTextView().
		SetTextAlign(tview.AlignRight).
		SetText("VR: ")

	vr := tview.NewTextView().
		SetTextAlign(tview.AlignLeft).
		SetText("PN")

	lengthLabel := tview.NewTextView().
		SetTextAlign(tview.AlignRight).
		SetText("Length: ")

	length := tview.NewTextView().
		SetTextAlign(tview.AlignLeft).
		SetText("123")

	valueLabel := tview.NewTextView().
		SetTextAlign(tview.AlignRight).
		SetText("Value: ")

	value := tview.NewTextView().
		SetTextAlign(tview.AlignLeft).
		SetText("SOMATOM Definition")

	grid.AddItem(header, 0, 0, 1, 2, 0, 0, false)

	grid.AddItem(vrLabel, 1, 0, 1, 1, 0, 0, false)
	grid.AddItem(vr, 1, 1, 1, 1, 0, 0, false)

	grid.AddItem(lengthLabel, 2, 0, 1, 1, 0, 0, false)
	grid.AddItem(length, 2, 1, 1, 1, 0, 0, false)

	grid.AddItem(valueLabel, 3, 0, 1, 1, 0, 0, false)
	grid.AddItem(value, 3, 1, 1, 1, 0, 0, false)

	return &tagDescViews{grid, header, vr, length, value}
}
