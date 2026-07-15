import { getAuthenticationInfo, restoreOfficialAuthenticationConfig, saveCustomAuthentication } from "./backend";

export const OFFICIAL_RENDERNORTH_CLIENT_ID = "f6321a78ea0e4ed78fc52ab2ba85d502";
export const CUSTOM_CLIENT_ID_SETTING = "esi_custom_client_id";

export type AuthenticationMode = "official" | "custom";

export interface AuthenticationConfig {
  mode: AuthenticationMode;
  clientId: string;
  applicationName: string;
}

export async function getAuthenticationConfig(): Promise<AuthenticationConfig> {
  return getAuthenticationInfo();
}

export async function saveCustomClientId(clientId: string): Promise<AuthenticationConfig> {
  return saveCustomAuthentication(clientId);
}

export async function restoreOfficialAuthentication(): Promise<AuthenticationConfig> {
  return restoreOfficialAuthenticationConfig();
}
