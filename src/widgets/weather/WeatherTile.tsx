import "../system/system.css";
import { Sky } from "./WeatherPanel";

/** The tile of a Clima outside the dock: a sun. The dock draws the live
 * one natively (`native_dock/faces.rs`, `weather`). Decorative. */
function WeatherTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <Sky condition="clear" className="" />
    </span>
  );
}

export default WeatherTile;
