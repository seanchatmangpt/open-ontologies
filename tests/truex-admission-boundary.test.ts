import { sha256Digest, sha256Text } from '../packages/truex/canonical';
import { RefusalState, VerificationState, Verifier } from '../packages/truex/verifier';
import { RawEvidence, Receipt } from '../packages/truex/receipt';

const raw: RawEvidence = {
  exit_code: 0,
  stdout: 'manufactured',
  filesystem_mutations: [
    { path: 'dist/result.json', hash: 'b3:artifact', operation: 'write' }
  ]
};

function admittedReceipt(): Receipt {
  return new Receipt({
    raw_evidence: raw,
    raw_evidence_hash: sha256Digest(raw),
    stdout_hash: sha256Text('manufactured'),
    observed_ocel: {
      'ocel:events': [
        {
          'ocel:id': 'event-1',
          'ocel:activity': 'ExecutionComplete_Manufacture',
          'ocel:timestamp': '2026-07-31T00:00:00.000Z',
          'ocel:vmap': {
            exit_code: 0,
            stdout_hash: sha256Text('manufactured'),
            artifact_path: 'dist/result.json',
            artifact_hash: 'b3:artifact'
          }
        }
      ]
    }
  });
}

describe('TrueX admission boundary', () => {
  it('admits a fully derived observation and emits deterministic hashes', async () => {
    const verifier = new Verifier();
    const first = await verifier.verify(admittedReceipt());
    const second = await verifier.verify(admittedReceipt());

    expect(first.state).toBe(VerificationState.Admitted);
    expect(first.evidence_hash).toBe(sha256Digest(raw));
    expect(first.ocel_hash).toBe(second.ocel_hash);
  });

  it('canonicalizes object keys before hashing', () => {
    expect(sha256Digest({ z: 1, a: 2 })).toBe(sha256Digest({ a: 2, z: 1 }));
  });

  it('refuses artifact claims without a matching filesystem mutation', async () => {
    const receipt = admittedReceipt();
    const observed = receipt.observed_ocel!;
    const result = await new Verifier().verify(
      new Receipt({
        raw_evidence: raw,
        raw_evidence_hash: sha256Digest(raw),
        observed_ocel: {
          'ocel:events': [
            {
              ...observed['ocel:events'][0],
              'ocel:vmap': {
                ...observed['ocel:events'][0]['ocel:vmap'],
                artifact_hash: 'b3:invented'
              }
            }
          ]
        }
      })
    );

    expect(result.state).toBe(VerificationState.Refused);
    expect(result.refusal_state).toBe(RefusalState.ArtifactOriginMismatch);
  });

  it('refuses temporally reversed observations', async () => {
    const result = await new Verifier().verify(
      new Receipt({
        raw_evidence: raw,
        raw_evidence_hash: sha256Digest(raw),
        observed_ocel: {
          'ocel:events': [
            {
              'ocel:activity': 'Observed_Start',
              'ocel:timestamp': '2026-07-31T00:01:00.000Z'
            },
            {
              'ocel:activity': 'ExecutionComplete_Manufacture',
              'ocel:timestamp': '2026-07-31T00:00:00.000Z',
              'ocel:vmap': { exit_code: 0 }
            }
          ]
        }
      })
    );

    expect(result.state).toBe(VerificationState.Refused);
    expect(result.refusal_state).toBe(RefusalState.TemporalOrderingViolation);
  });
});
