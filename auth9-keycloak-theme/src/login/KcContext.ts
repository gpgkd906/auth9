import type { KcContext as KcContextBase } from "keycloakify/login/KcContext";

/**
 * Extended KcContext type for auth9 theme.
 * Add any custom theme properties here.
 */
export type KcContext = KcContextBase & {
  properties: {
    auth9ApiUrl?: string;
  };
};
