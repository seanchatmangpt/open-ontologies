import { Verifier, VerificationState } from '../packages/truex/verifier';
import { Receipt } from '../packages/truex/receipt';

describe('Truex Autonomic E2E Validation: MissingRawEvidenceTest', () => {
  /**
   * Scenario: MissingRawEvidenceTest
   * Expected: Refused (SummaryOnlyProof)
   */
  it('MissingRawEvidenceTest', async () => {
    const verifier = new Verifier();
    const inputStr = `{'raw_evidence':null,'stdout_hash':'abc'}`.replace(/'/g, '"');
    const receiptData = JSON.parse(inputStr);
    const receipt = new Receipt(receiptData);

    const result = await verifier.verify(receipt);
    expect(result.state).toBe('Refused');
    
    expect(result.refusal_state).toBe('SummaryOnlyProof');
    
  });
});