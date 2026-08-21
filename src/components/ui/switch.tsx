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
        // Beckon: the whole track. Stock unchecked is `bg-input`, a filled mid-grey
        // pill — but on this surface a fill means "on" and nothing else (the nav's
        // current row, the selected segment), so an off switch that is filled says
        // the opposite of what it is. Off is therefore the pane's own paper with a
        // bounded edge and a grey knob; on is the ink fill. The geometry is the
        // other half: stock is a 16px knob in an 18.4px track, 1.2px of air, which
        // reads as a lozenge rather than a switch. 13-in-19 is the mock's.
        "peer group/switch inline-flex shrink-0 items-center rounded-full border transition-all duration-200 ease-out motion-reduce:transition-none outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 data-[size=default]:h-4.75 data-[size=default]:w-8.5 data-[size=sm]:h-4 data-[size=sm]:w-7 data-[state=checked]:border-primary data-[state=checked]:bg-primary data-[state=unchecked]:border-input data-[state=unchecked]:bg-muted",
        className
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className={cn(
          // Beckon: 13px, inset 2px, and the knob carries the state too — grey when
          // off so nothing on the control is at full strength, paper when on so it
          // reads against the ink. Travel is the track's inner width less the knob
          // and its two insets.
          // Beckon: the knob travels. The mock froze it (`transition: none`)
          // because a static mock cannot show motion, and a switch is the one
          // control whose whole job is to show which of two states it is in —
          // watching the knob cross is how you know your click landed on the
          // switch rather than on the row. 200ms `ease-out` is the same curve
          // the track's fill runs on, so the two read as one movement.
          "pointer-events-none mx-0.5 block rounded-full ring-0 transition-[transform,background-color] duration-200 ease-out motion-reduce:transition-none group-data-[size=default]/switch:size-3.25 group-data-[size=sm]/switch:size-2.5 data-[state=checked]:bg-primary-foreground data-[state=unchecked]:bg-muted-foreground group-data-[size=default]/switch:data-[state=checked]:translate-x-3.75 group-data-[size=sm]/switch:data-[state=checked]:translate-x-2.75 data-[state=unchecked]:translate-x-0"
        )}
      />
    </SwitchPrimitive.Root>
  )
}

export { Switch }
