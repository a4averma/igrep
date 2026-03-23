"""Command-line interface for the search tool."""

import argparse
import sys
import os


def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="Fast code search tool")
    parser.add_argument("pattern", help="Search pattern")
    parser.add_argument("path", nargs="?", default=".", help="Search path")
    parser.add_argument("-i", "--ignore-case", action="store_true")
    parser.add_argument("-n", "--line-number", action="store_true")
    # TODO: add --hidden flag to include dotfiles
    return parser.parse_args()


def main():
    """Entry point for the CLI."""
    args = parse_args()
    print(f"Searching for: {args.pattern}")
    print(f"In path: {args.path}")


class OutputFormatter:
    """Formats search output for terminal display."""

    def __init__(self, color: bool = True):
        self.color = color

    def format_match(self, filename: str, line_num: int, text: str) -> str:
        if self.color:
            return f"\033[35m{filename}\033[0m:\033[32m{line_num}\033[0m:{text}"
        return f"{filename}:{line_num}:{text}"


if __name__ == "__main__":
    main()
