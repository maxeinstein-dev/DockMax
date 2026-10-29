import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";
import { setLanguage } from "../i18n";

// The tests read the PT-BR texts; jsdom's own language is English.
setLanguage("pt-BR");

afterEach(() => {
  cleanup();
});
