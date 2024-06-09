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

type SortMode int

const (
	ByFilename SortMode = iota
	ByTag
	ByTagDiffsOnly
)

type DatasetEntry struct {
	filename string
	dataset  dicom.Dataset
}

func main() {
	var args args
	p := arg.MustParse(&args)
	if args.Input == "" {
		p.Fail("Missing DICOM input file or directory")
	}

	datasetsByFilename, err := parseDicomFiles(args.Input)
	if err != nil {
		fmt.Printf("Error reading input: '%s'\n", err.Error())
		return
	}

	// global state
	searchText := ""
	sortMode := ByFilename
	showTagValueSummaryList := true

	// create tree nodes with dicom tags
	app := tview.NewApplication()

	rootDir := args.Input

	pages := tview.NewPages()

	singleTagDescViews := singleTagDescView()
	multipleTagsDescViews := multipleTagsDescView()
	statusLine := tview.NewTextView()

	tree := tview.NewTreeView()
	tree, root := sortTreeByFilename(rootDir, tree, datasetsByFilename[:])
	collapseAllRecursive(root)
	statusLine.SetText("Sort by filename")
	cmdline := tview.NewInputField().SetFieldBackgroundColor(tcell.ColorBlack)

	app.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		switch event.Key() {
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
			case '?':
				addAndShowHelpPage(pages)
				return nil
			}
		}
		return event
	})

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
				if cmdlineText == ":" {
					cmdline.SetText("")
					app.SetFocus(tree)
					return nil
				}
			}
			if strings.HasPrefix(cmdlineText, "/") {
				app.SetFocus(tree)
				return nil
			}
		}

		return event
	})

	mainGrid := tview.NewGrid().
		SetRows(-1, 1, 1).
		SetColumns(-1, -2).
		SetBorders(true).
		AddItem(tree, 0, 0, 1, 1, 0, 0, true).
		AddItem(multipleTagsDescViews.grid, 0, 1, 1, 1, 0, 0, false).
		AddItem(statusLine, 1, 0, 1, 2, 0, 0, false).
		AddItem(cmdline, 2, 0, 1, 2, 0, 0, false)

	changedHandler := func(node *tview.TreeNode) {
		if len(node.GetChildren()) > 0 || node.GetReference() == nil {
			mainGrid.RemoveItem(singleTagDescViews.grid)
			mainGrid.AddItem(multipleTagsDescViews.grid, 0, 1, 1, 1, 0, 0, false)
			multipleTagsDescViews.valueList.Clear()

			if showTagValueSummaryList && sortMode != ByFilename && node.GetReference() != nil {
				e := node.GetReference().(*dicom.Element)
				var tagName string
				if tagInfo, err := tag.Find(e.Tag); err == nil {
					tagName = tagInfo.Name
				}
				multipleTagsDescViews.tagNameView.SetText(tagName)
				multipleTagsDescViews.vrLabel.SetText("VR: ")
				multipleTagsDescViews.vrView.SetText(e.RawValueRepresentation)

				entryNumber := 1
				for _, child := range node.GetChildren() {
					var value string
					e := child.GetReference().(*dicom.Element)
					if e.RawValueRepresentation != vrraw.Sequence && e.ValueLength < 150 {
						value = e.Value.String()
					}
					entryText := fmt.Sprintf("%5d    length: %d    value: %s", entryNumber, e.ValueLength, value)
					multipleTagsDescViews.valueList.AddItem(entryText, "", 0, nil)
					entryNumber++
				}
			} else {
				multipleTagsDescViews.tagNameView.SetText("")
				multipleTagsDescViews.vrLabel.SetText("")
				multipleTagsDescViews.vrView.SetText("")
			}
		} else {
			mainGrid.RemoveItem(multipleTagsDescViews.grid)
			mainGrid.AddItem(singleTagDescViews.grid, 0, 1, 1, 1, 0, 0, false)

			e := node.GetReference().(*dicom.Element)
			var tagName string
			if tagInfo, err := tag.Find(e.Tag); err == nil {
				tagName = tagInfo.Name
			}
			singleTagDescViews.tagNameView.SetText(tagName)
			singleTagDescViews.vrView.SetText(e.RawValueRepresentation)
			singleTagDescViews.lengthView.SetText(fmt.Sprint(e.ValueLength))

			var value string
			if e.RawValueRepresentation != vrraw.Sequence && e.ValueLength < 150 {
				value = e.Value.String()
			}
			singleTagDescViews.valueView.SetText(value)
			// elementText := fmt.Sprintf("\t%04x %s (%s): %s", e.Tag.Element, tagName, e.RawValueRepresentation, value)
		}
	}

	tree.SetChangedFunc(changedHandler)

	cmdline.SetChangedFunc(func(text string) {
		cmdlineText := text
		if strings.HasPrefix(cmdlineText, "/") && len(cmdlineText) > 1 {
			searchText = strings.ToLower(cmdlineText[1:])
			jumpToNthFoundNode(searchText, 0, tree)
		}
	})

	tree.SetSelectedFunc(func(node *tview.TreeNode) {
		node.SetExpanded(!node.IsExpanded())
	})

	// key handlings
	tree.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		currentNode := tree.GetCurrentNode()

		switch key := event.Key(); key {
		case tcell.KeyCtrlD:
			_, _, _, height := tree.GetInnerRect()
			tree.Move(height / 2)
		case tcell.KeyCtrlU:
			_, _, _, height := tree.GetInnerRect()
			tree.Move(-height / 2)
		case tcell.KeyLeft:
			if event.Modifiers() == tcell.ModShift {
				moveToParent(tree)
			} else {
				collapseOrMoveToParent(tree)
			}
		case tcell.KeyRight:
			if event.Modifiers() == tcell.ModShift {
				moveToFirstChild(tree)
			} else {
				expandOrMoveToFirstChild(tree)
			}
		case tcell.KeyUp:
			if event.Modifiers() == tcell.ModShift {
				moveUpSameLevel(tree)
			} else {
				return event // not handled, pass on
			}
		case tcell.KeyDown:
			if event.Modifiers() == tcell.ModShift {
				moveDownSameLevel(tree)
			} else {
				return event // not handled, pass on
			}
		case tcell.KeyHome:
			jumpToRoot(tree)
		case tcell.KeyEnd:
			jumpToLastVisibleNode(tree)
		case tcell.KeyRune:
			switch event.Rune() {
			case '1':
				tree, root = sortTreeByFilename(rootDir, tree, datasetsByFilename[:])
				collapseAllRecursive(root)
				sortMode = ByFilename
				statusLine.SetText("Sort by filename")
			case '2':
				tree, root = sortTreeByTag(rootDir, tree, datasetsByFilename[:])
				collapseAllLeaves(root)
				sortMode = ByTag
				statusLine.SetText("Sort by tag")
			case '3':
				tree, root = sortTreeByUniqueTags(rootDir, tree, datasetsByFilename[:])
				collapseAllLeaves(root)
				sortMode = ByTagDiffsOnly
				statusLine.SetText("Sort by tag, show only different tag values")
			case 'q':
				app.Stop()
			case 'J':
				moveDownSameLevel(tree)
			case 'K':
				moveUpSameLevel(tree)
			case 'h':
				collapseOrMoveToParent(tree)
			case 'l':
				expandOrMoveToFirstChild(tree)
			case 'H':
				moveToParent(tree)
			case 'L':
				moveToFirstChild(tree)
			case '0', '^':
				moveToFirstSibling(tree)
			case '$':
				moveToLastSibling(tree)
			case 'e':
				expandCurrentAndAllSiblings(tree)
			case 'c':
				collapseCurrentAndAllSiblings(tree)
			case 'E':
				currentNode.ExpandAll()
			case 'C':
				currentNode.CollapseAll()
			case 'g':
				jumpToRoot(tree)
			case 'G':
				jumpToLastVisibleNode(tree)
			case 'n':
				jumpToNextFoundNode(searchText, tree)
			case 'N':
				jumpToPrevFoundNode(searchText, tree)
			case 'v':
				showTagValueSummaryList = !showTagValueSummaryList
				changedHandler(tree.GetCurrentNode())

			default:
				return event // not handled, pass on
			}
		default:
			return event // not handled, pass on
		}

		if currentNode != tree.GetCurrentNode() {
			changedHandler(tree.GetCurrentNode())
		}

		return nil
	})

	pages.AddPage("main", mainGrid, true, true)

	if err := app.SetRoot(pages, true).Run(); err != nil {
		panic(err)
	}
}

