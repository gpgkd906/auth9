export interface LoginStats {
  totalLogins: number;
  successfulLogins: number;
  failedLogins: number;
  uniqueUsers: number;
  byEventType: Record<string, number>;
  byDeviceType: Record<string, number>;
  periodStart: string;
  periodEnd: string;
}

export interface LoginEvent {
  id: number;
  userId?: string;
  email?: string;
  tenantId?: string;
  eventType: string;
  ipAddress?: string;
  userAgent?: string;
  deviceType?: string;
  location?: string;
  sessionId?: string;
  failureReason?: string;
  createdAt: string;
}

export interface AuditLog {
  id: number;
  actorId?: string;
  actorEmail?: string;
  actorDisplayName?: string;
  action: string;
  resourceType: string;
  resourceId?: string;
  oldValue?: unknown;
  newValue?: unknown;
  ipAddress?: string;
  createdAt: string;
}

export interface SecurityAlert {
  id: string;
  userId?: string;
  tenantId?: string;
  alertType: "bruteForce" | "newDevice" | "impossibleTravel" | "suspiciousIp";
  severity: "low" | "medium" | "high" | "critical";
  details?: Record<string, unknown>;
  resolvedAt?: string;
  resolvedBy?: string;
  createdAt: string;
}
