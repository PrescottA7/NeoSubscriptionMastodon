import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Provider } from "react-redux";
import { BrowserRouter } from "react-router-dom";

import "normalize.css";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/inter/700.css";

import App from "./App.tsx";
import ColorModeProvider from "./components/ColorModeProvider";
import { store } from "./store/store";

declare global {
  interface Window {
    /** Injected by the server; the admin path prefix, empty at the root. */
    __NSM_BASE__?: string;
  }
}

/**
 * Router base. The server substitutes `__NSM_BASE__` in `index.html`, so the
 * same binary works at the root and under a prefix such as `/subsc-admin`.
 */
const base = window.__NSM_BASE__ ?? "";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Provider store={store}>
      <BrowserRouter basename={base || undefined}>
        <ColorModeProvider>
          <App />
        </ColorModeProvider>
      </BrowserRouter>
    </Provider>
  </StrictMode>,
);
