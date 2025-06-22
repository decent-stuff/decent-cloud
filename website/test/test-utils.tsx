import React, { ReactElement } from "react";
import { render, RenderOptions } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// Create a custom render function that includes providers and portal root
function customRender(
  ui: ReactElement,
  options?: Omit<RenderOptions, "wrapper">
) {
  // Create portal root for dialogs
  if (!document.getElementById("portal-root")) {
    const portalRoot = document.createElement("div");
    portalRoot.setAttribute("id", "portal-root");
    document.body.appendChild(portalRoot);
  }

  return {
    user: userEvent.setup(),
    ...render(ui, {
      ...options,
      wrapper: ({ children }) => <div id="portal-root">{children}</div>,
    }),
  };
}

// Re-export everything
export * from "@testing-library/react";

// Override render method
export { customRender as render };
