import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const inputVariants = cva(
  "w-full rounded-xl border border-border bg-surface px-3 py-2 text-sm text-white placeholder:text-zinc-600 transition-colors outline-none focus:border-blue-400/70 focus:ring-2 focus:ring-blue-500/20 disabled:pointer-events-none disabled:opacity-50 [&::-webkit-inner-spin-button]:opacity-50",
  {
    variants: {
      size: {
        default: "py-2",
        sm: "py-1.5",
      },
    },
    defaultVariants: {
      size: "default",
    },
  }
);

export function Input({
  className,
  ...props
}: React.InputHTMLAttributes<HTMLInputElement> &
  VariantProps<typeof inputVariants>) {
  return <input className={cn(inputVariants({ className }))} {...props} />;
}

export function Textarea({
  className,
  ...props
}: React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      className={cn(
        "resize-y rounded-xl border border-border bg-surface px-3 py-2 text-sm text-white",
        "placeholder:text-zinc-600 transition-colors outline-none",
        "focus:border-blue-400/70 focus:ring-2 focus:ring-blue-500/20",
        className
      )}
      {...props}
    />
  );
}

export { inputVariants };