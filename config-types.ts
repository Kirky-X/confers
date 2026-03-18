// Generated TypeScript types from ConfigSchema
// This file is auto-generated - do not edit manually

export interface SimpleConfig {
  name: string;
  port: number;
  debug: boolean;
}

export interface ServerConfig {
  host: string;
  port: number;
  timeout_seconds: number;
}

export interface DatabaseConfig {
  host: string;
  port: number;
  database: string;
  max_connections: number;
}

export interface CacheConfig {
  enabled: boolean;
  ttl_seconds: number;
  max_entries: number;
}

export interface FullAppConfig {
  name: string;
  version: string;
  server: ServerConfig;
  database: DatabaseConfig;
  cache: CacheConfig;
}

export interface FeatureFlags {
  new_ui: boolean;
  beta_api: boolean;
  realtime_notification: boolean;
}

// Usage example:
// import type { FullAppConfig } from './types';
//
// const config: FullAppConfig = await fetchConfig();
//
// console.log(config.server.host);
