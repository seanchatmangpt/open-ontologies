// Supabase Edge Function: ocel-export
// Exports OCEL 2.0 event log for wasm4pm process mining consumption
// Compatible with packages/evidence/OcelEvents.ts event type definitions

import { serve } from "https://deno.land/std@0.177.0/http/server.ts";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

interface OcelRelationship {
  objectId: string;
  qualifier: string;
}

interface OcelEvent {
  "ocel:id": string;
  "ocel:type": string;
  "ocel:time": string;
  "ocel:attributes": Record<string, unknown>;
  "ocel:relationships": OcelRelationship[];
}

interface OcelExportResult {
  "ocel:global-event-types": string[];
  "ocel:events": OcelEvent[];
  "ocel:objects": Record<string, unknown>[];
}

// Row shape returned from the ocel_events table
interface OcelEventRow {
  id: string;
  event_type: string;
  object_id: string;
  object_type: string;
  route_id: string;
  stage_code: string;
  action_class: string;
  run_id: string;
  ts_ns: number;
  fields: Record<string, unknown>;
  receipt_hash: string | null;
  created_at: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Row → OCEL 2.0 event
// ─────────────────────────────────────────────────────────────────────────────

function rowToOcelEvent(row: OcelEventRow): OcelEvent {
  // ts_ns is nanoseconds; Date constructor takes milliseconds
  const ts = new Date(row.ts_ns / 1_000_000).toISOString();

  return {
    "ocel:id": row.id,
    "ocel:type": row.event_type,
    "ocel:time": ts,
    "ocel:attributes": {
      stage_code: row.stage_code,
      action_class: row.action_class,
      route_id: row.route_id,
      run_id: row.run_id,
      ...(row.receipt_hash ? { receipt_hash: row.receipt_hash } : {}),
      ...row.fields,
    },
    "ocel:relationships": [
      { objectId: row.object_id, qualifier: "responsible" },
    ],
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Handler
// ─────────────────────────────────────────────────────────────────────────────

serve(async (req: Request): Promise<Response> => {
  if (req.method !== "GET" && req.method !== "POST") {
    return new Response("Method not allowed", { status: 405 });
  }

  // Accept run_id via query param (GET) or JSON body (POST)
  let runId: string | null = null;
  let routeId: string | null = null;
  let fromTs: string | null = null;
  let toTs: string | null = null;

  const url = new URL(req.url);
  runId = url.searchParams.get("run_id");
  routeId = url.searchParams.get("route_id");
  fromTs = url.searchParams.get("from");
  toTs = url.searchParams.get("to");

  if (req.method === "POST") {
    try {
      const body = await req.json();
      runId = runId ?? body.run_id ?? null;
      routeId = routeId ?? body.route_id ?? null;
      fromTs = fromTs ?? body.from ?? null;
      toTs = toTs ?? body.to ?? null;
    } catch {
      // ignore parse errors — params already set from query string
    }
  }

  // Build query against ocel_events via Supabase PostgREST
  // SUPABASE_URL and SUPABASE_SERVICE_ROLE_KEY are injected by the runtime
  const supabaseUrl = Deno.env.get("SUPABASE_URL") ?? "http://localhost:54321";
  const serviceKey = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY") ?? "";

  let queryUrl = `${supabaseUrl}/rest/v1/ocel_events?select=*&order=ts_ns.asc`;

  if (runId) {
    queryUrl += `&run_id=eq.${encodeURIComponent(runId)}`;
  }
  if (routeId) {
    queryUrl += `&route_id=eq.${encodeURIComponent(routeId)}`;
  }
  if (fromTs) {
    // fromTs is ISO-8601; compare against created_at
    queryUrl += `&created_at=gte.${encodeURIComponent(fromTs)}`;
  }
  if (toTs) {
    queryUrl += `&created_at=lte.${encodeURIComponent(toTs)}`;
  }

  const pgResp = await fetch(queryUrl, {
    headers: {
      "apikey": serviceKey,
      "Authorization": `Bearer ${serviceKey}`,
      "Accept": "application/json",
    },
  });

  if (!pgResp.ok) {
    const errText = await pgResp.text();
    return new Response(
      JSON.stringify({ error: "upstream query failed", detail: errText }),
      { status: 502, headers: { "Content-Type": "application/json" } },
    );
  }

  const rows: OcelEventRow[] = await pgResp.json();

  // Collect unique event types for global-event-types
  const eventTypeSet = new Set<string>(rows.map((r) => r.event_type));

  const result: OcelExportResult = {
    "ocel:global-event-types": Array.from(eventTypeSet).sort(),
    "ocel:events": rows.map(rowToOcelEvent),
    "ocel:objects": [],
  };

  return new Response(JSON.stringify(result, null, 2), {
    headers: { "Content-Type": "application/json" },
    status: 200,
  });
});
