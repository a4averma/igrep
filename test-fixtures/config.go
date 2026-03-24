package config

import (
	"encoding/json"
	"fmt"
	"os"
)

// Config holds application configuration
type Config struct {
	IndexPath      string   `json:"index_path"`
	MaxFileSize    int64    `json:"max_file_size"`
	IgnorePatterns []string `json:"ignore_patterns"`
	NumWorkers     int      `json:"num_workers"`
}

// DefaultConfig returns a Config with sensible defaults
func DefaultConfig() Config {
	return Config{
		IndexPath:      "/tmp/index",
		MaxFileSize:    1024 * 1024 * 10, // 10MB
		IgnorePatterns: []string{".git", "node_modules", "target"},
		NumWorkers:     4,
	}
}

// LoadConfig reads configuration from a JSON file
// TODO: support TOML and YAML formats
func LoadConfig(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read config: %w", err)
	}

	var cfg Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("failed to parse config: %w", err)
	}

	return &cfg, nil
}
