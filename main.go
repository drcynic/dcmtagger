package main

import (
	"fmt"
	"strings"

	"github.com/alexflint/go-arg"
	"github.com/gdamore/tcell/v2"
	"github.com/rivo/tview"
	"github.com/suyashkumar/dicom"
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

func main() {
	var args args
	p := arg.MustParse(&args)
	if args.Input == "" {
		p.Fail("Missing DICOM input file or directory")
	}

	datasetsWithFilename, err := parseDicomFiles(args.Input)
	if err != nil {
		fmt.Printf("Error reading input: '%s'\n", err.Error())
		return
	}

	// global state
	searchText := ""

	// create tree nodes with dicom tags
	app := tview.NewApplication()

	rootDir := args.Input

	pages := tview.NewPages()

	statusLine := tview.NewTextView()

	tree := tview.NewTreeView()
	tree, root := sortTreeByFilename(rootDir, tree, datasetsWithFilename[:])
	collapseAllRecursive(root)
	statusLine.SetText("Sort by filename")
	cmdline := tview.NewInputField().SetFieldBackgroundColor(tcell.ColorBlack)
	mainGrid := tview.NewGrid().
		SetRows(-1, 1, 1).
		SetColumns(-1).
		SetBorders(true).
		AddItem(tree, 0, 0, 1, 1, 0, 0, true).
		AddItem(statusLine, 1, 0, 1, 1, 0, 0, false).
		AddItem(cmdline, 2, 0, 1, 1, 0, 0, false)

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
				} else if cmdlineText == ":w" {
					if len(datasetsWithFilename) == 1 {
						writeDatasetToFile(datasetsWithFilename[0].dataset, "write_test_copy.dcm")
						statusLine.SetText("saved to write_test_copy.dcm")
					}
					cmdline.SetText("")
					app.SetFocus(tree)
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
		case tcell.KeyCtrlSpace:
			if isTagNode(currentNode) {
				addAndShowTagEditingPage(pages, currentNode.GetReference().(*dicom.Element))
			} else {
				return event
			}
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
				tree, root = sortTreeByFilename(rootDir, tree, datasetsWithFilename[:])
				collapseAllRecursive(root)
				statusLine.SetText("Sort by filename")
			case '2':
				tree, root = sortTreeByTags(rootDir, tree, datasetsWithFilename[:], 0)
				collapseAllLeaves(root)
				statusLine.SetText("Sort by tag")
			case '3':
				tree, root = sortTreeByTags(rootDir, tree, datasetsWithFilename[:], 1)
				collapseAllLeaves(root)
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

			default:
				return event // not handled, pass on
			}
		default:
			return event // not handled, pass on
		}

		return nil
	})

	pages.AddPage("main", mainGrid, true, true)

	if err := app.SetRoot(pages, true).Run(); err != nil {
		panic(err)
	}
}
