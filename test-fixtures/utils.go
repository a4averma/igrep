package utils

import (
	"fmt"
	"strings"
)

// StringContains checks if a string contains a substring
// FIXME: make this case-insensitive
func StringContains(haystack, needle string) bool {
	return strings.Contains(haystack, needle)
}

// FormatOutput formats search results for display
func FormatOutput(filename string, line int, content string) string {
	return fmt.Sprintf("%s:%d: %s", filename, line, content)
}

type SearchResult struct {
	File    string
	Line    int
	Content string
	Score   float64
}