type singleTagDescViews struct {
	grid                                       *tview.Grid
	tagNameView, vrView, lengthView, valueView *tview.TextView
	valueList                                  *tview.List
}

func singleTagDescView() *singleTagDescViews {
	grid := tview.NewGrid().SetRows(2, 1, 1, 1, -1).SetColumns(-1, -4)

	header := tview.NewTextView().SetTextAlign(tview.AlignCenter)

	vrLabel := tview.NewTextView().SetTextAlign(tview.AlignRight).SetText("VR: ")
	vr := tview.NewTextView().SetTextAlign(tview.AlignLeft)

	lengthLabel := tview.NewTextView().SetTextAlign(tview.AlignRight).SetText("Length: ")
	length := tview.NewTextView().SetTextAlign(tview.AlignLeft)

	valueLabel := tview.NewTextView().SetTextAlign(tview.AlignRight).SetText("Value: ")
	value := tview.NewTextView().SetTextAlign(tview.AlignLeft)

	grid.AddItem(header, 0, 0, 1, 2, 0, 0, false)

	grid.AddItem(vrLabel, 1, 0, 1, 1, 0, 0, false)
	grid.AddItem(vr, 1, 1, 1, 1, 0, 0, false)

	grid.AddItem(lengthLabel, 2, 0, 1, 1, 0, 0, false)
	grid.AddItem(length, 2, 1, 1, 1, 0, 0, false)

	grid.AddItem(valueLabel, 3, 0, 1, 1, 0, 0, false)
	grid.AddItem(value, 3, 1, 1, 1, 0, 0, false)

	valueList := tview.NewList().ShowSecondaryText(false)
	grid.AddItem(valueList, 4, 0, 2, 2, 0, 0, false)

	return &singleTagDescViews{grid, header, vr, length, value, valueList}
}

type multipleTagsDescViews struct {
	grid                         *tview.Grid
	tagNameView, vrLabel, vrView *tview.TextView
	valueList                    *tview.List
}

func multipleTagsDescView() *multipleTagsDescViews {
	grid := tview.NewGrid().SetRows(2, 2, -1).SetColumns(-1, -4)

	header := tview.NewTextView().SetTextAlign(tview.AlignCenter)

	vrLabel := tview.NewTextView().SetTextAlign(tview.AlignRight)
	vr := tview.NewTextView().SetTextAlign(tview.AlignLeft)

	grid.AddItem(header, 0, 0, 1, 2, 0, 0, false)

	grid.AddItem(vrLabel, 1, 0, 1, 1, 0, 0, false)
	grid.AddItem(vr, 1, 1, 1, 1, 0, 0, false)

	valueList := tview.NewList().ShowSecondaryText(false)
	grid.AddItem(valueList, 2, 0, 2, 2, 0, 0, false)

	return &multipleTagsDescViews{grid, header, vrLabel, vr, valueList}
}
