// frontend/src/App.jsx

import React from 'react';
import Header from './components/Header'; // Import the Header component

function App() {
  return (
    // We use a React Fragment (<>...</>) as the top-level container
    <>
      <Header /> 
      
      <main style={{ padding: '20px' }}>
        <h2>Welcome to the Tokenizer Application</h2>
        <p>This is the main content area. We will add routing and pages here soon.</p>
      </main>
      
      {/* You can add a Footer component here later */}
    </>
  );
}

export default App;