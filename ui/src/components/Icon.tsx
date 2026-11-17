import type { LucideIcon } from "lucide-react";

interface Props {
  icon: LucideIcon;
  size?: number;
  className?: string;
}

/** Lucide icons at macOS symbol weights: thin enough to sit next to 13 px text. */
export function Icon({ icon: Glyph, size = 16, className }: Props) {
  return <Glyph size={size} strokeWidth={size <= 14 ? 1.9 : 1.75} aria-hidden className={className} />;
}
