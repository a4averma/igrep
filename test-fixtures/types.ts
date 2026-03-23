/**
 * Type definitions for the search engine
 */

export interface SearchOptions {
    caseSensitive: boolean;
    wholeWord: boolean;
    regex: boolean;
    maxResults: number;
}

export interface Match {
    file: string;
    line: number;
    column: number;
    text: string;
}

export type ResultCallback = (match: Match) => void;

// TODO: add support for file type filters
export const DEFAULT_OPTIONS: SearchOptions = {
    caseSensitive: false,
    wholeWord: false,
    regex: false,
    maxResults: 1000,
};

export function isValidPattern(pattern: string): boolean {
    if (pattern.length === 0) return false;
    try {
        new RegExp(pattern);
        return true;
    } catch {
        return false;
    }
}
