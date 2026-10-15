import React from "react";
import ReactDOM from "react-dom/client";
import "../themes/tokens.css";
import "../themes/base.css";
import Appearance from "../components/Appearance";
import { ROOT_OPTIONS } from "../core/reactRoot";
import Panel from "./Panel";

ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement,
  ROOT_OPTIONS,
).render(
  <React.StrictMode>
    <Appearance window="panel">
      <Panel />
    </Appearance>
  </React.StrictMode>,
);
