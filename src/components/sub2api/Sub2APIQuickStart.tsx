import { useCallback, useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AlertTriangle, CheckCircle2, ExternalLink, KeyRound, Loader2, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";

type SetupPreview = {
  site_name: string;
  api_base_url: string;
  model: string;
  reasoning_effort: string;
  wire_api: string;
};

type SetupResult = {
  configured: boolean;
  verified: boolean;
  server_confirmed: boolean;
  backup_directory: string;
  restart_required: boolean;
};

type Props = { onAdvanced: () => void };

export function Sub2APIQuickStart({ onAdvanced }: Props) {
  const { t } = useTranslation();
  const [apiKey, setApiKey] = useState("");
  const [preview, setPreview] = useState<SetupPreview | null>(null);
  const [result, setResult] = useState<SetupResult | null>(null);
  const [connected, setConnected] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const prepareTicket = useCallback(async (ticket: string) => {
    setBusy(true);
    setError("");
    setResult(null);
    try {
      setPreview(await invoke<SetupPreview>("sub2api_prepare_ticket", { ticket }));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void invoke<boolean>("sub2api_setup_status").then(setConnected).catch(() => undefined);
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<{ ticket: string }>("sub2api-setup", (event) => {
      if (!disposed) void prepareTicket(event.payload.ticket);
    }).then((dispose) => {
      if (disposed) dispose();
      else unlisten = dispose;
    });
    void invoke<string | null>("sub2api_take_queued_ticket").then((ticket) => {
      if (!disposed && ticket) void prepareTicket(ticket);
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [prepareTicket]);

  const prepareManual = async () => {
    if (!apiKey.trim()) return;
    setBusy(true);
    setError("");
    try {
      setPreview(await invoke<SetupPreview>("sub2api_prepare_manual", { apiKey }));
      setApiKey("");
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const apply = async () => {
    setBusy(true);
    setError("");
    try {
      const clientVersion = await getVersion();
      const applied = await invoke<SetupResult>("sub2api_apply_setup", { clientVersion });
      setResult(applied);
      setPreview(null);
      setConnected(true);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="min-h-screen bg-slate-950 px-5 py-8 text-slate-100">
      <div className="mx-auto flex min-h-[calc(100vh-4rem)] max-w-3xl flex-col justify-center">
        <div className="mb-8 flex items-start justify-between gap-4">
          <div>
            <div className="mb-3 inline-flex items-center gap-2 rounded-full border border-cyan-400/30 bg-cyan-400/10 px-3 py-1 text-xs font-semibold text-cyan-300">
              <ShieldCheck className="h-4 w-4" /> Sub2API Client
            </div>
            <h1 className="text-3xl font-bold tracking-tight">{t("sub2api.title")}</h1>
            <p className="mt-3 max-w-xl text-sm leading-6 text-slate-400">{t("sub2api.subtitle")}</p>
          </div>
          <button className="text-xs text-slate-500 hover:text-slate-300" onClick={onAdvanced}>{t("sub2api.advanced")}</button>
        </div>

        <section className="rounded-2xl border border-slate-800 bg-slate-900/80 p-6 shadow-2xl shadow-cyan-950/20">
          <div className="mb-5 flex items-center gap-3">
            <span className={`flex h-10 w-10 items-center justify-center rounded-full ${connected ? "bg-emerald-500/15 text-emerald-400" : "bg-slate-800 text-slate-400"}`}>
              {connected ? <CheckCircle2 className="h-5 w-5" /> : <KeyRound className="h-5 w-5" />}
            </span>
            <div>
              <h2 className="font-semibold">{connected ? t("sub2api.connected") : t("sub2api.notConnected")}</h2>
              <p className="text-xs text-slate-500">ai.tavonilo.com · Codex Responses API</p>
            </div>
          </div>

          {busy && !preview ? (
            <div className="flex items-center justify-center gap-3 rounded-xl border border-slate-800 py-12 text-sm text-slate-400">
              <Loader2 className="h-5 w-5 animate-spin" /> {t("sub2api.preparing")}
            </div>
          ) : preview ? (
            <div className="space-y-5">
              <div className="rounded-xl border border-cyan-400/20 bg-cyan-400/5 p-4 text-sm">
                <h3 className="mb-3 font-semibold text-cyan-200">{t("sub2api.confirmTitle")}</h3>
                <dl className="grid grid-cols-[8rem_1fr] gap-x-3 gap-y-2 text-slate-300">
                  <dt className="text-slate-500">{t("sub2api.site")}</dt><dd>{preview.site_name}</dd>
                  <dt className="text-slate-500">Base URL</dt><dd className="break-all font-mono text-xs">{preview.api_base_url}</dd>
                  <dt className="text-slate-500">{t("sub2api.model")}</dt><dd>{preview.model}</dd>
                  <dt className="text-slate-500">{t("sub2api.reasoning")}</dt><dd>{preview.reasoning_effort}</dd>
                </dl>
              </div>
              <p className="text-xs leading-5 text-slate-500">{t("sub2api.backupNotice")}</p>
              <div className="flex gap-3">
                <button className="flex-1 rounded-lg bg-cyan-500 px-4 py-2.5 text-sm font-semibold text-slate-950 hover:bg-cyan-400 disabled:opacity-50" disabled={busy} onClick={() => void apply()}>
                  {busy ? t("sub2api.verifying") : t("sub2api.confirm")}
                </button>
                <button className="rounded-lg border border-slate-700 px-4 py-2.5 text-sm text-slate-300 hover:bg-slate-800" disabled={busy} onClick={() => setPreview(null)}>{t("sub2api.cancel")}</button>
              </div>
            </div>
          ) : result ? (
            <div className="space-y-3">
              <div className="rounded-xl border border-emerald-400/20 bg-emerald-400/5 p-5">
                <div className="flex gap-3"><CheckCircle2 className="mt-0.5 h-6 w-6 text-emerald-400" /><div><h3 className="font-semibold text-emerald-300">{t("sub2api.success")}</h3><p className="mt-2 text-sm leading-6 text-slate-400">{t("sub2api.restart")}</p></div></div>
              </div>
              {!result.server_confirmed && (
                <div className="flex gap-3 rounded-xl border border-amber-400/20 bg-amber-400/5 p-4 text-sm text-amber-200">
                  <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0" />
                  <p>{t("sub2api.confirmPending")}</p>
                </div>
              )}
            </div>
          ) : (
            <div className="space-y-4">
              <a href="https://ai.tavonilo.com/quick-start" target="_blank" rel="noreferrer" className="flex w-full items-center justify-center gap-2 rounded-lg bg-cyan-500 px-4 py-3 text-sm font-semibold text-slate-950 hover:bg-cyan-400">
                {t("sub2api.fromWebsite")} <ExternalLink className="h-4 w-4" />
              </a>
              <div className="flex items-center gap-3 text-xs text-slate-600"><span className="h-px flex-1 bg-slate-800" />{t("sub2api.orManual")}<span className="h-px flex-1 bg-slate-800" /></div>
              <div className="flex gap-2">
                <input type="password" autoComplete="off" value={apiKey} onChange={(event) => setApiKey(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") void prepareManual(); }} placeholder={t("sub2api.keyPlaceholder")} className="min-w-0 flex-1 rounded-lg border border-slate-700 bg-slate-950 px-3 py-2.5 text-sm outline-none focus:border-cyan-500" />
                <button disabled={!apiKey.trim() || busy} onClick={() => void prepareManual()} className="rounded-lg border border-slate-700 px-4 py-2.5 text-sm font-medium hover:bg-slate-800 disabled:opacity-40">{t("sub2api.continue")}</button>
              </div>
            </div>
          )}

          {error && <p className="mt-4 rounded-lg border border-red-400/20 bg-red-400/5 p-3 text-sm text-red-300">{error}</p>}
        </section>
        <p className="mt-5 text-center text-xs text-slate-600">{t("sub2api.security")}</p>
      </div>
    </main>
  );
}
