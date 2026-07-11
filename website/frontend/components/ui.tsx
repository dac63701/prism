import Link from "next/link";
import React from "react";
import { cn } from "@/lib/utils";

export function Button({
  className,
  variant = "primary",
  asChild,
  children,
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost";
  asChild?: boolean;
}) {
  const classes = cn(
    "inline-flex items-center justify-center gap-2 rounded-xl px-4 py-2.5 text-sm font-medium transition",
    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-accent)] focus-visible:ring-offset-2 focus-visible:ring-offset-[color:var(--color-bg)]",
    variant === "primary" &&
      "bg-[linear-gradient(135deg,var(--color-accent),var(--color-accent-2))] text-white shadow-lg shadow-blue-500/20 hover:brightness-110",
    variant === "secondary" &&
      "border border-[color:var(--color-border)] bg-white/5 text-zinc-100 hover:bg-white/10",
    variant === "ghost" && "text-zinc-300 hover:bg-white/5 hover:text-white",
    className
  );

  if (asChild && React.isValidElement(children)) {
    const child = children as React.ReactElement<{ className?: string }>;
    return React.cloneElement(child, {
      className: cn(child.props.className, classes),
    });
  }

  return (
    <button
      className={classes}
      {...props}
    >
      {children}
    </button>
  );
}

export function Card({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "rounded-3xl border border-white/10 bg-[linear-gradient(180deg,rgba(16,25,46,0.95),rgba(8,13,26,0.95))] shadow-lg shadow-black/20",
        className
      )}
      {...props}
    />
  );
}

export function Panel({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "rounded-2xl border border-white/10 bg-white/[0.03]",
        className
      )}
      {...props}
    />
  );
}

export function Input(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={cn(
        "w-full rounded-xl border border-white/10 bg-[#09111f] px-4 py-3 text-sm text-white placeholder:text-zinc-500",
        "focus:border-blue-400/70 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
        props.className
      )}
    />
  );
}


export function Badge({ className, ...props }: React.HTMLAttributes<HTMLSpanElement>) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full border border-blue-400/20 bg-blue-500/10 px-3 py-1 text-xs font-medium text-blue-200",
        className
      )}
      {...props}
    />
  );
}

export function StatCard({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <Panel className="p-5">
      <div className="text-sm text-zinc-400">{label}</div>
      <div className="mt-2 text-2xl font-semibold tracking-tight text-white">{value}</div>
      {hint ? <div className="mt-1 text-xs text-zinc-500">{hint}</div> : null}
    </Panel>
  );
}

export function SectionHeading({
  eyebrow,
  title,
  description,
}: {
  eyebrow?: string;
  title: string;
  description?: string;
}) {
  return (
    <div className="space-y-2">
      {eyebrow ? <div className="text-xs uppercase tracking-[0.28em] text-blue-300/70">{eyebrow}</div> : null}
      <h2 className="text-2xl font-semibold tracking-tight text-white">{title}</h2>
      {description ? <p className="max-w-2xl text-sm leading-6 text-zinc-400">{description}</p> : null}
    </div>
  );
}


