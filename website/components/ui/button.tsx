import * as React from "react";

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "outline" | "ghost";
  size?: "default" | "sm" | "lg" | "icon";
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = "default", size = "default", ...props }, ref) => {
    // Apply different classes based on variant and size
    const getVariantClasses = () => {
      switch (variant) {
        case "outline":
          return "border border-white/20 bg-transparent hover:bg-white/10 text-white";
        case "ghost":
          return "bg-transparent hover:bg-white/10 text-white";
        default:
          return "bg-blue-600 hover:bg-blue-700 text-white";
      }
    };

    const getSizeClasses = () => {
      switch (size) {
        case "sm":
          return "text-xs px-2 py-1";
        case "lg":
          return "text-lg px-5 py-3";
        case "icon":
          return "h-8 w-8 p-0 flex items-center justify-center";
        default:
          return "text-sm px-4 py-2";
      }
    };

    return (
      <button
        ref={ref}
        className={`rounded-md font-medium transition-colors
                  focus:outline-none focus:ring-2 focus:ring-blue-400 focus:ring-offset-2
                  disabled:opacity-50 disabled:pointer-events-none
                  ${getVariantClasses()} ${getSizeClasses()} ${className}`}
        {...props}
      />
    );
  }
);

Button.displayName = "Button";

export { Button };
