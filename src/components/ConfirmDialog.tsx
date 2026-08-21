// shadcn/ui's AlertDialog, which is Radix's: the focus trap, Esc-to-dismiss, the
// backdrop and the portal all come from the library.
//
// It replaces `confirm()`, which WebView2 renders as unthemed browser chrome
// with the app origin in the title, blocks the whole webview including any
// in-flight debounced save, and cannot name the file in the app's own voice.
import type * as React from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { buttonVariants } from "@/components/ui/button";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  children: React.ReactNode;
  confirmLabel: string;
  destructive?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  open,
  title,
  children,
  confirmLabel,
  destructive = false,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  return (
    <AlertDialog open={open} onOpenChange={(next) => !next && onCancel()}>
      <AlertDialogContent className="max-w-105">
        <AlertDialogHeader>
          <AlertDialogTitle className="font-display">{title}</AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div>{children}</div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          {/* Cancel comes first, and Radix focuses it: the default action of a
              destructive dialog must not be the destructive one. */}
          <AlertDialogCancel onClick={onCancel}>Cancel</AlertDialogCancel>
          {/* Filled, not outlined: this is the button that does the irreversible
              thing, and an outline reads as the quieter of the two on offer. */}
          <AlertDialogAction
            onClick={onConfirm}
            className={destructive ? buttonVariants({ variant: "destructive" }) : undefined}
          >
            {confirmLabel}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
