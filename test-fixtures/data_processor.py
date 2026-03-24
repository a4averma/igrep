"""Data processing utilities for testing."""

import os
import json
from typing import List, Dict, Optional


# TODO: add streaming support
class DataProcessor:
    """Processes data files and extracts patterns."""

    def __init__(self, base_path: str):
        self.base_path = base_path
        self.results: List[Dict] = []

    def load_file(self, filename: str) -> Optional[str]:
        """Load a file and return its contents."""
        path = os.path.join(self.base_path, filename)
        if not os.path.exists(path):
            return None
        with open(path, "r") as f:
            return f.read()

    def process(self, data: str) -> List[str]:
        """Process raw data into lines."""
        return data.strip().split("\n")


def find_pattern(text: str, pattern: str) -> List[int]:
    """Find all line numbers containing the pattern."""
    # FIXME: support regex patterns
    matches = []
    for i, line in enumerate(text.split("\n"), 1):
        if pattern in line:
            matches.append(i)
    return matches


class Config:
    """Configuration holder."""

    def __init__(self):
        self.max_results = 100
        self.case_sensitive = True
        self.include_hidden = False
