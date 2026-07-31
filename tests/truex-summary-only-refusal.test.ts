import { RefusalState, VerificationState, Verifier } from '../packages/truex/verifier';
import { Receipt } from '../packages/truex/receipt';

describe('TrueX refusal engine', () => {
  it('refuses summary-only proofs', async () => {
    const result = await new Verifier().verify(
      new Receipt({
        stdout_hash: 'abc',
        raw_evidence: null
      })
    );

    expect(result.state).toBe(VerificationState.Refused);
    expect(result.refusal_state).toBe(RefusalState.SummaryOnlyProof);
  });

  it('refuses a laundered raw-evidence hash', async () => {
    const result = await new Verifier().verify(
      new Receipt({
        raw_evidence: { exit_code: 0, stdout: 'Success' },
        raw_evidence_hash: 'mismatch',
        observed_ocel: {
          'ocel:events': [
            {
              'ocel:activity': 'ExecutionComplete_Fake',
              'ocel:vmap': { exit_code: 0 }
            }
          ]
        }
      })
    );

    expect(result.state).toBe(VerificationState.Refused);
    expect(result.refusal_state).toBe(RefusalState.OCELLaundering);
  });

  it('refuses an execution consequence not derivable from raw evidence', async () => {
    const result = await new Verifier().verify(
      new Receipt({
        raw_evidence: { exit_code: 1, stdout: 'error' },
        raw_evidence_hash: 'd30e046fc9cc2eaea7e3eb777035adb3cbd0b46af953c066dcf680007ce3f4a5',
        observed_ocel: {
          'ocel:events': [
            {
              'ocel:activity': 'ExecutionComplete_OA-1',
              'ocel:vmap': { exit_code: 0 }
            }
          ]
        }
      })
    );

    expect(result.state).toBe(VerificationState.Refused);
    expect(result.refusal_state).toBe(RefusalState.NonDerivableExecution);
  });
});
