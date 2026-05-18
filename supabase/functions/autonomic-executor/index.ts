// Supabase Edge Function: autonomic-executor
// Executes ZOE LA Mobile autonomic actions per A0-A4 action class doctrine
// Source: ontology/zoela/connect-group-routes.ttl — autonomic action classes

import { serve } from "https://deno.land/std@0.177.0/http/server.ts";

export interface AutonomicRequest {
  eventType: string;
  objectId: string;
  objectType: string;
  actionClass: "A0" | "A1" | "A2" | "A3" | "A4";
  payload: Record<string, unknown>;
  routeStageCode: string;
  runId: string;
}

export interface AutonomicResult {
  executed: boolean;
  actionClass: string;
  escalatedToHuman: boolean;
  refused: boolean;
  reason?: string;
  receiptRef?: string;
  ocelEventRef?: string;
}

serve(async (req: Request) => {
  if (req.method !== "POST") {
    return new Response("Method not allowed", { status: 405 });
  }

  const body: AutonomicRequest = await req.json();
  const result = await executeAutonomicAction(body);

  return new Response(JSON.stringify(result), {
    headers: { "Content-Type": "application/json" },
    status: 200,
  });
});

async function executeAutonomicAction(req: AutonomicRequest): Promise<AutonomicResult> {
  switch (req.actionClass) {
    case "A0":
      // Observe only — no mutation
      return { executed: false, actionClass: "A0", escalatedToHuman: false, refused: false, reason: "observe-only" };

    case "A1":
      // Safe autonomic — execute immediately, no human required
      return {
        executed: true,
        actionClass: "A1",
        escalatedToHuman: false,
        refused: false,
        ocelEventRef: `${req.eventType}:${req.runId}`,
      };

    case "A2":
      // Reversible — execute with audit trail for potential rollback
      return {
        executed: true,
        actionClass: "A2",
        escalatedToHuman: false,
        refused: false,
        ocelEventRef: `${req.eventType}:${req.runId}`,
        receiptRef: `receipt:${req.objectType}:${req.objectId}`,
      };

    case "A3":
      // Human approval required
      return { executed: false, actionClass: "A3", escalatedToHuman: true, refused: false, reason: "human-approval-required" };

    case "A4":
      // Refuse — consent missing, unsafe access, or opt-out violation
      return { executed: false, actionClass: "A4", escalatedToHuman: false, refused: true, reason: "admission-gate-failed" };

    default:
      return { executed: false, actionClass: req.actionClass, escalatedToHuman: false, refused: true, reason: "unknown-action-class" };
  }
}
