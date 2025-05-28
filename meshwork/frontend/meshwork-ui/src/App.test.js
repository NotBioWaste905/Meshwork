// src/App.test.js
import { render } from "@testing-library/react";
import App from "./App";

// Mock ResizeObserver
global.ResizeObserver = jest.fn().mockImplementation(() => ({
  observe: jest.fn(),
  unobserve: jest.fn(),
  disconnect: jest.fn(),
}));

test("renders without crashing", () => {
  render(<App />);
  // Basic test just to verify rendering
});
