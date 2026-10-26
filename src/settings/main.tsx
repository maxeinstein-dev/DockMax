import React from "react";
import ReactDOM from "react-dom/client";
import "../themes/tokens.css";
import "../themes/base.css";
import "../panel/Panel.css";
import Appearance from "../components/Appearance";
import { ROOT_OPTIONS } from "../core/reactRoot";
import SettingsApp from "./SettingsApp";

ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement,
  ROOT_OPTIONS,
).render(
  <React.StrictMode>
    <Appearance window="panel">
      <SettingsApp />
    </Appearance>
  </React.StrictMode>,
);
