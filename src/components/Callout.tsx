// A section-scoped message: something about this pane, not about the window.
// Window-level state lives in the status bar instead, so the two cannot pile up
// into a wall of coloured boxes.
import type * as React from "react";
import { TriangleAlertIcon } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { BrandMark } from "./BrandMark";

interface CalloutProps {
  tone?: "info" | "warn" | "danger";
  children: React.ReactNode;
}

export function Callout({ tone = "info", children }: CalloutProps) {
  return (
    <Alert
      variant={tone === "danger" ? "destructive" : "default"}
      role={tone === "danger" ? "alert" : undefined}
      className={[
        "mb-6 border-l-2",
        tone === "info" ? "border-l-primary" : "",
        tone === "warn" ? "border-l-warning text-warning" : "",
      ].join(" ")}
    >
      {tone === "info" ? <BrandMark className="size-4" /> : <TriangleAlertIcon className="size-4" />}
      <AlertDescription className="[&_p]:mb-1 [&_p:last-child]:mb-0 [&_ul]:mt-1 [&_ul]:list-disc [&_ul]:pl-5">
        {children}
      </AlertDescription>
    </Alert>
  );
}
