// The logo's summoning spark, redrawn as a UI-scale mark: the ring and the
// four-point core from assets/logo.svg, without the glow filter and drop shadow
// that collapse into a smear below 32px.
//
// The only icon not taken from lucide, because it is the identity rather than a
// glyph. It paints in `currentColor` like every lucide icon does, so the caller
// still decides the colour.
import * as React from "react";

export function BrandMark({ className, ...props }: React.ComponentProps<"svg">) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      aria-hidden="true"
      focusable="false"
      className={className}
      {...props}
    >
      <path
        d="M12 3.2A8.8 8.8 0 1 1 3.2 12"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        opacity="0.55"
      />
      <path
        d="M12 6.2q0 5.8 5.8 5.8-5.8 0-5.8 5.8Q12 12 6.2 12 12 12 12 6.2Z"
        fill="currentColor"
      />
    </svg>
  );
}
