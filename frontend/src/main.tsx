import React from 'react';
import { Auth0Provider } from '@auth0/auth0-react';
import { HashRouter } from 'react-router-dom';
import ReactDOM from 'react-dom/client';
import App from './App.tsx';
import './index.css';
import { ChakraProvider } from '@chakra-ui/react';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <HashRouter>
      <ChakraProvider>
        <Auth0Provider
          domain="dev-jzar8fywnhduze62.us.auth0.com"
          clientId="jU8OOMv4fsnCR07rqR9PRMyI6vs7SZfs"
          authorizationParams={{
            redirect_uri: window.location.origin,
          }}
        >
          <App />
        </Auth0Provider>
      </ChakraProvider>
    </HashRouter>
  </React.StrictMode>,
);
