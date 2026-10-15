import React from "react";
import ReactDOM from "react-dom/client";
import "../themes/tokens.css";
import "../themes/base.css";
import Appearance from "../components/Appearance";
import Dock from "./Dock";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Appearance window="dock">
      <Dock />
    </Appearance>
  </React.StrictMode>,
);
