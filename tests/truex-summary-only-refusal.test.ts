import { Verifier } from '../packages/truex/verifier';
import { Receipt } from '../packages/truex/receipt';

describe('Truex Refusal Engine', () => {// Failure Taxonomy from Law Surface:
  
  // BoundaryProjectionFailure: Failed to project raw boundary evidence into a valid OCEL structure.
  // TemporalOrderingViolation: Observed event sequence violates the causal/temporal laws defined in the expected path.
  // ArtifactOriginMismatch: Emitted artifact hash or path does not match the derivation origin in the boundary evidence.
  // StateTransitionMismatch: Observed object state transitions do not match the expected state graph mutations.
  // NonDerivableExecution: The claimed execution path cannot be physically derived from raw boundary evidence.
  // MissingBoundary: Absence of required physical execution evidence.
  // CloneTrace: Using expected OCEL as observed evidence (cloning).
  // SummaryOnlyProof: Attempting closure with only high-level hashes (stdout_hash) without raw evidence.
  // OCELLaundering: Formatting OCEL from summary data without raw boundary derivation.

  it('should refuse summary-only proofs', async () => {
    const verifier = new Verifier();
    const summaryOnlyReceipt = new Receipt({
      stdout_hash: 'abc',
      raw_evidence: null // Missing raw boundary
    });

    const result = await verifier.verify(summaryOnlyReceipt);
    expect(result.state).toBe('Refused');
    expect(result.refusal_state).toBe('SummaryOnlyProof');
  });

  it('should refuse OCEL laundering (fake paths)', async () => {
    const verifier = new Verifier();
    const launderedReceipt = new Receipt({
      observed_ocel: { /* summary-formatted data */ },
      raw_evidence_hash: 'mismatch' 
    });

    const result = await verifier.verify(launderedReceipt);
    expect(result.state).toBe('Refused');
    expect(result.refusal_state).toBe('OCELLaundering');
  });

  it('should refuse OCEL not derivable from raw evidence', async () => {
    const verifier = new Verifier();
    const nonDerivableReceipt = new Receipt({
      raw_evidence: { exit_code: 1, stdout: 'error' },
      observed_ocel: {
        'ocel:events': [{
          'ocel:activity': 'ExecutionComplete_OA-1',
          'ocel:vmap': { exit_code: 0 } // Mismatch: OCEL says success, Raw says fail
        }]
      }
    });

    const result = await verifier.verify(nonDerivableReceipt);
    expect(result.state).toBe('Refused');
    expect(result.refusal_state).toBe('OCELLaundering');
  });
});
