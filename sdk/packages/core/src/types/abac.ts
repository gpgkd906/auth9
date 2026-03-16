export interface AbacPolicy {
  id: string;
  tenantId: string;
  name: string;
  description?: string;
  versionId: string;
  status: "draft" | "published" | "archived";
  rules: AbacRule[];
  createdAt: string;
  updatedAt: string;
}

export interface AbacRule {
  effect: "allow" | "deny";
  subjects: Record<string, string>;
  resources: Record<string, string>;
  actions: string[];
  conditions?: Record<string, unknown>;
}

export interface CreateAbacPolicyInput {
  name: string;
  description?: string;
  rules: AbacRule[];
}

export interface UpdateAbacPolicyInput {
  name?: string;
  description?: string;
  rules?: AbacRule[];
}

export interface SimulateAbacInput {
  subject: Record<string, string>;
  resource: Record<string, string>;
  action: string;
}

export interface AbacSimulationResult {
  allowed: boolean;
  matchedPolicies: string[];
  reason?: string;
}
