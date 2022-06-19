package main

import (
	"fmt"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestSomething(t *testing.T) {
	fmt.Println("aber hallo")

	assert := assert.New(t)

	// assert equality
	input := 12
	assert.Equal(16, myFunc(input), "they should be equal")
}
