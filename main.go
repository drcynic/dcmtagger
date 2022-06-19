package main

import (
	"fmt"
	"os"

	"github.com/alexflint/go-arg"
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

	var currentGroup uint16
	for _, e := range dataset.Elements {
		if currentGroup != e.Tag.Group {
			currentGroup = e.Tag.Group
			fmt.Printf("%04x\n", currentGroup)
		}

		var tagName string
		if tagInfo, err := tag.Find(e.Tag); err == nil {
			tagName = tagInfo.Name
		}

		var value string
		if e.RawValueRepresentation != vrraw.Sequence && e.ValueLength < 150 {
			value = e.Value.String()
		}
		fmt.Printf("\t%04x %s (%s): %s\n", e.Tag.Element, tagName, e.RawValueRepresentation, value)
	}
}

func myFunc(param int) int {
	return param + 4
}
