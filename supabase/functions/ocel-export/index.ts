// Supabase Edge Function: ocel-export
// Exports OCEL 2.0 event log for wasm4pm process mining consumption
// Compatible with packages/evidence/OcelEvents.ts event type definitions

import { serve } from "https://deno.land/std@0.177.0/http/server.ts";

export interface OcelExportRequest {
  routeType?: string;      // e.g. "ConnectGroupJoinRoute"
  fromTimestamp?: string;  // ISO-8601
  toTimestamp?: string;    // ISO-8601
  objectTypes?: string[];  // e.g. ["GroupInterest", "GroupInvite"]
}

export interface OcelEvent {
  ocel_id: string;
  ocel_type: string;
  ocel_time: string;
  ocel_changed_field?: string;
  "ocel:type:GroupInterest"?: Record<string, unknown>;
  "ocel:type:GroupInvite"?: Record<string, unknown>;
  "ocel:type:GroupAttendance"?: Record<string, unknown>;
}

export interface OcelExportResult {
  "ocel:global-log": {
    "ocel:attribute-names": string[];
    "ocel:object-types": string[];
  };
  "ocel:events": OcelEvent[];
  "ocel:objects": Record<string, unknown>;
}

serve(async (req: Request) => {
  if (req.method !== "POST") {
    return new Response("Method not allowed", { status: 405 });
  }

  const _body: OcelExportRequest = await req.json();

  // In production: query Supabase ocel_events table filtered by routeType + timestamps
  // For now: return the OCEL 2.0 schema skeleton for wasm4pm ingestion
  const result: OcelExportResult = {
    "ocel:global-log": {
      "ocel:attribute-names": ["routeStageCode", "objectId", "runId", "autonomicClass"],
      "ocel:object-types": ["GroupInterest", "GroupInvite", "GroupAttendance", "GroupMembership"],
    },
    "ocel:events": [],
    "ocel:objects": {},
  };

  return new Response(JSON.stringify(result), {
    headers: { "Content-Type": "application/json" },
    status: 200,
  });
});
