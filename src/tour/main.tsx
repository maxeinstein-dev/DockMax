import React from "react";
import ReactDOM from "react-dom/client";
import "../themes/tokens.css";
import "../themes/base.css";
import "../panel/Panel.css";
import "../panel/parts/parts.css";
import Appearance from "../components/Appearance";
import { ROOT_OPTIONS } from "../core/reactRoot";
import TourApp from "./TourApp";

ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement,
  ROOT_OPTIONS,
).render(
  <React.StrictMode>
    <Appearance window="panel">
      <TourApp />
    </Appearance>
  </React.StrictMode>,
);
