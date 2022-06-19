package main

import (
	"fmt"
	"log"
	"os"

	"github.com/alexflint/go-arg"
	"github.com/suyashkumar/dicom"
	"github.com/suyashkumar/dicom/pkg/tag"
	"github.com/suyashkumar/dicom/pkg/vrraw"

	ui "github.com/gizak/termui/v3"
	"github.com/gizak/termui/v3/widgets"
)

var (
	version = "unknown"
	commit  string
)

type args struct {
	Input string `arg:"positional" help:"The DICOM input file"`
}

func (args) Version() string { return "Version " + version + " (" + commit + ")" }


// tree nodes
type nodeValue string

func (nv nodeValue) String() string {
	return string(nv)
}

func main() {
	var args args
	p := arg.MustParse(&args)
	if args.Input == "" {
		p.Fail("missing DICOM input file")
	}

	// init UI
	if err := ui.Init(); err != nil {
		log.Fatalf("failed to initialize termui: %v", err)
	}
	defer ui.Close()

	dataset, err := dicom.ParseFile(args.Input, nil)
	if err != nil {
		fmt.Printf("Could not read DICOM file: '%s'\n", err.Error())
		os.Exit(1)
	}

	// create tree nodes with dicom tags
	var nodes []*widgets.TreeNode
	var currentGroupNode *widgets.TreeNode
	var currentGroup uint16
	for _, e := range dataset.Elements {
		if currentGroup != e.Tag.Group {
			currentGroup = e.Tag.Group
			groupTagText := fmt.Sprintf("%04x", e.Tag.Group)
			currentGroupNode = &widgets.TreeNode{Value: nodeValue(groupTagText), Nodes: []*widgets.TreeNode{}}
			nodes = append(nodes, currentGroupNode)
		}

		var tagName string
		if tagInfo, err := tag.Find(e.Tag); err == nil {
			tagName = tagInfo.Name
		}

		var value string
		if e.RawValueRepresentation != vrraw.Sequence && e.ValueLength < 150 {
			value = e.Value.String()
		}
		elementText := fmt.Sprintf("\t%04x %s (%s): %s\n", e.Tag.Element, tagName, e.RawValueRepresentation, value)
		elementNode := &widgets.TreeNode{Value: nodeValue(elementText), Nodes: nil}
		currentGroupNode.Nodes = append(currentGroupNode.Nodes, elementNode)
	}

	l := widgets.NewTree()
	l.TextStyle = ui.NewStyle(ui.ColorYellow)
	l.WrapText = false
	l.SetNodes(nodes)

	x, y := ui.TerminalDimensions()

	l.SetRect(0, 0, x, y)

	ui.Render(l)

	previousKey := ""
	uiEvents := ui.PollEvents()
	for {
		e := <-uiEvents
		switch e.ID {
		case "q", "<C-c>":
			return
		case "j", "<Down>":
			l.ScrollDown()
		case "k", "<Up>":
			l.ScrollUp()
		case "<C-d>":
			l.ScrollHalfPageDown()
		case "<C-u>":
			l.ScrollHalfPageUp()
		case "<C-f>":
			l.ScrollPageDown()
		case "<C-b>":
			l.ScrollPageUp()
		case "g":
			if previousKey == "g" {
				l.ScrollTop()
			}
		case "<Home>":
			l.ScrollTop()
		case "<Enter>":
			l.ToggleExpand()
		case "l":
			l.Expand()
		case "h":
			l.Collapse()
		case "G", "<End>":
			l.ScrollBottom()
		case "E":
			l.ExpandAll()
		case "C":
			l.CollapseAll()
		case "<Resize>":
			x, y := ui.TerminalDimensions()
			l.SetRect(0, 0, x, y)
		}

		if previousKey == "g" {
			previousKey = ""
		} else {
			previousKey = e.ID
		}

		ui.Render(l)
	}

}

func myFunc(param int) int {
	return param + 4
}
