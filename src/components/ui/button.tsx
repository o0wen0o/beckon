import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { Slot } from "radix-ui"

import { cn } from "@/lib/utils"

const buttonVariants = cva(
  // Beckon: `duration-150 ease-out` and `active:scale-[0.98]`. The base animates
  // `all` at Tailwind's unset duration, which lands the colour change in one
  // frame; the press is what acknowledges a click. Both stop under
  // `prefers-reduced-motion`.
  "inline-flex shrink-0 items-center justify-center gap-2 rounded-md text-sm font-medium whitespace-nowrap transition-all duration-150 ease-out active:scale-[0.98] motion-reduce:transition-none motion-reduce:active:scale-100 outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/90",
        destructive:
          "bg-destructive text-white hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40",
        // Beckon: `font-normal`. `font-medium` is right for the one filled
        // button on a pane; at that weight an outlined button competes with
        // the row labels beside it.
        outline:
          "border bg-background shadow-xs font-normal hover:bg-accent hover:text-accent-foreground dark:border-input dark:bg-input/30 dark:hover:bg-input/50",
        // Beckon's addition: red text and edge at rest, fill on hover only.
        // Danger has to be legible at rest, since a keyboard never passes
        // through hover, but must not be the loudest thing on the pane —
        // solid `destructive` is reserved for the confirmation dialog. The
        // dimmed dark fill is that variant's own (6.48:1 vs 2.77:1 on solid).
        "destructive-outline":
          "border border-destructive/50 text-destructive shadow-xs hover:bg-destructive dark:hover:bg-destructive/60 hover:text-white hover:border-destructive focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40",
        // Beckon's addition, and the one chromatic control on the surface:
        // storing a credential is the pane's single commit, the one control
        // that does not follow "written as you type" (ADR-0003). Outlined
        // rather than filled because it shares its line with Remove, and two
        // solid buttons there would each read as the thing to press.
        "success-outline":
          "border border-success/50 text-success shadow-xs hover:bg-success hover:text-white hover:border-success dark:hover:text-background focus-visible:ring-success/30",
        secondary:
          "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        // Beckon: muted and `font-normal`. Without a box, weight and colour are
        // the only things saying it is quieter than the value beside it.
        ghost:
          "text-muted-foreground font-normal hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
        // Beckon: underlined at rest, and no box (see `compoundVariants`). Its
        // only use is a link inside `Callout` prose, where stock's hover-only
        // underline and inherited button padding would both read as a control.
        link: "text-primary underline underline-offset-4",
      },
      size: {
        default: "h-9 px-4 py-2 has-[>svg]:px-3",
        // Beckon's addition, for the quiet buttons that carry no box: the
        // Popover's, and the hotkey row in Settings, which is a chip plus the two
        // buttons that change it. `text-note` rather than Tailwind's `text-xs`:
        // the same 12px, on our scale. The padding is deliberately *not* split by
        // `has-[>svg]` — narrowing a button with an icon is right for a boxed
        // control and wrong for a borderless one, where it parks the labels 2px
        // apart.
        xs: "h-6 gap-1.5 rounded-md px-1.5 text-note [&_svg:not([class*='size-'])]:size-3",
        sm: "h-8 gap-1.5 rounded-md px-3 has-[>svg]:px-2.5",
        // Beckon's addition, and `xs`'s boxed sibling: the same 12px register,
        // still carrying a box. The Settings panes read one register down since
        // ADR-0019/ADR-0020, so a 36px 14px button beside those controls read as
        // the thing to press on a pane whose point is the rows above it. The
        // weight is part of the size, not the call site's business: `outline`
        // and `ghost` drop to 400, and at 12px that is thinner than the label
        // beside it — cva emits `size` after `variant`, so saying it here is
        // what keeps two panes from each answering it differently.
        "sm-note": "h-8 gap-1.5 rounded-md px-3 has-[>svg]:px-2.5 text-note font-medium",
        lg: "h-10 rounded-md px-6 has-[>svg]:px-4",
        icon: "size-9",
        "icon-xs": "size-6 rounded-md [&_svg:not([class*='size-'])]:size-3",
        "icon-sm": "size-8",
        "icon-lg": "size-10",
      },
    },
    // Beckon's addition. A link runs inside a sentence, so it takes no height
    // and no padding from whatever `size` it is given — and cva emits compound
    // variants after `size`, which is the only place that can be said once
    // rather than as a class string at each call site.
    compoundVariants: [{ variant: "link", class: "h-auto p-0" }],
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
