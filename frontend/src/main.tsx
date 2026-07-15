import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./i18n";
import { AppPreferencesProvider } from "./contexts/AppPreferencesProvider";
import "./styles.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AppPreferencesProvider><App /></AppPreferencesProvider>
  </StrictMode>,
);
