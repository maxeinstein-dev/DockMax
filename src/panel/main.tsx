import React from "react";
import ReactDOM from "react-dom/client";
import "../themes/tokens.css";
import "../themes/base.css";
import Appearance from "../components/Appearance";
import Panel from "./Panel";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Appearance window="panel">
      <Panel />
    </Appearance>
  </React.StrictMode>,
);
