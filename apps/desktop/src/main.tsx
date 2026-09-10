import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './App';
import { QuickTools } from './QuickTools';
import { Overlay } from './Overlay';
import './styles.css';

// One bundle serves both windows; the overlay is the same page with a flag, so
// there is no second build and no duplicated logic.
const params = new URLSearchParams(window.location.search);
const isQuickTools = params.has('quicktools');
const isOverlay = params.has('overlay');

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>{isOverlay ? <Overlay /> : isQuickTools ? <QuickTools /> : <App />}</React.StrictMode>,
);
