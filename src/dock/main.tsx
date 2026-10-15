import React from "react";
import ReactDOM from "react-dom/client";
import "../themes/tokens.css";
import "../themes/base.css";
import Appearance from "../components/Appearance";
import { ROOT_OPTIONS } from "../core/reactRoot";
import Dock from "./Dock";

ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement,
  ROOT_OPTIONS,
).render(
  <React.StrictMode>
    <Appearance window="dock">
      <Dock />
    </Appearance>
  </React.StrictMode>,
);
