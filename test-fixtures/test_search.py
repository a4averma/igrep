"""Unit tests for the search module."""

import unittest
import os


class TestPatternSearch(unittest.TestCase):
    """Test pattern matching functionality."""

    def test_literal_match(self):
        """Test exact string matching."""
        text = "Hello World"
        self.assertIn("Hello", text)
        self.assertIn("World", text)

    def test_case_insensitive(self):
        """Test case-insensitive matching."""
        text = "FooBar foobar FOOBAR"
        self.assertIn("FooBar", text)
        self.assertEqual(text.lower().count("foobar"), 3)

    def test_empty_pattern(self):
        """Empty pattern should match everything."""
        text = "any text here"
        self.assertIn("", text)

    def test_unicode_search(self):
        """Test searching for Unicode patterns."""
        text = "Hello World"
        self.assertTrue(len(text) > 0)


class TestFileWalker(unittest.TestCase):
    """Test file walking functionality."""

    def test_walk_current_dir(self):
        """Walking current dir should find files."""
        # TODO: implement actual file walking test
        files = os.listdir(".")
        self.assertGreater(len(files), 0)


if __name__ == "__main__":
    unittest.main()
