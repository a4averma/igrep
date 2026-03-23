import React, { useState, useEffect } from 'react';

interface SearchProps {
    query: string;
    maxResults?: number;
}

// TODO: add debounce to search input
export default function SearchComponent({ query, maxResults = 10 }: SearchProps) {
    const [results, setResults] = useState<string[]>([]);
    const [loading, setLoading] = useState(false);

    useEffect(() => {
        if (query.length > 0) {
            setLoading(true);
            // FIXME: replace with actual API call
            const filtered = mockData.filter(item =>
                item.toLowerCase().includes(query.toLowerCase())
            );
            setResults(filtered.slice(0, maxResults));
            setLoading(false);
        }
    }, [query, maxResults]);

    return (
        <div className="search-results">
            {loading && <p>Loading...</p>}
            {results.map((result, i) => (
                <div key={i} className="result-item">{result}</div>
            ))}
        </div>
    );
}

const mockData = ["alpha", "beta", "gamma", "delta"];

export function formatResult(text: string, highlight: string): string {
    return text.replace(new RegExp(highlight, 'gi'), `<mark>${highlight}</mark>`);
}
