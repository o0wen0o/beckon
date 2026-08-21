// shadcn/ui's generated helper: merges a component's own classes with the ones
// a caller passes, so the caller's win without duplicating a Tailwind property.
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
