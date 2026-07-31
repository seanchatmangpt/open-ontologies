/**
 * @file Care Follow-up execution harness.
 *
 * This test proves the CoreExecution boundary for completeCareFollowUp(...).
 * It does not claim native mobile-runtime closure.
 */
// @ts-nocheck

import { describe, expect, it, jest, beforeEach } from '@jest/globals';
import { createHash } from 'node:crypto';
import { writeFileSync, readFileSync } from 'node:fs';
import * as path from 'node:path';

import { completeCareFollowUp } from '../careFollowUp';

const OBSERVED_OCEL_PATH = path.resolve(__dirname, '../../../../../artifacts/zoela/care-follow-up/observed.ocel.json');

jest.mock('expo-crypto', () => ({
  CryptoDigestAlgorithm: {
    SHA256: 'SHA-256',
  },
  digestStringAsync: jest.fn(async (_algorithm: string, data: string) => {
    return createHash('sha256').update(data).digest('hex');
  }),
}), { virtual: true });

const updateMock = jest.fn();
const eqMock = jest.fn();
const selectMock = jest.fn();

const supabaseMock = {
  from: jest.fn(() => ({
    update: updateMock.mockReturnValue({
      eq: eqMock.mockResolvedValue({
        data: [
          {
            id: 'followup_001',
            care_request_id: 'care_001',
            status: 'FollowUpCompleted',
          },
        ],
        error: null,
      }),
    }),
    insert: jest.fn().mockResolvedValue({
      data: [{ id: 'event_001' }],
      error: null,
    }),
    select: selectMock,
  })),
};

describe('completeCareFollowUp CoreExecution boundary', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('transitions AwaitingFollowUp to FollowUpCompleted and emits observed OCEL', async () => {
    const result = await completeCareFollowUp({
      supabase: supabaseMock as any,
      careRequestId: 'care_001',
      followUpId: 'followup_001',
      actorId: 'actor_zoela_care_001',
      priorStatus: 'AwaitingFollowUp',
      observedOcelPath: OBSERVED_OCEL_PATH,
      now: () => '2026-05-20T00:00:00.000Z',
    });

    if (result.status === 'Failed') {
      console.error(result.error);
    }
    expect(result.status).toBe('FollowUpCompleted');
    expect(result.receiptHash).toMatch(/^[a-f0-9]{64}$/);

    const observed = JSON.parse(readFileSync(OBSERVED_OCEL_PATH, 'utf8'));

    expect(observed.events).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          activity: 'CareFollowUpCompleted',
          timestamp: '2026-05-20T00:00:00.000Z',
        }),
      ])
    );

    expect(JSON.stringify(observed)).not.toContain('simulated execution');
    expect(JSON.stringify(observed)).not.toContain('mock observed ocel');
  });
});
