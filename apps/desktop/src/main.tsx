import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './App';
import { QuickTools } from './QuickTools';
import './styles.css';

// One bundle serves both windows; the overlay is the same page with a flag, so
// there is no second build and no duplicated logic.
const isQuickTools = new URLSearchParams(window.location.search).has('quicktools');

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>{isQuickTools ? <QuickTools /> : <App />}</React.StrictMode>,
);
