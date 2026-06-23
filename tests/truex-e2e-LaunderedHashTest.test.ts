import { Verifier, VerificationState } from '../packages/truex/verifier';
import { Receipt } from '../packages/truex/receipt';

describe('Truex Autonomic E2E Validation: LaunderedHashTest', () => {
  /**
   * Scenario: LaunderedHashTest
   * Expected: Refused (OCELLaundering)
   */
  it('LaunderedHashTest', async () => {
    const verifier = new Verifier();
    const inputStr = `{'raw_evidence':{'exit_code':0,'stdout':'Real Output'},'observed_ocel':{'ocel:events':[{'ocel:activity':'ExecutionComplete_Checkout','ocel:vmap':{'exit_code':0}}]},'raw_evidence_hash':'fake_hash_laundering'}`.replace(/'/g, '"');
    const receiptData = JSON.parse(inputStr);
    const receipt = new Receipt(receiptData);

    const result = await verifier.verify(receipt);
    expect(result.state).toBe('Refused');
    
    expect(result.refusal_state).toBe('OCELLaundering');
    
  });
});