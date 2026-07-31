
// Generated from ontology/zoela/*.ttl — DO NOT EDIT
// Regenerate with: ggen sync --rule zoela-receipts
//
// PROV-O receipt emitters for ZOE LA Mobile
// Source: ontology/zoela/*.ttl → extract-receipt-models.rq → prov-receipt.tera
import { blake3Hash } from '../utils/crypto';



// Completion receipt: CareRouteClosedReceipt
// Used in route: FOOD_DIST_V1
// PROV-O subclass of prov:Entity
export interface CareRouteClosedReceiptReceipt {
  receiptId: string;
  routeInstanceId: string;
  subjectId: string;
  subjectType: string;
  campusId: string;
  ministryCode: string;
  routeStageCode: string;
  evidenceIds: string[];
  actorId: string;
  issuedAt: string; // ISO 8601
  blake3Hash: string;
  ocelEventType: 'care_route.closed';
}

export function emitCareRouteClosedReceiptReceipt(
  params: Omit<CareRouteClosedReceiptReceipt, 'receiptId' | 'issuedAt' | 'blake3Hash'>
): CareRouteClosedReceiptReceipt {
  const issuedAt = new Date().toISOString();
  const payload = JSON.stringify({ ...params, issuedAt });
  return {
    ...params,
    receiptId: crypto.randomUUID(),
    issuedAt,
    blake3Hash: blake3Hash(payload),
    ocelEventType: 'care_route.closed',
  };
}


// Completion receipt: ResourceDistributedReceipt
// Used in route: FOOD_DIST_V1
// PROV-O subclass of prov:Entity
export interface ResourceDistributedReceiptReceipt {
  receiptId: string;
  routeInstanceId: string;
  subjectId: string;
  subjectType: string;
  campusId: string;
  ministryCode: string;
  routeStageCode: string;
  evidenceIds: string[];
  actorId: string;
  issuedAt: string; // ISO 8601
  blake3Hash: string;
  ocelEventType: 'food.delivered';
}

export function emitResourceDistributedReceiptReceipt(
  params: Omit<ResourceDistributedReceiptReceipt, 'receiptId' | 'issuedAt' | 'blake3Hash'>
): ResourceDistributedReceiptReceipt {
  const issuedAt = new Date().toISOString();
  const payload = JSON.stringify({ ...params, issuedAt });
  return {
    ...params,
    receiptId: crypto.randomUUID(),
    issuedAt,
    blake3Hash: blake3Hash(payload),
    ocelEventType: 'food.delivered',
  };
}


// Receipt Emitted
// A cryptographic receipt was emitted for a completed route stage, providing tamper-evident proof.
// PROV-O subclass of prov:Entity
export interface ReceiptEmittedReceipt {
  receiptId: string;
  routeInstanceId: string;
  subjectId: string;
  subjectType: string;
  campusId: string;
  ministryCode: string;
  routeStageCode: string;
  evidenceIds: string[];
  actorId: string;
  issuedAt: string; // ISO 8601
  blake3Hash: string;
  ocelEventType: '';
}

export function emitReceiptEmittedReceipt(
  params: Omit<ReceiptEmittedReceipt, 'receiptId' | 'issuedAt' | 'blake3Hash'>
): ReceiptEmittedReceipt {
  const issuedAt = new Date().toISOString();
  const payload = JSON.stringify({ ...params, issuedAt });
  return {
    ...params,
    receiptId: crypto.randomUUID(),
    issuedAt,
    blake3Hash: blake3Hash(payload),
    ocelEventType: '',
  };
}


// Receipted
// A cryptographically-signed ServiceReceipt has been emitted for this route instance, providing tamper-evident proof of service delivery in the audit chain.
// PROV-O subclass of prov:Entity
export interface ReceiptedReceipt {
  receiptId: string;
  routeInstanceId: string;
  subjectId: string;
  subjectType: string;
  campusId: string;
  ministryCode: string;
  routeStageCode: string;
  evidenceIds: string[];
  actorId: string;
  issuedAt: string; // ISO 8601
  blake3Hash: string;
  ocelEventType: '';
}

export function emitReceiptedReceipt(
  params: Omit<ReceiptedReceipt, 'receiptId' | 'issuedAt' | 'blake3Hash'>
): ReceiptedReceipt {
  const issuedAt = new Date().toISOString();
  const payload = JSON.stringify({ ...params, issuedAt });
  return {
    ...params,
    receiptId: crypto.randomUUID(),
    issuedAt,
    blake3Hash: blake3Hash(payload),
    ocelEventType: '',
  };
}

