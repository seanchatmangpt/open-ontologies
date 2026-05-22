import { Verifier, VerificationState } from '../packages/truex/verifier';
import { Receipt } from '../packages/truex/receipt';
import { ReplayEngine } from '../packages/truex/replay';
import * as fs from 'fs';

async function main() {
  const args = process.argv.slice(2);
  const command = args[0];

  if (command === 'verify') {
    const receiptPath = args[1];
    if (!receiptPath) {
      console.error('Usage: truex verify <receipt_path>');
      process.exit(1);
    }

    const data = JSON.parse(fs.readFileSync(receiptPath, 'utf8'));
    const receipt = new Receipt(data);
    const verifier = new Verifier();
    const result = await verifier.verify(receipt);

    console.log(JSON.stringify(result, null, 2));

    if (result.state === VerificationState.Admitted) {
      process.exit(0);
    } else {
      process.exit(1);
    }
  } else if (command === 'replay') {
    const receiptPath = args[1];
    if (!receiptPath) {
      console.error('Usage: truex replay <receipt_path>');
      process.exit(1);
    }

    const data = JSON.parse(fs.readFileSync(receiptPath, 'utf8'));
    const receipt = new Receipt(data);
    const engine = new ReplayEngine();
    const trail = engine.generateAuditTrail(receipt);

    console.log(trail);
    process.exit(0);
  } else {
    console.error(`Unknown command: ${command}`);
    process.exit(1);
  }
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
