// frontend/src/components/Header.jsx

import React from 'react';

function Header() {
  return (
    <header style={{ 
      padding: '10px 20px', 
      backgroundColor: '#333', 
      color: 'white', 
      textAlign: 'center' 
    }}>
      <h1>TOKENIZER Project</h1>
      <p>Your Authentication Solution</p>
    </header>
  );
}

export default Header;