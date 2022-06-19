package main

import (
	"fmt"

	"github.com/alexflint/go-arg"
	"github.com/suyashkumar/dicom"
	"github.com/suyashkumar/dicom/pkg/tag"
	"github.com/suyashkumar/dicom/pkg/vrraw"
)

var version = "unknown"

type args struct {
	Input string `arg:"positional" help:"The DICOM input file"`
}

func (args) Version() string { return version }

func main() {
	var args args
	arg.MustParse(&args)
	fmt.Printf("input file: %s\n", args.Input)

	dataset, _ := dicom.ParseFile("testdata/test.dcm", nil) // See also: dicom.Parse which has a generic io.Reader API.
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
