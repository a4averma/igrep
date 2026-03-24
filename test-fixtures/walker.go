package walker

import (
	"os"
	"path/filepath"
	"strings"
)

// FileWalker traverses directories and yields file paths
type FileWalker struct {
	Root           string
	IgnoreHidden   bool
	IgnorePatterns []string
}

// NewFileWalker creates a walker with default settings
func NewFileWalker(root string) *FileWalker {
	return &FileWalker{
		Root:           root,
		IgnoreHidden:   true,
		IgnorePatterns: []string{".git", "node_modules"},
	}
}

// Walk traverses the directory tree and calls fn for each file
// FIXME: add support for .gitignore parsing
func (w *FileWalker) Walk(fn func(path string) error) error {
	return filepath.Walk(w.Root, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		name := info.Name()

		// Skip hidden files if configured
		if w.IgnoreHidden && strings.HasPrefix(name, ".") && name != "." {
			if info.IsDir() {
				return filepath.SkipDir
			}
			return nil
		}

		// Skip ignored patterns
		for _, pattern := range w.IgnorePatterns {
			if name == pattern {
				if info.IsDir() {
					return filepath.SkipDir
				}
				return nil
			}
		}

		if !info.IsDir() {
			return fn(path)
		}
		return nil
	})
}
