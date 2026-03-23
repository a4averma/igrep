// Application entry point
const express = require('express');

const PORT = process.env.PORT || 3000;

function createApp() {
    const app = express();
    
    // TODO: add middleware
    app.get('/', (req, res) => {
        res.json({ status: 'ok', message: 'instant-grep test fixture' });
    });
    
    return app;
}

function parseQuery(raw) {
    // FIXME: sanitize input properly
    return raw.trim().toLowerCase();
}

const app = createApp();
module.exports = { app, parseQuery };
