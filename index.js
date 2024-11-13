console.log(`Server running on port ${process.env.PORT}`);
const express = require('express');
const app = express();
const path = require('path');

// Serve static files from public directory
app.use(express.static('public'));

// Modern JSON API endpoint
app.get('/api/hello', (req, res) => {
    res.json({
        message: '✅ Hello from your modern containerized app! ✨',
        version: '1.0.0',
        timestamp: new Date().toISOString()
    });
});

// Serve modern SPA frontend
app.get('/', (req, res) => {
    res.sendFile(path.join(__dirname, 'public', 'index.html'));
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
    console.log(`🚀 Modern server running at http://localhost:${PORT}`);
});