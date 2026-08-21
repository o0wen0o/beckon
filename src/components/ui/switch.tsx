"use client"

import * as React from "react"
import { Switch as SwitchPrimitive } from "radix-ui"

import { cn } from "@/lib/utils"

function Switch({
  className,
  size = "default",
  ...props
}: React.ComponentProps<typeof SwitchPrimitive.Root> & {
  size?: "sm" | "default"
}) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      data-size={size}
      className={cn(
        // Beckon: the whole track. Stock unchecked is `bg-input`, a filled
        // mid-grey pill — but here a fill means "on" and nothing else, so a
        // filled off switch says the opposite of what it is. Off is paper with
        // a bounded edge; on is the ink fill. Geometry too: stock's 16px knob
        // in an 18.4px track leaves 1.2px of air and reads as a lozenge.
        "peer group/switch inline-flex shrink-0 items-center rounded-full border transition-all duration-200 ease-out motion-reduce:transition-none outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 data-[size=default]:h-4.75 data-[size=default]:w-8.5 data-[size=sm]:h-4 data-[size=sm]:w-7 data-[state=checked]:border-primary data-[state=checked]:bg-primary data-[state=unchecked]:border-input data-[state=unchecked]:bg-muted",
        className
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className={cn(
          // Beckon: 13px, inset 2px, and the knob carries the state too — grey
          // off, paper on. Travel is the track's inner width less the knob and
          // its two insets, and the knob *moves*: 200ms `ease-out`, the same
          // curve the track's fill runs on, so the two read as one movement.
          "pointer-events-none mx-0.5 block rounded-full ring-0 transition-[transform,background-color] duration-200 ease-out motion-reduce:transition-none group-data-[size=default]/switch:size-3.25 group-data-[size=sm]/switch:size-2.5 data-[state=checked]:bg-primary-foreground data-[state=unchecked]:bg-muted-foreground group-data-[size=default]/switch:data-[state=checked]:translate-x-3.75 group-data-[size=sm]/switch:data-[state=checked]:translate-x-2.75 data-[state=unchecked]:translate-x-0"
        )}
      />
    </SwitchPrimitive.Root>
  )
}

export { Switch }
