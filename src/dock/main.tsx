import React from "react";
import ReactDOM from "react-dom/client";
import "../themes/tokens.css";
import "../themes/base.css";
import { ROOT_OPTIONS } from "../core/reactRoot";
import Dock from "./Dock";

// The dock applies its own appearance (useAppearance): it lays itself out
// from the same settings, so it reads them once.
ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement,
  ROOT_OPTIONS,
).render(
  <React.StrictMode>
    <Dock />
  </React.StrictMode>,
);
