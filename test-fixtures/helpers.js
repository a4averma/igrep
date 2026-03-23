/**
 * Helper utilities for search operations
 */

function debounce(fn, delay) {
    let timer;
    return function (...args) {
        clearTimeout(timer);
        timer = setTimeout(() => fn.apply(this, args), delay);
    };
}

function escapeRegex(str) {
    // FIXME: handle all special regex characters
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

const MAX_LINE_LENGTH = 2000;
const TRUNCATION_MARKER = "...";

function truncateLine(line, maxLen = MAX_LINE_LENGTH) {
    if (line.length <= maxLen) return line;
    return line.substring(0, maxLen) + TRUNCATION_MARKER;
}

export default { debounce, escapeRegex, truncateLine };
