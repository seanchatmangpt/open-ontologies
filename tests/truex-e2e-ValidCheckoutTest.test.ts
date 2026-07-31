import { Verifier, VerificationState } from '../packages/truex/verifier';
import { Receipt } from '../packages/truex/receipt';

describe('Truex Autonomic E2E Validation: ValidCheckoutTest', () => {
  /**
   * Scenario: ValidCheckoutTest
   * Expected: Admitted
   */
  it('ValidCheckoutTest', async () => {
    const verifier = new Verifier();
    const inputStr = `{'raw_evidence':{'exit_code':0,'stdout':'Success'},'observed_ocel':{'ocel:events':[{'ocel:activity':'ExecutionComplete_Checkout','ocel:vmap':{'exit_code':0}}]},'raw_evidence_hash':'4939c1dbc64f610745c001f895854b2e5a1be33ca845b81b4b7a56a9f42a9c3d'}`.replace(/'/g, '"');
    const receiptData = JSON.parse(inputStr);
    const receipt = new Receipt(receiptData);

    const result = await verifier.verify(receipt);
    expect(result.state).toBe('Admitted');
    
  });
});