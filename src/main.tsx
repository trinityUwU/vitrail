import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./shared/styles/tokens.css";
import "./shared/styles/base.css";
import "./shared/styles/components.css";
import "./shared/styles/layout.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
