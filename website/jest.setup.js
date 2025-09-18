import "@testing-library/jest-dom";
import { jest } from "@jest/globals";
import { TextDecoder, TextEncoder } from "util";

// Polyfill web APIs required by @dfinity packages when running in Node
if (typeof global.TextEncoder === "undefined") {
  global.TextEncoder = TextEncoder;
}

if (typeof global.TextDecoder === "undefined") {
  global.TextDecoder = TextDecoder;
}

// Silence console warnings for tests
const originalWarn = console.warn;
const originalError = console.error;

beforeAll(() => {
  // Suppress Radix UI dialog warnings
  console.warn = jest.fn((...args) => {
    if (
      typeof args[0] === "string" &&
      (args[0].includes("Missing `Description`") ||
        args[0].includes("aria-describedby"))
    ) {
      return;
    }
    originalWarn(...args);
  });

  // Suppress React warnings about act()
  console.error = jest.fn((...args) => {
    if (
      typeof args[0] === "string" &&
      (args[0].includes("not wrapped in act") ||
        args[0].includes("inside a test was not wrapped"))
    ) {
      return;
    }
    originalError(...args);
  });
});

afterAll(() => {
  console.warn = originalWarn;
  console.error = originalError;
});

// Set up a basic DOM environment for portals
beforeEach(() => {
  if (!document.getElementById("portal-root")) {
    const portalRoot = document.createElement("div");
    portalRoot.setAttribute("id", "portal-root");
    document.body.appendChild(portalRoot);
  }
});

// Clean up after each test
afterEach(() => {
  const portalRoot = document.getElementById("portal-root");
  if (portalRoot) {
    portalRoot.remove();
  }
});

// Mock IntersectionObserver
class MockIntersectionObserver {
  constructor(callback) {
    this.callback = callback;
  }

  observe() {
    return null;
  }

  unobserve() {
    return null;
  }

  disconnect() {
    return null;
  }
}

global.IntersectionObserver = MockIntersectionObserver;
