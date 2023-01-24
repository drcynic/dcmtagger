package main

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestSomething(t *testing.T) {
	assert := assert.New(t)

	// assert equality
	input := 16
	assert.Equal(16, input, "just a test test")
}
