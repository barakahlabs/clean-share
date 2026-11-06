import React from "react";
import ReactDOM from "react-dom/client";
import "./theme/colors.css";
import "./theme/appearance";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
