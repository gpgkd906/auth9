export interface SessionInfo {
  id: string;
  deviceType?: string;
  deviceName?: string;
  ipAddress?: string;
  location?: string;
  lastActiveAt: string;
  createdAt: string;
  isCurrent: boolean;
}
