export type RiskLevel =
  | "safe"
  | "low-risk"
  | "medium-risk"
  | "high-risk"
  | "dangerous"
  | "unproven";

export type ExpectedBenefit = "none" | "low" | "medium" | "high";
export type TweakStatus = "idle" | "applying" | "reverting" | "applied" | "error";
export type LogLevel = "info" | "success" | "error" | "warning";

export * from "./startup";
export * from "./backup";

export interface UnifiedTweak {
  id: string;
  name: string;
  description: string;
  category: string;
  riskLevel: RiskLevel;
  isExperimental: boolean;
  isImplemented: boolean;
  expectedBenefit: ExpectedBenefit;
  riskExplanation: string;
  sideEffects: string[];
  tags?: string[];
  requiresRestart?: boolean;
  requiresRestorePoint?: boolean;
  nvidiaOnly?: boolean;
  amdOnly?: boolean;
  requiresAdmin?: boolean;
}

export interface TweakState {
  tweak: UnifiedTweak;
  status: TweakStatus;
  isApplied: boolean;
}

export interface TweakOpResult {
  success: boolean;
  message: string;
  error: string | null;
}

export interface TweakCategory {
  id: string;
  name: string;
  icon: string;
}

export interface LogEntry {
  id: string;
  timestamp: Date;
  message: string;
  level: LogLevel;
  tweakId?: string;
  /** Feature that generated this entry – used for global log aggregation. */
  source?: "tweak" | "startup" | "backup";
}
