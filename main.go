package main

import (
	"fmt"
	"strings"

	"github.com/alexflint/go-arg"
	"github.com/gdamore/tcell/v2"
	"github.com/rivo/tview"
	"github.com/suyashkumar/dicom"
	"github.com/suyashkumar/dicom/pkg/tag"
	"github.com/suyashkumar/dicom/pkg/vrraw"
)

var version = "unknown"

type args struct {
	Input string `arg:"positional" help:"The DICOM input file or directory"`
}

func (args) Version() string { return "Version " + version }

type EditMode int

const (
	TreeMode EditMode = iota
	CmdlineMode
)

type DatasetEntry struct {
	filename string
	dataset  dicom.Dataset
}

func main() {
	var args args
	p := arg.MustParse(&args)
	if args.Input == "" {
		p.Fail("Missing DICOM input file")
	}

	datasetsByFilename, err := parseDicomFiles(args.Input)
	if err != nil {
		fmt.Printf("Error reading input: '%s'\n", err.Error())
		return
	}

	// create tree nodes with dicom tags
	app := tview.NewApplication()

	rootDir := args.Input
	tree := tview.NewTreeView()
	tree, root := sortTreeByFilename(rootDir, tree, datasetsByFilename[:])
	collapseAllRecursive(root)

	app.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		switch event.Key() {
		case tcell.KeyRune:
			switch event.Rune() {
			case '1':
				tree, root = sortTreeByFilename(rootDir, tree, datasetsByFilename[:])
				collapseAllRecursive(root)
			case '2':
				tree, root = sortTreeByTag(rootDir, tree, datasetsByFilename[:])
				collapseAllRecursive(root)
			}
		}
		return event
	})

	tagDescriptionViews := tagDescView()
	cmdline := tview.NewInputField()

	cmdline.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		switch event.Key() {
		case tcell.KeyEsc:
			cmdline.SetText("")
			app.SetFocus(tree)
			return nil
		case tcell.KeyEnter:
			cmdlineText := cmdline.GetText()
			if strings.HasPrefix(cmdlineText, ":") {
				if cmdlineText == ":q" {
					app.Stop()
					return nil
				}
			}
		}

		return event
	})

	cmdline.SetChangedFunc(func(text string) {
		cmdlineText := text // cmdline.GetText()
		if strings.HasPrefix(cmdlineText, "/") && len(cmdlineText) > 1 {
			searchText := cmdlineText[1:]
			searchText = strings.ToLower(searchText)
			for _, child := range root.GetChildren() {
				for _, element := range child.GetChildren() {
					if strings.Contains(strings.ToLower(element.GetText()), searchText) {
						tree.SetCurrentNode(element)
						break
					}
				}
			}
		}
	})

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
			// elementText := fmt.Sprintf("\t%04x %s (%s): %s", e.Tag.Element, tagName, e.RawValueRepresentation, value)
		}
	})

	// key handlings
	tree.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		switch key := event.Key(); key {
		case tcell.KeyCtrlD:
			_, _, _, height := tree.GetInnerRect()
			tree.Move(height / 2)
			return nil
		case tcell.KeyCtrlU:
			_, _, _, height := tree.GetInnerRect()
			tree.Move(-height / 2)
			return nil
		case tcell.KeyRune:
			switch event.Rune() {
			case '/':
				app.SetFocus(cmdline)
				cmdline.SetText("/")
				return nil
			case ':':
				app.SetFocus(cmdline)
				cmdline.SetText(":")
				return nil
			case 'E':
				for _, child := range root.GetChildren() {
					child.ExpandAll()
				}
				return nil
			case 'C':
				collapseAll(tree)
				return nil
			case 'J':
				currentNode := tree.GetCurrentNode()
				expaned := currentNode.IsExpanded()
				if expaned {
					currentNode.Collapse()
				}
				tree.Move(1)
				if expaned {
					currentNode.Expand()
				}
				return nil
			case 'K':
				currentNode := tree.GetCurrentNode()
				level := currentNode.GetLevel()
				nextNode := tree.Move(-1).GetCurrentNode()
				for nextNode != root && nextNode.GetLevel() != level {
					nextNode = tree.Move(-1).GetCurrentNode()
				}
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
	grid := tview.NewGrid().SetRows(2, 1, 1, 1, -1).SetColumns(-1, -4)

	header := tview.NewTextView().SetTextAlign(tview.AlignCenter).SetText("<tag name here>")

	vrLabel := tview.NewTextView().SetTextAlign(tview.AlignRight).SetText("VR: ")
	vr := tview.NewTextView().SetTextAlign(tview.AlignLeft).SetText("PN")

	lengthLabel := tview.NewTextView().SetTextAlign(tview.AlignRight).SetText("Length: ")
	length := tview.NewTextView().SetTextAlign(tview.AlignLeft).SetText("123")

	valueLabel := tview.NewTextView().SetTextAlign(tview.AlignRight).SetText("Value: ")
	value := tview.NewTextView().SetTextAlign(tview.AlignLeft).SetText("SOMATOM Definition")

	grid.AddItem(header, 0, 0, 1, 2, 0, 0, false)

	grid.AddItem(vrLabel, 1, 0, 1, 1, 0, 0, false)
	grid.AddItem(vr, 1, 1, 1, 1, 0, 0, false)

	grid.AddItem(lengthLabel, 2, 0, 1, 1, 0, 0, false)
	grid.AddItem(length, 2, 1, 1, 1, 0, 0, false)

	grid.AddItem(valueLabel, 3, 0, 1, 1, 0, 0, false)
	grid.AddItem(value, 3, 1, 1, 1, 0, 0, false)

	return &tagDescViews{grid, header, vr, length, value}
}
