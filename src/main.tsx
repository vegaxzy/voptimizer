import React from "react";
import ReactDOM from "react-dom/client";
import { Toaster } from "sonner";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
    <Toaster
      position="bottom-right"
      theme="dark"
      toastOptions={{
        style: {
          background: "var(--surface-alt, #171b25)",
          border: "1px solid var(--border, #2a2f3a)",
          color: "var(--text, #e6eaf2)",
          fontSize: "12px",
          fontFamily: "'Segoe UI', system-ui, sans-serif",
          borderRadius: "4px",
          boxShadow: "0 4px 12px rgba(0,0,0,0.4)",
        },
      }}
      richColors
    />
  </React.StrictMode>,
);
