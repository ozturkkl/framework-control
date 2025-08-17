const API_BASE =
  (import.meta as any).env?.VITE_API_BASE || "http://127.0.0.1:8090";

export type CliEnvelope = { ok: boolean; stdout?: string; error?: string };
export type Config = any;

export async function getPower(): Promise<CliEnvelope> {
  const r = await fetch(`${API_BASE}/api/power`);
  if (!r.ok) throw new Error(`power ${r.status}`);
  return r.json();
}

export async function getThermal(): Promise<CliEnvelope> {
  const r = await fetch(`${API_BASE}/api/thermal`);
  if (!r.ok) throw new Error(`thermal ${r.status}`);
  return r.json();
}

export async function getVersions(): Promise<CliEnvelope> {
  const r = await fetch(`${API_BASE}/api/versions`);
  if (!r.ok) throw new Error(`versions ${r.status}`);
  return r.json();
}

export async function checkHealth(): Promise<boolean> {
  try {
    const r = await fetch(`${API_BASE}/api/health`, { method: "GET" });
    return r.ok;
  } catch (_) {
    return false;
  }
}

export async function getConfig(): Promise<{ ok: boolean; config: Config }> {
  const r = await fetch(`${API_BASE}/api/config`);
  if (!r.ok) throw new Error(`config ${r.status}`);
  return r.json();
}

export async function updateConfig(patch: Partial<Config>): Promise<boolean> {
  const r = await fetch(`${API_BASE}/api/config`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ config: patch }),
  });
  if (!r.ok) throw new Error(`config ${r.status}`);
  const j = await r.json();
  return !!j.ok;
}

export type SystemInfo = {
  ok: boolean;
  cpu: string;
  memory_total_mb: number;
  os: string;
  dgpu: string | null;
};
export async function getSystemInfo(): Promise<SystemInfo> {
  const r = await fetch(`${API_BASE}/api/system`);
  if (!r.ok) throw new Error(`system ${r.status}`);
  return r.json();
}
