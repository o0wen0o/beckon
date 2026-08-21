import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { Slot } from "radix-ui"

import { cn } from "@/lib/utils"

const buttonVariants = cva(
  // Beckon: `duration-150 ease-out` and `active:scale-[0.98]`. The base already
  // animates `all`, but at Tailwind's unset duration the colour change lands in
  // one frame, which is what made every control on the pane feel like it was
  // switching rather than responding. The press is the other half — a button
  // that only changes colour under the pointer gives a click no acknowledgement
  // at all — and it is disabled with the rest of the motion under
  // `prefers-reduced-motion`.
  "inline-flex shrink-0 items-center justify-center gap-2 rounded-md text-sm font-medium whitespace-nowrap transition-all duration-150 ease-out active:scale-[0.98] motion-reduce:transition-none motion-reduce:active:scale-100 outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/90",
        destructive:
          "bg-destructive text-white hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40",
        // Beckon: `font-normal`. The base is `font-medium`, which is right for
        // the one filled button on a pane; an outlined button is the ordinary
        // register, and at medium it competes with the row labels beside it.
        outline:
          "border bg-background shadow-xs font-normal hover:bg-accent hover:text-accent-foreground dark:border-input dark:bg-input/30 dark:hover:bg-input/50",
        // Beckon's addition. A destructive action sitting beside an ordinary
        // one must not be the loudest thing on the pane — but the danger has to
        // be legible at rest, not only under the pointer, because a keyboard
        // never passes through hover. So red text and a red edge always, the
        // fill on hover only, and `destructive` proper reserved for the
        // confirmation dialog the user has already chosen to open. The dimmed
        // dark fill is the `destructive` variant's own: white on solid
        // `--destructive` is 2.77:1 there, and 6.48:1 on `destructive/60`.
        "destructive-outline":
          "border border-destructive/50 text-destructive shadow-xs hover:bg-destructive dark:hover:bg-destructive/60 hover:text-white hover:border-destructive focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40",
        // Beckon's addition, and the one chromatic control on the surface.
        // Storing a credential is the pane's single commit — it writes to the
        // Credential Manager rather than to a TOML file, and it is the one
        // control here that does not follow the "written as you type" promise
        // (ADR-0003) — so it is the one place a button says so in colour.
        // Outlined for the same reason `destructive-outline` is: it shares its
        // line with Remove, and two solid buttons side by side each say "press
        // me". Green text and edge at rest, the fill on hover only, which is
        // the same shape as the danger treatment on the other end of the row.
        // 7.12:1 at rest light and 11.92:1 dark; on the hover fill, white light
        // and `--background` dark, the same two ratios the other way round.
        "success-outline":
          "border border-success/50 text-success shadow-xs hover:bg-success hover:text-white hover:border-success dark:hover:text-background focus-visible:ring-success/30",
        secondary:
          "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        // Beckon: muted and `font-normal`. A ghost button carries no box, so
        // weight and colour are the only things saying it is quieter than the
        // value it sits beside; hover is what brings it up to full ink.
        ghost:
          "text-muted-foreground font-normal hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
        link: "text-primary underline-offset-4 hover:underline",
      },
      size: {
        default: "h-9 px-4 py-2 has-[>svg]:px-3",
        // Beckon's addition, and the Popover's alone: the quiet button under a
        // turn. `text-note` rather than Tailwind's `text-xs` — the same 12px,
        // but on our scale and so at our leading. The padding is deliberately
        // *not* split by `has-[>svg]`: shadcn narrows a button with an icon in
        // it, which is right for a boxed control and wrong for a borderless one
        // pulled back by a negative margin to sit flush with the text above it —
        // `Copy` carries an icon and `Show what it thought` does not, and the
        // split parked their labels 2px apart.
        xs: "h-6 gap-1.5 rounded-md px-1.5 text-note [&_svg:not([class*='size-'])]:size-3",
        sm: "h-8 gap-1.5 rounded-md px-3 has-[>svg]:px-2.5",
        lg: "h-10 rounded-md px-6 has-[>svg]:px-4",
        icon: "size-9",
        "icon-xs": "size-6 rounded-md [&_svg:not([class*='size-'])]:size-3",
        "icon-sm": "size-8",
        "icon-lg": "size-10",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

function Button({
  className,
  variant = "default",
  size = "default",
  asChild = false,
  ...props
}: React.ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean
  }) {
  const Comp = asChild ? Slot.Root : "button"

  return (
    <Comp
      data-slot="button"
      data-variant={variant}
      data-size={size}
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  )
}

export { Button, buttonVariants }
