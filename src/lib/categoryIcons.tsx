import {
  Zap,
  Lock,
  Globe,
  Gamepad2,
  Rocket,
  LayoutGrid,
  Settings2,
} from "lucide-react";

/** Maps a tweak category id to a monochrome Lucide icon. */
export function categoryIcon(id: string, size = 18): React.ReactNode {
  const props = { size, strokeWidth: 1.8 } as const;
  switch (id) {
    case "performance": return <Zap {...props} />;
    case "privacy":     return <Lock {...props} />;
    case "network":     return <Globe {...props} />;
    case "gaming":      return <Gamepad2 {...props} />;
    case "startup":     return <Rocket {...props} />;
    case "ui":          return <LayoutGrid {...props} />;
    default:            return <Settings2 {...props} />;
  }
}
