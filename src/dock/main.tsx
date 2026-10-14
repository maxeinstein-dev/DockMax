import React from "react";
import ReactDOM from "react-dom/client";
import "../themes/base.css";
import Dock from "./Dock";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Dock />
  </React.StrictMode>,
);
