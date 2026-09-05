import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import "./index.css";

const params = new URLSearchParams(globalThis.location.search);
const isSettings =
  params.get("view") === "settings" || getCurrentWindow().label === "settings";

if (isSettings) {
  document.documentElement.dataset.window = "settings";
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
