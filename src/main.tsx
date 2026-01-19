import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./i18n";

// Nunito Sans Font from Fontsource
import "@fontsource/nunito-sans/400.css";
import "@fontsource/nunito-sans/700.css";
import "@fontsource/nunito-sans/900.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
