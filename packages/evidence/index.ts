/**
 * packages/evidence/index.ts
 *
 * Re-exports all ZOE LA Mobile evidence types and adapters.
 *
 * Consumers should import from this barrel rather than individual modules:
 *   import { ZoeOcelEvent, GapRecord, CONNECT_GROUP_OCEL_VARIANTS } from '../evidence/index.js';
 */

export * from './OcelEvents.js';
export * from './receipts.js';
export * from './ocelSchema.js';
export * from './spineTraceAdapter.js';
export * from './argrBridge.js';
